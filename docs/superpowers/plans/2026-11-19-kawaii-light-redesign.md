# 全站換色 kawaii 亮底 ＋ 星座頁追上 zodiac1 — 實作計劃 v3

日期：2026-11-19　狀態：**v3 已核准（2026-11-19 站長裁示 §2.5 三項全收、同意本機預覽開 server），實作中**（流程：Claude 自審 → Kimi → Claude 核實 → Fable 5.2 → Claude 核實 → 站長 → 多 agent 實作）

## 0. 站長裁示

- 2026-11-19：全站顏色**與**風格都換掉，不再用 cosmic-silver（深色星空藍＋銀色鏡面玻璃）。
- 方向＝比較稿 **A 亮底**（https://claude.ai/artifact/5EzsDLVxEkDpbeoaqmguDv）：奶粉色底、白卡、紫色主色、粉色強調、圓角、立體按鈕。
- 最優先：**換色**＋**星座頁追上 `zodiac1`**（`/home/yanggf/a/zodiac1`：3D 翻牌、十二星座各一組漸層、立體圓鈕）。
- 需要產圖時用 `m4m`（`~/.local/bin/m4m`）。

## 1. 現況盤點（2026-11-19 實查）

| 位置 | 內容 | 規模 |
|---|---|---|
| `crates/web/style.css` | 全站唯一樣式表，1315 行；開頭 1–249 行是 cosmic-silver 主題（token、星空、鏡面標題、glint 動畫） | token 28 個，寫死色值約 150 處 |
| `crates/web/index.html` | Google Fonts（Playfair Display＋Noto Serif TC）、星空三層 div、`galaxy.js` canvas | 23 行 |
| `crates/web/galaxy.js` | 銀河粒子動畫；CLAUDE.md 有一條 `for(;;)` 紅線 | `build-web.sh:75` 複製 |
| Rust 行內樣式 | `layout.rs` 16、`divination.rs` 15、`login.rs` 9、`personality.rs` 6、`profile.rs` 4、`admin.rs` 3、`lib.rs`／`palace_grid.rs`／`birth_form.rs` 各 1 | 多數是早期為白底寫的（下拉選單、星盤白卡），只有少數深底專用 |
| 西洋星盤 `divination.rs WesternView` | 太陽／月亮／上升三格（行內淡色卡）＋行星藥丸（`.star.main.on-light`）；星座名是英文 key（`leo`…），已有 `sign_symbol()` | 要登入（`/divination/western`） |
| 塔羅卡面 `crates/web/assets/tarot/` | 79 張 WebP，cosmic-silver 風格生成圖 | 換風格後會不搭 |
| 文件 | CLAUDE.md「色彩/對比紅線」「galaxy.js」兩條；ARCHITECTURE.md §Cosmic-silver theme；spec `2026-08-30-cosmic-silver-theme-design.md` | |

## 2. 設計決策

### 2.1 色彩 token（沿用名稱、改值）

沿用既有 token 名稱，讓約 200 處 `var(--…)` 自動換色、不必逐行改；新增少數 kawaii 專用 token。名稱語意不變（`--heading` 標題、`--silver-dim` 次要字、`--silver-faint` 最淡字），只是名字留著舊時代的「silver」——在檔頭註明，不另做改名（改名是純 churn）。

