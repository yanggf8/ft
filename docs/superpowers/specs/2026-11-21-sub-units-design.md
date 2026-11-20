# 子單元擴充設計 v2（2026-11-21）

狀態：v1 草稿 → Claude 自審 → 獨立審（R01–R24，見 §9）→ Claude 查核 → **v2（本版）**
→ 站長裁示 §8 → 多 AI 開工。
範圍：站長 2026-11-20 待辦中的六個「點進去的子單元」。**首頁「開運小物」不在本 spec**
（金流／商品／商業模式未定，且與 GUARDRAIL「傾向非注定」話術可能衝突，待站長裁示；
cort `backlog-units-shop`）。

| 主單元 | 新子單元 | 軌道 |
|---|---|---|
| 紫微斗數 | 命盤、流年大運、今年運勢 | A |
| 塔羅 | 感情、事業 | B |
| 空間風水 | 辦公室風水 | C |

三軌互相獨立，可平行開工；共用檔案見 §5。

---

## 0. 意圖與邊界

**為什麼**：站長要讓三個主單元「點進去有東西可選」，每個子單元是一個聚焦的入口，
比一整頁通用內容更貼近使用者想問的事（感情、工作、今年）。

**目標**
- 每個主單元有子單元選擇畫面，子單元有獨立網址（可分享、可從首頁直連）。
- 新內容沿用各單元既有的紅線與版本常數規則，不另立例外。

**非目標（v1 不做）**
- 開運小物／任何金流。
- 流月、流日、流時、流耀、流年十二神（紫微只做到流年＋流年四化）。
- 塔羅新牌陣（仍固定三張 現況／阻礙／建議）、78 張牌逐張的感情／事業牌義。
- 風水全屋格局、拍照 AI、理氣（沿用風水 v1 §8）；辦公室的 Batch 3 觀察題（Batch 3 本身未做）。

**反目標（做出來就算失敗）**
- 今年運勢寫成「注定會發生」的預言、給月份吉凶或具體事件預告。
- 任何子單元資料寫入 `predictions`／`prediction_feedback`／`f8_assignments`，
  或讓 `services::ai`／`services::predictions` import `ft_schema::symbolic`。
- 塔羅感情／事業變成「對方會不會回來」「會不會升職」的是非裁決。
- 辦公室風水卡片出現沒有原文核對的研究引用（evidence 帳本 `full_text_read` 規則）。
- 沒填出生時辰的使用者看到用「預設午時」推出來的大限／流年（R04）。

**推翻舊決定（站長 2026-11-20 指示）**：2026-08-26 engine-modernization spec 把
「流年、流月、流曜等時間維度命理」列為非目標（:436、:628、:1113 三處）。本 spec 只開放
**大限＋小限＋流年（含流年四化、流年命宮）**，其餘維持非目標。實作時三處都加註
「2026-11-21 由 sub-units spec 部分推翻」。

---

## A. 紫微斗數：命盤／流年大運／今年運勢

### A.1 現況（2026-11-21 查證）
- `/divination/:type`（Protected）以 `use_params_map` 讀 type；ziwei 一頁兩卡：
  命盤資料（`ZiWeiView`＋`ZiWeiPalaceGrid`）、AI 解讀。
- 引擎 `crates/domain/ziwei` 只呼叫 `by_solar(…, Config::default())`；`majorLimits[]`
  已在 DTO 但只顯示成一行字。**`horoscope()` 從未呼叫**。
- **宮位索引慣例不同**：`ZiWeiChartV3` 的 `palaces[].index`／`lifePalaceIndex`／
  `majorLimits[].palaceIndex` 是 **子=0**（ziwei/lib.rs:7-9）；x-iztro horoscope 的
  `index`／`palace_names` 是 **寅=0**（models/horoscope.rs:11）。宮格以**地支字串**定位。
- x-iztro 0.4.0 `Astrolabe::horoscope(target_date, time_index)`，`Config::default()` 下：
  流年干支依 `target_lunar.get_year_gan()`（**農曆正月初一換年**，非立春）；
  虛歲＝`target_lunar_year − birth_lunar_year + 1`；小限、大限由虛歲推。
  起運前 `decadal.name_key == Childhood`（童限）：宮位取 `CHILDHOOD_PALACES[(age−1)%6]`
  （命/財/疾/夫/福/官），干支用本命宮干支，無年齡區間。`horoscope_now()` 讀本機時鐘，**禁用**。
  `HoroscopeData` 沒有農曆年數字的 accessor。
