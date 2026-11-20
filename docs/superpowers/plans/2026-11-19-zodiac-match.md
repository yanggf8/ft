# 文字 AI 全面改走 Workers AI ＋ 星座配對 — 實作計劃 v3

日期：2026-11-19　狀態：**v3.1（D3 已裁示；Neon 一審 14 條＋Argon 二審 8 條已納），**已核准（2026-11-20 站長：D1／D2／D4／D5 全照建議，開工；實作審查＝Argon 一審＋Gemini 3.8 Flash 二審，commit 註明替代）**。審查替代：Codex（至 11/25）、Neon／Hydrogen（約 5 天）、Kimi（週額度）皆用盡，二審由 **Gemini 4 Argon High** 代替。

## 0. 站長裁示

- 2026-11-19：zodiac1 的缺口依序補；先做**星座配對**（問題標籤、財運月曆排後）。zodiac1 是 Render 上的 Python 舊專案，只參考功能，不搬程式、不呼叫它。
- 對方輸入＝**點選星座**（12 宮格點一個，不收對方生日；生日合盤留待日後）。
- 開放範圍＝**免登入可玩規則版**；登入且有出生資料者自動帶入太陽／月亮／上升，並可產生 AI 解讀。
- **全部文字 AI 改走 Cloudflare Workers AI**（紫微／西洋／人生故事／塔羅／配對），並挑可用的新模型。

## 1. 本機實測（2026-11-19；腳本與輸出在 session scratchpad `aitest/`、`promptgen/`）

### 1.1 現行供應商

| 環 | 模型 | 結果 |
|---|---|---|
| 主 | Groq `openai/gpt-oss-20b`（`ai_mutex_do.rs model_for`） | 配對 prompt：2s，但 `max_tokens` 1500 下**截斷**、把對方星座寫錯 |
| 備 | Cerebras `gpt-oss-120b` | **HTTP 402 payment_required — 線上備援已失效** |

### 1.2 Workers AI 模型篩選（41 個文字模型）

- **2026-11-20 下架**（排除）：kimi-k2.6、kimi-k2.7-code、deepseek-v4-flash/pro、glm-5.2、glm-5.3-flash、nemotron-3-120b、qwen3-30b、qwq-32b、deepseek-r1-distill、llama-3.2-1b/3b、llama-3.1-8b 等。
- **免費方案 403**：deepseek-v4.1-flash、glm-5.3、inkling-2-small。
- **非聊天模型**：cloudflare clairic*、clef*（結構化工具，要 `fields`／`questions`）。
- **逾時／空回**（推理吃光輸出）：qwen3.8-27b、glm-4.7-flash。
- **品質不合格**：gpt-oss-20b（亂編相位）、gemma-sea-lion-v4（假裝知道對方月亮）、mistral-small-3.1（空泛）。

### 1.3 決選：用**現行真實 prompt**（`prompts.rs` 複製產生；紫微、人生故事）＋配對 prompt

| 設定 | 配對 | 紫微 | 人生故事 | 輸出 tokens（紫微／故事） | 結論 |
|---|---|---|---|---|---|
| **`@cf/google/gemma-4-26b-a4b-it`＋`chat_template_kwargs.enable_thinking=false`** | 13s | 34s | 30s | 1415／1197 | **主環**：品質佳、護欄語氣主動、0 簡體 |
| `@cf/meta/llama-3.3-70b-instruct-fp8-fast` | 11s | 22s | 26s | 1236／1628 | **備環**：快、正確；但 neurons 單價高、偶有大陸用語 |
| gpt-oss-120b `reasoning.effort=low` | 23s | **83s** | 59s | 4037／1381 | 不選（紫微太慢） |
| gpt-oss-120b 預設 | 31s | **132s** | 80s | 5029／2849 | 不選（超過 100s） |
| gemma-4 預設（思考開） | 46s | 83s | 67s | 2781／2841 | 不選 |

> 注意：v2 依短 prompt 選的 gpt-oss-120b 在長 prompt 上不可行——決選必須用真實 prompt。

## 2. 設計

### 2.1 階段 0：文字 AI 鏈改 Workers AI（全站）

**鏈**：`TEXT_CHAIN = [gemma-4-26b-a4b-it（不思考）, llama-3.3-70b-instruct-fp8-fast]`。