| token | 新值 | 用途 |
|---|---|---|
| `--void`／`--deep-space` | `#fff5fa`／`#f4eeff` | 頁底漸層兩端 |
| `--heading`／`--text` | `#2a1b3d`／`#3d2e50` | 標題／內文 |
| `--silver-dim`／`--silver-faint` | `#5c4b70`／`#6b5b7b` | 次要字／最淡字 |
| `--starlight` | `#7c3aed` | 強調（焦點框、重點數字、圖示） |
| `--rose` | `#be185d` | 錯誤、四化星 |
| `--glass-bg`／`--glass-border` | `#ffffff`／`#eadcf5` | 卡片底／框 |
| `--glass-bg-strong` | `#ffffff`（**不透明**，K01） | nav、sticky quiz、塔羅正面 |
| `--metal-hi`／`--metal-mid` | `#5b21b6`／`#6d28d9`（白底 9.0／7.1） | 可互動字與 hover（原為近白銀色，K01） |
| `--metal-lo` | `#8b6fd6`（白 3.93／`--void` 3.69／`--deep-space` 3.47／tint 3.55；F03。v2 的 `#9a7fe0` 在頁底只有 2.86–3.04） | **所有控制項邊框**（輸入框、選項、`.fs-room`、`.cta-alt`、`.nav-logout`、`birth_form` chip）；`--glass-border` 只留給卡片裝飾框 |
| `--metal-line`／`--metal-edge` | `#f0e4f7`／粉紫綠三色漸層 | 分隔線／卡片框 |
| 拆掉 `--ink-on-chrome`（K04） | 主紫板上用新 `--ink-on-primary #fff`；淺底（含 `.on-light`）改用 `--heading`／`--text`；全部引用遷移後刪除 | 避免一個 token 兩種相反語意 |
| 新增 `--success` | `#047857`（白 5.48、tint ≥4.9；K05） | 成功字（取代 `#34d399` 1.92） |
| 新增 `--primary`／`--primary-hover`／`--primary-deep` | `#7c3aed`／`#6d28d9`／`#5b21b6` | 主按鈕、選取態 |
| 新增 `--accent`／`--ink-on-accent` | `#ec4899`（白 3.53）／`#2a0f1e`（5.02）；F18，原 `#ff5fa2` 邊界對白 2.83 | 粉色強調鈕、NEW 徽章 |
| 新增 `--surface-tint` | `#f6f1ff` | 蓋牌區、次級底板 |
| 新增 `--sign-<12 星座>` | 見下方十二星座表（F01）；全部配深字 `--ink-on-sign #2a1b3d` | 星座牌、行星藥丸 |

| `--gen-title` | → `--primary-deep`（白 8.98；F05，`profile.rs:121/790` 仍引用） | 世代標題 |

十二星座漸層（F01；深端＝漸層起點與行星藥丸圓點，淺端＝漸層終點與藥丸底；淺端藥丸在白卡上必加 `--metal-lo` 框線）。天秤／天蠍／射手／摩羯／水瓶／雙魚已由 zodiac1 原值調亮以配深字：

| 星座 | 深端 | 淺端 | 深字 on 深端 | 深字 on 淺端 | zodiac1 原深端（深字） |
|---|---|---|---|---|---|
| 牡羊 `aries` | `#ff6f61` | `#ffb199` | 5.82 | 9.07 | `#ff6f61`（5.82） |
| 金牛 `taurus` | `#b5c334` | `#e0ffb3` | 8.18 | 14.46 | `#b5c334`（8.18） |
| 雙子 `gemini` | `#ffd700` | `#fff7b2` | 11.32 | 14.52 | `#ffd700`（11.32） |
| 巨蟹 `cancer` | `#00bcd4` | `#b2f7ff` | 6.91 | 13.34 | `#00bcd4`（6.91） |
| 獅子 `leo` | `#ff9800` | `#ffe0b2` | 7.37 | 12.52 | `#ff9800`（7.37） |
| 處女 `virgo` | `#8bc34a` | `#e6ffb2` | 7.56 | 14.60 | `#8bc34a`（7.56） |
| 天秤 `libra` | `#f472b6` | `#ffd1e8` | 6.00 | 11.75 | `#e91e63`（3.65） |
| 天蠍 `scorpio` | `#a98fe0` | `#e4d6ff` | 5.82 | 11.61 | `#673ab7`（2.17） |
| 射手 `sagittarius` | `#42a5f5` | `#b2d7ff` | 6.00 | 10.63 | `#2196f3`（5.08） |
| 摩羯 `capricorn` | `#a1887f` | `#e5d5cb` | 4.80 | 11.12 | `#795548`（2.42） |
| 水瓶 `aquarius` | `#26c6b0` | `#b2fff7` | 7.40 | 14.02 | `#00bfae`（6.85） |
| 雙魚 `pisces` | `#7f8fe6` | `#c9d4ff` | 5.27 | 10.85 | `#3f51b5`（2.31） |

**B1 交付物包含 `:root` 每個舊 token 的「新值／取代／刪除」完整對照表（K01）**，不得只靠 B2 掃 literal。

`--ink-on-chrome` 的 12 處 CSS＋1 處 Rust 逐處標註「板是紫還是淺」再遷移（F04）：紫板（`.btn-primary`、`.cta`、`.quiz-choice:checked`、`.fs-opt.selected`、`.fs-guess.selected`、`select option:checked`）→ `--ink-on-primary`；選取板上的內嵌 SVG（`.fs-fig .fs-hl` 原用 `--starlight`＝主紫，在紫板上 1.00）一律改 `--ink-on-primary`；淺板（`.quiz-no`、`birth_form.rs:182` chip）改淺紫底＋`--primary-deep` 字。`.on-light`／`.text-frame` 改為直接刪除（F19：唯一使用者 `divination.rs:255` 由 B4 重寫）。