- 沒填出生時辰時 charts 以 `birth_hour.unwrap_or(12)` 排盤（charts.rs:260、494）。
- 既有 bug：ziwei prompt（prompts.rs:95-100）與 story prompt（:149-153）以
  `"male"/"female"` 比對性別，但盤面存 `"男"/"女"`，性別永遠送「未知」。
- 首頁紫微是**不可點的 `<div class="feature">`**（home.rs:65-68）；hero CTA 與 nav
  （layout.rs:26）連 `/divination/ziwei`。

### A.2 網址與畫面
| 網址 | 內容 | 登入 |
|---|---|---|
| `/ziwei` | 子單元選擇：三張 `.feature` 卡 | 免登入；未登入點子單元 → `/login?next=/ziwei/…` |
| `/ziwei/chart` | 命盤＋AI 解讀（現 ziwei 內容） | Protected |
| `/ziwei/luck` | 流年大運 | Protected |
| `/ziwei/year` | 今年運勢 | Protected |

**路由實作（R11）**
- `DivinationPage` 改為接 `chart_type` prop；新增薄 wrapper `DivinationParamPage`
  讀 `:type` 給 `/divination/:type`（western 與非法 type 的 /profile 轉址行為不變）。
- `/divination/ziwei` 轉址到 `/ziwei/chart` 的靜態 Route **必須宣告在
  `/divination/:type` 之前**（leptos_router 依宣告順序取第一個符合者）；轉址本身不需包 Protected。
- 改連結：首頁紫微 `<div>` 改為 `<A href="/ziwei" class="feature">`（沿用 `FeatureIcon kind="ziwei"`）、
  hero CTA 與 nav 改 `/ziwei`。

```
/ziwei
┌────────────┐ ┌────────────┐ ┌────────────┐
│ 命盤        │ │ 流年大運    │ │ 今年運勢    │
│ 十二宮與主星 │ │ 十年一運、  │ │ 今年的流年  │
│            │ │ 每年流年    │ │ AI 解讀     │
└────────────┘ └────────────┘ └────────────┘
```

### A.3 命盤（`/ziwei/chart`）
- 版面不變。修性別 bug：ziwei 與 story 兩個 prompt 都改為 `"男"|"male"`→男、
  `"女"|"female"`→女，加單元測試。不 bump 版本（ziwei/story 解讀無 prompt 版本鍵；
  既有快取文字不追溯，屬品質修正）。

### A.4 流年大運（`/ziwei/luck`）
**前置條件（R04）**：使用者未填出生時辰 → 不排流年，頁面顯示
「流年大運需要出生時辰，命宮與大限都由時辰決定」＋前往 /profile 補填的按鈕；
API 回 400 `NO_BIRTH_HOUR`。今年運勢同。

**內容**
1. **大限時間軸**：`majorLimits` 依序列出（例：`2–11 甲寅`），所選年份所在大限高亮。
   所選年份處於童限時：時間軸不高亮任何段，前置一段「童限（虛歲 1–N）」。
2. **年份選擇**：預設＝API 回傳的「今天的 `luckYear`」（R14，春節前即去年），
   可前後切換；範圍＝出生農曆年到 +100，超出 → 400 `INVALID_YEAR`。
3. **宮格疊層**：`ZiWeiPalaceGrid` 加選用 prop `overlay: Option<LuckOverlay>`（不傳＝現行為，
   命盤頁不受影響）：
   - 大限命宮（童限時標「童限」）、流年命宮、小限：宮位徽章，以**地支**定位。
   - 流年四化：對應星曜旁加「流祿／流權／流科／流忌」小標，與生年 `化X` 區分。
   - 中心格（現「命盤中心」placeholder）顯示：該年干支、虛歲、大限範圍（童限顯示「童限」）、
     流年四化四顆星。
   - 新徽章顏色走既有 token；疊在 `.palace.life` 近白高光底上的徽章要跑
     `contrast-check.py`（新配對加進 `PAIRS`）。
