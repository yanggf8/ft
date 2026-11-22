# 空間風水結果頁「你的房間」縮景（sprite stacking）— 設計 v1

日期：2026-11-22　狀態：設計 v1.1（自審修：書房底座牆、書房光線對照、產生檔 fmt 與一致性檢查）
來源：站長 2026-11-22「B 做吧，風水結果頁也放房間縮景」（cort question 19c9f831 選 B）。

## 1. 目標／非目標

**目標**：結果頁頂端放一個依使用者答案組出的自轉房間縮景，讓人一眼看到「自己描述的房間」：
床或座位靠什麼、門在哪、頭頂有什麼、有沒有柱角、雜物多寡。

**非目標**：
- 不標示好壞（不畫紅框、不寫「煞」）：縮景只重現使用者的描述；評論仍在下方翻卡。
  「先猜再翻」的遊戲不受影響（縮景不透露研究立場）。
- 不畫任何風水物件、不建議擺設；「可以試試看」仍只在卡片裡。
- 不做可拖曳旋轉、不做縮放（v2 再議）。
- 選房間按鈕上的小縮景不變（固定樣板，`rooms.py`）。

## 2. 作法：預產零件切片＋前端依答案組合

答案組合太多，無法預產整間房；改成**每個可視選項一個零件**，零件都在同一個房型座標系、
同一套層高，前端把選中的零件按層合併疊起來：

- 層 z 的 `.stack-layer` 裡放所有選中零件在 z 層的切片（多張 `<img>` 疊在同一層 div），
  因此不同零件之間的前後遮擋是正確的（sprite stacking 依層高繪製）。
- 每個零件只輸出有像素的層；清單由產生器寫成 Rust 常數（見 §3）。

### 2.1 房型標準佈局（格數 N=40，牆只畫後牆 y 小、左牆 x 小，矮牆避免遮擋）

**臥室**：床頭朝後牆（y 小）、床居中偏左。
| 題目 | 選項 → 零件 |
|---|---|
| 一律 | `base`：地板＋矮後牆＋矮左牆；`bed`（靠牆位置） |
| bed_back | wall→（無額外）；window→`bed-back-window`（床頭後牆上的窗）；passage→`bed-back-door`（床頭後牆的門洞）；floating→ `bed` 換成 `bed-floating`（床離牆，床頭後是空地） |
| bed_door_view | visible→`door-diag`（右前方門框）；facing→`door-foot`（床尾正前方門框）；hidden→`door-behind`（左牆近床頭處的門） |
| bed_overhead | beam→`beam`（床上方橫樑）；heavy→`shelf-over`（床頭上方吊櫃） |
| bed_mirror | visible→`mirror`（左牆面向床的立鏡） |
| bed_corner | present→`corner`（尖角朝床的方柱） |
| bed_door_door | facing→`door-opposite`（與房門相對的另一扇門框） |
| bed_light | leak→`window-glow`（右側窗＋暖光地面光斑） |
| bed_clutter | some→`clutter-some`（2 堆箱子）；lots→`clutter-lots`（5 堆） |

**書房（在家）**：書桌與椅子居中，椅背方向＝人背後（y 大＝背後）。
| 題目 | 選項 → 零件 |
|---|---|
| 一律 | `base`（地板＋矮左牆；前後牆由題目零件決定）、`desk`、`chair` |
| seat_back | wall→`back-wall`（椅後矮牆）；window→`back-window`；passage→`back-door` |
| seat_door_view | visible→`door-diag`；facing→`door-front`（桌前正對門）；hidden→`door-behind` |
| seat_overhead | beam→`beam` |
| seat_front | open→`front-window`（桌前牆上的窗）；wall→`front-wall`（桌前貼近的高牆） |
| seat_corner | present→`corner` |
| seat_light | enough→`sun-patch`（桌面與地面的暖光斑）；dim→ 無 |
| seat_clutter | some→`clutter-some`；lots→`clutter-lots` |