刪除：`--ink-on-chrome`、`--text-muted-on-light`、`--chrome-mirror-lg/sm`、`--chrome-glint`、`--galaxy-y-offset`、`--metal-deep(-small)`、`--text-muted-on-dark`、`--error-on-*`、`--gen-title`（已查無引用者直接刪，有引用者改指新 token）。

### 2.2 風格

- 卡片：白底、`border-radius: 20px`（取消 tailfin 不對稱圓角）、2px 淡粉紫框、柔和紫色陰影；取消 `filter: brightness`。**`.card` 是內容容器，不加 hover 效果**；「上浮＋陰影加深」只給可點的 `a.feature`，且包在 `@media (hover: hover)`（F15）。
- 按鈕：主鈕紫底白字、全圓角、底部 4px 深紫立體陰影，按下沉 3px；次鈕白底紫框紫字。
- 標題：圓體實色（Latin/數字用 Google Fonts `Nunito`，中文走系統字 PingFang TC／Noto Sans TC／微軟正黑），取消鏡面漸層與 glint 動畫。不載入中文 webfont（體積大、M PLUS Rounded 1c 缺繁中字會混字型）。
- 選取態：紫底白字（取代「反相鉻板」）。
- 星空：移除星點、銀河 haze、`galaxy.js` 與其 canvas；頁底改兩團淡彩光暈。
- 動畫：保留風水翻牌、塔羅翻牌；尊重 `prefers-reduced-motion`。

### 2.3 星座頁追上 zodiac1（`WesternView`）

- 太陽／月亮／上升 → **三張 3D 翻牌**：正面＝白卡＋天體圖示（☀ 月 ↑ 以 inline SVG 畫，不用 emoji），點擊翻到背面＝該星座漸層＋星座符號（`♌︎` 加 U+FE0E 強制文字樣式）＋中文名＋**度數**（上升原本顯示 `asc.degree`，`divination.rs:245`，不得遺失；太陽／月亮度數在行星清單已有，K09）。
- 尺寸：牌面絕對定位，容器必須有明確 `aspect-ratio`／`min-height`（比照 `.tarot-card` `style.css:849`），點擊區 ≥44px（K08）。
- 可及性（K06）：真 `<button>`＋`aria-pressed`；固定的視覺隱藏文字「太陽星座：獅子座 23.4°」當無障礙名稱，兩個視覺面都 `aria-hidden`、inline SVG `aria-hidden`；`:focus-visible` 樣式比照塔羅（`style.css:865-868`）；驗收用 Tab／Enter／Space 與無障礙樹檢查。
- reduced-motion（K07）：`@media (prefers-reduced-motion: reduce)` 一律放在**被覆寫規則之後**（現行 `style.css:239-249` 寫在 `.tarot-inner` 基本規則 `:858` 之前，同權重後者勝，塔羅的 reduce 其實無效——B2 一併修正）；翻牌改為直接換面。
- 星座 key → 中文名、漸層 class、行星中文名：純函式 `sign_zh()`／`sign_class()`／`planet_zh()` 放 **`crates/schema`**（緊鄰 `ZODIAC`，`lib.rs:243`），加原生測試：12 個 `ZODIAC` 全部對到非 fallback 值、大小寫不敏感、未知值回傳原字串（F13、F20）。wire 值是**首字大寫**（`"Leo"`），不是 `leo`。
- 太陽／月亮度數從 `planets` 找 `name == "Sun"/"Moon"`，用 `Option`，缺就不顯示（F13）。牌背另加 `element`／`quality`（已在 wire 上未使用，F17）。
- 排版：固定 `repeat(3, minmax(0,1fr))`＋`aspect-ratio: 3/4`（現行 `auto-fit minmax(140px)` 在 390 寬會排成 2+1，F12）。星座符號改 **inline SVG**（Android 上 U+FE0E 可能無效、變彩色 emoji，F12）。
- 行星清單 → 彩色藥丸：圓點色＝該星座漸層深端，底色＝淺端。
- 翻牌狀態只在頁面 signal，不存、不上傳；可用鍵盤（真 `<button>`、`aria-pressed`）。
- 不動 API、不動 schema 的 wire 欄位。

