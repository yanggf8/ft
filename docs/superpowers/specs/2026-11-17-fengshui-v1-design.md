# 風水 v1 設計(A only:命卦+個人八方)— v2,2026-11-17

上游:2026-11-02 三審完成的研究(codex 報告→Claude 交叉查證→Opus 終審;四檔在
NUC,**收進 repo 是 Batch 1 硬前置**,見 §0)。方向框架:2026-11-17-talent-direction.md
v3。流程:本 spec→自審→Kimi K3.1 Pro 審→Claude 核→站長核→多 AI 開工。
v2:吸收 Kimi 審(docs/superpowers/reports/2026-11-17-fengshui-kimi-review.md,
32 findings,verdict 需重改)經 Claude 核後的修正;站長 2026-11-17 裁示:分鐘碰立春
標不確定、v1 不記住生日、RIASEC 下架獨立部署。

## 0. 事實地基與 normative 來源

- **v1 = A only**:命卦+個人八方。B(八宅:宅卦配命卦)另立 spec;年紫白備胎不動。
- 三審查證結論:命卦分世紀公式 ≡ 四位數字和版 mod 9(1900–2099 代數等價、無分歧
  年份);男命遇 5 寄坤、女命遇 5 寄艮(各派一致);換年界立春為主流,《八宅明鏡》
  原文按農曆年(須並陳揭露);乾命八方合大遊年歌;2024 立春為 **2024-02-04
  16:26:53**(UTC+8,紫金山;新華社 16:27 為進位版)。
- **Normative 來源(F02)**:研究四檔收進 `docs/superpowers/reports/fengshui-research/`
  並 commit,是 Batch 1 開工前的硬前置(未入庫不得開工)。入庫時同步在本 spec
  **附錄 A** 補上可審查的定案文本:命卦公式(含世紀別、mod 9 餘數映射、餘 0 處理、
  寄卦)與大遊年 **完整 8×8 表**。附錄 A 為唯一 normative 表;實作與測試 fixture
  皆以它為準,`mingli-luoshu` 只當交叉比對 oracle(dev-dependency),不是規範來源。

## 1. 定位(掛統一框架 v3)

- **行動面、素材層、迴圈入口(非閉環)**(F07):v1 只提供「傳統配置說法+觀察
  提示」,不記錄、不對答案、不更新——它是統一迴圈的**入口**,不是完整迴圈。
  文案不得暗示閉環或效度。
- 使用者的實際感受**只用來調整自己的空間安排,不用來證明或反證風水**。
- 素材層內容只允許兩類(方向 v3 §2):**傳統來源描述**(「傳統上認為…」)與
  **個人反思提示**;禁止**效果宣稱**(「這樣擺會改善專注」)。
- 隱性 PP:介面不說理論。對外名稱:「風水·個人八方」。

## 2. 邊界紅線(頂層;每條附驗收方式)

1. **零持久化**:輸入(出生日期/時刻/公式別)與結果不寫入任何儲存——DB、KV、
   瀏覽器 localStorage、分析平台、錯誤監控。風水資料永不寫入 predictions /
   prediction_feedback / f8_assignments(route 檔頭註明)。
   驗收:T6(row-count 差值)+route 拿不到 DB client(型別隔離)+靜態審查
   (無 analytics/log 呼叫帶輸入)。
2. **換年派別並陳(揭露式)**:主判定用立春時刻;出生日落在 **1/21–2/20(含首尾,
   UTC+8 國曆日期)** 時,回應帶 `disclosures: ["school_boundary"]`,web 顯示:
   「傳統另有派別以農曆年(正月初一)為界;冬季換年期間出生者,兩派年命認定可能
   不同,本站以立春為準。」(F06:正月初一國曆範圍 1/21–2/20、立春 2/3–2/5,
   此區間為所有可能分歧日的保守超集,不需農曆表;文案只說「可能」,不暗示帶內
   每日必然不同。精確到某年某日才需正月初一表,v1 不做。)驗收:T4。