**providers.rs**
- 保留 palm 專用 `call_workers_ai`（45s、帶圖、不動契約）。
- 新增 `call_workers_ai_text(ai, model, system, user, max_tokens, timeout_ms)`；純函式 `build_text_payload(model, …)` 依模型組 `messages`＋（gemma 時）`chat_template_kwargs:{enable_thinking:false}`；`run_with_timeout` 參數化 `timeout_ms`。
- `parse_workers_ai_content` 補 Responses 形（`output[]` 只取 `type=="message"` 下 `output_text`，跳過 `reasoning`；容忍 `/result/` 前綴）——即使目前鏈不用 gpt-oss，也防日後換模型外洩英文思考；`parse_workers_ai_finish_reason` 補 `status=="incomplete"`。三種 fixture 單元測試。
- 移除 `call_provider`、`GROQ_BASE`、`CEREBRAS_BASE` 與其測試；`services/ai/mod.rs` 匯出同步。

**ai_mutex_do.rs**
- `handle_request` 文字分支改跑 `TEXT_CHAIN`；`metric:` 補 `model=`；`ai_provider` 寫 `workers-ai`、`ai_model` 寫模型名。
- 刪除 `RPM_LIMITS`／`RPD_LIMITS`／`rpd_blocked`／`model_for`（避免 clippy dead code 撞 CI 硬閘）；RPM 自我節流改 `TEXT_RPM`（以 `"workers-ai-text"` 為 key；註明 DO 儲存是 per-instance：`global`／`tarot`／`palm`／`zodiac-match` 各自獨立）。
- `max_tokens_for` 改依 `(model, chart_type)`：gemma story 3072、其他 2048；llama story 2048、其他 1536（實測輸出 1.2–1.6k）；`palm_chain_and_max_tokens_mapping` 測試同步改。
- **截斷**：文字鏈 `length`／`incomplete` → 換下一環（palm 的 `is_truncated` → 500 `AI_TRUNCATED` 契約不變）。
- **時間預算**（Cloudflare 對代理請求 100s 524）：DO 收到請求即訂 `deadline = now + 88s`；`MAX_QUEUE_WAIT_MS` 降為 25s；每環逾時＝`min(環上限, deadline − now)`，主環上限 55s、剩不到 15s 不進備環。
- **額度**：新增 `is_daily_quota_error(msg)`：比對 `3036`、`4006`、`daily free allocation`、`10,000 neurons`；命中時**立即中斷、不走 stub**，回 503 `{"code":"AI_QUOTA"}`。`3040`（容量暫滿）與其他 429 照常換環。`handle_palm` 同樣處理。
- stub：全環失敗仍回既有 stub 200（紫微／西洋／故事原行為），**但 `charts.rs` 不得把 stub 寫入 `interpretations` 快取**（見下）。

**呼叫端**
- `routes/charts.rs`：`anything_configured` 改檢 `ctx.env.ai("AI").is_ok()`；移除 `secret_or` 與兩處 `keys`；`call_ai_mutex` 解析 DO 503 的 `code`，以 `error::error_code` 轉發 `AI_QUOTA`／`AI_UNAVAILABLE`；`stub:true` 不寫快取、照常回給前端（標示離線範本）。
- `routes/tarot.rs`：移除 `keys`；`AI_QUOTA` → 直接回 503 `AI_QUOTA`、不覆寫 DB，並改用可退次的限流（比照 palm 的 `rate_limit_strict_refundable`，實作時確認名稱）。
- `routes/palm.rs`：`forward_ai_error` 白名單加 `AI_QUOTA`＋測試。
- `crates/schema/src/storage.rs`：`AIMUTEX_PROVIDERS` 改 `["workers-ai"]`＋測試。

**前端**
- `crates/web/src/api.rs`：AI 端點加 90s client timeout（`AbortController`）；`ApiErr` 的 HTTP 524／HTML／Network 轉友善中文，**不得直印 `e.to_string()`**（`divination.rs:114`、`story.rs:438` 現況會印整頁 HTML）。
- `divination.rs`／`story.rs`／`tarot.rs`／`palm.rs`：等待文案「約需半分鐘」；`AI_QUOTA` 顯示「今日 AI 解讀額度已滿，明天再試」（配對頁另加「規則版仍可用」）。
- `story.rs render_markdown`／`parse_blocks`：清單支援 `* `（gemma 用 `*   ` 開頭）。