### 2.4 對比度守門（取代舊紅線；K03 改為量實際畫面）

- 兩層守門：
  1. `scripts/contrast-check.py`（快速、離線）：token 配對＋每個星座漸層**淺端與深端**；一般字 < 4.5、大字 < 3.0、非文字（焦點框、輸入框、按鈕邊界）< 3.0 即失敗。
  2. `scripts/contrast-audit.js`（Playwright，真畫面）：在 390×844（`hasTouch`、`isMobile`＝觸控分支）與 1280×800 兩種情境，逐頁走訪，對每個可見文字節點取 computed `color` 與**往上合成的實際背景**（含半透明層），列出 < 門檻者；並強制進入狀態：hover／focus／selected／checked、下拉選單展開、登入註冊成功／錯誤訊息、塔羅已翻、星座已翻、風水蓋牌與翻開。
- 稽核腳本必須：讀 `-webkit-text-fill-color`／`background-clip:text`（`.ocean-score` `style.css:645-648` 換值後 1.01–2.16，但 `color` 是深色會誤判通過）、SVG `fill`／`stroke`（`personality.rs:576`、`style.css:1037-1040`、`:1221`）、偽元素與 placeholder（`:778`、`:307`）、沿祖先鏈乘 `opacity`（`:1098`、`:283`、`:549`）（F02）。
- 非文字必檢清單（≥3.0）：`.ocean-track`（`:654`，白 .08 疊白卡＝隱形）、`.ocean-band` 淡端（`:659`）、進度條（`:725-726`）、range slider、捲軸 thumb（`:784-785`）、雷達圖格線（`personality.rs:550` `#e5e7eb` 1.24）、`.palm-figure`、`.fs-fig`、輸入框邊界（F02）。
- literal 守門改**白名單制**：`:root` 以外的 `style.css` 與 `crates/web/src` 出現任何 hex／rgb／rgba 即失敗，除非列在 allowlist（每條附理由）；另檢查每個 `var(--x)` 都存在於 `:root`（F05、F06：v2 的四字串 grep 漏掉 `rgba(5,13,31`、`rgba(8,18,42`、`rgba(16,20,31`、`#cfd6e0` 1.46、`login.rs:189 #fb7185` 2.69、`#9aa3b2` 2.54 等）。
- 手相拍攝導引線（`.palm-overlay` `:1090-1107`）與相機底板（`:1082`）同屬底色不可控：改固定色（白線＋深色描邊、深色底板），不跟主題 token，列入 allowlist（F08）。
- 圖片上的字（塔羅 `tarot-caption` 疊在卡面上，`style.css:889-910`）無法用色值保證：caption 底改不透明淺色板，不依賴圖片。
- CLAUDE.md「色彩/對比紅線」改寫為新規則並指向此腳本。

## 3. 分批（每批獨立 commit、可單獨回退）

| 批 | 內容 | 檔案 | 負責 |
|---|---|---|---|
| **B1 主題底座** | `style.css` 1–249 行整段換成新 token＋頁底＋標題，**加上 `:root` 在檔內所有出現處**（含 `@media (pointer: coarse)` 的 `:802` 覆寫，F07）；**B1 單獨不可部署，B1＋B2 才是最小可看單位**；`index.html` 移除星空 div、`galaxy.js`、換字型；刪 `galaxy.js` 與 `build-web.sh:75`；`contrast-check.py` | style.css、index.html、galaxy.js、build-web.sh、scripts/ | agent A |
| **B2 元件換色** | `style.css` 250 行以後所有寫死色值：card／feature／按鈕／表單／palace／ocean／quiz／tarot／palm／fengshui／scrollbar／coarse-pointer／on-light | style.css | agent A（接 B1，同檔不可平行） |
| **B3 行內樣式**（**排在 B2 之後序列做**，F09：色值要配 B2 定案的底板） | **8** 個 Rust 檔（`divination.rs` 歸 B4，K02）；含 `profile.rs:121/790` 的 `--gen-title`、`login.rs:189 #fb7185`的行內色值改用 token；深底專用的（`#9aa3b2` 註腳、`#34d399` 成功字、`birth_form` 鉻板 chip、`personality` 雷達圖藍色）換成亮底版 | layout/login/personality/profile/admin/lib/palace_grid/birth_form .rs | agent A（接 B2 序列；不碰 divination.rs） |
| **B4 星座翻牌** | **整個 `divination.rs`**（含 `ZiWeiView` 註腳 `:211` 的 `#9aa3b2`，白底 2.54，K02）＋`WesternView` 三張翻牌＋彩色行星藥丸；`sign_zh`／`sign_class` 純函式；新增 `.zodiac-*` 樣式（**寫在獨立段落並由 agent C 交付 CSS 片段，最後由 Claude 合入 style.css**，避免與 agent A 衝突） | divination.rs＋CSS 片段 | agent C（平行） |
| **B5 文件與清理** | CLAUDE.md 兩條紅線、ARCHITECTURE.md 主題章節（cosmic-silver 標「已退役」）、cosmic-silver spec 標 superseded、**AGENTS.md:171-181／200**（仍寫 galaxy.js／gen-stars.js 為現役）、刪 `crates/web/scripts/gen-stars.js`、`build-web.sh:72` echo 字串、`build-web.sh` 開頭清空 `dist/`（避免舊 `galaxy.js` 殘留部署，K11–K12） | 4 份 md＋2 腳本 | Claude |
| **B6 驗收＋部署** | 見 §4 | — | Claude |

