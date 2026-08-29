# FortuneT V2 程式碼稽核報告

**日期**:2026-08-29
**範圍**:整個 Cargo workspace(`crates/`)、CI、部署腳本、`docs/`
**基準 commit**:`6e726ea`
**結論**:架構與授權層健康,但**認證層完全失效**,且該缺陷在文件中被記錄為「已通過」。

---

## 1. 專案現況

FortuneT V2 是 AI 命理平台(紫微斗數 + 西洋占星 + Big5 人格測驗),**全 Rust 單一技術棧**部署於 Cloudflare。

| Crate | 行數 | 職責 |
| --- | --- | --- |
| `crates/api` | 3,241 | `fortunet-api` Worker:auth / users / charts / personality 路由、SessionDO、AIMutexDO、D1 |
| `crates/web` | 1,707 | Leptos CSR 前端(已一比一取代原 React) |
| `crates/schema` | 939 | 共用 DTO,消除 TS↔Rust 漂移 |
| `crates/domain/ziwei` | 303 | 紫微引擎(包裝 x-iztro) |
| `crates/domain/big5` | 296 | IPIP-15 計分、常模、careless detection |
| `crates/domain/western` | 111 | 西洋占星(solar-ephemeris 全天體,geocentric apparent) |
| `crates/worker` | 110 | `fortunet-engine`,經 service binding 提供兩條計算端點 |

git log 顯示這是一次完成度很高的 TS→Rust 遷移(Phase A→D),近期工作集中在 Big5 F1 切片。

### 做得好的部分

這些在稽核中確認無誤,列出以免後續重構誤傷:

- **授權層一致**:`routes/mod.rs` 中五組資料路由(`users`、`charts`、`personality`)全部先過 `common::auth_user`,且所有 D1 查詢都以 session 帶出的 `user_id` 作為 scope。沒有 IDOR。
- **SQL 全參數化**:未發現任何字串拼接查詢。
- **CORS 用精確主機名比對**:`lib.rs::resolve_origin` 以 `web_sys::Url` 解析後比對 hostname,而非 substring,`https://notlocalhost.attacker.com` 會被正確拒絕。
- **快取失效邏輯嚴謹**:`common::extracted_version` 刻意不回退到頂層 `engineVersion`,註解說明了為何回退會讓修正前的錯誤命盤永久快取。
- **DO 儲存格式相容**:`ft_schema::storage` 保住了與舊 JS 版的 bit 相容,遷移沒有丟資料。

---

## 2. 稽核發現

### P0-01 — 登入端點沒有任何身分驗證

**位置**:`crates/api/src/routes/auth.rs:71-100`、`crates/api/src/routes/auth.rs:150-153`

`/api/auth/login` 的請求 body 只有一個欄位:

```rust
#[derive(serde::Deserialize)]
struct LoginBody {
    #[serde(default)]
    email: Option<String>,
}
```

流程為:接收 email → 查 `users` 表 → 直接呼叫 `create_session` 發出 session token。過程中**沒有密碼、沒有 magic link、沒有 OTP、沒有任何 email 所有權證明**。`scripts/schema.sql` 的 `users` 表也確認沒有任何憑證欄位。

**影響**:知道任一使用者的 email 即可完整接管該帳號。可存取的資料包含:

- `users`:出生年、月、日、時、分、性別、經緯度、時區
- `personality_profiles`:Big5 人格側寫與原始作答
- `interpretations`:全部 AI 命盤解讀

出生時間與地點、心理測量結果皆屬敏感個資,且服務正在 production 對外提供。

**加重因素**:`docs/security-checklist.md` 將此設計記錄為安全優點:

```
- [x] Passwordless auth (email-only)
- [x] No passwords stored (passwordless auth)
7. **Auth Failures** - ✅ Session-based, rate limited
```

業界所稱 passwordless(magic link / OTP / WebAuthn)一律包含「以其他方式證明 email 所有權」。此處缺少的正是該步驟。文件把驗證繞過標記為通過,使問題無法在後續 review 中被重新檢視 —— 這比缺陷本身更需要修正。

**建議解法**(magic link,最貼近現有敘事、改動最小):