**辦公室座位**：桌子居中，人坐 y 大側。
| 題目 | 選項 → 零件 |
|---|---|
| 一律 | `base`（格紋地毯，無牆）、`desk`（含螢幕）、`chair` |
| office_back | wall→`back-wall`（高櫃）；aisle→`back-aisle`（黃色走道線＋路人方塊）；low→`back-low`（矮隔板＋後面的空桌） |
| office_door | entrance→`door-front`（前方出入口門框）；boss→`boss-desk`（前方較大的桌子） |
| office_front | open→`front-open`（前方窗）；close→`front-partition`（緊貼的高隔板） |
| office_overhead | beam→`beam`；lamp→`lamp-tube`（頭頂長條燈管） |
| office_corner | present→`corner` |
| office_light | enough→`window-side`（側邊窗）；dim→ 無 |
| office_clutter | some→`clutter-some`；lots→`clutter-lots` |

`unknown`（不確定）與未作答：不畫該題零件。缺 `bed` 互斥：`floating` 時用 `bed-floating`
取代 `bed`（零件函式處理，不會同時出現）。

## 3. 程式結構

- **產生器** `scripts/sprite-stack/fengshui_parts.py`：
  定義三房型所有零件的體素模型（盒子拼法，同 `rooms.py` 風格），輸出
  `crates/web/assets/stack/fs/<room>/<part>/NN.webp`（只輸出非空層，NN＝全域層號），
  並**產生** `crates/schema/src/fengshui/diorama_parts.rs`（檔頭註明「由產生器產生，勿手改」）：
  `pub const DIORAMA_GRID: usize = 40;`、`pub struct DioramaPart { pub room: Room, pub id: &'static str, pub layers: &'static [u8] }`、
  `pub const DIORAMA_PARTS: &[DioramaPart]`。產生後必跑 `cargo fmt --all`（CI 閘 fmt --check）。
  零件切片不得有「全空層」以外的空檔（零件層號清單即實際檔案；schema 測試不讀檔，
  `scripts/sprite-stack/fengshui_parts.py --check` 比對檔案與常數一致）。
- **schema 純邏輯** `crates/schema/src/fengshui/diorama.rs`：
  `pub fn diorama_parts(room: Room, answers: &BTreeMap<String, String>) -> Vec<&'static DioramaPart>`
  依 §2.1 表；回傳順序固定（base 先）。`pub const DIORAMA_VERSION: &str = "1"`（改零件或對照必 bump；
  目前僅記錄用）。測試：
  1. 每個房型每題每個非 unknown 選項，回傳的零件都存在於 `DIORAMA_PARTS`（id＋room）。
  2. `unknown`／缺答只有 base 與固定家具。
  3. floating 時沒有 `bed`、有 `bed-floating`。
  4. 所有 `QUESTIONS` 的題目 id 都在對照表裡（新增題目沒對照會紅燈）。
- **web**：`components/sprite_stack.rs` 新增 `SpriteStackParts { base: String, parts: Vec<(&'static str, &'static [u8])>, grid, label, class }`：
  層 z 從 0 到所有零件最大層，每層 div 內放有該層的零件 `<img src="{base}/{part}/{z:02}.webp">`，
  同樣每層兩份（整格＋半格）。既有 `SpriteStack` 不動。
- **結果頁**（`pages/fengshui.rs` 結果區頂端，`fs-result-head` 之後、卡片之前）：
  `<figure class="fs-diorama">` 內含縮景（max 320px、置中）＋ `<figcaption>`：
  「依你的答案組出的示意圖（不確定的題目不畫）」。`aria-label`＝「{房型}示意縮景」。
  答案變動（修改答案後回結果）重新組合——用答案訊號計算。
- 減少動態效果：沿用 `.stack-layer img` 規則停在 30°。

## 4. 成本與效能