**整合順序與回退（K10、F09、F10）**：agent A 序列做 B1 → B2 → B3；agent C 在獨立 worktree 平行做 B4（只用 B1 token 表定稿的名稱，schema 純函式＋測試在 C）；Claude 合入 B4 的 CSS 片段後做 B5（含刪除過渡 token、跑 `var()` 存在性檢查）。**不提供單批回退**：部署前記下目前的 Pages deployment，正式站出事直接在 Cloudflare dashboard rollback，再修。

**塔羅（F16，見 §2.5-2）**：B2 給 `.tarot-face` 加白色相框邊與柔和陰影；CSS 牌背（`style.css:912-919`）固定為深色，避免圖片載入前先閃粉白；`tarot.rs` 的 caption 調整歸 agent A（B2）。

**不在本計劃**（列為後續）：塔羅 79 張卡面用 m4m 重產成 kawaii 風（需先 spike 一張比對）、吉祥物／空狀態插圖、風水 stance 徽章分色、首頁版面重排。

## 4. 驗收

1. `cargo fmt --check`、`cargo test`（native 五 crate）、`cargo clippy --target wasm32-unknown-unknown` 警告數不增加（現 24）、`build-web.sh` 成功。
2. `scripts/contrast-check.py` 全過。
3. `grep` 守門：`style.css` 與 `crates/web/src` 不再出現 `rgba(10, 23, 51`、`#0a1733`、`chrome-mirror`、`galaxy`；剩餘寫死色值逐一列表說明理由。
4. 無頭 Chromium 逐頁截圖（390×844 觸控＋1280×800 桌機，含 §2.4 的狀態矩陣）：首頁、登入、`/fengshui`（問答＋結果翻牌）、`/tarot`、`/palm`、`/glossary`、`/naming`；要登入的頁（profile／personality／divination 紫微＋西洋）用 `scripts/f8-acceptance.sh` 的 DB 自造 login token 方式取得 session 後截圖。逐張人工看：無白字白底、無深底殘留、翻牌可用。
5. 部署後前端斷言（F14；`verify-deployment.sh` 只打 API）：`curl` 正式站 `index.html` 無 inline `<script>`、無 `galaxy`，`/boot.js`、`/style.css` 200、`/galaxy.js` 404；Playwright 開正式站，等 `main` 有子節點且 console 無 CSP 違規。低亮度手機看一眼白卡與頁底的分界（白 vs `#fff5fa` 僅 1.07，F21）。
6. CSP：`index.html` 改動後在套用 `_headers` 的環境載入確認 app 會 mount（CLAUDE.md CSP 紅線；本機用 `wrangler pages dev`，**需站長同意才開 server**）。
7. 部署：只動 web（不需 API 部署）；commit → push → `deploy-web.sh` → `verify-deployment.sh` → 正式站再截一輪。

## 5. 風險