1. 新增 D1 表 `login_tokens(token_hash TEXT PRIMARY KEY, email TEXT NOT NULL, expires_at TEXT NOT NULL, used_at TEXT)`。token 以 `crypto.getRandomValues` 產生 32 bytes,資料庫只存 SHA-256 hash,TTL 10 分鐘,單次使用。
2. `/api/auth/login` 改為寄出驗證信並回 `202`。**不論 email 是否存在都回傳相同內容**,避免帳號枚舉。
3. 新增 `/api/auth/verify`,驗證 token 後才呼叫既有的 `create_session`(該函式本身無需改動)。
4. 寄信管道:Cloudflare Email Routing,或 Resend / Postmark(一個 `fetch` 即可)。
5. `/api/auth/register` 同步改為需通過信件驗證才建立帳號。

**過渡措施**:若無法立即接上寄信服務,應先將服務下線或加上 IP allowlist,不要讓現況持續曝露。

---

### P1-01 — CI 從未執行測試

**位置**:`.github/workflows/deploy.yml`

workflow 僅有三步:`cargo fmt --check`、`cargo clippy`、`cargo build`。專案內有 **26 個 `#[test]`**,分布於 `ft-big5`(計分、careless detection)、`ft-ziwei`、`ft-western`、`ft-schema`,**全部未被執行**。

西洋占星的上升點公式曾經算錯(commit `11ba0b4` — *"correct ascendant formula — was returning the descendant"*),正是單元測試應該攔下的類型。

**解法**:domain 與 schema crate 可原生測試,不需 wasm target。於 workflow 加入:

```yaml
      - name: test (native)
        run: cargo test -p ft-schema -p ft-ziwei -p ft-western -p ft-big5
```

---

### P1-02 — `Cargo.lock` 未納入版控

**位置**:`.gitignore:41`

`.gitignore` 排除了 `Cargo.lock`,`git ls-files Cargo.lock` 確認為未追蹤。

此 workspace 產出的是**應用程式而非函式庫**(`ft-api` / `ft-worker` 為 `cdylib` 部署產物),且 `ft-western` 依賴 `solar-ephemeris`(經 vendor patch 走 geocentric apparent 路徑;`vsop87` 已確認**未使用** — 見 `crates/domain/western/src/lib.rs:1-5`,其 heliocentric J2000 正是 Phase A 被 §8.2 事件表抓到的舊 bug)。依賴靜默升版可能改變命盤輸出,而缺少 lockfile 將無法重現舊版行為或定位回歸來源。

**解法**:自 `.gitignore` 移除該行,提交 `Cargo.lock`。此為 Rust 官方對 binary crate 的建議。配套:CI 與 `build-web.sh` / `deploy-engine.sh` 改用 `--locked`,真正禁止隱性重解依賴。

---

### P2-01 — `panic = "abort"` 搭配 28 處非測試 `unwrap` / `expect`

**位置**:`Cargo.toml`(`panic = "abort"`);非測試程式碼共 28 處 `unwrap()` / `expect()` / `panic!()`(含測試碼則 47 處),集中於 `error.rs` response builders、各路由的 `ok_json`、`Mutex::lock().unwrap()`、`personality.rs` 的 serde 序列化 `expect`

Worker 全域設定 `panic = "abort"`,panic 會直接 abort isolate,使用者收到沒有堆疊資訊的 500。

**性質分類**(核實後修正):28 處多數屬於「理論上不會失敗」的不變式捷徑 — 例如 `common.rs:232` 的 `get_mut().unwrap()` 緊接同一 `&mut self` 上成功的 `get()`;`ai_mutex_do.rs:246` 的 `unwrap` 有短路 `is_none()` 保護;`personality.rs:63` 解的是 Mutex 鎖而非外部資料。真實風險是**鎖中毒**與 response builder 對 `Response::from_json` 的假設,而非外部輸入。

**解法**:逐類處理 — response builders 失敗時回退至 const 靜態錯誤 response;`lock().unwrap()` 統一改為容忍中毒或映射 500;可由型別證明的不變式保留但加上 `expect` 理由說明。

---

### P2-02 — Rate limiter 在 Workers 上實質無效

**位置**:`crates/api/src/routes/common.rs:204-243`

limiter 是 isolate 層級的 `OnceLock<Mutex<HashMap>>`。原始碼註解正確指出並修正了「每請求重建 limiter」的舊 bug,但仍有結構性限制:Cloudflare 會在全球啟動大量 isolate,且冷啟動即歸零。

因此 auth 的「10 req/min」實際語意是「**每個 isolate** 10 req/min」,無法阻擋真實濫用。另兩個核實追加的缺陷:

1. **auth 與 personality 共用同一個 bucket**(`personality.rs:61` 也用 `limiter()`、limit 同為 10)— 打 personality 測驗端點會吃掉 auth 的限流額度。
2. **charts 的 AI limiter 仍是 per-request**:`charts.rs:76` 在 `register()` 內 `Arc::new(...)`,而 `lib.rs:37` 每次 fetch 都重建 router — 這正是 baicodex F2 修掉 auth limiter 的同一個 bug,在 AI 端點上仍然存在(10 req/min 形同虛設)。

**解法**:限流移入 Durable Object(依 IP/識別碼分片,避免單一 global DO 瓶頸)或 Cloudflare Rate Limiting rules;auth、email(magic link)、endpoint 各自獨立 bucket;charts 的 AI limiter 一併收編。

---

### P2-03 — CORS 對 `*.workers.dev` / `*.pages.dev` 開放並允許 credentials

**位置**:`crates/api/src/lib.rs:129-146`

`resolve_origin` 允許任何以 `.workers.dev` 或 `.pages.dev` 結尾的主機,並在 `decorate` 中一併回傳 `Access-Control-Allow-Credentials: true`。任何人都可以免費部署 `evil.workers.dev` 取得此信任。

目前實際衝擊有限(session 存於 localStorage 並以 Bearer header 傳送,非 cookie,瀏覽器不會自動附帶),但這道防線等同虛設。

**解法**:改為明確白名單 —— production origin 加 localhost;pages.dev 的 preview 部署以環境變數列舉。

---

### P3-01 — 環境雜物與文件債

| 項目 | 現況 | 處置 |
| --- | --- | --- |
| `backend/`、`frontend/` | 549MB `node_modules`,對應 TS 原始碼已於 `98d3521` 刪除;未被 git 追蹤 | `rm -rf backend frontend` |
| `docs/gap-analysis-*.md` | 8 份檔案共 1,315 行:`FINAL`、`HONEST`、`correction-2`、`corroboration`、`summary`、`quick-ref`、`visual`、`index` | 合併為單一 `docs/gap-analysis.md`,其餘刪除 |
| 根目錄 `personality-*.png` | 5 張 E2E 截圖,未追蹤 | 加入 `.gitignore` 或刪除 |
| `target/` | 3.4GB | `cargo clean` |

關於 gap-analysis:當一份文件需要另一份名為 `HONEST` 的檔案來更正時,兩份的可信度都已失效。這組檔案的存在本身就是需要收斂的訊號。

---

## 3. 建議執行順序

1. **立即** —— 處理 P0-01。決定採用 magic link 或先行下線。這是唯一正在持續曝露的缺陷。
2. **順手** —— P1-01 與 P1-02(CI 加一步、`.gitignore` 移一行)。兩行改動,永久收益。
3. **接著** —— 改寫 `docs/security-checklist.md`,將 passwordless 三行改為誠實狀態。否則下一輪 review 仍會被其誤導。
4. **之後** —— P2-01 / P2-02 / P2-03 可打包為單一 commit。
5. **有空** —— P3-01 環境與文件清理。

---

## 4. 外部核實修訂(2026-08-29)

本報告經 Codex 對抗性核實(委派任務 `task-mte556ke-0hlb04`,runtime log 見
`~/.claude/plugins/data/grok-build-xai-grok-build/state/ft-ca343078d5a67b61/jobs/task-mte556ke-0hlb04.log`),
裁決:**P0-01、P1-01、P1-02、P2-02、P3-01 = CONFIRMED;P2-01、P2-03 = PARTIAL**;
所有裁決已由委派方逐條對照原始碼二次驗證屬實。前述 §2 各條已依核實結果修訂。
核實過程另外追加以下發現:

### 追加發現(納入修復範圍)