4. **純規則文字**：每種標記一行固定說明（「流年命宮＝這一年關注焦點落在的宮位」），
   不呼叫 AI。文案放 `crates/schema/src/ziwei_luck.rs` 常數，**新增**禁字測試
   （沿用 `symbolic::BANNED_WORDS` 的「吉凶／注定」類詞表另立一份）。

**引擎（ft-ziwei＋worker）**
- `ft_ziwei::horoscope(birth…, target: &str) -> ZiWeiHoroscopeV1`：
  - 呼叫 `astrolabe.horoscope(target, 6)`（target 時辰固定午；流年以上層級不受其影響）。
  - **一律重新編號為 子=0**，並同時輸出 `branch`（R01）。`palaceNames` 改以地支為鍵
    （`[{branch, name}]`）。
  - `luckYear = astrolabe.raw_dates.lunar_date.lunar_year + nominal_age − 1`（R05；
    僅在 `AgeDivide::Normal` 成立）。加測試 pin 住 `Config::default()` 的三個 divide
    設定；改任何 divide 必 bump `ENGINE_VERSION_ZIWEI_HOROSCOPE`。
  - 童限：`decadal.isChildhood=true`、`startAge/endAge=None`（R06）。
- 某年 Y 的流年一律以 `target = Y-7-1` 計算（7/1 必在農曆 Y 年內——春節介於 1/21–2/20）。
- worker 新端點 `GET /engine/ziwei/horoscope?date&hour&gender&fixLeap&target=YYYY-M-D`
  → `{horoscope: ZiWeiHoroscopeV1}`；回應 key 是 `horoscope`，不是 `chart`。
- **不改 `ZiWeiChartV3`**（見 §6）。

**`ZiWeiHoroscopeV1`**（`crates/schema`，camelCase；索引一律 子=0）
```
{ targetDate, luckYear, nominalAge,
  decadal: { palaceIndex, branch, stem, startAge?, endAge?, isChildhood },
  age:     { palaceIndex, branch, stem },               // 小限
  yearly:  { palaceIndex, branch, stem,
             palaceNames: [{ branch, name }],            // 以流年命宮起排，12 筆
             mutagen: { lu, quan, ke, ji } } }           // 星名 zh-TW
```
版本不放進引擎 DTO（R16）：api 回應外層加 `engineVersionZiweiHoroscope`，由 api 常數蓋章。

**API**
- `GET /api/ziwei/horoscope?year=Y`（auth）。省略 `year` → 今天（台北，`tarot::taipei_today`）。
- 新 resolver `resolve_ziwei_birth`（不用 `resolve_chart`：它在缺 ziwei 時退回 western、
  缺性別時默默略過，chart_resolver.rs:14-19）＋新 `fetch_engine_horoscope`
  （`fetch_engine_chart` 要求 `chart` key，engine.rs:74）。
- 錯誤碼：`NO_BIRTH_DATA`、`NO_GENDER`、`NO_BIRTH_HOUR`、`INVALID_YEAR`、
  引擎失敗 502 `ENGINE_ERROR`。**錯誤不 echo 引擎細節**（不要抄 charts.rs:503/511 的
  `format!("{}", e)`）。
- **不落庫**。快取標頭用 `apply_cache_headers(res, 300, true)`（含 `Vary: Authorization`）；
  5 分鐘上限讓生辰修改後很快生效。
- 常數 `ENGINE_VERSION_ZIWEI_HOROSCOPE = "1"` 放 `engine_version.rs`（權威）。

**測試**
- golden（schema／ft-ziwei，native）：3 組固定生辰，至少一組含童限年、一組含春節前日期；
  斷言大限／小限／流年宮位、流年四化、`luckYear`，並斷言
  `yearly.branch == chart.palaces[yearly.palaceIndex].branch`（索引一致性）。
- 數值以 x-iztro 為準，**另與一個獨立來源（文墨天機或 iztro JS）人工核對一組**，
  核對紀錄寫進本 spec 附錄 A。

### A.5 今年運勢（`/ziwei/year`）
**內容**：上方一列純規則摘要（今年干支、所在大限、流年命宮、流年四化）＋一張 AI 解讀卡。
AI 讀「本命＋目前大限＋今年流年」三層，寫 400–600 字的年度主題解讀。

**「今年」**：target＝台北今天；`luckYear` 由引擎回傳（春節前即去年流年）。

