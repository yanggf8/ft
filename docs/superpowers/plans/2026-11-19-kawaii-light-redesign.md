# 全站換色 kawaii 亮底 ＋ 星座頁追上 zodiac1 — 實作計劃 v2

日期：2026-11-19　狀態：**v2（已吸收 Kimi K01–K12），待 Fable 審**（流程：Claude 自審 → Kimi → Claude 核實 → Fable 5.2 → Claude 核實 → 站長 → 多 agent 實作）

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
| `--metal-lo` | `#9a7fe0`（白底 3.24，非文字框線；初稿 `#c4b0ef` 實算僅 1.94 已更正） | 控制項邊框 |
| `--metal-line`／`--metal-edge` | `#f0e4f7`／粉紫綠三色漸層 | 分隔線／卡片框 |
| 拆掉 `--ink-on-chrome`（K04） | 主紫板上用新 `--ink-on-primary #fff`；淺底（含 `.on-light`）改用 `--heading`／`--text`；全部引用遷移後刪除 | 避免一個 token 兩種相反語意 |
| 新增 `--success` | `#047857`（白 5.48、tint ≥4.9；K05） | 成功字（取代 `#34d399` 1.92） |
| 新增 `--primary`／`--primary-hover`／`--primary-deep` | `#7c3aed`／`#6d28d9`／`#5b21b6` | 主按鈕、選取態 |
| 新增 `--accent`／`--ink-on-accent` | `#ff5fa2`／`#2a0f1e` | 粉色強調鈕、NEW 徽章 |
| 新增 `--surface-tint` | `#f6f1ff` | 蓋牌區、次級底板 |
| 新增 `--sign-<12 星座>` | zodiac1 漸層，全部配深字 `--ink-on-sign #2a1b3d`；**天蠍座深端由 `#9575cd`（4.31，不過）改 `#a98fe0`（5.82）** | 星座牌、行星藥丸 |

**B1 交付物包含 `:root` 每個舊 token 的「新值／取代／刪除」完整對照表（K01）**，不得只靠 B2 掃 literal。

刪除：`--ink-on-chrome`、`--chrome-mirror-lg/sm`、`--chrome-glint`、`--galaxy-y-offset`、`--metal-deep(-small)`、`--text-muted-on-dark`、`--error-on-*`、`--gen-title`（已查無引用者直接刪，有引用者改指新 token）。

### 2.2 風格

- 卡片：白底、`border-radius: 20px`（取消 tailfin 不對稱圓角）、2px 淡粉紫框、柔和紫色陰影；hover 改「上浮＋陰影加深」，取消 `filter: brightness`。
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
- 星座 key（英文）→ 中文名、漸層 class 的對照：新增純函式（`sign_zh()`、`sign_class()`），與既有 `sign_symbol()` 同檔；未知 key 退回中性白卡並**顯示原字串**（不留白）、不 panic。
- 行星清單 → 彩色藥丸：圓點色＝該星座漸層深端，底色＝淺端。
- 翻牌狀態只在頁面 signal，不存、不上傳；可用鍵盤（真 `<button>`、`aria-pressed`）。
- 不動 API、不動 schema 的 wire 欄位。

### 2.4 對比度守門（取代舊紅線；K03 改為量實際畫面）

- 兩層守門：
  1. `scripts/contrast-check.py`（快速、離線）：token 配對＋每個星座漸層**淺端與深端**；一般字 < 4.5、大字 < 3.0、非文字（焦點框、輸入框、按鈕邊界）< 3.0 即失敗。
  2. `scripts/contrast-audit.js`（Playwright，真畫面）：在 390×844（`hasTouch`、`isMobile`＝觸控分支）與 1280×800 兩種情境，逐頁走訪，對每個可見文字節點取 computed `color` 與**往上合成的實際背景**（含半透明層），列出 < 門檻者；並強制進入狀態：hover／focus／selected／checked、下拉選單展開、登入註冊成功／錯誤訊息、塔羅已翻、星座已翻、風水蓋牌與翻開。
- 圖片上的字（塔羅 `tarot-caption` 疊在卡面上，`style.css:889-910`）無法用色值保證：caption 底改不透明淺色板，不依賴圖片。
- CLAUDE.md「色彩/對比紅線」改寫為新規則並指向此腳本。

## 3. 分批（每批獨立 commit、可單獨回退）