3. **公式明選、不預選**:UI 以「男命公式/女命公式」radio 明選,無預設值;**不從
   profile gender 自動帶入**;radio 旁常駐說明:「這是傳統公式的歷史命名,不代表或
   推定你的性別;請選擇要使用的算法。」wire 值凍結為 `"male"|"female"`。缺省→400。
   驗收:Web E2E(初次載入無選取、登入有 gender 的帳號亦無選取)+T-API。
4. **範圍外回 400**:公開輸入範圍 1900-01-01 00:00 至 2099-12-31 23:59(UTC+8);
   不 `unwrap_or(fallback_year)`。內部 `effectiveYear` 範圍 **1899–2099**(1900 年
   立春前出生者用 1899;F01)。驗收:T1/T3/T4。
5. **吉凶呈現對稱**(F08):八星採**同一卡片結構**——標題一律用方位主題(中性),
   傳統名一律放在「傳統名」欄,八星同款小註「古籍用語,非吉凶保證」。目標是
   **降低恐懼措辭,不是隱藏傳統的吉凶評價**(傳統名本身仍可辨識,這是誠實揭露)。
   「裸出現」定義:傳統名出現在標題或一般內文=違規;出現在「傳統名」欄=合法。
   驗收:T5 結構化欄位掃描(非 substring)。
6. **內容安全**(由 §5 提升為頂層):禁必然化/保證(「一定」「必」「保證」「招財」
   「剋」;英文 will/guaranteed/ensure 等)、禁科學背書(研究/大腦/統計/科學;
   research/science/proven/study)、禁醫療/法律/財務斷言、禁恐懼式行銷與付費化解
   (「破解」「化煞商品」等)。傳統古籍出處(如《八宅明鏡》)屬來源描述,允許。
   驗收:T5 雙語禁詞掃描+站長語義審查(掃描不能取代)。
7. **v1 無 AI**;日後接 AI 時 GUARDRAIL 逐字適用+語言測試守門(CLAUDE.md 規則)。

## 3. 立春表閉環(前置工程 1)

- **表格式**(F04,v2 改):`LI_CHUN_UNIX_1900_2099: [i64; 200]`——每年立春 instant 的
  Unix 秒(UTC),整數避免浮點;取代研究期命名 `LI_CHUN_JD_*`。
- **產生器**:`crates/domain/western` 新 example `gen_lichun`,以既有
  `sun_apparent_ecliptic`(VSOP2013,**of-date 視黃經,TT 時間尺度**)二分求根太陽
  視黃經 = 315°,再以 **ΔT 模型(Espenak–Meeus 多項式)** 由 TT 轉 UTC,輸出表。
  產生器檔頭固定上述參數;產生器版本與 ΔT 模型名納入 `FENGSHUI_RULESET_VERSION`
  的語義(改任一者必 bump)。表為產生物,禁止手改。
- **驗證**(取代「抽樣 ≤1 分鐘」):
  1. **日期級全量**:200 年立春日期(UTC+8)對照香港天文台公曆農曆對照表節氣日期,
     逐年一致。
  2. **時刻級**:已發布的過去年份(至少 1950–2026 中 10 個分散年份,含 2024)對照
     紫金山天文台/中央氣象署天文年曆時刻,容差 ≤60 秒。
  3. **未來年份誠實**:ΔT 外推使未來年份誤差可能達分鐘級——由 §3 的「不確定窗」
     吸收,並在 spec/ARCHITECTURE 記載,不宣稱秒級精確。
- **時區語意**(F05):日期與時刻一律以 **UTC+8 國曆 civil date/time** 解讀;wire
  是純字串(`"YYYY-MM-DD"`、`"HH:MM"`),前端**不得**經瀏覽器本地時區解析
  (禁 `new Date("YYYY-MM-DD")` 類轉換)。非 UTC+8 出生者:v1 不支援自動換算,表單
  常駐說明「請以台灣時間(UTC+8)輸入;在其他時區出生者請先自行換算」。移除
  內部 `timezone: Option` 預留(不做假的未來相容)。