**AI 接線（R03，逐點列出）**
- `prompts.rs`：`ziwei_year` 加進中文清單（:24 那行）、新 `base_system_prompt` 分支、
  新 `build_ziwei_year_prompt` 與 `build_prompt` 分派；`GUARDRAIL_ZH` 附加；
  `guardrail_matches_prompt_language` 測試加 `("ziwei_year", None)`。
- route 一律送 `"language":"zh"`（同 zodiac.rs:190）。
- AIMutexDO `validate_entry` 白名單加 `ziwei_year`（`max_tokens_for` 非白名單，不用改）；
  專屬實例 `id_from_name("ziwei-year")`。
- 常數 `ZIWEI_YEAR_PROMPT_VERSION = "1"`（schema），改 prompt 必 bump。
- 本功能 prompt 約束：不得預告具體事件、不得給月份吉凶、不得用「一定／注定／必然」，
  以「今年可以多留意的主題」「可以怎麼用」框架寫。

**送給 AI 的資料**：本命十二宮主星＋亮度＋生年四化、五行局、性別（修正後）、
目前大限宮位與干支（或童限）、流年命宮、流年十二宮名、流年四化。
**不送**生日、出生地、時辰、虛歲（R23：大限區間已透露年齡範圍，屬可接受；不再多給精確年齡）。
資料在 routes 層組裝；`services::ai` 不 import `symbolic`。

**輸出檢查 `output_ok`（R07）**：拒絕（＝視為失敗）當
- 含「一定／注定／必然／保證」；
- 含月份字樣（regex `[一二三四五六七八九十０-９0-9]+月`）；
- 含「預測」「本週」；
- 去空白後少於 200 字。
結尾提醒句「流年是傳統的觀察框架，不是預測。」由 route **附加**，不靠 AI 生成。
`output_ok` 需單元測試。

**流程（R20，照 zodiac）**
- `GET /api/ziwei/year` → `{summary, reading|null}`；有快取就給。
- `POST /api/ziwei/year/interpret`（**無 body**）：
  1. auth → 生辰檢查（含 `NO_BIRTH_HOUR`）→ 查快取，命中直接回（不計次）。
  2. 每分鐘 IP 預限流（同 zodiac `MINUTE_LIMIT` 做法，回 `RATE_LIMIT`）。
  3. `rate_limit_strict_refundable`：IP 10／24h、使用者 3／24h（fail-closed；
     使用者上限回 429 `AI_DAILY_LIMIT`）。
  4. 呼叫 AI；**任何失敗（stub、空字串、`output_ok` 不過、佇列滿／逾時）→ 不落庫、
     `refund_strict` 退兩把，回 503 `AI_UNAVAILABLE`**。
  5. 成功才 INSERT。
- 快取表（migration `scripts/migrations/2026-11-21-ziwei-year.sql`＋`schema.sql` 同步；
  **只有 PRIMARY KEY，無 UNIQUE／CHECK／NOT NULL／FK**，R02）：
  ```
  ziwei_year_readings(
    id TEXT PRIMARY KEY, user_id, luck_year,
    engine_version_ziwei, horoscope_version, prompt_version,
    ai_interpretation, ai_provider, ai_model, created_at)
  INDEX (user_id, luck_year)
  ```
  快取鍵＝(user_id, luck_year, engine_version_ziwei, horoscope_version, prompt_version)；
  先查後插，併發最多多一筆，讀取取最新。
- **生辰變更（R13）**：`PUT /api/users/me/birth`（routes/users.rs，zodiac 刪除那段旁）一併
  `DELETE FROM ziwei_year_readings WHERE user_id = ?`。不用 `birth_data_hash`
  （它含 generation_tags，改標籤會白燒 AI 次數）。
- 與 F5 的區隔：頁面不出現「預測」「本週」；不讀也不寫 F4 strengths／predictions。

### A.6 文件與 glossary
- glossary 新增「流年」「小限」「流年四化」「童限」，bump `GLOSSARY_VERSION`。
- ARCHITECTURE.md 新增 §紫微子單元；CLAUDE.md 規則清單加一條 ziwei_year（比照星座配對那條）。

---

## B. 塔羅：感情／事業