| 批 | 內容 | 檔案 | 負責 |
|---|---|---|---|
| **B1 主題底座** | `style.css` 1–249 行整段換成新 token＋頁底＋標題；`index.html` 移除星空 div、`galaxy.js`、換字型；刪 `galaxy.js` 與 `build-web.sh:75`；`contrast-check.py` | style.css、index.html、galaxy.js、build-web.sh、scripts/ | agent A |
| **B2 元件換色** | `style.css` 250 行以後所有寫死色值：card／feature／按鈕／表單／palace／ocean／quiz／tarot／palm／fengshui／scrollbar／coarse-pointer／on-light | style.css | agent A（接 B1，同檔不可平行） |
| **B3 行內樣式** | **8** 個 Rust 檔（`divination.rs` 歸 B4，K02）的行內色值改用 token；深底專用的（`#9aa3b2` 註腳、`#34d399` 成功字、`birth_form` 鉻板 chip、`personality` 雷達圖藍色）換成亮底版 | layout/login/personality/profile/admin/lib/palace_grid/birth_form .rs | agent B（與 B4 平行，不碰 style.css、不碰 divination.rs） |
| **B4 星座翻牌** | **整個 `divination.rs`**（含 `ZiWeiView` 註腳 `:211` 的 `#9aa3b2`，白底 2.54，K02）＋`WesternView` 三張翻牌＋彩色行星藥丸；`sign_zh`／`sign_class` 純函式；新增 `.zodiac-*` 樣式（**寫在獨立段落並由 agent C 交付 CSS 片段，最後由 Claude 合入 style.css**，避免與 agent A 衝突） | divination.rs＋CSS 片段 | agent C（平行） |
| **B5 文件與清理** | CLAUDE.md 兩條紅線、ARCHITECTURE.md 主題章節（cosmic-silver 標「已退役」）、cosmic-silver spec 標 superseded、**AGENTS.md:171-181／200**（仍寫 galaxy.js／gen-stars.js 為現役）、刪 `crates/web/scripts/gen-stars.js`、`build-web.sh:72` echo 字串、`build-web.sh` 開頭清空 `dist/`（避免舊 `galaxy.js` 殘留部署，K11–K12） | 4 份 md＋2 腳本 | Claude |
| **B6 驗收＋部署** | 見 §4 | — | Claude |

**整合順序與回退（K10）**：B1 → B2（agent A，序列）；B3、B4 與 B1 同時開工但只用 B1 定義的 token 名稱（token 表先於程式碼定稿），各在獨立 git worktree；合併順序 B1 → B2 → B3 → B4（Claude 合入 B4 的 CSS 片段）→ B5。B3/B4 依賴 B1 的 token，**不能單獨回退 B1**；正式站出事時整串回退（`git revert` 依反序）再重部署，而不是挑單批。

**不在本計劃**（列為後續）：塔羅 79 張卡面用 m4m 重產成 kawaii 風（需先 spike 一張比對）、吉祥物／空狀態插圖、風水 stance 徽章分色、首頁版面重排。

## 4. 驗收

1. `cargo fmt --check`、`cargo test`（native 五 crate）、`cargo clippy --target wasm32-unknown-unknown` 警告數不增加（現 24）、`build-web.sh` 成功。
2. `scripts/contrast-check.py` 全過。
3. `grep` 守門：`style.css` 與 `crates/web/src` 不再出現 `rgba(10, 23, 51`、`#0a1733`、`chrome-mirror`、`galaxy`；剩餘寫死色值逐一列表說明理由。
4. 無頭 Chromium 逐頁截圖（390×844 觸控＋1280×800 桌機，含 §2.4 的狀態矩陣）：首頁、登入、`/fengshui`（問答＋結果翻牌）、`/tarot`、`/palm`、`/glossary`、`/naming`；要登入的頁（profile／personality／divination 紫微＋西洋）用 `scripts/f8-acceptance.sh` 的 DB 自造 login token 方式取得 session 後截圖。逐張人工看：無白字白底、無深底殘留、翻牌可用。
5. CSP：`index.html` 改動後在套用 `_headers` 的環境載入確認 app 會 mount（CLAUDE.md CSP 紅線；本機用 `wrangler pages dev`，**需站長同意才開 server**）。
6. 部署：只動 web（不需 API 部署）；commit → push → `deploy-web.sh` → `verify-deployment.sh` → 正式站再截一輪。

## 5. 風險

- **白字白底**：舊樣式大量假設深底（`--ink-on-chrome` 原本是深墨、`.on-light` 是「淺底例外」）。改值後語意反轉：`--ink-on-chrome` 變白、`.on-light` 要改成指向 `--heading`。漏改處會出現隱形字 → §4.3 grep＋§4.4 截圖兩道。
- **塔羅卡面**：深色卡面在亮底上會突兀，`tarot-caption` 漸層要改淺色並確認卡名可讀；卡面本身留到後續 m4m 重產。
- **觸控裝置分支**（`@media (pointer: coarse)`）有自己的深色實底，最容易漏。
- **多機器**：開工前 `git fetch`；本計劃只動 web，`rustc 1.96` 不影響 web 部署。
- **回退**：每批獨立 commit；正式站出事可 `git revert` 單批後重新 `deploy-web.sh`。

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

## 6. 待審重點（給審查者）

1. token 沿用舊名改值 vs 全面改名：可維護性與一次改動量的取捨。
2. B1/B2 同檔序列、B3/B4 平行的切法是否會衝突。
3. 對比度腳本的組合是否涵蓋最緊表面（星座漸層深端、粉色鈕、`--surface-tint` 上的 `--silver-faint`）。
4. 移除 `galaxy.js` 是否有其他引用（CSP、`_headers`、測試腳本）。
5. 星座翻牌的可及性（鍵盤、螢幕報讀、reduced-motion）。
