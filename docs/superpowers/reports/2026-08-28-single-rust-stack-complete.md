# FortuneT V2 — 單 Rust Stack 遷移完成報告

**日期**：2026-08-28
**範圍**：Phase 0–D（TypeScript/Hono/React → 單一 Rust stack）+ §8.2 西洋精度驗證
**狀態**：全部落地並生產驗證（`fortunet-api` / `fortunet-engine` / Leptos Pages）

---

## 1. 為什麼做

原 stack：TS Worker（Hono + Durable Objects + D1 + AI failover）+ React 前端，
引擎是「佔位程式」（紫微接了 iztro adapter，西洋是月/日查表的「粗略近似」，spec §4.2 標記為非引擎）。

遷移目標（spec §1）：消除前后端 type 漂移、把佔位引擎換成真星曆、收斂成單一 Rust stack。

## 2. 最終架構

```
crates/
  schema        ft-schema  : api 契約（前後端共同反序列化） + storage 位元相容契約
  domain/ziwei  ft-ziwei   : 紫微（x-iztro 封裝）
  domain/western ft-western: 西洋（solar-ephemeris 所有天體）
  worker        ft-worker  : fortunet-engine（引擎，service binding）
  api           ft-api     : fortunet-api（路由 / DO / D1 / AI failover）
  web           ft-web     : Leptos CSR 前端（取代 React 945 行）
scripts/
  deploy-engine.sh, deploy-web.sh, schema.sql, verify-deployment.sh
```

部署：API + 引擎雙 Worker（OAuth 手動部署），前端 Pages（`fortunet.pages.dev`）。
D1 唯一權威 schema：`scripts/schema.sql`。CI：fmt / clippy / build wasm。

## 3. Phase 對照

| Phase | 內容 | 關鍵 commit |
|---|---|---|
| 0 探針 | vsop87 無月球（改 solar-ephemeris）、體積/CPU 實測、x-iztro 對拍 | `c62c961` `4a03a83` `59faf21` |
| A 引擎 | 引擎 Worker + service binding | `307d889` `a5d96e2` `f54b42a` |
| B Worker | ft-api + DO/D1/AI failover；覆蓋部署保 session（單向門）| `2bf442f` `0d7a662` `5ac379b` |
| C 前端 | Leptos 取代 React | `1acb860` |
| D 清理 | 刪 TS 殘留、Rust 化 CI/docs | `98d3521` |
| §8.2 | 行星/上升點（兩個 Phase A 真 bug）| `869dc4d` `11ba0b4` |
| 收尾 | 清西洋快取 + SPA 絕對路徑 | `0bfdb80` |

## 4. 驗證

- **紫微**：`ft-ziwei` wasm 對拍生產 `iztro-adapter.ts` 逐欄位一致。
- **西洋**：對 JPL HORIZONS DE441（ObsEcLon 地心視黃經）——太陽/月亮/行星/上升全 ≤0.00051°，
  上升 alt≈0 全東方。月球 ≤0.00008°。
- **時代**：canary session 覆蓋部署後仍有效（DO storage 位元相容）。
- **整合測試**：`charts.test.ts` + `ziwei-iztro.test.ts` 17/17 綠。
- **前端 E2E**：Playwright 生產全頁面，0 console error。
- **工程**：`cargo fmt` 乾淨、三 wasm crate 編譯通過。

## 5. 抓到的關鍵 bug（「編譯過 ≠ 行為對」的印證）

1. vsop87 無月球（`earth_moon` 是地月質心）
2. `Date.UTC` 誤用——`Reflect::apply(Date, …)` 呼叫建構子回 NaN
3. 行星日心/地心混淆——vsop87 `longitude()` 是日心 J2000（Venus 差 99°）
4. 上升點公式返回降點（atan2 負號 + 錯 δ）
5. 西洋快取永不失效（`extracted_version` 兜底頂層 `engineVersion`）
6. CORS：缺 `cache-control`、substring 比對、無 OPTIONS
7. SPA 相對路徑子路由錯位

## 6. 後續（不在本次範圍）

- Big5 人格×情境行為預測（`crates/domain/big5` 預留目錄）
- AI provider 正名（多數 provider 現走 offline stub 兜底）
- 前端每生辰 AI interpret / story 目前離線 stub（外部帳號狀態）
