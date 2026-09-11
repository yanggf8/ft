# F4 情境輸入 + love 第三領域擴充 + D2-A 推廣 — 技術設計切片

- 日期：2026-09-11
- 狀態：**已過計畫審查（Claude Code 三 Agent 探勘 + Plan 審查，tie-break 陷阱親自驗證）；實作落地中**
- 上游規格：`2026-08-26-engine-modernization-big5-design.md`（rev.4 §F4/§F5/§F8）、
  `2026-09-03-f5-rule-anchors-design.md`（目錄形狀/命中/擇一/coverage）、
  `2026-09-04-f5-api-predictions-design.md`（API 層/凍結/遮罩；D1/D2 登記處）
- 範圍裁決（2026-09-11，owner）：本切片 = 三領域擴充 + F4 情境輸入（承 2026-09-07 F2/F3
  裁決「F4 併入三領域錨點擴充時一併做」）；第三領域 = **love（感情）**；upstream F8
  Barnum 對照條件**另立切片，本切片不做**。

## 0. 對既有登記的偏離（明記）

1. **擴張閘門跳過**：`2026-09-03-f5-rule-anchors-design.md` §「領域擴張」登記擴張條件
   「本縱深驗證 `high` 可達且 F8 樣本充足後」。owner 於 2026-09-11 裁決先行擴充，
   接受「love 錨點未經縱深驗證即上線」；此偏離回登 F8 帳本——F8 分析須按
   `rules_version`（rules-1/rules-2）分層，love 列僅存在於 rules-2。
2. **D1 退役**：F5 API 設計 D1「v1 對有目錄領域（work/money）一律視為強度 ≥1」佔位
   自本切片起移除——F4 閘門為真：強度 ≥1 且該領域有錨點才生成。
3. **D2 廢除**：D2-A「n==2 全負面週保留 1 條」特例廢除，由推廣版 floor 語意涵蓋（§3）。
4. **行為變更（對使用者可見）**：見 §8。

## 1. love 錨點（rules-2）

- 12 條新錨點：love × T1–T6 每格 2 條，id `love-t{n}-{slug}-{lo|hi}-1`（同格兩條不同
  維，seq 恆 1，沿 work/money 慣例）；p1 有 `experiment: Some`，p2 為 `None`。
- 結構槽位（主維模式鏡射 work/money；love 特意**不全押友善性**——A 是本量表最弱維
  （α .67、AVE .41，upstream §33/§68），錨點設計須留餘裕）：

  | 格 | p1 維/檔 | p2 維/檔 | p1 效價 | p2 效價 |
  |---|---|---|---|---|
  | T1 人際摩擦 | agr-Low | emo-Low | Negative | Negative |
  | T2 時限壓力 | con-Low | emo-Low | Negative | **Neutral**（頭寸翻轉，見下） |
  | T3 生疏社交 | ext-Low | agr-High | Neutral | Neutral |
  | T4 被指出問題 | emo-Low | con-High | Negative | Negative |
  | T5 計畫被打亂 | con-High | int-High | Neutral | Neutral |
  | T6 有選擇要做 | int-High | con-High | Positive | Neutral |

- **效價頭寸**：v1 目錄 12/24 負面，正好貼著 `valence_not_over_half` 上限。加 love 後
  若鏡射原效價會是 18/36（仍過但零頭寸）；故 love-T2-p2 取 Neutral → **17/36**，留一個
  頭寸。文案審核時可改回（屆時貼上限，仍是合法狀態）。
- **內容紅線**（一體適用 tendency/forecast/experiment）：行為傾向比較非人格缺陷斷言；
  禁用詞表（F2/F3 設計 §2.4）不變；forecast 一律高基率、使用者自己可觀察的行為；
  **低基率事件禁入**——新增 lint 測試 `love_has_no_low_base_rate_event_forecast`
  （分手/離婚/出軌/劈腿/求婚/復合），承 rev.4「禁止把辭職、分手、就醫這類低基率事件
  寫成 7 天 forecast」。**文案須真人審核後才部署**（rev.4 上線檢核項）。
- `RULES_VERSION` → `"rules-2"`（write-only，無比較邏輯；F8 分析按此分層）。
- 深度不變式測試改寫：`every_v1_cell_has_at_least_two` →
  `catalog_depth_matches_rules_version`（work/money/love 每格 ≥2；family/health 恆 0）。
- F5 紅線不動：work/money 錨點一字未改，兩條 golden
  （`f5_selection_outputs_are_pinned_against_chart_side_effects`、
  `f5_all_negative_low_fixture_pinned`）**逐字未動、原樣通過**——love 只在
  `select_for_domain(Domain::Love, …)` 被明確呼叫時進入選則。

