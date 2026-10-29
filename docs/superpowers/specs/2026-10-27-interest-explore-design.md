# 設計：興趣探索 v1（RIASEC 興趣剖面）

- 日期：2026-10-27（v2：2026-10-30 查證＋自審）
- 狀態：設計中、未實作。審查鏈：v1→（2026-10-30 查證＋自審 S1–S8）→**v2**→Codex 審→…（後續輪次見 §8b）
- 入口：`我的命格`（`crates/web/src/pages/profile.rs`）
- 相關功能：F1 IPIP-15 人格測驗、本週小練習、命理 AI 護欄（`GUARDRAIL_ZH/EN`）
- 長期方向：「優勢探索」三面向——能力剖面、學習反應、發展環境。本文件只涵蓋 v1 的興趣剖面

## 1. 目的與依據

讓使用者用一份有公開研究基礎的量表，看見自己**偏好哪類活動**，再連到本週小練習去實際嘗試。產品**不測量天賦、智力或能力**，也不預測職涯成就。

- 遺傳率和多基因分數描述的是群體變異，不能決定個人；教育成就的多基因指標也不是天賦預測工具。本站不收集、不引用任何遺傳或腦影像資訊。見 [2015 雙胞胎統合分析](https://pubmed.ncbi.nlm.nih.gov/25985137/) 與 [2022 教育成就 GWAS](https://www.nature.com/articles/s41588-022-01016-z)。
- 多元智能「各自獨立的智能」缺乏實證支持，所以不用「你是 X 型智能」的分類，也不用分類結果決定使用者該做什麼。
- 興趣（喜歡做什麼）和能力（做得多好）是不同的構念。結果頁只能說「你比較喜歡這類活動」，不能說「你擅長」「你有這方面的天賦」。
- 刻意練習與成長型思維的限制已在 [本週小練習 spec §1](2026-10-20-weekly-practice-design.md) 記錄，本功能沿用：練習只提供結構，不承諾能力提升。

**成功定義**：使用者看完結果後，能說出一個自己想多試試的活動方向，並自願到本週小練習選一件小事。完成率與連到小練習的比例可作產品指標；興趣剖面本身不作任何能力或成效的證據。

## 2. 產品決策

1. **量表**：採用 Armstrong、Allison 與 Rounds（2008，*Journal of Vocational Behavior* 73(2), 287–299，DOI 10.1016/j.jvb.2008.06.003）的 RIASEC 量表 **Activities **Set A****（每型 8 題，共 48 題；選版依據見附錄 A.3）。授權定位見 §2.2——非商業使用＋註明出處，非無條件 public domain。不用 O\*NET Interest Profiler 本體，原因見 §2.2。
2. **不自行刪減題目**：48 題全部保留。刪題會讓已發表的信度與結構失效；如果日後要更短的版本，必須另外引用有驗證的短版，不能自己挑題。
3. **作答**：每題五點量表——非常不喜歡／不喜歡／不確定／喜歡／非常喜歡（1–5）。必須 48 題全答才能送出；作答只暫存在前端記憶體，關掉分頁就消失，不存草稿。
4. **計分**：每型取 8 題平均（1.0–5.0）。計分在 `ft-schema` 純函式完成，API 端重算後才寫入，不信任前端分數。
5. **不給常模分數**：中文版沒有台灣常模，所以**不顯示百分位、不跟他人比較、不寫「高於平均」**，只呈現使用者自己六型之間的相對高低。
6. **結果呈現**：**六條長條**（已決，2026-10-30：長條易讀、符合色彩對比紅線；六角形留給後續版本評估），固定 R-I-A-S-E-C 順序。「較明顯的方向」= 與最高分差距 ≤ 0.5 的類型（最多 3 個；**平手時依 R-I-A-S-E-C 順序取前 3**）。若最高分 < 3.0，改顯示「目前沒有特別突出的偏好，這很常見」，不硬湊結果。0.5 和 3.0 是產品門檻，不是心理計量標準，文件與程式註解都要寫明。
7. **不對應職業清單**：v1 不提供職業推薦。職業對應需要 O\*NET 資料庫與另一份授權，留到後續版本另案設計。
8. **重測**：可隨時重做，每次保留為一筆紀錄；畫面只顯示最新一次，歷史可分頁查看。
9. **不接命盤、不接 AI**：v1 不把興趣結果送進 AI，也不跟紫微／西洋命盤疊加。日後若要做「命盤 × 興趣」的反思文字，必須帶 `GUARDRAIL_ZH/EN`，並另寫 spec。
10. **跟其他功能分開**：興趣資料不寫入 predictions、prediction_feedback、f8_assignments；不讀 `forecast`、`experiment`、`isControl`。跟小練習的關係**只有連結**：結果頁提供「去選一件小事」按鈕，不自動指派練習（與小練習 spec §2 第 2 點一致）。

### 2.1 六型與結果頁文字

| 代碼 | 名稱 | 結果頁描述（偏好的活動類型，不寫能力） |
|---|---|---|
| R | 實作型 | 喜歡動手操作、使用工具或在戶外進行具體的工作。 |
| I | 研究型 | 喜歡觀察、分析問題，弄清楚事情運作的原理。 |
| A | 藝術型 | 喜歡用文字、圖像、聲音或表演表達想法。 |
| S | 社會型 | 喜歡教導、協助、照顧或與人合作。 |
| E | 企業型 | 喜歡說服、帶領團隊，推動計畫往前走。 |
| C | 事務型 | 喜歡整理資料、遵循清楚的流程，把細節處理好。 |

每型附一題反思提問（例如「最近一次做這類事情是什麼時候？感覺如何？」），不附職業或能力判斷。

### 2.2 授權

| 來源 | 授權 | 能否中譯使用 |
|---|---|---|
| Armstrong 等人（2008）RIASEC Markers（Activities） | 論文宣告 public domain 但**限研究用途**（p. 292）；作者實驗室現行頁面（2023）為**非商業使用＋註明出處**之免費授權（見附錄 A） | 可以——站上現為非商業；中譯版需在頁面註明原始出處，並說明為本站翻譯、未經常模驗證。**日後商業化前必須重新取得授權或換量表** |
| O\*NET Interest Profiler（含 Mini-IP 30 題） | CC BY-ND 4.0 或 O\*NET Tools Developer License | CC BY-ND 不允許衍生作品（翻譯算衍生）；Developer License 允許修改，但要求「validate all products incorporating content」（§3b），本站沒有驗證能力，所以不採用 |

參考：[O\*NET Career Exploration Tools 授權](https://www.onetcenter.org/license_tools.html)、[Mini-IP 開發報告](https://www.onetcenter.org/dl_files/Mini-IP.pdf)。量表題目是從美國勞工部 Interest Profiler 題池**逐字選題**（非改寫；論文 p. 289 主張 IP items 為 public domain）；論文全文已取得並完成查證（2026-10-30，見附錄 A）。

### 2.3 翻譯流程

1. Claude 初譯 48 題 → 外部模型（codex、kimi 至少一個）**獨立複譯** → 對照合併、分歧題逐題定案 → 再由另一模型**回譯成英文**比對語意（一人專案的「兩人＋第三人」替代流程，審查紀錄留附錄 B）。
2. 題目必須是一般人都能想像的活動，避免台灣少見的情境；需要替換情境的題目要記錄原文與理由。
3. 中英對照存在 `crates/schema/src/interest/items.rs`，每題標註附錄 A.4 的題號與 form `'A'`。改任何一題都要 bump `INTEREST_INSTRUMENT_VERSION`。

## 3. 使用者流程與文案

```text
卡片初次進入
  → 說明 + 「開始（約 5 分鐘，48 題）」
作答
  → 每頁 8 題、進度條、可回上一頁；全答才能送出
結果
  → 六型長條 + 較明顯的方向 + 每型反思提問
  → 「去本週小練習選一件小事」／「重新作答」／「查看過去結果」
再次進入
  → 顯示最近一次結果與作答日期
```

卡片說明：「看看你比較喜歡哪些類型的活動。這是興趣，不是能力或天賦測驗；結果只供你自己參考，可以隨時刪除。」

結果頁固定註腳：「題目來自公開的研究量表（Armstrong 等人，2008），中文為本站翻譯，沒有台灣常模，所以只顯示你自己六型之間的相對高低。興趣會隨經驗改變，歡迎之後再做一次。」

沒有突出偏好時：「目前沒有特別突出的偏好，這很常見。可以從任一類型挑一件小事試試，再回來看看感覺有沒有不同。」

### 3.1 狀態與邊界

- 未登入：卡片顯示說明與登入入口，不能作答。
- 作答中離開：不存草稿，回來重新開始。
- 不顯示：百分位、排名、「天賦」「擅長」「適合當 X」、IQ、能力等級。
- 不要求生日、命盤、人格測驗或訂閱。

## 4. 資料與 API 契約

### 4.1 Turso（`scripts/schema.sql` 為單一權威）

`users` 新增 `interest_epoch INTEGER NOT NULL DEFAULT 0`（既有資料庫另執行 `ALTER TABLE`），用途與 `practice_epoch` 相同：刪除後擋掉其他分頁的過期送出。

```sql
CREATE TABLE IF NOT EXISTS interest_results (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  instrument_version TEXT NOT NULL,
  form TEXT NOT NULL,              -- 'A'；保留給平行版本
  answers TEXT NOT NULL,           -- JSON 陣列，48 個 1..5；保留原始作答以便改版重算
  scores TEXT NOT NULL,            -- JSON {"R":3.25,...}，伺服器計分
  request_id TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_interest_request
  ON interest_results(user_id, request_id);
CREATE INDEX IF NOT EXISTS idx_interest_user_created
  ON interest_results(user_id, created_at);
```

依專案資料庫原則不加 FK 與 CHECK，值域由 API 驗證；只保留 `(user_id, request_id)` 唯一索引做冪等，這是並發重送時唯一可靠的防線。時間由伺服器寫入 UTC ISO。

### 4.2 新端點

| 方法與路徑 | 輸入 | 成功輸出／規則 |
|---|---|---|
| `GET /api/interest` | 無 | `interestEpoch`、`totalResults`、最新一筆結果（含 `scores`、`instrumentVersion`、`createdAt`，不含 `answers`）。`Cache-Control: private, no-store`。 |
| `GET /api/interest/history` | `cursor` 可選，`limit=1..20`（預設 20，照 practice 前例） | 按 `(created_at, id)` 倒序分頁，只回本人資料；游標由伺服器產生。 |
| `POST /api/interest/results` | `{instrumentVersion, form, answers: [48 × 1..5], expectedEpoch, requestId}` | 先呼叫 `body_too_large`。`instrumentVersion` 必須是目前版本，否則 409 `STALE_INSTRUMENT`；`form` 必須 `'A'`（v1 僅支援 Set A），否則 400；answers 長度或值域錯誤回 400。`expectedEpoch` 不符回 409 `STALE_INTEREST_STATE`。同 `requestId` 同內容直接回傳原結果，內容不同回 409 `IDEMPOTENCY_CONFLICT`；唯一索引敗者要重讀比對，不可回 500。用 `INSERT ... SELECT ... WHERE EXISTS(users.interest_epoch = :expectedEpoch)` 條件寫入。 |
| `DELETE /api/interest` | 無 | 單一 `db::batch`：刪除本人全部 `interest_results`，再把 `interest_epoch` 加一。 |

共用 session 驗證與每 IP 速率限制；錯誤只回通用 `"db error"`。API 不接收自由文字，不把興趣資料送給 AI。

### 4.3 資料權利與揭露

- 卡片內提供「刪除全部興趣探索紀錄」，先顯示筆數再確認。
- `DELETE /api/personality/me` 與小練習刪除的確認文字都補一句「興趣探索紀錄需在該卡片另行刪除」。
- 若未來提供整個帳號刪除，`interest_results` 必須列入同一筆交易。
- 名詞解釋頁（`glossary`）新增「RIASEC」與「興趣 vs 能力」兩條。

## 5. 衡量與上線條件

上線前必須全部完成：

1. §2.2 的授權與題目原文已用論文全文確認，出處寫進結果頁註腳。
2. 48 題中譯完成 §2.3 的回譯比對，紀錄放在本文件附錄。
3. `ft-schema` 單元測試：計分（全 1、全 5、單型高分）、門檻邊界（差距剛好 0.5、最高分剛好 3.0、「最多 3 個」平手排序）、answers 長度與值域驗證、題庫完整性（48 題、每型恰 8 題、題號連續）。
4. `ft-api` 測試：冪等重送、epoch 過期、刪除後重送 409、版本不符（409 `STALE_INSTRUMENT`）、`form ≠ 'A'` 400。
5. 前端文案掃描：結果頁與卡片不出現「天賦」「擅長」「適合」「百分位」「平均」。
6. 在 `dist/` 用 `_headers` 同款 CSP 在瀏覽器驗證，並跑 `verify-deployment.sh`。
7. 部署順序：Turso 遷移 → API → web。

上線後觀察（不作為能力證據）：開始率、完成率、結果頁連到小練習的比例、重測間隔。

## 6. 待確認事項

| 項目 | 狀態 | 處理方式 |
|---|---|---|
| 公開領域聲明與 48 題原文 | **已確認（2026-10-30，附錄 A）** | 論文全文（作者上傳版）已取得並逐字核對；授權為非商業＋註明出處，日後商業化前重評 |
| 用 Form A 還是 Form B | **已決：Set A** | 兩樣本男女四組信度 Set A 全面略高、無低於 .80 之尺度（Set B 的 Enterprising 兩次 .79）；CUS 結構適配亦較高。數字見附錄 A |
| 結果圖用六角形還是長條 | **已決：長條**（2026-10-30） | 長條較易讀、也符合色彩對比紅線；六角形留後續版本評估 |
| 唯一索引是否違反「不加 constraints」原則 | 依小練習前例保留 | 冪等需要它；若要移除，必須改用其他並發防線 |

## 7. 後續版本（不在 v1 範圍）

- **v2 學習反應**：小練習可以選擇掛一個 RIASEC 類型（使用者自選，不自動指派），在興趣卡顯示該類型 4 週內的嘗試次數與主觀回饋。
- **v3 發展環境**：每月自評時間、回饋、資源、榜樣是否足夠，只呈現使用者自己的變化。
- **v3 AI 反思**：把興趣剖面、命盤與練習紀錄整理成反思提問，必須帶 `GUARDRAIL_ZH/EN`，只輸出問題與可選的下一步，不輸出結論或標籤。
- **職業對應**：若要做，需另行評估 O\*NET 資料庫授權（CC BY 4.0）與中文職業名稱對照。

## 8. 自審紀錄（2026-10-27）

| 檢查 | 結果 |
|---|---|
| 科學主張 | 只說「偏好的活動」；不測能力、不給常模、不引用遺傳或腦科學；門檻標明為產品選擇。 |
| 授權 | 避開 CC BY-ND 的翻譯限制與 Developer License 的驗證義務；公開領域聲明列為上線前必確認。 |
| 研究資料隔離 | 與 F5/F6/F8 完全分開；跟小練習只有連結，不自動指派。 |
| 專案紅線 | body 上限、通用錯誤訊息、request 冪等、epoch 防過期、CSP 驗證、部署順序都已列入。 |

## 8a. 自審紀錄（2026-10-30，v2——查證後複審）

| # | 發現 | 處置 |
|---|---|---|
| S1 | §2.1 仍寫「公開領域量表」，與查證後的授權定位（非商業＋出處）不一致 | 改為 RIASEC 量表 Activities Set A＋DOI，授權指向 §2.2 |
| S2 | §2.2 參考段「從 O\*NET 題目**改寫**而來」與事實不符（論文為逐字選題）；「上線前要取得全文」已過時 | 改為逐字選題＋查證完成指向附錄 A |
| S3 | 結果圖「六角形或長條」未決，但 §3 流程已寫「六型長條」 | 決策：長條（§2.6、§6 同步） |
| S4 | 「最多 3 個」方向在平手時的取法未定 | 定：平手依 R-I-A-S-E-C 順序取前 3（§2.6、§5.3 測項） |
| S5 | POST 的 `form` 欄位缺驗證規則 | 定：`form ≠ 'A'` → 400（§4.2、§5.4 測項） |
| S6 | §2.3「兩人翻譯＋第三人回譯」在一人專案不可行 | 改為 Claude 初譯→外部模型複譯→合併→另模型回譯（紀錄留附錄 B） |
| S7 | §5.3「版本不符」測試屬 API 層行為，列錯層 | 移至 §5.4 ft-api；ft-schema 補題庫完整性測項 |
| S8 | history `limit` 未定預設值 | 定：預設 20（照 practice 前例） |

## 8b. 外部審查紀錄

（待補：Codex 輪 → Opus 終審）

## 附錄 A：出處與題目查證紀錄（2026-10-30）

### A.1 出處

- Armstrong, P. I., Allison, W., & Rounds, J. (2008). Development and initial
  validation of brief public domain RIASEC marker scales. *Journal of Vocational
  Behavior*, **73**(2), 287–299. **DOI: 10.1016/j.jvb.2008.06.003**
- 全文來源：通訊作者 James Rounds 上傳之電子檔（ResearchGate publication 222697129，
  2018-02-25 上傳；實驗室頁面另掛同份 Dropbox PDF），非盜版。
- 題目現行託管處（論文中的 netfiles.uiuc.edu 已搬家）：Rounds 實驗室 Interest Item Pool
  `https://jrounds.weebly.com/riasec-markers-scalesitems.html`（2023-09-10 更新）。
- 混淆釐清：openpsychometrics 把題組歸給「Liao, Armstrong & Rounds (2008)」——那是另一篇
  （Basic Interest Markers, *JVB* 73(1), 159–183, DOI 10.1016/j.jvb.2007.12.002），非本量表。

### A.2 授權聲明（逐字）

論文（p. 292，Discussion）：

> "…a set of brief, alternate forms, activity- and occupation-based RIASEC
> measures that would be available in the public domain **for research
> purposes**."
> "These scales will be made available on a public domain website providing
> open access to the scales and items."

實驗室現行頁面（"Reference and Permission to Use"）：

> "RIASEC Markers are copyrighted by Patrick Ian Armstrong, Allison Wyndolyn,
> and James Rounds. Permission to use RIASEC Markers is granted for free to all
> professionals (researchers and practitioners) if it is used for noncommercial
> purposes. Appropriate credit should be given to the authors of RIASEC Markers."

**結論**（2026-10-30 決策：照走非商業授權）：站上現為非商業＋結果頁註明出處，符合授權；
譯文屬「使用」範圍（授權未禁衍生）。日後商業化前必須重新取得授權或換量表。題目淵源為
美國勞工部 Interest Profiler 題池之逐字選題（論文 p. 289 主張 IP items 為 public domain）。

### A.3 Form 決策數據（Activities，Cronbach's α）

| 樣本 | Set A 範圍 | Set B 範圍 | Set A 均值 F/M | Set B 均值 F/M |
|---|---|---|---|---|
| 樣本 1（n=816） | .80–.92 | .79–.93 | .863 / .878 | .857 / .875 |
| 樣本 2（n=534） | .84–.93 | .79–.94 | .887 / .890 | .880 / .882 |

Set A 無任何尺度低於 .80（Set B 的 Enterprising 兩次 .79）；CUS 結構適配 Set A 亦較高
（VAF .76/.79 vs .73/.72）。**採用 Set A（Activities）**；Occupations 兩組與 Mini-IP 不採用。

### A.4 題目原文（Activities Set A，48 題）

論文 Appendix A 未編號，題號為本規格自訂（依結果頁固定 R→I→A→S→E→C 順序、型內依論文
排列順序）；原文以論文附錄與實驗室頁面交叉核對一致。中譯時每題保留此英文原文對照。

| 題號 | 型 | 原文 |
|---|---|---|
| 1 | R | Test the quality of parts before shipment |
| 2 | R | Lay brick or tile |
| 3 | R | Work on an offshore oil-drilling rig |
| 4 | R | Assemble electronic parts |
| 5 | R | Operate a grinding machine in a factory |
| 6 | R | Fix a broken faucet |
| 7 | R | Assemble products in a factory |
| 8 | R | Install flooring in houses |
| 9 | I | Study the structure of the human body |
| 10 | I | Study animal behavior |
| 11 | I | Do research on plants or animals |
| 12 | I | Develop a new medical treatment or procedure |
| 13 | I | Conduct biological research |
| 14 | I | Study whales and other types of marine life |
| 15 | I | Work in a biology lab |
| 16 | I | Make a map of the bottom of an ocean |
| 17 | A | Conduct a musical choir |
| 18 | A | Direct a play |
| 19 | A | Design artwork for magazines |
| 20 | A | Write a song |
| 21 | A | Write books or plays |
| 22 | A | Play a musical instrument |
| 23 | A | Perform stunts for a movie or television show |
| 24 | A | Design sets for plays |
| 25 | S | Give career guidance to people |
| 26 | S | Do volunteer work at a non-profit organization |
| 27 | S | Help people who have problems with drugs or alcohol |
| 28 | S | Teach an individual an exercise routine |
| 29 | S | Help people with family-related problems |
| 30 | S | Supervise the activities of children at a camp |
| 31 | S | Teach children how to read |
| 32 | S | Help elderly people with their daily activities |
| 33 | E | Sell restaurant franchises to individuals |
| 34 | E | Sell merchandise at a department store |
| 35 | E | Manage the operations of a hotel |
| 36 | E | Operate a beauty salon or barber shop |
| 37 | E | Manage a department within a large company |
| 38 | E | Manage a clothing store |
| 39 | E | Sell houses |
| 40 | E | Run a toy store |
| 41 | C | Generate the monthly payroll checks for an office |
| 42 | C | Inventory supplies using a hand-held computer |
| 43 | C | Use a computer program to generate customer bills |
| 44 | C | Maintain employee records |
| 45 | C | Compute and record statistical and other numerical data |
| 46 | C | Operate a calculator |
| 47 | C | Handle customers' bank transactions |
| 48 | C | Keep shipping and receiving records |

Set B（另 48 題）不使用，原文見論文 Appendix A 與實驗室頁面。