**額度估算（以官方 neurons 單價；實作後以 dashboard 校正）**
- gemma-4：輸出約 27,273 neurons／1M tokens（輸入單價實作時查證）→ 紫微約 40–50、故事約 35–45 neurons／次。
- llama-3.3-70b：輸出約 204,805／1M → 紫微約 260、故事約 340 neurons／次（3–7 倍貴，只當備環）。
- 免費方案 10,000 neurons／日（與手相共用）→ 主環正常時約 200 次／日；備環常態觸發時降到約 30 次／日。部署後一週看 dashboard 實際用量。

**Secret 清理**：穩定一週、確認 `~/.secrets` 留存後才 `wrangler secret delete GROQ_API_KEY CEREBRAS_API_KEY`；`docs/rollback-procedures.md` 同步改寫。

### 2.2 階段 A：配對規則層（純函式，`crates/schema/src/zodiac_match.rs`，前後端共用）

- 版本常數 `ZODIAC_MATCH_CORPUS_VERSION: &str = "1"`（比照 `TAROT_*` 慣例）。
- **相位**依星座距離 `d = min(|i−j|, 12−|i−j|)`，台灣繁中占星術語：合相 0、半六分相 1、六分相 2、四分相 3、三分相 4、梅花相（補十二分相）5、對分相 6。文案註明「以星座為單位的簡化看法，不是行星度數相位」。
- **質性標籤（互斥 6 類，不給分數）**：`d=0` 熟悉的鏡子、`d=4` 和諧、`d=2` 互補、`d=6` 互補也拉鋸、`d=3` 拉扯、`d=1/5` 需要磨合。
- 元素組合（10 種無序）、模式組合（6 種）各一句；全部「傾向」語氣；測試掃描禁用字（注定、不合、速配、分手、%）。
- 三列框架語明寫「**你的**太陽／月亮／上升 × **對方的太陽星座**」，避免被讀成月亮對月亮。
- 測試：12×12 全組合有輸出、對稱、距離、未知星座 `None`、禁用字。

### 2.3 階段 A：前端 `/zodiac`（公開路由，`crates/web/src/pages/zodiac.rs`）

1. **我的星座**：`Effect` 監聽 `auth.user`（公開頁 bootstrap 非同步，初始為 `None`）。
   - 未登入／`hasBirthData=false`／西洋盤 API 失敗 → 12 宮格手選「我的太陽星座」。
   - `hasBirthData=true` → `GET /api/charts/western` 取三星座。**未填時辰**（`birth_hour` 為空）→ 上升列不顯示，註明「未填出生時辰，無法判讀上升」；月亮列加註「時辰不詳時月亮可能有誤差」。
2. **對方星座**：12 宮格，點選即翻牌出結果；`sign_glyph_path`（`divination.rs:462` 私有）抽到共用模組。
3. **結果**：每列框架語 → 相位標籤 → 元素句 → 模式句；底部「以星座為單位的簡化看法，描述互動傾向，不是關係結局」。
4. 入口：首頁功能卡「星座配對」（新 `fi-match` 圖示）；西洋星盤頁底「拿你的星座去配對 →」。
5. a11y：宮格 `button`＋`aria-pressed`；結果區 `aria-live="polite"`；`zm-*` 的 reduced-motion 規則放在 `zm-*` 基本規則**之後**（檔尾既有 reduced-motion 區塊不會涵蓋新規則）。
6. 規則運算在瀏覽器（wasm 直接呼叫 `ft_schema::zodiac_match`），**免登入路徑不打 API**。

### 2.4 階段 B：配對 AI 解讀 API

- `POST /api/zodiac/match/interpret`，body `{"target":"Leo"}`。
- 順序：`auth_user` → `body_too_large` → 便宜的每分鐘限流（`rate_limit` 20／min）→ 驗 target ∈ 12 星座（400 `INVALID_SIGN`）→ **伺服器讀西洋盤**（新 `resolve_western_chart(ctx, user)`：驗生辰否則 400 `NO_BIRTH_DATA` → 讀 `interpretations` 驗 `birth_data_hash`＋`ENGINE_VERSION_WESTERN`＋`CHART_SCHEMA_VERSION` → 缺或過期打 `FT_ENGINE /engine/western` 現算；引擎失敗 502 且不扣額度；**不用** `chart_resolver::resolve_chart`，它在紫微成功時 `western: None`）→ 查快取 → `rate_limit_strict` IP 10／24h、user 3／24h → AIMutexDO 專屬實例 `id_from_name("zodiac-match")` → 寫回。
- **失敗不落庫**：AI 失敗／stub → 503 `AI_UNAVAILABLE`、`AI_QUOTA` → 503 `AI_QUOTA`；只有成功才寫入。回應 `interpretation: String` 恆有值。
- chartData：`{ mySigns:{sun,moon,rising?}, target, rules:[…], corpusVersion }`（未填時辰不送 rising）；不含生日、出生地、時辰。
- Prompt：chart type `zodiac_match`；`get_system_prompt` 把 `zodiac_match` 列為**預設中文**（否則落入 `_ => language != Some("zh")` 掛上 `GUARDRAIL_EN`）；`build_prompt` 補明確分支（否則落入 western）；專屬規則：不得斷言關係成敗、不得建議分手／交往、對方只知太陽星座；測試 `guardrail_matches_prompt_language` 加 `("zodiac_match", None)`／`Some("zh")`。`ZODIAC_MATCH_PROMPT_VERSION: &str = "1"`。
- 錯誤一律 generic，不回 DB／AI 細節。