## 2. F4 情境輸入

- **輸入形狀**：五領域 `work/love/family/money/health` 各 0–3（rev.4 §F4：預設全 0、
  只點有感的、≤3 次點擊送出）。摩擦設計：每領域一列 radio（0/1/2/3，預設 0），
  標記 1–2 個有感領域 + 送出 = 2–3 次點擊。
- **wire**（`ft-schema::api`）：`DomainStrengths { work/love/family/money/health: u8 }`；
  `GeneratePredictionsRequest { #[serde(default)] strengths: Option<DomainStrengths> }`。
  值域 0–3 由 route 層 `predict::strengths_in_range` 把關：缺欄/越界 →
  **400 `INVALID_STRENGTHS`**（型別壞 → 既有 `INVALID_JSON`）。
- **閘門純函數**（`ft-schema::predict`）：`GATE_ORDER = [Work, Money, Love, Family,
  Health]`、`gated_domains()`（強度 ≥1 過濾，GATE_ORDER 序）、`strengths_in_range()`。
  family/health 無錨點 → `select_for_domain` 恆 None——閘門對五域一致運作，前瞻擴充零改動。
- **儲存**：新表 `prediction_strengths`（PK(user_id, cycle_id)、五 INTEGER 欄 + created_at；
  無 CHECK，app 層驗證沿 repo 慣例；`CREATE TABLE IF NOT EXISTS` 冪等施行）。
  不 ALTER 既有表（不可冪等），沿「新表為無痛路徑」先例。
- **寫入與凍結（原子）**：generate 的凍結段從「單句 freeze + 迴圈逐句插 predictions」
  改為**單一 `db::batch`**（Hrana v2 原子隱含交易，F7 同款）：
  `steps[0]` strengths INSERT OR IGNORE → `steps[1]` generations INSERT OR IGNORE →
  `steps[2..]` 各 domain predictions INSERT WHERE NOT EXISTS。
  - 併發敗者偵測 = `steps[1]` affected==0：敗者的整個 batch 是結構性 no-op
    （兩快照 OR IGNORE + predictions WHERE NOT EXISTS），回 200 `generated:false` 現況。
  - 相比舊制的改進：freeze 後中途失敗不再留下「凍結空週」（任一步失敗整批 rollback，
    重試完整重跑）。
- **凍結語意不變**：一週一快照；週中重送 generate 只回現況，strengths 不更新；
  空週也凍結；`profile_id` 綁當時 complete 側寫。
- **全 0 = 凍結誠實空週**：UI 送出前明示「全部留 0 表示本週不會產生預測」；
  凍結後 GET 回 `strengths` 全 0 快照，前端 `Empty{all_zero}` 顯示專屬文案
  （與「無錨點命中」的空週文案**不共用**——Codex F2 缺席兩態分文案同精神）。
- **F7 資料刪除**：`DELETE /api/personality/me` batch 加第 6 句
  `DELETE FROM prediction_strengths WHERE user_id = ?1`（幽靈列不得進 F8）。

## 3. D2-A 推廣（`filter_negative_half`）

- 刪除 `len == 2` 特例；主迴圈加 floor：`total <= 1` 即停——**永不丟最後一條**。
- 語意：嚴格「負面不過半」（`neg*2 <= total`），唯全負週保留最佳 1 條（任何 n）。
  驅逐鍵不變：`low_bonus(10 if Low) + priority`，大者先丟。
- **tie-break 方向（承重牆，親自驗證）**：Rust `max_by`/`max_by_key` 同分取**最後**一個。
  v1 特例用 `max_by`（keep 語意）→ 同分留後者（money）；naive 推廣改用主迴圈的
  `max_by_key`（drop 語意）→ 同分丟後者（money）→ **兩條紅線 golden 反向**。
  故推廣版 drop-argmax 用手動掃描、嚴格 `>` 才替換（**首同分勝**）：平手丟前者、
  留後者，golden 原樣通過。此方向依賴輸入序 = `GATE_ORDER`（work 先 money 後），
  已寫入函數 doc 與 `GATE_ORDER` doc，並有 `drop_tie_keeps_later_domain` 釘死。
- `GATE_ORDER` 同時決定 `list_cycle` 的 SQL ORDER BY 順位（work 0 / money 1 / love 2 /
  其餘 3）——三處耦合寫死在 doc，改動須同步並重跑 golden。
- 新增測試：`three_negative_keep_single_best`、`single_negative_week_is_kept`、
  `mixed_two_neg_two_neu_unchanged`、`drop_tie_keeps_later_domain`；
  原 D2-A 兩測試保留（語意現為 floor 路徑）。