- **白字白底**：舊樣式大量假設深底（`--ink-on-chrome` 原本是深墨、`.on-light` 是「淺底例外」）。改值後語意反轉：`--ink-on-chrome` 變白、`.on-light` 要改成指向 `--heading`。漏改處會出現隱形字 → §4.3 grep＋§4.4 截圖兩道。
- **塔羅卡面**：深色卡面在亮底上會突兀，`tarot-caption` 漸層要改淺色並確認卡名可讀；卡面本身留到後續 m4m 重產。
- **觸控裝置分支**（`@media (pointer: coarse)`）有自己的深色實底，最容易漏。
- **多機器**：開工前 `git fetch`；本計劃只動 web，`rustc 1.96` 不影響 web 部署。
- **回退**：見 §3 整合順序——不做單批回退，正式站出事走 Pages dashboard rollback（F10）。

## 5.1 自審紀錄（Claude，2026-11-19）

- 對比試算（WCAG，相對亮度公式）：白 on 主紫 5.70、白 on hover 紫 7.10、`--rose` on 白 6.04／on 淡粉 `#ffe4ee` 5.06、`--silver-faint` on 白 6.16／頁底 5.78／頁底遠端 5.44／`--surface-tint` 5.56、紫字 on tint 5.14、深字 on 粉鈕 6.25；深字 on 十二星座深端 4.80–11.32，**唯一不過的天蠍座 `#9575cd` 4.31 已改 `#a98fe0` 5.82**。最緊表面是摩羯座深端 `#a1887f` 4.80。
- `galaxy` 引用只在 `index.html`、`galaxy.js` 本身、`build-web.sh:75`、`style.css`、CLAUDE.md；`_headers`／`boot.js`／`scripts/`／CI 無引用。

## 5.2 Kimi K3.1 Pro 審查紀錄（2026-11-19；Claude 逐條到 repo 核實，全數屬實並吸收）

Kimi 的最終回答在第 5 條被輸出上限截斷，第 6–12 條取自它的推理清單，同樣逐條核實。

| # | 級別 | 摘要 | 核實 | 處置 |
|---|---|---|---|---|
| K01 | Blocking | token 表漏 `--glass-bg-strong`、`--metal-*`，會殘留深底與近白字（`select option:checked` 1.55、`a:hover` 近白） | `style.css:28/56-68/271-274/300-305/324/768/790-802` 屬實 | 收：§2.1 補齊、B1 交完整對照表 |
| K02 | Blocking | `divination.rs` 無主＋B3/B4 同檔；B3 寫 9 個實列 8 個 | `divination.rs:211` `#9aa3b2` 屬實 | 收：B4 擁有整檔、B3 改 8 檔 |
| K03 | Blocking | 對比腳本只算人工配對，量不到實際合成底與狀態 | 屬實 | 收：加 Playwright 實畫面稽核＋狀態矩陣 |
| K04 | Should | `--ink-on-chrome` 同時代表板上字與淺底字 | `style.css:394/758/815-824` 屬實 | 收：拆成 `--ink-on-primary`＋`--heading` |
| K05 | Should | 成功色無 token（`#34d399` 白底 1.92） | `login.rs:170` 屬實 | 收：`--success #047857` |
| K06 | Should | 翻牌可及性不足（backface 不會對報讀器隱藏） | 塔羅已有 `aria-hidden` 先例 `tarot.rs:121` | 收：§2.3 |
| K07 | Should | reduced-motion 規則寫在基本規則前面，塔羅的 reduce 無效 | `style.css:248` vs `:858` 屬實（既有 bug） | 收：B2 修正並規定順序 |
| K08 | Should | 翻牌絕對定位需明確尺寸 | 屬實 | 收 |
| K09 | Should | 上升度數會遺失 | `divination.rs:245` 屬實 | 收 |
| K10 | Should | 「每批可單獨回退」不成立（B3/B4 依賴 B1） | 屬實 | 收：整合順序＋整串回退 |
| K11 | Nit | AGENTS.md 仍寫 galaxy.js／gen-stars.js 為現役；自審說「只有 5 處引用」不完整 | `AGENTS.md:171-181` 屬實 | 收：B5 |
| K12 | Nit | `build-web.sh` 不清 `dist/`，舊 `galaxy.js` 會被部署 | 屬實 | 收：B5 |

## 2.5 站長裁示（2026-11-19：三項全收）

