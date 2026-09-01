# 啟動記錄 — Magic Link 認證上線(2026-08-29)

P0-01(登入無身分驗證)修復正式上線的完整記錄。背景與修復明細見
`docs/audit/2026-08-29-codebase-audit.md`;安全狀態見 `docs/security-checklist.md`。

## 上線內容(版本時序)

| Version | 內容 |
| --- | --- |
| `a35644d3` | magic link 主體 + MAIL_FROM var 首次部署 |
| `109f02ee` | 帶 vars 重部署(secret 未接上) |
| `3e8dcf3b` | 修 register 不寄驗證信(`exists \|\| new_full_name.is_some()`) |
| `4b0a8e87` | 修 `pending_full_name` match 反寫(`(true, Some)` 才是 fresh register) |
| `31b83bd0` | 信件改中文 + 絕對過期時刻(台灣時間) |

## 生產環境配置

- **D1**:`login_tokens` 表已套用(`wrangler d1 execute fortunet-db --remote --file scripts/schema.sql`,冪等)
- **Secret**:`RESEND_API_KEY`(Resend, Sending access;上傳走 `wrangler secret bulk` + KEY=VALUE 檔轉 JSON,值不經對話)
- **Vars**(`crates/api/wrangler.toml [vars]`):`MAIL_FROM=onboarding@resend.dev`(測試寄件者,只能寄給 Resend 帳號信箱 `yanggf@msn.com`)、`ENVIRONMENT=development`
- **選填未設**:`WEB_ORIGIN`(預設 fortunet.pages.dev)。`ALLOWED_ORIGINS` 已於 2026-08-31 加入
  `wrangler.toml [vars]`(空字串 = 只收內建 allowlist;每次 preview 部署後把該 preview 的
  hashed origin 逐筆加入即可,CORS 比對是 exact origin,不收 wildcard)

## E2E 驗證(通過)

register(`yanggf@msn.com`)→ 202 → 信件 → 點連結 → token 單次消費 →
`users` 建 row(`full_name='Yanggf'`、`trial_ends_at=+30天`)→ session → 前端登入態。
`login_tokens` 過期列由 verify 的清理語句掃除。

## 上線日發現並修掉的 3 個 bug(部署期 commit)

1. **register 不寄信**:寄信條件誤限 `exists`,新用戶收不到驗證信 → 改 `exists || new_full_name.is_some()`。
2. **`pending_full_name` 恆 NULL**:match tuple 寫反(`(false, Some)` 應為 `(true, Some)`),fresh register 一律落到 NULL → 帳號永遠建不出來。
3. **信件只有相對時效**:「10 分鐘後失效」讓人錯過窗口 → 改標絕對失效時刻(UTC+8)。

## 踩坑記錄(下次直接避開)

- **`wrangler secret put` 在非 TTY(含 `!` 前綴、Bash 工具)會把空字串存成 secret** —
  顯示 Success 但 runtime 拿到空值。症狀:`binding_present=true` 但值空。正確路徑:
  KEY=VALUE 檔 + `wrangler secret bulk`。診斷日誌保留在
  `auth.rs::email_delivery_configured`(會指出哪個 binding 缺/空)。
- **Resend Sending-access key 對 `/domains` 等 account 端點回 401** — 那不是 key 壞掉,
  是權限範圍;驗 key 要用真實寄信,或選 account 端點時意識到這點。
- **Resend 測試寄件者**(`onboarding@resend.dev`)只能投遞 Resend 帳號本人的信箱;
  開放真人註冊前須驗證網域並換 `MAIL_FROM`(屆時 `wrangler deploy` 一次即可)。
- **Resend 刪除重建同名 key 會產生新值**,舊值立即失效;值只在建立當下顯示一次。

## 待辦

- [x] 部署期修復 commit(見 working tree:auth.rs / email.rs / wrangler.toml / .gitignore)— 已提交 `d034bb4`
- [ ] 正式網域寄件(Resend 驗證網域 → 換 `MAIL_FROM`)
- [x] `scripts/verify-deployment.sh` 的 ZiWei 測試打已刪除的 debug 路由,擇期修正 — 已改打引擎 worker(`7c9eaa3`,2026-08-31)
- [x] `ALLOWED_ORIGINS` 於首次 preview 部署時補設 — var 已預留(空值=僅內建 allowlist),每次 preview 部署後補該 origin(2026-08-31)