| # | 發現 | 位置 | 處置 |
| --- | --- | --- | --- |
| A-01 | `random_uuid()` 在 `globalThis.crypto` 不可用時**fail-open** 退回 `Math::random()` — 用於 session id,不可為認證 token 所沿用 | `crates/api/src/services/uuid.rs:20-38` | magic link token 產生必須 fail-closed(crypto 不可用即拒絕);`fallback_uuid` 移除或改 panic |
| A-02 | P0 修復上線時,既有被冒領的 7 日 session 仍有效 — 缺少撤銷機制 | `SessionDO` / `auth_user` | session 帶上 `created_at`;全域 epoch(舊 session 一律無此欄位 → 視為過期)一次撤銷所有修復前 session |
| A-03 | register 409「User already exists」/ login 404「User not found」構成帳號枚舉 | `auth.rs` | magic link 化後,login/register 對存在與否回應一致 |
| A-04 | `Access-Control-Allow-Credentials: true` 在無 cookie 的 Bearer 模型下毫無必要,應移除 | `lib.rs::decorate` | CORS 白名單化時一併移除 credentials header |
| A-05 | gap-analysis 文件互相矛盾(`HONEST.md` 宣稱 18 tests 已驗證,`correction-2.md` 隨即承認未重新驗證),且 `doc-code-gap-analysis.md` 是 index 引用的第 9 份同類文件 | `docs/` | 清理時先分類「歷史紀錄 vs 應刪」,不盲目合併矛盾內容;以單一 canonical 文件取代 |
| A-06 | `backend/`、`frontend/` 內含 `.wrangler/` 本地狀態,刪除前需確認無需保留 | `backend/.wrangler`、`frontend/.wrangler` | 刪除清單確認時一併檢視 |

### 核實方法備註

- Grok 先行委派因 402 Payment Required 額度用盡未產出;Kimi 因 403 每週配額未產出;
  Codex 完整跑完。三家委派 prompt 均為同一份 7-finding 對抗性核實契約。
- Codex 對原報告的兩處糾正已接受並修訂:§P1-02 的 vsop87 描述錯誤、§P2-01 的
  「外部資料」點名失準(`personality.rs:63` 是鎖、`ai_mutex_do.rs:246/259` 有短路保護、
  `common.rs:232` 是邏輯保護的不變式)。
- `scripts/verify-big5.sh:12-14` 的活體證據性質修正:它打的是 `/register` 而非 `/login`,
  證明的是「註冊僅憑 email 即取得 session」;login 路徑(`auth.rs:74-100`)由程式碼直接證明,
  兩者共用同一個 `create_session`。

---

## 5. 修復後對抗性 review(同日第二輪)

修復工作流完成後,獨立 review agent 對整個 working-tree diff(+946/-366,22 檔修改 +
3 新檔)做 read-only 對抗式審查,提出 3 個高信心發現,已全部 corroborate 屬實並處置:

| # | 發現 | 處置 |
| --- | --- | --- |
| F1 | TTL 雙源:`auth.rs` 10 分 vs `login_token.rs` 15 分,信件文案取 15 — 第 10–15 分鐘點信必 401 | **已修**:`login_token::TOKEN_TTL_MS = 10 分` 成唯一來源,routes 與信件文案都讀它;死碼時間 helper 一併刪除 |
| F2 | 防枚舉被 timing 側通道削弱:未知地址 SELECT 後即回,已知地址多跑 Resend HTTPS(數百 ms) | **部分緩解**:兩路徑均為 1 SELECT + 1 INSERT 才回應;殘餘(Resend 延遲)已記錄於 security-checklist — worker 0.8.5 的 `RouteContext` 無 `wait_until`,升級路徑為 vendor patch 或 queue 寄信 |
| F3 | `RateLimitDO` 計數器只寫不刪,DO storage 無界成長(攻擊者可用唯一 email/IP 灌大) | **已修**:DO alarm 背景清理 — `check` 發現無 alarm 就武裝 1 秒後的 sweep,`alarm()` 列出 `rl:` 前綴、刪除過期計數;穩態每請求僅多一次 `get_alarm` 呼叫 |

Review 的風險備註同時處置:register 改為 **verify 時才建帳**(`pending_full_name` 隨
token 暫存,清單面:資料污染不再可能);CI 測試步驟加 `-p ft-api`(token hash 測試
原本在 CI 永遠不跑);email 限流 bucket 改小寫(防大小寫繞過 5/min);寄信失敗原因
進 worker log(`EmailError` 實作 `Display`)。未處置(記錄在案):engine worker 建置
仍 unlocked(`worker-build` 無 `--locked` passthrough);`login:email` 殘餘 timing 側
通道見 F2。

最終驗證:`cargo fmt` + `cargo test --locked`(ft-schema 8 / ft-ziwei 3 / ft-western 3 /
ft-big5 12 / ft-api 3,全過)+ 三個 wasm check 全綠,警告維持既有基線 11。
