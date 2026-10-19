# 設計：本週小練習（自選行動與回顧）

- 日期：2026-10-20
- 狀態：已部署（2026-10-20 正式 Turso 遷移、API 與前端上線，verify-deployment 全過）
- 入口：`我的命格`（`crates/web/src/pages/profile.rs`）
- 相關功能：F1 IPIP-15 人格測驗、F5/F6 本週預測與回饋、F8 對照組

## 1. 目的與依據

讓使用者從「認識自己的傾向」走到一次具體、可自選、可回顧的日常行動。產品提供練習的結構與紀錄，不評定智力、神經可塑性或心理健康狀態。

- 成長型思維的介入對學業表現沒有穩定的大幅效果；有效與否受情境影響。這些研究主要在學生與學校情境，不能直接推論 FortuneT 使用者會受益。介面應提供可改變的策略與下一步，而非只說「再努力」。見 [2023 統合分析](https://pubmed.ncbi.nlm.nih.gov/36326645/) 與 [2019 大型隨機研究](https://www.nature.com/articles/s41586-019-1466-y)。
- 刻意練習研究支持有目的的重複與回饋，但練習並不能解釋所有表現差異；本站的自助功能也沒有專業教練或客觀技能測量，因此對外名稱只用「小練習」。見 [2014 跨領域統合分析](https://www.psychologicalscience.org/journals/psychological-science/0956797614535810/)。
- EEG／fMRI 神經回饋涉及設備和專業照護。現有證據不足以把網站互動當作 ADHD 或中風復健；本站不提供這類服務。見 [ADHD 神經回饋統合分析](https://pmc.ncbi.nlm.nih.gov/articles/PMC11800020/) 與 [中風 fMRI 隨機試驗](https://pubmed.ncbi.nlm.nih.gov/35960166/)。

**成功定義**：使用者能自願選一件小事、實際嘗試、說明有沒有幫助，並在下一次調整做法。回訪率可作產品指標；任何短期分數變化都不作為「能力提升」證據。

## 2. 產品決策

1. `我的命格` 在 `PersonalityCard` 之後、`PredictionsCard` 之前放獨立的「本週小練習」卡。未使用時只顯示標題、簡短說明與「選一件小事」入口，避免把原有預測卡推得太遠；開始後才展開。登入即可使用；不要求生日、命盤、人格測驗或訂閱升級。
2. 使用者從三個日常目標中選一個，再選對應的具體行動。v1 使用固定題庫；不以 IPIP 分數或命盤自動指派，也不讓 AI 生成醫療、財務或心理治療建議。
3. 一次只有一項進行中的練習。可以持續數週、結束或更換；更換會保留舊紀錄。沒有回報也不會受罰，不需另外設計「暫停」狀態。週期與 F5 一樣用台北時間週一，但練習不隨換週自動消失。
4. 每次碰到機會後可記一次「有嘗試／還沒嘗試」。有嘗試才問「有幫助／還不確定／沒有幫助」；沒嘗試則問原因。每個目標有一個較小與一個較進一步的動作，只有選了較進一步動作時才提供「縮小步驟」；換動作會結束舊計畫並建立新計畫。沒有打卡、連續天數獎勵或失敗標籤。
5. 每週摘要只顯示使用者自己的嘗試次數、主觀回饋及下一步選擇。不要顯示 IQ、能力等級、「大腦成長率」或與他人排名。
6. 練習與 F5/F6/F8 的預測及研究資料完全分離：不得引用 `forecast`、`experiment`、`isControl` 或命中結果來選練習、給回饋、計算成果。F8 有對照列，直接轉成練習會污染研究與使用者體驗。

### 2.1 首版題庫

| goal_code | 顯示目標 | action_code | 步驟 | 一次練習的指示 |
|---|---|---|---|---|
| `express_need` | 說清楚自己的需求 | `write_one_sentence` | 小 | 下次要討論事情前，先寫下一句自己想表達的需求。 |
| `express_need` | 說清楚自己的需求 | `say_one_sentence` | 進一步 | 下次討論時，說出一句自己想表達的需求。 |
| `start_task` | 開始一件拖延的事 | `prepare_first_step` | 小 | 前一晚準備好隔天開始所需的一樣物品或資料。 |
| `start_task` | 開始一件拖延的事 | `ten_minute_start` | 進一步 | 選一件事，只做可以在十分鐘內開始的第一步。 |
| `review_choice` | 想清楚一個選擇 | `name_one_tradeoff` | 小 | 面對一個非緊急選擇時，先說出自己最在意的一項取捨。 |
| `review_choice` | 想清楚一個選擇 | `two_options` | 進一步 | 做非緊急決定前列出兩個選項，各寫一個好處與代價。 |

題庫文字保持一般日常情境，不要求使用者輸入事件、診斷、他人姓名或私密筆記。像「寫下一句」只是在使用者自己的媒介中完成，網站不儲存句子。`action_code` 必須屬於送入的 `goal_code`，由 API 驗證。「縮小步驟」只將同一目標的進一步動作換成小動作；已選小動作時不顯示這個選項。題庫版本以 `catalog_version` 存到計畫列；讀歷史時必須用原版本文字顯示，舊版本對照不能因新文案發布而刪掉。

## 3. 使用者流程與文案

```text
卡片初次進入
  →「選一件這週想試的小事」→ 選目標與行動 →「開始這項練習」
已有進行中的練習
  → 顯示原句 +「記一次嘗試」／「換個做法」／「結束練習」
記一次嘗試
  → 有嘗試：選主觀幫助程度；還沒嘗試：選阻礙原因
  → 顯示「下次沿用／縮小步驟（若有）／換個做法」的下一步入口
進入新週
  → 顯示上週摘要 → 自選繼續或換動作；不自動建立新計畫
```

卡片說明：「選一件小事試試看。紀錄只供你回顧，儲存在本站帳號，可隨時刪除。」

空狀態：「這週想試哪一件小事？選一個就好。」

沒嘗試後：「沒關係。你可以等下次機會、把步驟縮小，或換一個做法。」

有嘗試且沒幫助後：「這個做法對你可能不合適。下次要保留、縮小或換一種方法？」

週摘要：「本週你記了 N 次嘗試；其中 M 次覺得有幫助。」N 只計 `tried=1`，M 只計 `tried=1 && helpfulness=helped`。`tried=0` 的回報另列「尚未嘗試」次數，不混入 N 的分母。若 N=0，只顯示「本週尚無嘗試紀錄」與自選下一步，不推論意志力或人格。

### 3.1 狀態與邊界

| 狀態 | UI 行為 |
|---|---|
| 無計畫 | 顯示題庫；`選擇` 不自動送出，明確按「開始」才寫入。 |
| 有計畫、本週零回報 | 顯示動作與「記一次嘗試」；不催促。 |
| 有計畫、本週有回報 | 顯示本週摘要與記錄入口。 |
| 換週 | 依 API `cycleId` 更新摘要；計畫維持進行中。 |
| 換做法 | 選好新目標／動作後一次提交；舊計畫結束、新計畫開始，歷史可回看。 |
| 網路或 DB 錯誤 | 保留畫面輸入，顯示「儲存失敗，請重試」；不假裝成功。 |
| 其他分頁已更換／刪除 | 收到 409 後重新 GET，避免對已結束的計畫加紀錄。 |

## 4. 資料與 API 契約

### 4.1 Turso（`scripts/schema.sql` 為單一權威）

新建資料庫在 `users` 定義加入 `practice_epoch INTEGER NOT NULL DEFAULT 0`；既有 Turso 資料庫另執行一次 `ALTER TABLE users ADD COLUMN practice_epoch INTEGER NOT NULL DEFAULT 0`。這個世代數供刪除後阻擋其他分頁的過期建立請求。

```sql
CREATE TABLE IF NOT EXISTS practice_plans (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  goal_code TEXT NOT NULL,
  action_code TEXT NOT NULL,
  catalog_version TEXT NOT NULL,
  replace_active INTEGER NOT NULL CHECK (replace_active IN (0, 1)),
  replaces_plan_id TEXT, -- 比對同 requestId 重送時的原計畫
  request_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  ended_at TEXT,
  ended_by_request_id TEXT, -- 換計畫時標記由哪次請求結束；手動結束為 NULL
  CHECK ((replace_active = 0 AND replaces_plan_id IS NULL)
    OR (replace_active = 1 AND replaces_plan_id IS NOT NULL)),
  UNIQUE(user_id, request_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_practice_one_active
  ON practice_plans(user_id) WHERE ended_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_practice_plans_user_created
  ON practice_plans(user_id, created_at);

CREATE TABLE IF NOT EXISTS practice_attempts (
  id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL REFERENCES practice_plans(id) ON DELETE CASCADE,
  user_id TEXT NOT NULL,
  cycle_id TEXT NOT NULL,
  tried INTEGER NOT NULL CHECK (tried IN (0, 1)),
  helpfulness TEXT,     -- tried=1: helped | unsure | not_helped
  obstacle TEXT,        -- tried=0: no_chance | forgot | too_big | other
  request_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  CHECK (
    (tried = 1 AND helpfulness IS NOT NULL
      AND helpfulness IN ('helped', 'unsure', 'not_helped') AND obstacle IS NULL)
    OR
    (tried = 0 AND obstacle IS NOT NULL
      AND obstacle IN ('no_chance', 'forgot', 'too_big', 'other') AND helpfulness IS NULL)
  ),
  UNIQUE(user_id, request_id)
);
CREATE INDEX IF NOT EXISTS idx_practice_attempts_user_cycle
  ON practice_attempts(user_id, cycle_id, created_at);
CREATE INDEX IF NOT EXISTS idx_practice_attempts_plan_created
  ON practice_attempts(plan_id, created_at);
```

所有時間由伺服器用 UTC ISO 寫入；`cycle_id` 由 `ft_schema::cycle::week_start_asia_taipei` 算出。前端不能指定週期或回報時間。Turso 的外鍵設定不可代替應用層刪除：刪除時必須以單一 `db::batch` 先刪 attempts、再刪 plans。

### 4.2 新端點

| 方法與路徑 | 輸入 | 成功輸出／規則 |
|---|---|---|
| `GET /api/practice` | 無 | `cycleId`、`practiceEpoch`、`totalPlans`、`totalAttempts`、當週與前週摘要、`activePlan` 及目前計畫最近 10 筆回報；摘要合計該使用者在各週所有計畫的回報，即使週中曾換做法。總數供刪除確認使用。`activePlan` 無論建立多久都回傳。只回本使用者資料，`Cache-Control: private, no-store`。 |
| `GET /api/practice/history` | `cursor` 可選，`limit=1..20`，預設 20 | 按 `(created_at, id)` 倒序分頁回傳舊計畫及其摘要數量，不一次載入全部回報。游標由伺服器提供，不用 offset，只有本人可讀。 |
| `GET /api/practice/plans/:id/attempts` | `cursor` 可選，`limit=1..20`，預設 20 | 按 `(created_at, id)` 倒序分頁回傳本人指定計畫的回報；用於查看歷史細節。 |
| `POST /api/practice/plans` | `{goalCode, actionCode, replaceActive?: bool, replacesPlanId?, expectedEpoch, requestId}` | `expectedEpoch` 必須等於目前 `users.practice_epoch`，否則 409 `STALE_PRACTICE_STATE` 並重新 GET。先按本人 `requestId` 查已建立計畫；同鍵同內容直接回傳，避免換做法時網路重試又換一次。否則驗證題庫；無進行中計畫時建立；`replaceActive=true` 須提供目前的 `replacesPlanId`，以一筆交易結束該計畫並建立新計畫，若它已不是進行中則 409 `NO_ACTIVE_PLAN`。預設有進行中則 409 `ACTIVE_PLAN_EXISTS`。 |
| `POST /api/practice/plans/:id/attempts` | `{tried, helpfulness?, obstacle?, requestId}` | 先查本人是否已有同鍵且同內容的回報，有則直接回傳，即使計畫已結束；否則只可寫本人進行中計畫。`tried=1` 必帶合法 helpfulness 且 obstacle=null，`tried=0` 反之。 |
| `POST /api/practice/plans/:id/end` | 空 body | 僅本人進行中計畫可結束；已結束回相同狀態，避免連點錯誤。 |
| `DELETE /api/practice` | 無 | 同一交易內先刪本人 attempts、plans，再將 `users.practice_epoch` 加一；之後其他分頁沿用舊 epoch 的新計畫請求必須 409。 |

共用 session 驗證；新 POST 使用既有 request body 64 KiB 限制和每 IP 的速率限制。API 不接收自由文字、不回傳其他人的紀錄、不向 AI 服務送練習資料。`GET` 不依人格、出生資料、訂閱或 F5 狀態。兩個建立型 POST 都要求 `requestId`：client 為一次提交產生 UUID（36 字元），重試時沿用；計畫同鍵而 `goalCode/actionCode/replaceActive/replacesPlanId` 不同、回報同鍵而 `planId/tried/helpfulness/obstacle` 不同，都回 409 `IDEMPOTENCY_CONFLICT`。題庫碼、列舉值和 UUID 格式不合法回 400。回報新增用有界條件寫入：只在 `practice_plans.id = :id AND user_id = :me AND ended_at IS NULL` 時插入；受影響列數 0 回 409 並重新 GET。並發請求同時插入相同 `requestId` 時，唯一索引敗者要重讀既有列並比對內容，不能直接回 500。部分唯一索引防止同時產生兩個進行中計畫。所有按計畫 ID 的讀寫都須同時限定登入者 `user_id`。

更換計畫使用 `db::batch` 的兩個步驟：① `UPDATE practice_plans SET ended_at=:now, ended_by_request_id=:requestId WHERE id=:replacesPlanId AND user_id=:me AND ended_at IS NULL AND EXISTS (SELECT 1 FROM users WHERE id=:me AND practice_epoch=:expectedEpoch)`；② `INSERT ... SELECT`，從剛結束的原計畫（`id=:replacesPlanId AND ended_by_request_id=:requestId`）選一列建立新計畫。兩步受同一 Hrana 交易保護；原計畫是本人唯一進行中計畫，且請求欄位已驗證，所以步驟 ① 影響 1 列時步驟 ② 必須插入 1 列；索引或其他插入錯誤應讓整批回滾。兩步皆為 0 時重新 GET 並回 409。若實作觀察到 `[1,0]`，不可只回錯誤而留下已結束的舊計畫：需先修正 SQL／交易設計，並把這種情況納入實作驗收，才可上線。`replaceActive=false` 時 `replacesPlanId` 必須省略；`replaceActive=true` 時必填。一般建立計畫也必須用 `INSERT ... SELECT` 對 `practice_epoch` 做條件寫入。這讓另一分頁較早載入的計畫無法覆蓋使用者已換好的新計畫，也不能在刪除後用舊畫面重建資料。

### 4.3 資料權利與揭露

練習紀錄與人格資料沒有 FK 或推導關係。卡片內提供「刪除全部小練習紀錄」，先顯示數量與明確確認文字。現有 `DELETE /api/personality/me` 仍只刪人格／預測資料；在該刪除確認文字補充「小練習紀錄可在我的命格另行刪除」，避免使用者誤以為一個按鈕刪光所有自述資料。若未來提供整個帳號刪除，練習資料須列入該交易。

## 5. 衡量與上線條件

首版只做可退出的 beta 卡片。只用練習資料表中已有的紀錄計算：建立計畫後 7 天內至少一次 `tried=1` 的比例、建立後四週仍有主動回報的比例、`helped/unsure/not_helped` 分布。刪除資料的使用者不再出現在表內，分析必須註明這項選擇偏差；首版不另蒐集卡片曝光或刪除事件。這些是可用性與主觀效用訊號，不對外稱療效或神經變化。練習的參與資料不可併入 F8 對照組的命中率或分析。

實作驗收：

1. 無生日、無人格測驗、無預測時也能完成建立→回報→換做法→刪除全流程。
2. 同一使用者並發建立最多一個進行中計畫；更換前後紀錄完整且彼此可區分。
   更換計畫的條件寫入不得出現「舊計畫已結束、新計畫未建立」的提交結果。
3. 重送同一計畫建立或回報不新增第二筆；換做法成功後重試不得再次結束新計畫。其他人的計畫 ID 無法讀、寫、結束。
4. 週日與週一交界按台北時間換週，原計畫仍可繼續；舊週回報不被改寫。
5. 原始／測驗人格、命盤與預測資料不進練習 API；練習資料也不進 F5/F8。
6. 刪除後 GET 不再回練習資料；其他分頁沿用刪除前 `practiceEpoch` 的建立請求收到 409，不能重建資料。`DELETE /api/personality/me` 的提示準確描述兩套資料的刪除範圍。
7. 手機窄版、鍵盤與螢幕閱讀器都能完成選擇、提交、回顧；卡片不依賴顏色表達狀態。

## 6. 自審紀錄（2026-10-20）

| 檢查 | 結果 |
|---|---|
| 科學主張 | 收斂為自選行動與主觀回饋；學校研究不外推本站受眾，不以神經可塑性、刻意練習或成長型思維名義承諾結果。 |
| F5/F8 污染 | 題庫、資料、摘要皆不依預測內容或對照分組；共享週期只供日曆顯示。 |
| 使用者控制 | 可不開始、換做法、結束、刪除；不回報即自然中斷，無連續打卡壓力。 |
| 隱私 | 不收自由文字或診斷；刪除範圍明示，`practice_epoch` 阻擋過期頁面在刪除後重建紀錄；帳號刪除需納入練習資料。 |
| 資料競態 | 部分唯一索引、交易、條件寫入和兩種建立請求的 idempotency key 共同限制重複計畫／回報；更換必須指定原計畫，重送已成功請求要先於計畫狀態判斷。 |
| 自審修正 | 移除沒有 API 支持的「暫停」；UI 刪除腦科學術語；摘要分母明定；歷史與回報細節分頁，避免長期計畫產生無界回應；移除首版無法量測的曝光與刪除率；「縮小步驟」補上明確的題庫對應；建立計畫與回報都加同鍵重送保護及並發重讀分支；刪除加入 `practice_epoch`，擋過期分頁重新建立。 |
| 仍待實作時確認 | 現有部署環境對 Turso 部分唯一索引與 Hrana 交易條件寫入的行為，須在實作階段用測試 DB 驗證；未驗證前不遷移正式 DB。 |

## 7. 本地實作與 TDD 紀錄（2026-10-20）

- 共用題庫、DTO 與資料表分別位於 `crates/schema/src/practice.rs`、`scripts/schema.sql`；既有 Turso 的一次性遷移腳本為 `scripts/migrations/2026-10-20-practice.sql`。
- 七個 API 端點位於 `crates/api/src/routes/practice.rs`；Leptos 卡片位於 `crates/web/src/components/practice_card.rs`，接入 `我的命格`。練習資料與 F5/F8 沒有讀寫依賴。
- 測試先觀察缺少契約／元件等紅燈，再補實作轉綠；本地 `ft-schema`、`ft-api`、`ft-web` 的 native lib tests 合計 169 項通過，`cargo fmt --all --check` 通過。另以本機 SQLite 確認新 schema、一次性遷移、部分唯一索引、CHECK 約束，以及換計畫兩步 SQL 得到 `(1,1)` 且只留一項進行中計畫。
- 2026-10-20 接手完成：正式 Turso 已執行一次性遷移（`practice_plans`／`practice_attempts`／索引／`users.practice_epoch` 皆確認存在），API（Version `8939dc56`）與前端（`82ccae5f.fortunet.pages.dev`）已部署；`scripts/verify-deployment.sh` 全過，未登入 `GET/POST /api/practice*` 皆回 401。Hrana `batch` 的正式交易行為仍以部署後實際寫入為準，尚待登入態 E2E 補驗。部署時一併輪換已過期的 `TURSO_AUTH_TOKEN`（group token，never expire）。