### 2.5 資料表與 Wire DTO

```sql
-- scripts/migrations/2026-11-19-zodiac-match.sql（同步 scripts/schema.sql）
-- 依站長全域規則：不加 FK／UNIQUE／CHECK constraint
CREATE TABLE IF NOT EXISTS zodiac_match_readings (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  target_sign TEXT NOT NULL,
  sign_key TEXT NOT NULL,           -- 固定格式 "Taurus|Aquarius|Virgo"（無上升為 "Taurus|Aquarius|-"）
  corpus_version TEXT NOT NULL,
  prompt_version TEXT NOT NULL,
  ai_interpretation TEXT NOT NULL,
  ai_provider TEXT,
  ai_model TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_zmr_lookup ON zodiac_match_readings(user_id, target_sign, sign_key, prompt_version);
```

- 快取鍵＝`(user_id, target_sign, sign_key, prompt_version)`；先查後插，併發雙寫最多多一筆，讀取取最新。
- 清理：本站目前沒有刪帳端點；`PUT /api/users/me/birth`（`users.rs:221` 已刪 `interpretations`）一併 `DELETE FROM zodiac_match_readings WHERE user_id=?`。
- DTO（`crates/schema/src/api.rs`）：`ZodiacMatchInterpretRequest { target }`、`ZodiacMatchInterpretResponse { interpretation, provider, model, cached }`，camelCase。

## 3. 檔案清單與部署

| 階段 | 檔案 |
|---|---|
| 0 換 AI 鏈 | `crates/api/src/services/ai/{providers,mod}.rs`、`durable_objects/ai_mutex_do.rs`、`routes/{charts,tarot,palm}.rs`、`crates/schema/src/storage.rs`、`crates/web/src/api.rs`、`crates/web/src/pages/{divination,story,tarot,palm}.rs` |
| A 規則版 | `crates/schema/src/{zodiac_match,lib}.rs`、`crates/web/src/pages/{zodiac,mod,home,divination}.rs`、`crates/web/src/lib.rs`、`crates/web/style.css`、`scripts/contrast-check.py`（新配對） |
| B 配對 AI | `scripts/migrations/2026-11-19-zodiac-match.sql`、`scripts/schema.sql`、`crates/schema/src/api.rs`、`crates/api/src/routes/{zodiac,mod,users}.rs`、`crates/api/src/services/chart_resolver.rs`、`crates/api/src/services/ai/prompts.rs`、`crates/web/src/{api.rs,pages/zodiac.rs}`、`scripts/verify-deployment.sh`（未認證 POST 回 401） |
| 文件 | `docs/ARCHITECTURE.md`、`CLAUDE.md`（AI 鏈、版本常數紅線）、`docs/rollback-procedures.md`、memory |

**部署順序**（每步前 commit）
- 階段 0：`deploy-api.sh` → 正式站實測紫微／西洋／故事／塔羅／手相 AI 各一次 → `deploy-web.sh`（新 web 對舊 API 也相容：只是文案與錯誤處理）。回滾：Pages／Workers dashboard 回上一版；secret 一週內不刪即可直接回舊 API。
- 階段 A：只部署 web。
- 階段 B：`turso db shell fortunet < scripts/migrations/2026-11-19-zodiac-match.sql` → API → web → `verify-deployment.sh`。

## 4. 待站長決策