- **不確定窗**(F03,站長裁示):判定出生 instant 與該年立春 instant 的先後時,
  - 無時刻且出生日 = 立春日 → 不確定(reason `missing_time`)。
  - 有時刻:出生分鐘涵蓋 `[m, m+60s)`;若與 `[立春−60s, 立春+60s]` 重疊 → 不確定
    (reason `boundary_minute`;同時吸收分鐘精度與產生器 ≤60s 容差)。
  - 其餘情形一律確定(無時刻但非立春日 → **確定**,不得標不確定)。

## 4. 純邏輯(crates/schema/src/fengshui/)

- `mingua.rs`:`effectiveYear`(1899–2099)+公式別 → 命卦(附錄 A 公式;含寄卦)。
- `dayou.rs`:附錄 A 大遊年 8×8 表(命卦×八方 → 八星 id)。
- `lichun.rs`:`LI_CHUN_UNIX_1900_2099` 查詢+§3 判定(回 certain(effectiveYear) |
  uncertain(reason, [前年, 後年]))。
- `mod.rs`:`FENGSHUI_RULESET_VERSION`(公式+歌訣+判定規則+立春表產生器/ΔT 模型)/
  `FENGSHUI_CORPUS_VERSION`(語料);型別:八卦、八方位、八星 id。邏輯層不帶
  「吉/凶」字串。
- **不依賴 domain**(schema 純邏輯規約);產生器在 domain example,產出物複製進 schema。

## 5. 語料(前置工程 2,時程主力)

八星 × 雙語(中/英),每星每語四欄,共 64 欄,schema 固定:

1. `title`:方位主題式中性標題(八星同規則)。
2. `body`:傳統說法釋義,中 80–120 字/英 50–90 words;只寫「傳統上認為…」的
   來源描述,不寫效果宣稱。
3. `reflection`:恰 1 題純觀察提問(不預設好壞方向,例:「你在這個方位待著時,
   注意到什麼?」)。
4. `traditionalName`:傳統名(生氣/延年/天醫/伏位/絕命/五鬼/六煞/禍害)+共用小註。

共用文案(八卡之外):「提問只供你自己觀察,不會被上傳或記錄;不符合傳統說法的
經驗一樣值得留意。不需要為了驗證而勉強維持讓你不舒服的配置。」(F20)

**Batch 2 驗收**(F19,掃描通過≠內容通過):
- 每星傳統主題明確、八星彼此可區分,無通用空話(站長逐卡審)。
- 英文由具能力者校訂(非直譯);中英語義一致。
- 提問不預設好壞;無高風險建議;傳統名與來源性質一致。
- 通過 §2.5/§2.6 掃描(T5)。

## 6. API(crates/api/src/routes/fengshui.rs)

- **`POST /api/fengshui`**(F13:無狀態≠必須 GET;個資不進 URL):
  - body:`{ "date": "YYYY-MM-DD", "time": "HH:MM"?, "formula": "male"|"female" }`;
    先 `body_too_large`(64 KiB)再 `req.json()`;未知欄位拒絕。
  - 回應標頭:`Cache-Control: no-store`。
- **回應(discriminated,F11/F15)**:

```json
// 確定
{ "status": "certain",
  "effectiveYear": 1984,
  "lichunInstant": "1984-02-04T15:19:00Z",
  "mingua": "kan",
  "directions": [ { "compass": "n", "starId": "fuwei" }, ... 8 筆,固定順序 n,ne,e,se,s,sw,w,nw ],
  "disclosures": ["school_boundary"],
  "rulesetVersion": "1", "corpusVersion": "1" }

// 不確定:不得含單一 mingua/directions
{ "status": "uncertain",
  "reason": "missing_time" | "boundary_minute",
  "lichunInstant": "...Z",
  "candidates": [ { "effectiveYear": 1983, "mingua": "li" },
                  { "effectiveYear": 1984, "mingua": "kan" } ],
  "disclosures": [...], "rulesetVersion": "1", "corpusVersion": "1" }
```
  `lichunInstant` 為 RFC 3339 UTC(`Z`);`disclosures` 只回穩定 id,雙語文字由 web
  依 id 呈現;enum 值(八卦、八方、八星)凍結於 schema。(上例數值為格式示意,
  非 fixture。)
- **錯誤**(F14):`{ "code": "...", "message": "..." }`(message generic,不洩 parser/
  domain/DB 細節):`INVALID_DATE`(格式/不存在日期/範圍外)、`INVALID_TIME`
  (格式、24:00、越界)、`INVALID_FORMULA`(缺省或未知值)、`INVALID_BODY`(非 JSON、
  未知欄位、型別錯)、`body_too_large` 照既有慣例。
- 限流:比照 chart 類一般端點(無特例、無放寬)。
- **為何走 API**(F16,v2 改寫):web 與 API 共用 schema crate,本來就沒有實作漂移
  問題;走 API 的實際收益是 **`verify-deployment.sh` 可對正式環境做計算正確性
  probe**,以及與 chart 類功能架構一致、未來其他 client 共用。成本:多一次網路
  往返與版本配對(下條處理);個資風險由 POST+no-store+零持久化處理。
- **版本配對 fail-closed**(F12):web 編譯時固定 expected `rulesetVersion`/
  `corpusVersion`(取自同一 schema 常數);回應任一不符 → **不顯示結果**,提示
  「網站剛更新,請重新整理頁面」。v1 首次部署無舊 web 頁,無窗口;日後 bump 時
  API→web 背靠背部署,窗口內舊 web 一律 fail-closed(可接受)。

## 7. Web(crates/web)

- 新頁 `/fengshui`:首頁卡入口(比照塔羅);不綁帳號。
- 表單:出生日期(必)、時刻(選)、公式別(§2.3);UTC+8 說明(§3)。
  **v1 不記住任何輸入**(F26,站長裁示):無 localStorage/sessionStorage。
- 結果(certain):命卦卡(卦名+一句中性定位)+**八方文字列表**(必備,8 列固定
  順序)+八方圖(上南下北,圖側註明「上方為南」;上北切換延後)+點開方位卡
  (§5 四欄)。
- 結果(uncertain):不顯示單一命卦與八方;說明原因(`missing_time`:「你的生日剛好
  是立春,請補出生時刻」/`boundary_minute`:「出生時間非常接近立春交接,無法判定
  屬哪一年」)+並列兩個候選命卦名(不展開八方)。
- 揭露帶:`school_boundary`(§2.2)、素材層定位一句:「本功能整理傳統風水的說法,
  供你觀察自己的空間感受;實際感受可作為你調整空間的個人參考,不用來證明或反證
  風水。」(F07)
- 版本不符 fail-closed(§6)。

## 8. 測項(TDD;native)

- **T1 命卦**:effectiveYear 1899–2099 全年份×兩公式,對附錄 A 公式獨立 fixture
  逐年斷言,並與 mingli-luoshu oracle 交叉比對(分歧即停工查原因);寄卦專測。
  1899 若 oracle 與附錄 A 分歧且無法裁定 → 改公開下限為 1900 年立春 instant(F01 退路)。
- **T2 大遊年**:附錄 A 8×8 全 64 格逐格斷言;oracle 交叉比對。
- **T3 立春表**:長度 200、嚴格遞增;日期級全量 vs 香港天文台(§3);時刻級 10 年
  vs 天文年曆 ≤60s;2024 = 2024-02-04 16:26:53 UTC+8;1900 與 2099 兩端存在。