### B.1 現況
- `/tarot`（今日一張，純前端）、`/tarot/spread`（Protected，三張＋規則解牌＋AI 整合）。
- 沒有主題概念；唯一自由輸入是 `question`（≤200 字）。interpret 路由無 body，資料讀 DB 列。
- `tarot_readings.spread` 有 `CHECK (spread IN ('three'))`（舊表，不動）。
- `position_intro(pos: usize, reversed)` 為私有、無 topic 參數；**塔羅目前沒有禁字測試**，
  只有 `assemble_interpretation` ≤600 字檢查。
- `tarot_prompt_unchanged` 只確認 system prompt 不含 GUARDRAIL，**不鎖 `build_tarot_prompt` 輸出**。

### B.2 網址與畫面
| 網址 | 內容 |
|---|---|
| `/tarot` | 上半維持今日抽牌；下方子單元三卡：綜合三張／感情／事業 |
| `/tarot/spread` | 綜合（`topic=general`，現狀） |
| `/tarot/love` | 感情三張（Protected） |
| `/tarot/career` | 事業三張（Protected） |

- 三者共用 `TarotSpreadPage`，以 prop 傳 topic。差異：頁首標題與一句引導、提問框
  placeholder（感情「例如：最近和他的相處」；事業「例如：要不要接新專案」）、牌位副標、
  歷史清單顯示主題標籤。
- `/tarot` 子單元卡未登入時連 `/login?next=/tarot/love` 等（R21；`Protected` 本身轉址不帶 next）。

### B.3 規則解牌
- 牌位仍是 現況／阻礙／建議；`position_intro` 改為 `position_intro(topic, pos, reversed)`，
  general 輸出與現在**逐字相同**；love／career 各 3 牌位×2 正逆＝12 段新文案
  （例：感情「阻礙」→「在這段關係裡卡住的地方」）。
- bump `TAROT_CORPUS_VERSION` 2→3；`TAROT_DRAW_ALGO_VERSION` 不動。
- **新增測試（R09）**：每段主題引言 ≤40 字、主題引言禁字（新立塔羅詞表：吉凶裁決／
  「一定會」「注定」「復合」「分手」等是非裁決語）、三主題組裝後 ≤600 字；
  `version_constants_are_pinned` 更新。

### B.4 API 與資料
- `TarotTopic { General, Love, Career }`（wire `general|love|career`）；
  `TarotDrawRequest.topic: Option<TarotTopic>`（省略＝general），`TarotReadingView.topic: TarotTopic`。
- DB：`ALTER TABLE tarot_readings ADD COLUMN topic TEXT`（**無 CHECK**）；
  新寫入一律存字串（含 `'general'`）；舊列 NULL 讀成 general（R21）。
  migration `2026-11-21-tarot-topic.sql`＋`schema.sql` 同步。
- draw：驗證 topic；`matches_reading` 比對 topic 時 **NULL ≡ general**；不一致 → 409。
- interpret：topic **從 DB 列讀**（無 body）。`build_tarot_prompt` 依 topic：
  - 加一行框架（感情：「聚焦關係中的互動與自己的需要，不判定對方心意或關係結局」；
    事業：「聚焦工作上的選擇與行動，不判定升遷、錄取、收入結果」）；
  - 無提問時的預設句「請就整體運勢方向解讀」只用於 general，love／career 改
    「請就感情／工作方向解讀」（R08）。
  - bump `TAROT_PROMPT_VERSION` 1→2（只寫進 `ai_prompt_version`，不使舊解讀失效）。
- **新增 golden 測試（R08）**：general（含有提問／無提問兩例）的 `build_tarot_prompt`
  輸出與凍結的 v1 字串逐字相同。
- 限流、DO 實例 `"tarot"`、chartType `"tarot"`、命盤背景勾選沿用；三主題共用次數。

---

## C. 空間風水：辦公室風水

### C.1 現況
- `/fengshui` 純前端；`Room { Bedroom, Study }`（`ALL`、`wire`、`from_wire`、`label` 皆為
  窮舉 match），入口頁「選一個房間」即子單元選擇。localStorage `fengshui_room_v1_{wire}`。
- `Study` 標籤「書房／工作位」，題目前綴 `seat_`；卡片已引用辦公室研究
  （`R_SEAT_VIEW`＝psathiti2017＋sailer2021、`R_DAYLIGHT`＝boubekri2014，corpus.rs:115-125）。