- **D1 不給分數**（建議）：6 類質性標籤取代百分比。
- **D2 AI 結果存庫**（建議存）：同組合重看不燒每日 3 次額度；失敗不落庫。
- ~~D3 AI 供應商~~：**已裁示全改 Workers AI**；實測後主環 gemma-4（不思考）、備環 llama-3.3-70b。
- **D4 分階段**（建議）：0 → A → B 各自部署驗證。
- **D5 額度不足**：先降級顯示（建議），不升級 Workers 付費方案。

## 5. 驗證

- `cargo test -p ft-schema -p ft-api`（規則層 12×12、護欄語言、Responses 解析三 fixture、額度錯誤辨識、`max_tokens` 對照、storage 常數）；`cargo fmt --check`；clippy wasm32 `--all-targets` 不增警告。
- `css-guard.py --allow-local-vars`、`contrast-check.py`；無頭假資料 harness 截 `/zodiac` 手機／桌機：未登入選太陽、登入三星座、未填時辰、AI 產生中／完成／`AI_QUOTA`／`AI_UNAVAILABLE`／429；稽核 0 違規。
- 階段 0 上線前：本機用 Workers AI REST 跑 `prompts.rs` 全部 chart type（含 tarot、palm 不受影響的確認）各 3 次，記錄耗時與 tokens。
- 部署後：`verify-deployment.sh`；正式站各 AI 功能實測一次；一週後看 Workers AI dashboard 用量校正 §2.1 估算。
- 審查：Neon xhigh 一審（已完成）→ Neon 複審代替 Codex 二審（Codex 11/25 前不可用）；實作階段每階段同樣兩審。

## 6. 二審（Gemini 4 Argon High）修正——實作時以本節為準，覆蓋上文

1. **[B] DO 白名單**：`ai_mutex_do.rs validate_entry` 非影像分支加 `"zodiac_match"`（階段 0 就加），同步 `validate_entry_whitelist_and_pairing` 與 max_tokens 測試；`ai_mutex_do.rs` 列入階段 B 檔案。
2. **[B] 未填時辰**：`resolve_western_chart` 回 `ResolvedWestern { chart, has_birth_hour }`，錯誤型別區分 `NoBirthData`（400）／`Db`（500）／`Engine`（502，不扣額度）。`has_birth_hour=false` → `sign_key` 第三段為 `-`、chartData 不送 rising（引擎以正午算出的上升不可用）。前端以 `auth.user.birth_hour.is_none()` 判定（西洋盤 API 回應無此欄）。
3. **[S] deadline 起點**：`deadline = entry.queued_at + 88_000`（自入列起算，含佇列等待），`process_queue` 把 `queued_at` 傳進 `handle_request`。
4. **[S] 配對 prompt 角色**：實測 gemma 仍寫出「你們的月亮」。`rules[].pair` 明寫「你的月亮（水瓶座）× 對方的太陽（獅子座）」；prompt 規定月亮／上升列皆為「使用者的月亮／上升 × 對方太陽」、禁止「你們的月亮／上升」；無 rising 時只輸出 `## 核心價值`、`## 情感需求`、`## 小提醒`。配對 prompt 定稿前本機再實測 3 組（含無 rising）。
5. **[S] 退次與 stub 快取**：
   - `palm.rs classify_ai_failure`：`(503,"AI_QUOTA")` 歸入無條件退次。
   - `tarot.rs call_tarot_ai` 改回傳 `(status, code)`；`AI_QUOTA`／`AI_QUEUE_FULL`／`AI_QUEUE_TIMEOUT` 呼叫 `refund_strict` 退次；配對路由同款（`rate_limit_strict_refundable`）。
   - `charts.rs`：以 `provider == "stub"`（或新增 `stub: bool`）跳過 L346／L649 寫快取；L227、L590 讀快取時把含「離線模板」／`offline-template` 的舊資料視同未命中，自我修復。
6. **[S] 依賴與 dead code**：`crates/web/Cargo.toml` web-sys 加 `AbortController`、`AbortSignal`（列入階段 0）；`providers.rs` 移除 `build_request`、`send_with_timeout` 與無用 imports；`InterpretMsg.keys` 移除。
7. **[N] `*` 清單判定**：只有 `*` 後接空白才算清單（`**粗體**` 開頭的段落不可誤判）；`tarot.rs:432` 與 `zodiac.rs` 的 AI 解讀也改用 `render_markdown`。
8. **[N] 快取鍵**：索引與查詢加入 `corpus_version` → `(user_id, target_sign, sign_key, corpus_version, prompt_version)`；`target` 先正規化成標準英文名（`"Leo"`）再查寫。