- 零件切片預估：每房型 15–18 個零件、每件 3–14 層，總量 < 600 KB（lossless webp，多數層 < 2 KB）。
- 頁面 `<img>` 數：每層平均 4–6 張 × 2 份 × ~18 層 ≈ 150–220。全部是同一動畫（合成器層 transform），
  手機可接受；若量測卡頓，v1.1 改成只在縮景可見時才播（IntersectionObserver）。
- 圖片 `loading="lazy"`；縮景在結果頁頂端，首次進入即載入。

## 5. 驗收

- `cargo test -p ft-schema`（新 diorama 測試）、fmt、clippy wasm32、build-web、css-guard、contrast-check。
- 預覽部署（Pages `--branch` 預覽，非正式）以三房型各一組典型答案截圖檢查：零件位置合理、
  沒有穿模、遮擋正確、手機 420px 版面。
- 正式部署後在 ahexagram.com 再看一次（有快取的瀏覽器）。

## 6. 部署

純前端＋schema（不動 API／DB）：build → deploy-web。

## 7. 獨立審查修正（v1.2，2026-11-22；1 blocker＋7 should-fix＋4 nit 全採納，**本節優先於上文**）

1. **（blocker）層內多圖**：結構改為 `.stack-stage > .stack-layer（translate＋scaleY）> .stack-spin（自轉動畫）> img（position:absolute; inset:0）`。
   動畫只掛在 `.stack-spin`（每層一個，2×L 個），不掛在 img。既有 `SpriteStack` 一併改用此結構
   （減少動態效果規則改套 `.stack-spin`）。
2. **效能**：同上，合成層降為 2×L；預覽部署時用手機寬度量測（Performance 面板或至少目測流暢）。
3. **零件重疊**：零件一律放在互不重疊的位置（座標規劃見產生器檔頭表）；產生器 `--check` 檢查
   「不同題目、可同時被選中」的零件兩兩無共用體素（同題互斥選項不檢查）。仍需同牆的就做合併零件
   （例：門嵌在牆段裡）。零件有固定繪製順序（`DIORAMA_PARTS` 順序）。
4. **臥室床頭後牆**：底座不含床頭後的牆段；`bed_back` 的 wall／window／passage 各帶自己的牆段
   （`bed-back-wall`／`bed-back-window`／`bed-back-door`），unknown 時床頭後空著。底座只留非床頭處的矮牆。
5. **door-opposite**：依房門位置出三個變體（`door-opposite-diag`／`-foot`／`-behind`）；房門題 unknown 時不畫。
6. **陰影與接縫**：明暗用全域固定 `TOP`（所有房型同一值），邊緣加深只看同一零件內的鄰格，接受零件交界的細線。
7. **只畫答案說的東西**：seat_front／office_front 的 open 改成「前方空地」——畫一小塊淺色地墊，不畫窗；
   會出現在沒牆一側的窗（bed_light leak、office_light enough）零件自帶一段矮牆。leak 的光用中性淺色，不用暖色。
8. **測試**：對照表改成常數 `MAPPING: &[(&str /*qid*/, &str /*oid*/, &[&str] /*parts*/)]`，畫不出東西的選項
   寫空陣列。測試：表內 (qid, oid) 都存在於 `QUESTIONS`；`QUESTIONS` 每個非 unknown 選項都在表內；
   `DIORAMA_PARTS` 每個零件至少被一個答案或固定家具用到；表內零件都存在。`--check` 需要 Python＋PIL，
   CI 沒有，屬手動步驟（產生後必跑）。產生器輸出 rustfmt 穩定格式（產生後 `cargo fmt` 不應改動）。
9. figcaption 改為「依你的答案組出的示意圖：只畫你描述到的東西，『不確定』和『沒有』不畫」。
10. 配色中性可愛：樑、柱角、雜物不得用深色或紅色，避免暗示好壞、影響先猜。
11. `DioramaPart` 加 `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`；`room_stack_dims` 也改由 `rooms.py`
    產生常數（`crates/web/src/pages/room_stacks.rs` 或同檔常數），消除手動同步。