- 版本：`FENGSHUI_ROOM_RULESET_VERSION="1"`、`FENGSHUI_ROOM_CORPUS_VERSION="3"`。
- 測試：T1 題數（ruleset.rs:222-226 寫死 8/7/15）、id 前綴 match（:229-238）、
  T2 卡數 `== 38`、T3 禁字與 `fixed_texts()`（含所有 `Room::label()`）。

### C.2 與書房的分工（R12，**待站長裁示，見 §8-1**）
現有 `seat_*` 本來就是「工作位」題組。兩案：

- **甲案：在家／公司拆開（Claude 推薦）**
  - `Study` 改「書房（在家）」，`Office`「辦公室座位」各有完整題組。
  - 兩房型可能都有「椅背」「面門」「採光」這類主題，但**題目與卡片各寫各的**
    （公司座位通常不能換、能改的只有桌面）；**研究引用可共用同一批文獻**，卡片的
    限制欄照實寫「研究是在辦公室做的」，在家書房的卡也照寫，不改寫來源。
  - Study 的卡片文字若有「工作位」措辭要改 → CORPUS bump、Study 卡需重審。
- **乙案：書房維持工作位，辦公室只收公司特有題**
  - `Study` 不動；`Office` 只問公司才有的事（隔板高度、正對主管視線、背後走道人流）。
  - 題目少（約 3–4 題，且要有風水說法），使用者可能困惑「工作位」與「辦公室」差在哪。

### C.3 共同規則
- 新增 `Room::Office`（wire `office`、題目前綴 `office_`、標籤「辦公室座位」），
  補齊 `ALL`／`wire`／`from_wire`／`label` 與 id 前綴 match。
- **只收有風水說法的題目**（v1 兩欄並陳結構：風水說法 vs 研究怎麼說）。冷氣出風口、
  鄰座噪音這類沒有風水說法的題目 v1 不收。
- 研究：每題找文獻寫進 `docs/superpowers/reports/fengshui-research/evidence-ledger.md`；
  `direct` 徽章只能引 `full_text_read=true`；**沒有能原文核對的研究就標「沒人測過」，不硬湊**。
- 調整建議沿用「不花錢、不搬重物」，加辦公室限制語「公司座位通常不能自己換，
  以桌面上能動的東西為主」。
- 人工逐卡審：Claude → 站長（含手機上確認徽章不會被讀成「這條風水有研究支持」）。
- 網址：新增 `/fengshui/:room`（`bedroom|study|office`）直達；非法 room → 轉址 `/fengshui`。
  `/fengshui` 仍是選單。
- 程式：`ruleset.rs` 題目、`corpus.rs` 卡片、`fengshui.rs` `figure()` 俯視圖
  （新圖元：隔板、鄰座桌、主管座位）、清除確認文案「兩個房型」→「所有房型」、
  `.fs-rooms` 改自動換行三欄。
- 版本：RULESET 1→2、CORPUS 3→4；T1／T2／id 前綴測試同步。
- 首頁風水卡說明改「臥室、書房或辦公室座位…」；新詞（例「藏風」）進 glossary 則一起 bump。

---

## 4. 首頁與導覽
- 首頁三張主單元卡（紫微、塔羅、風水）連到各自子單元選單（`/ziwei`、`/tarot`、`/fengshui`）；
  紫微卡由 `<div>` 改 `<A>`（A.2）。
- 子單元選單一律用既有 `.feature` 金邊卡，不另設計新元件。

## 5. 平行開工的衝突點與順序
| 共用檔案 | 軌道 | 處理 |
|---|---|---|
| `crates/web/src/lib.rs`（路由） | A B C | 各軌只加自己的 Route 行；整合者合併並確認 `/divination/ziwei` 在 `:type` 之前 |
| `crates/web/src/pages/home.rs`、`components/layout.rs` | A B C | 整合階段一次改 |
| `crates/api/src/services/ai/prompts.rs` | A B | A 新增函式＋中文清單、B 改 tarot 函式；A 也改 ziwei/story 性別 |
| `crates/api/src/durable_objects/ai_mutex_do.rs` | A | 只 A 動 |
| `crates/api/src/routes/mod.rs`、`routes/users.rs` | A | 只 A 動（B 無新路由前綴） |
| `crates/schema/src/lib.rs`（mod 宣告） | A B | 各加一行 |
| `crates/web/src/components/palace_grid.rs` | A | 新增選用 prop，命盤頁呼叫端不變 |
| `scripts/schema.sql`、`scripts/migrations/` | A B | 各自新段落／新檔 |
| `glossary.rs`／`GLOSSARY_VERSION` | A C | 整合時合併成一次 bump |
| `style.css`（新徽章色、`.fs-rooms`） | A C | 色值只進 `:root`；整合時跑 css-guard／contrast-check |