## 4. API 契約變更

| 端點 | 變更 |
|---|---|
| `POST /api/predictions/generate` | body 必帶 `{"strengths":{五域 0–3}}`；缺/越界 → 400 `INVALID_STRENGTHS`。回應 += `strengths` echo。**不再接受空 body** |
| `GET /api/predictions` | 回應 += `generated: bool`（`#[serde(default)]`）與 `strengths: Option<DomainStrengths>`（凍結快照；未生成/舊週為 null） |

`generated` 的用途：前端區分「本週尚未生成（→ 顯示 F4 輸入）」vs「已凍結空週（→ 顯示
空狀態）」。舊形 JSON（無此二欄）可照樣反序列化（serde default，有測試釘死）。

## 5. Web 動線（`crates/web/src/pages/profile.rs` PredictionsCard)

- `PState` 新增 `NeedStrengths`；`Empty` → `Empty { all_zero: bool }`。
- **自動生成移除**：舊制「空 GET → 自動 POST generate」改為「GET → `generated==false`
  → NeedStrengths（F4 輸入）→ 使用者送出 → generate → refetch」。無 profile 者在送出時
  得 409 `PROFILE_INCOMPLETE` → NoProfile 動線不變。
- F4 輸入：五列 radio（`strength_row`，沿 quiz-choice 體例、四欄 grid）、預設全 0、
  生成中 disabled（`pending_gen`）；muted 註記「目前會產生預測的領域：工作、金錢、感情」
  與「全部留 0 表示本週不會產生預測」（誠實揭露目錄現狀）。
- **換週偵測重構**：舊制只在 `Ready` 比對 cycleId（NeedStrengths 中跨週一會漏）。
  新制 `cycle_seen: RwSignal<Option<String>>`，`card_init_inner` 每次回應都記錄並比對，
  rollover → 重置 `strengths` 為全 0；window focus handler 比對 `cycle_seen`（任何狀態）。
- 防禦序：非空 predictions 優先於 `generated` 檢查（`generated=false ∧ predictions
  非空` 不應發生，但以資料為準）。
- `latch`（per-mount generate 一次）隨自動生成移除而刪除；`initing` 重入鎖保留。

## 6. 測試

**native（`cargo test -p ft-schema`，70+ 全綠）**：
- 紅線 golden 兩條**逐字未動**、原樣通過（§1/§3）。
- 推廣 filter 四新測試 + 原 D2-A/迴圈測試保留。
- `gated_domains_order_and_filter`、`strengths_in_range_bounds`。
- `catalog_depth_matches_rules_version`、`love_has_no_low_base_rate_event_forecast`。
- wire：`domain_strengths_roundtrip_and_default`、`list_response_defaults_for_legacy_json`。

**runtime（部署後）**：`scripts/predictions-e2e.sh` 已改為帶 F4 strengths body
（work/love/money=1、family/health=0）+ `strengths` echo 與 `generated` 旗標斷言；
遮罩閘門①②與兩段式鏈結不變。測試帳號用完即 F7 清除（F8 零污染）。

## 7. 部署順序

**web 先、api 後**。理由：
- 舊 web + 新 api：未凍結週的自動 generate 吃空 body → 400 可見報錯（**壞**）。
- 新 web + 舊 api：舊 api 忽略 body 照常生成（閘門暫失效）；空週者短暫重複看到 F4
  輸入（**可接受的降級**，過渡窗口僅分鐘級）。

步驟：schema.sql（冪等，需同意）→ `deploy-web.sh` → `deploy-api.sh` →
`verify-deployment.sh` → `predictions-e2e.sh -t <session>` → F7 清除 + SQL 驗證
`prediction_strengths` 清空。

## 8. 登記的行為變更（上線觀察項）

1. 單一負面條目週：保留（原丟至空）。
2. 全負面 n≥3 週：保留最佳 1 條（原僅 n==2 有例外）。
3. 未生成週顯示 F4 輸入（原掛載即自動生成）——**首次動線多一步，觀察完成率**。
4. 全 0 送出 = 凍結誠實空週（UI 文案區分）——**觀察誤觸率**。
5. generate 無有效 strengths body → 400（原接受空 body）。

## 9. 不做的事

- upstream F8 Barnum 對照條件（`is_control` 恆 0；另立切片）。
- family/health 錨點、`personal_record` 切換、F2 落庫/疊圖快取/LLM 潤寫、付費計次。
- F4 的選填自由文字 `target`（rev.4 §F4 有列；本切片只做五域強度，target 的儲存與
  用途未定，登記延後）。
- F4 強度的「每週 2–3 次輕量 check」取樣升級（9/3 備註 §396：先觀察回饋率再議）。