- **T4 判定與揭露**:
  - 1900-01-01(→1899)、1900 立春前一日/後一日、2099-12-31 23:59(→2099);
    1899-12-31、2100-01-01 → `INVALID_DATE`。
  - 立春 instant 前 61 秒、前 1 秒、當秒、後 1 秒、後 61 秒(內部 instant 層);
    API 層:立春所在分鐘、前一分鐘、後一分鐘、前兩分鐘、後兩分鐘的 uncertain 開關。
  - 立春日無時刻 → uncertain(`missing_time`);**非立春日無時刻 → certain**。
  - uncertain 回應**不含** `mingua`/`directions`,含兩個 candidates。
  - 揭露開關:1/20 無、1/21 有、2/20 有、2/21 無;正月初一先於立春之年
    (例 2014,正月初一 1/31)與後於立春之年(例 2024,正月初一 2/10)各一組。
- **T5 語料**:64 欄齊全與長度;結構化掃描:傳統名只在 `traditionalName` 欄;雙語
  禁詞(大小寫、全半形、詞形)正反 fixture(必攔/必過各至少 5 例)。
- **T6 隔離**:全流程前後 predictions/prediction_feedback/f8_assignments row count
  差值為 0(非絕對為空);route 型別上取不到 DB client。
- **T7 無狀態**:同輸入逐位一致;不同輸入交錯與並行請求互不串;回應帶
  `Cache-Control: no-store`。
- **T8 API 契約**:錯誤碼全表(含 24:00、2023-02-29、缺 formula、未知欄位、非 JSON、
  超 64 KiB);錯誤 message 不含內部細節。
- **Web E2E**:certain 全流程(八方列表+圖+方位卡+揭露帶)、uncertain 兩種原因
  不顯示單一命卦、公式無預選(含已登入帳號)、版本不符 fail-closed(攔截改回應
  版本)、400 顯示、重新整理後無殘留輸入、非 UTC+8 瀏覽器時區下日期不位移、
  CSP 無違規、中英 DOM 禁詞掃描。
- **verify-deployment.sh probe**(F17):固定合成測試向量(非真實個資):一個 certain
  完整斷言(命卦+8 方)、一個 uncertain(`boundary_minute`)、一個範圍外 400、
  版本欄位等於部署 expected、`no-store` 標頭存在。

## 9. 執行批次與部署

| 批 | 內容 | 驗收閘 |
|----|------|--------|
| 前置 | 研究四檔入庫+附錄 A(公式+8×8 表)| 站長確認附錄 A |
| 1 | schema 純邏輯+立春產生器+表+驗證 fixture | T1–T4(邏輯層)|
| 2 | 語料 64 欄 | T5+站長逐卡審+分層 release gate(方向 v3 §5)|
| 3 | API 端點+verify probes | T4(API 層)/T6–T8 |
| 4 | web 頁+E2E+ARCHITECTURE.md 章節 | §8 Web E2E |

每批 fmt+test+clippy(wasm32)過才進下一批。部署:API→web 背靠背。
**RIASEC 下架獨立部署**(F25,站長裁示):不併本線窗口,單獨一次變更、單獨 verify、
單獨回退;規格見原 interest spec 處置段(入口移除+作答端點 410+history/DELETE 保留)。

## 10. 已定案(原開放問題)

1. 揭露範圍:1/21–2/20(§2.2)。2. 上北切換:延後;文字列表必備(§7)。
3. wire 命名:`male|female` 凍結+UI 說明(§2.3)。4. 語料文化敏感度:§5 驗收+站長
逐卡審。5. 生日留存:v1 不留(§7)。

## 附錄 A(待前置批次填入)

命卦公式定案文本、mod 9 餘數→卦映射、餘 0 處理、寄卦規則、大遊年 8×8 全表——
自研究四檔轉錄,站長確認後凍結。