**部署順序**：Turso migration（A、B）→ engine（A）→ api → web。
web 對舊 API 相容：topic 省略即 general；`/ziwei/luck`、`/ziwei/year` 在 API 404 時顯示
「服務準備中」不白屏。C 只有 web。

**驗收**
- `cargo fmt --check`、`cargo test`（native 五 crate）、`cargo clippy --target wasm32-unknown-unknown --all-targets`、
  web build、`css-guard.py --allow-local-vars`、`contrast-check.py` 全過。
- A：golden＋一組人工外部核對（附錄 A）；`verify-deployment.sh` 把
  `/api/ziwei/horoscope`、`/api/ziwei/year`、`/api/ziwei/year/interpret` 加進 401 探測清單
  （同 tarot 8b 段），並直打 `ENGINE_URL/engine/ziwei/horoscope` 做一組 golden 比對。
- B：general prompt golden；舊 web 不帶 topic 仍可抽牌、舊列 NULL 顯示綜合。
- C：T1/T2/T3；站長逐卡審過才部署。
- 正式站瀏覽器走一遍六個子單元（手機寬度＋桌面）；無時辰帳號走一遍 A 的擋板。

## 6. Approaches（考慮過的做法）
- **紫微流年資料放哪**：(a) 擴充 `ZiWeiChartV3` → 要 bump `ENGINE_VERSION_ZIWEI`，
  清全站命盤與 AI 快取，且一張盤只能帶一個年份，否決；(b) **獨立 horoscope 端點與 DTO（採用）**；
  (c) 前端自算 → 農曆年界與四化表重寫一份、與引擎不同源，否決。
- **今年運勢 AI 走既有 `/api/charts/:type/interpret`**：global DO、fail-open 限流、無 prompt
  版本，不符 tarot／zodiac 之後的 AI 成本紅線，否決；改新路由（採用）。
- **塔羅主題**：(a) **topic 欄位＋主題牌位引言＋AI 框架（採用）**；(b) 78 張×2 主題牌義
  → 312 段新文案，列 v1.1；(c) 主題當新 spread 值 → 要重建表放寬 CHECK，否決。
- **辦公室風水**：見 C.2 甲／乙案；第三種「擴充書房題組」→ 在家與公司能調整的事不同，否決。

## 7. 留給實作者的事
- 各頁排版、CSS class（沿用 `fs-*`／`zc-*`；紫微新前綴 `zw-*`）。
- 規則說明與主題引言的實際措辭（受禁字測試＋Claude 逐條審）。
- 辦公室題目最終清單（由研究帳本決定）。

## 8. 待站長裁示
1. **辦公室與書房**：甲案（在家／公司拆開，書房改「書房（在家）」）還是乙案（書房維持工作位，
   辦公室只收公司特有題）？Claude 推薦甲。
2. **沒填出生時辰**：流年大運與今年運勢直接擋住並引導補填（Claude 推薦），還是只顯示不依賴
   時辰的部分？
3. **今年運勢 AI 次數**：每人每天 3 次，與紫微命盤 AI 分開計——可以嗎？
4. **塔羅感情／事業 v1** 只換牌位引言＋AI 框架，不寫 78 張逐張主題牌義——可以嗎？

## 9. 審查紀錄
- 外部審查：Codex（額度至 11/25）、Gemini（429 額度，約 105h 後重置）、Grok（未登入）
  皆無法執行；改由全新 Claude 代理做獨立審（只給 spec 與程式碼路徑）。
- 獨立審 R01–R24：Claude 抽查 R01、R02、R03、R04、R05、R10、R12 對照程式碼——
  R10 部分不實（首頁**有**紫微卡，但是不可點的 div），其餘屬實。全部 24 條已吸收進 v2；
  R12、R04 屬產品決定，列 §8-1、§8-2。

## 附錄 A：流年外部核對紀錄
（實作時填：生辰、target、x-iztro 輸出、外部來源輸出、是否一致）