1. **星座牌預設翻開**（F11）：進頁自動依序翻開（間隔約 150ms），點擊可翻回正面看天體圖示；reduced-motion 直接顯示背面。理由：現況三格直接可見，改成「每次要點三下才看得到自己的星座」是退步。
2. **塔羅牌背拉進本次**（F16）：用 m4m 重產一張 kawaii 風 `back.webp`，兼作 79 張卡面重產的風格試做；正面 78 張仍留後續（樣張是深色實體牌風格，放在亮桌面上不算破碎）。理由：塔羅是首頁第一張卡，翻牌前先看到的就是牌背。
3. **直切順序＋本機預覽**（F17）：B1 → B2 先做首頁、nav、card、按鈕、表單、西洋星盤頁 → B4 → **站長本機看一眼**（需同意開 `wrangler pages dev`）→ 再掃其餘頁 → B3 → B5 → 一次上線。理由：最有感的星座翻牌原本排最後，而且要登入才看得到。

本次保留、列後續：首頁九個 emoji 圖示（F20）、塔羅正面 78 張、吉祥物。

## 5.3 Fable 5.2 審查紀錄（2026-11-19；Claude 核實）

Verdict：有條件開工（F01–F05 補進計劃後）。Fable 已重算計劃中的對比數字，全部正確。Claude 抽驗：`--gen-title` 引用（`profile.rs:121/790`）、wire 首字大寫（`lib.rs:243`）、`verify-deployment.sh` 無前端檢查、`.on-light` 唯一使用者 `divination.rs:255`、`#eadcf5` 對白 1.31、`#8b6fd6` 3.93／3.47、`#ec4899` 3.53／5.02，皆屬實；F01 指出 Claude 在比較稿私自調過四個星座色、計劃未載，屬實。

| # | 級別 | 摘要 | 處置 |
|---|---|---|---|
| F01 | Blocking | 十二星座色值未入計劃、且非 zodiac1 原值（原值四個配深字不過） | 收：§2.1 星座表 |
| F02 | Blocking | 實畫面稽核量不到 text-fill/SVG/opacity/非文字 | 收：§2.4 |
| F03 | Blocking | 控制項邊框吃 `--glass-border`（1.31）；`--metal-lo` 在頁底 <3 | 收：改 `#8b6fd6`、邊框改指 `--metal-lo` |
| F04 | Blocking | 選取態子元素（`.fs-hl` 1.00、`.quiz-no` 白字白底） | 收：逐處標板色 |
| F05 | Blocking | `--gen-title` 仍被引用、grep 抓不到 | 收：→ `--primary-deep`＋`var()` 存在性檢查 |
| F06 | Should | grep 守門漏網甚多 | 收：白名單制 |
| F07 | Should | `:802` 觸控覆寫不在 B1 範圍 | 收 |
| F08 | Should | 手相導引線疊相機畫面 | 收：固定色 |
| F09 | Should | B3 平行效益小風險大 | 收：B3 改序列 |
| F10 | Should | 回退說明矛盾 | 收：dashboard rollback |
| F11 | Should | 翻牌預設蓋住是退步 | §2.5-1 待裁示 |
| F12 | Should | 2+1 排版、U+FE0E 在 Android 不可靠 | 收：固定三欄、inline SVG |
| F13 | Should | 純函式無測試、大小寫寫錯、度數來源 | 收：移到 schema＋測試 |
| F14 | Should | 驗收沒檢查前端 | 收：§4.5 |
| F15 | Should | `.card` hover 上浮會跳、觸控黏住 | 收 |
| F16 | Should | 塔羅牌背應拉進本次 | §2.5-2 待裁示 |
| F17 | Should | 最有感的排最後 | §2.5-3 待裁示 |
| F18 | Nit | 粉鈕邊界 2.83 | 收：`#ec4899` |
| F19 | Nit | `.on-light`／`.text-frame` 成死碼 | 收：刪除 |
| F20 | Nit | 行星英文名、首頁 emoji | 收 `planet_zh`；emoji 列後續 |
| F21 | Nit | 白卡與頁底分界弱 | 收：驗收看一眼 |

## 6. 待審重點（給審查者）

1. token 沿用舊名改值 vs 全面改名：可維護性與一次改動量的取捨。
2. B1/B2 同檔序列、B3/B4 平行的切法是否會衝突。
3. 對比度腳本的組合是否涵蓋最緊表面（星座漸層深端、粉色鈕、`--surface-tint` 上的 `--silver-faint`）。
4. 移除 `galaxy.js` 是否有其他引用（CSP、`_headers`、測試腳本）。
5. 星座翻牌的可及性（鍵盤、螢幕報讀、reduced-motion）。
