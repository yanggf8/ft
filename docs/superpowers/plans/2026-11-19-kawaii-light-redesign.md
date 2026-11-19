# 全站換色 kawaii 亮底 ＋ 星座頁追上 zodiac1 — 實作計劃 v1

日期：2026-11-19　狀態：**草稿，待審**（流程：Claude 自審 → Kimi → Claude 核實 → Fable 5.2 → Claude 核實 → 站長 → 多 agent 實作）

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
| `--ink-on-chrome` | `#ffffff` | 主色紫板上的字 |
| 新增 `--primary`／`--primary-hover`／`--primary-deep` | `#7c3aed`／`#6d28d9`／`#5b21b6` | 主按鈕、選取態 |
| 新增 `--accent`／`--ink-on-accent` | `#ff5fa2`／`#2a0f1e` | 粉色強調鈕、NEW 徽章 |
| 新增 `--surface-tint` | `#f6f1ff` | 蓋牌區、次級底板 |
| 新增 `--sign-<12 星座>` | zodiac1 漸層，全部配深字 `--ink-on-sign #2a1b3d`；**天蠍座深端由 `#9575cd`（4.31，不過）改 `#a98fe0`（5.82）** | 星座牌、行星藥丸 |

刪除：`--chrome-mirror-lg/sm`、`--chrome-glint`、`--galaxy-y-offset`、`--metal-deep(-small)`、`--text-muted-on-dark`、`--error-on-*`、`--gen-title`（已查無引用者直接刪，有引用者改指新 token）。

### 2.2 風格

- 卡片：白底、`border-radius: 20px`（取消 tailfin 不對稱圓角）、2px 淡粉紫框、柔和紫色陰影；hover 改「上浮＋陰影加深」，取消 `filter: brightness`。
- 按鈕：主鈕紫底白字、全圓角、底部 4px 深紫立體陰影，按下沉 3px；次鈕白底紫框紫字。
- 標題：圓體實色（Latin/數字用 Google Fonts `Nunito`，中文走系統字 PingFang TC／Noto Sans TC／微軟正黑），取消鏡面漸層與 glint 動畫。不載入中文 webfont（體積大、M PLUS Rounded 1c 缺繁中字會混字型）。
- 選取態：紫底白字（取代「反相鉻板」）。
- 星空：移除星點、銀河 haze、`galaxy.js` 與其 canvas；頁底改兩團淡彩光暈。
- 動畫：保留風水翻牌、塔羅翻牌；尊重 `prefers-reduced-motion`。

### 2.3 星座頁追上 zodiac1（`WesternView`）

- 太陽／月亮／上升 → **三張 3D 翻牌**：正面＝白卡＋天體圖示（☀ 月 ↑ 以 inline SVG 畫，不用 emoji），點擊翻到背面＝該星座漸層＋星座符號（`♌︎` 加 U+FE0E 強制文字樣式）＋中文名。
- 星座 key（英文）→ 中文名、漸層 class 的對照：新增純函式（`sign_zh()`、`sign_class()`），與既有 `sign_symbol()` 同檔；未知 key 退回中性白卡、不 panic。
- 行星清單 → 彩色藥丸：圓點色＝該星座漸層深端，底色＝淺端。
- 翻牌狀態只在頁面 signal，不存、不上傳；可用鍵盤（真 `<button>`、`aria-pressed`）。
- 不動 API、不動 schema 的 wire 欄位。

### 2.4 對比度守門（取代舊紅線）

- 新增 `scripts/contrast-check.py`：列舉「字色 × 實際底色」組合（白卡、頁底、`--surface-tint`、主色紫板、粉色鈕、每個星座漸層的**淺端與深端**），算 WCAG 比值；一般字 < 4.5、大字（≥24px 或 ≥18.66px 粗體）< 3.0 即失敗。
- 基準＝實際底色（亮底下沒有半透明玻璃疊加問題，`.palace.life` 與 `input::placeholder` 的舊陷阱消失，但星座漸層深端是新的最緊表面）。
- CLAUDE.md「色彩/對比紅線」改寫為新規則並指向此腳本。

## 3. 分批（每批獨立 commit、可單獨回退）

| 批 | 內容 | 檔案 | 負責 |
|---|---|---|---|
| **B1 主題底座** | `style.css` 1–249 行整段換成新 token＋頁底＋標題；`index.html` 移除星空 div、`galaxy.js`、換字型；刪 `galaxy.js` 與 `build-web.sh:75`；`contrast-check.py` | style.css、index.html、galaxy.js、build-web.sh、scripts/ | agent A |
| **B2 元件換色** | `style.css` 250 行以後所有寫死色值：card／feature／按鈕／表單／palace／ocean／quiz／tarot／palm／fengshui／scrollbar／coarse-pointer／on-light | style.css | agent A（接 B1，同檔不可平行） |
| **B3 行內樣式** | 9 個 Rust 檔的行內色值改用 token；深底專用的（`#9aa3b2` 註腳、`#34d399` 成功字、`birth_form` 鉻板 chip、`personality` 雷達圖藍色）換成亮底版 | layout/login/personality/profile/admin/lib/palace_grid/birth_form .rs | agent B（與 B1/B2 平行，不碰 style.css） |
| **B4 星座翻牌** | `WesternView` 三張翻牌＋彩色行星藥丸；`sign_zh`／`sign_class` 純函式；新增 `.zodiac-*` 樣式（**寫在獨立段落並由 agent C 交付 CSS 片段，最後由 Claude 合入 style.css**，避免與 agent A 衝突） | divination.rs＋CSS 片段 | agent C（平行） |
| **B5 文件** | CLAUDE.md 兩條紅線、ARCHITECTURE.md 主題章節（cosmic-silver 標「已退役」）、cosmic-silver spec 標 superseded | 3 份 md | Claude |
| **B6 驗收＋部署** | 見 §4 | — | Claude |

**不在本計劃**（列為後續）：塔羅 79 張卡面用 m4m 重產成 kawaii 風（需先 spike 一張比對）、吉祥物／空狀態插圖、風水 stance 徽章分色、首頁版面重排。

## 4. 驗收

1. `cargo fmt --check`、`cargo test`（native 五 crate）、`cargo clippy --target wasm32-unknown-unknown` 警告數不增加（現 24）、`build-web.sh` 成功。
2. `scripts/contrast-check.py` 全過。
3. `grep` 守門：`style.css` 與 `crates/web/src` 不再出現 `rgba(10, 23, 51`、`#0a1733`、`chrome-mirror`、`galaxy`；剩餘寫死色值逐一列表說明理由。
4. 無頭 Chromium（390×844）逐頁截圖：首頁、登入、`/fengshui`（問答＋結果翻牌）、`/tarot`、`/palm`、`/glossary`、`/naming`；要登入的頁（profile／personality／divination 紫微＋西洋）用 `scripts/f8-acceptance.sh` 的 DB 自造 login token 方式取得 session 後截圖。逐張人工看：無白字白底、無深底殘留、翻牌可用。
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

## 6. 待審重點（給審查者）

1. token 沿用舊名改值 vs 全面改名：可維護性與一次改動量的取捨。
2. B1/B2 同檔序列、B3/B4 平行的切法是否會衝突。
3. 對比度腳本的組合是否涵蓋最緊表面（星座漸層深端、粉色鈕、`--surface-tint` 上的 `--silver-faint`）。
4. 移除 `galaxy.js` 是否有其他引用（CSP、`_headers`、測試腳本）。
5. 星座翻牌的可及性（鍵盤、螢幕報讀、reduced-motion）。
