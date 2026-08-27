# 設計：Big5 人格×情境行為預測（rev.4）

日期：2026-08-26
狀態：**Part I 功能設計 — 待審**（§0 量表分歧已裁決；Grok + Codex 兩輪審查已對帳修訂，待最終核可）；**Part II 技術設計 — 待審**（rev.4 已寫，Grok 審查已對帳修訂）
前置：A1 紫微換庫已合入 `origin/main`

> **rev.4 的兩項變更**
> 1. 使用者拍板**單一 Rust stack**（Worker + 前端皆 Rust/WASM），rev.3 的 Part II 技術設計整份基於 TypeScript，**已作廢**，保留於下方僅供方法論與審查歷史參考。
> 2. 審查改為兩關：**先過功能設計（本 Part I），再過技術設計**。功能不因實作語言而變，本 Part I 刻意寫成 stack-neutral。

---

# Part I — 功能設計（rev.4，待審）

## 0. 根本矛盾與裁決（✅ 已定案：路線 B — 換量表為繁中 IPIP-15）

Grok 對抗式審查的第一項 P0，經我逐條回原始文獻對帳後**成立**：

> **TIPI 不能同時是「篩檢級工具」又是「個人層級行為預測的唯一輸入」。**

對帳過的證據（全部已驗證，非二手轉述）：

| 證據 | 內容 |
|---|---|
| Gosling, Rentfrow & Swann (2003) *JRP* 37:504–528，摘要原文 | TIPI 提供給「**人格不是主要研究主題**，或研究者能容忍較差心理計量特性」的情境 |
| 同上 §4.2 Limitations | 「less reliable and correlates less strongly with other variables」；**無法提供 facet 分數**，並引 Paunonen & Ashton (2001)：**窄面向往往才是特定效標的較佳預測因子** |
| Thørrisen & Sadeghi (2023) *Front. Psychol.* 14 掃描回顧（27 版本／18 語言／27,427 人） | 平均 **α = 0.53**（門檻 .70）；原文建議：「use of the TIPI should be **discouraged in studies that primarily aim to explore personality**」 |
| Credé et al. (2012) *JPSP* 102:874–888 | 極短量表**系統性低估**特質對行為的效標關聯，並同時抬高 Type I／II 錯誤 |
| Shi, Li & Chen (2022) *PRBM*（中文版 TIPI，N=2223） | α 範圍 0.119–0.785；**親和性與開放性**內部一致性過低、五因子結構不被支持；作者僅建議 E／ES／C 可用 |
| Mischel (1968)；Epstein (1979) *JPSP* | 特質與**單次具體行為**相關約 .20–.30；需跨時間加總後才穩定到可用 |

本產品把人格當**主軸**（不是共變項），要的是**個人層級、7 天、具體**的預測——正好落在原作者與後續文獻都劃掉的格子。
而中文版最爛的兩維（親和性、開放性）正是愛情／家庭領域錨點與 F3 落差敘事最吃重的兩維。

**這不是措辭問題，是測量與主張不匹配。** 三條路（A. 降主張／B. 換量表／C. 縮至 E-ES-C 三維）交付使用者裁決，
**使用者裁決：採路線 B（換量表）**。

### 0.1 選定量表：IPIP-15 繁體中文版

裁決後我對「繁中效度資料是否存在」做了查證——這正是我提 B 案時標為 **UNVERIFIED** 的風險。**結果推翻了我原本建議的兩個候選**：

| 候選 | 繁中效度資料 | 判定 |
|---|---|---|
| Mini-IPIP（20 題） | **無**。IPIP 官方翻譯清單中明載為 traditional Chinese 的只有 **50 題**（Sih-Ci Jhu，National Dong Hwa University，並在臺灣樣本檢驗信效度）；另有一份 50 題中文版（Ya-Tzu Wang，Southern Taiwan University，清單未載繁簡、未載信效度）；唯一的中文 Mini-IPIP 效度研究是 Li, Sang, Wang & Shi (2012) *Psychological Reports* 111(5):641–651，樣本為**中國大陸地震倖存者**（N=1,563，α .79–.84） | ✗ 無繁中常模 |
| BFI-2-S（30 題） | **無**。中文版 BFI-2 為 Zhang et al. (2022) *Assessment*，**大陸樣本**。且 BFI-2 表單頁尾明載「BFI-2 items copyright 2015 by Oliver P. John and Christopher J. Soto」，Berkeley Personality Lab permissions 頁原文：「**At this time, the BFI-2 is for non-commercial uses only**」 | ✗ 無繁中常模 **且不可商用** |
| **IPIP-15 繁中版** ✅ | **有**。李仁豪、陳怡君（2016）。IPIP 五大人格量表簡版的發展及其跨年齡層的測量不變性檢定。《教育研究與發展期刊》12(4), 87–119 | ✅ **採用** |
| BFI-15 繁中版 | 有（李仁豪、鍾芯瑜，2020，《測驗學刊》67(4), 271–299，臺灣大學生 N=730+732，α .67–.81） | 備案；但同受 BFI 版權限制，商用需授權 |

**授權面（對本產品是硬條件——FortuneT 有付費機制）**：IPIP 官網原文「The items and scales are **in the public domain**, which means that one can copy, edit, translate, or use them **for any purpose without asking permission and without paying a fee**」。BFI-2 則明文非商用。**這一條單獨就足以排除 BFI 系列。**

IPIP-15 已對帳的實測數據（表 2／表 3，N=455／合併 N=738）：

| 維度 | 題數 | 因素負荷 | 組合信度 | AVE | **Cronbach's α** | M | SD |
|---|---|---|---|---|---|---|---|
| 外向性 | 3 | .61–.73 | .70 | .44 | **.70** | 9.74 | 1.95 |
| 友善性 | 3 | .62–.65 | .67 | .41 | **.67** | 10.37 | 1.70 |
| 嚴謹性 | 3 | .66–.74 | .74 | .48 | **.74** | 10.50 | 1.99 |
| 情緒穩定性 | 3 | .74–.83 | .83 | .61 | **.83** | 10.02 | 2.31 |
| 智性／想像 | 3 | .72–.80 | .80 | .56 | **.80** | 9.08 | 2.07 |

**與中文 TIPI 的對比才是這次換表的真正理由**：中文 TIPI 的 α 下限是 **0.119**（Shi et al. 2022），IPIP-15 的 α 下限是 **.67**。這不是邊際改善，是量級差異。而且 IPIP-15 只有 **15 題**——比原本 B 案建議的 Mini-IPIP 20 題、BFI-2-S 30 題都短，**我當初標的「題數翻倍、完成率風險」代價實際上不存在**（僅比 TIPI 多 5 題）。

**必須誠實記錄的限制（作者自陳 §研究限制，全部逐條照抄要點）**：

1. **常模樣本是臺灣中老年人的立意取樣**（426 位未滿 60 歲 + 312 位 60 歲以上），非全國分層抽樣；作者明言「無法平衡蒐集北、中、南、東部與離島地區的樣本，可能多少影響研究結果」。**本產品目標客群若偏年輕，此常模不直接適用。**
2. **測量不變性只驗過中老年**；作者明言「未來可以擴展至青壯年乃至青少年」。**跨年齡可比性在青壯年上未經驗證。**
3. **繁體版並非重做英翻中**，而是「直接將中國大陸簡體版改為繁體版後稍做修飾」。
4. **友善性仍是最弱維**：α .67、AVE .41（未達 .50）；且潛在變項間 友善性–嚴謹性 r = **.79**、友善性–外向性 r = .64，區辨效度偏弱。愛情／家庭領域的錨點設計要對此保留餘裕。
5. **無重測信度**。該研究兩波樣本不重複，未報告 test–retest；下方 F3 門檻以 α（內部一致性）推導，**α 通常高於數週間隔的重測 r，因此據此算出的 SEM 是下界**，門檻須保守上調（見 F3）。

---

## 1. 產品主張

> **實測人格 × 當下情境 → 具體、可驗證的行為預測。**

> **「可驗證」的正確讀法（rev.4，Codex 審 P0 對帳後補）**：這裡的「可驗證」是指
> **每條 `forecast` 都寫成 7 天後有真值條件的陳述，因此本產品的主張在設計上可被推翻**——
> 它**不**表示效標效度已經成立。§0 的證據明確指出：廣義特質與**單次具體行為**相關約 .20–.30，
> 換更可靠的量表提高的是**信度**，**不會**自動帶來個人層級、領域特定、7 天視野預測的**效標效度**。
> 效標效度只能由 F8 的對照差值產出。**在 F8 產出符合判定規則的正差之前，本產品沒有「準」的證據**，
> 對外文案禁用「準確」（§1.1）。

角色定位（這是整份設計的地基，其餘規格都由此推導）：

| 元素 | 在產品中的角色 | 不是什麼 |
|---|---|---|
| **IPIP-15 實測 OCEAN**（繁中，15 題） | 預測的**唯一人格輸入** | — |
| **命盤（紫微／西洋）** | ① 先驗對照組 ② 敘事外衣 ③ 情境領域的象徵映射 | **不進入 F5 任何欄位的產生**（見下方精確界定） |
| **情境輸入** | 預測的**變異來源**，決定同一人格在不同處境的不同行為 | 不是心情紀錄 |
| **LLM** | **只**潤寫 F5 的 `experiment` 欄位措辭 | 不判斷、不打分、不新增因果；`tendency`／`trigger`／`forecast`／`anchor_coverage` 逐字來自規則 |

**「命盤不參與打分」的精確界定（rev.4，Codex 審 P1 對帳後補）**：原文寫「不參與打分」是**過度簡化**。
命盤實際上**確實**被轉成分數——F2 用 ±20／±10／0 產出一條 0–100 的象徵向量，F3 再用它與實測向量的數值差決定是否敘事。
準確的界線是：

| 命盤可以做 | 命盤絕不可以做 |
|---|---|
| 產出 F2 象徵向量（有分數） | 進入 F5 的 `tendency`／`trigger`／`forecast` |
| 觸發 F3 落差敘事（有數值門檻） | 影響 F5 的 `anchor_coverage` |
| 提供 F5 的敘事外衣與領域→宮位映射 | 修改 F5 任何欄位的**值** |

**這個定位的直接後果**：命理算錯不會讓 F5 預測變差（只讓 F3 敘事與外衣變差），但人格量表測不準會讓整個產品失效。因此量表的測量品質是 P0，命盤精度是 P1。

### 1.1 成功條件（可量測，供上線後檢核）

| 指標 | 門檻 | 為什麼是這個數 |
|---|---|---|
| IPIP-15 完成率（進入問卷 → 送出） | ≥ 70% | 十五題 <90 秒，低於此代表題目或動機設計失敗 |
| 亂答偵測觸發率 | **1%–15%（雙側）** | 上界：高於此代表誘因或題幹有問題。**下界：低於 1% 代表偵測器根本沒在動** |
| 預測回饋率（見 F6） | ≥ 25% | 低於此則「準確」宣稱無資料支撐 |
| **真實組 − 對照組的自評差**（見 F8） | **顯著 > 0** | **取代原先的「自評命中率 ≥ 60%」** |
| 二次情境輸入率（7 日內） | ≥ 30% | **回訪迴路的健康度**（注意：這只證明有人回來，**不**證明情境是回訪原因——歸因需另做） |

> **rev.4 修訂（Grok 審 P0，已對帳採納）**：原先訂「使用者自評命中率 ≥ 60%」為成功門檻，**已刪除**。理由是算術性的：
> Rosenthal & Rubin(1982) 的 BESD 中，**r = .20 的二分預測命中率恰好就是 60%**，而 r ≈ .20 正是單次行為與特質關聯的常見量級。
> 更關鍵的是 Forer(1949)：同一份**從占星書抄來的**通用人格描述發給全部 39 名學生，平均自評準確度 4.26／5（約 85%）。
> 也就是說 60% 這個門檻，一份完全空洞的巴納姆文本可以輕鬆超過。**自評命中率無論訂多少都沒有檢核力，除非有對照組。**
> 處置規則同步改寫：只有當「真實組 − 對照組」的差**不顯著**時，才回頭修 `rules`；原始「準」的佔比不作為任何決策依據。

**絕對禁止**：在對外文案使用「準確」二字，直到 F8 對照條件產出符合判定規則的正差。

> **rev.4 修訂（Codex 審 P1，已對帳採納）——這張表能檢核什麼、不能檢核什麼**：
> 前三項（完成率、亂答觸發率、回饋率）與第五項（二次輸入率）**全部是漏斗健康度指標，不檢核輸出是否有用**。
> 一個對所有人吐出同一段通用文字的系統，只要流程順、提示到位，這四項都能達標。
> **這張表裡唯一有檢核力的是 F8 的真實 − 對照差值**，其餘四項只用來判斷「流程有沒有壞」，
> **不得**被引用為產品有效的證據。兩項具體修正：
> - 「亂答偵測觸發率 ≤ 15%」原本是**單側**的，一個永遠不觸發的偵測器可以拿滿分。已改為雙側 1%–15%。
> - 「二次情境輸入率 ≥ 30%」原本的理由欄寫「驗證『情境』確實是回訪動機」，這是**歸因錯誤**：
>   回訪率高不代表是情境造成的（可能是推播、可能是好奇命盤）。理由欄已改為僅宣稱迴路健康度。

## 2. 使用者旅程

### 2.1 首次（目標：最短路徑先給價值）

```
註冊
 └─→ IPIP-15 十五題（90 秒，零命盤資訊）
      └─→ ★ 價值點 1：OCEAN 五維結果 + 白話人格側寫      ← 此處已可獨立交付價值
           └─→ 「想知道你的命盤怎麼說嗎?」→ 填生辰
                └─→ ★ 價值點 2：象徵傾向疊圖 + 落差洞察
                     └─→ 「最近哪方面卡住?」→ 勾情境
                          └─→ ★ 價值點 3：行為預測
```

**關鍵設計決定：人格量表先於生辰。** 理由：
- IPIP-15 是**不依賴生辰即可獨立施測**的心理量表（注意：「獨立」指流程上不需命盤，**不是**指其常模已適用本產品客群——見 §0.1 限制 1、2）,不需要生辰就能產出真實價值;生辰表單(年月日時分+時區+城市)是全流程最高的摩擦點,擺在第一關會流失大量使用者。
- 先答題再看命盤,天然滿足 §4.2 的**去 priming** 要求——使用者作答時根本還沒有命盤可被暗示。安全性從流程設計就達成,不必只靠 UI 隱藏。
- 反向(先生辰後量表)會讓「命盤先驗」污染實測值,直接摧毀 F3 落差洞察的效度。

### 2.2 回訪（目標：情境是回訪動機）

```
登入 → 直接落在「最近哪方面卡住?」→ 勾情境 → 新預測
                                    └─→ 上次預測的回饋提示（F6）
```

人格與命盤都是**靜態**的（人格量表半年內不重測、生辰永不變）。因此回訪迴路**只能**掛在情境上——情境輸入必須極低摩擦（≤ 3 次點擊）。

## 3. 功能清單

| 代號 | 功能 | 優先 | 依賴 |
|---|---|---|---|
| **F1** | IPIP-15 人格測量 → OCEAN 實測向量 | P0 | 無 |
| **F2** | 命盤象徵傾向（先驗向量） | P0 | 生辰 + 命盤 |
| **F3** | 落差洞察（實測 vs 象徵） | P0 | F1 + F2 |
| **F4** | 情境輸入 | P0 | 無 |
| **F5** | 行為預測 | P0 | F1 + F4（F2 選配） |
| **F6** | 預測準確度回饋 | **P0** | F5 |
| **F7** | 資料權利（檢視／刪除） | P0 | F1 |
| **F8** | **對照條件（Barnum 基線）** | **P0** | F5 + F6 |

> **F6 是 rev.4 新增,並且刻意訂為 P0。** rev.3 完全沒有回饋迴路。使用者的目標原話是「**準確**行為預測」——沒有回饋資料,「準確」就是一句無法證實也無法改進的行銷詞,§1.1 的兩個核心指標也全部無法計算。這是 rev.3 最大的功能缺口。

## 4. 功能規格

### F1 — IPIP-15 人格測量

> **rev.4 修訂（§0 路線 B 裁決）**：量表由 TIPI（10 題）改為 **IPIP-15 繁體中文版**（李仁豪、陳怡君，2016）。
> 依據與限制見 §0.1。IPIP 題庫為 **public domain，可商用**。

- **十五題,五點量表**（每維 3 題）,**中性題幹**（繁中版直接沿用該研究之題目文字）,作答畫面**零命盤資訊**
- 計分:每維 3 題（反向題翻轉）加總 → 原始分 3–15 → `(raw − 3) / 12 × 100`,輸出五維 0–100
  - **情緒穩定性**維的三題（「很容易不高興」「情緒變化很大」「經常感到憂鬱」）為反向計分，
    輸出以**情緒穩定性**呈現而非神經質；UI 一律不使用「神經質」字樣
- **常模**：對照組常模暫以該研究合併樣本（N=738）之 M／SD 呈現（見 §0.1 表），
  **但需在 UI 明示常模來源為臺灣中老年立意取樣**；上線後累積自有樣本即改用自有常模
- **亂答偵測**三訊號:總作答時長過短、全部同一選項、正反題矛盾
  - 觸發 → 提示重測一次;仍觸發 → 存 `measurement_status = 'skipped_prior_only'`
  - 使用者主動跳過亦同
- 重測政策:**180 天內不主動邀請重測**;使用者可主動重測,舊側寫保留
  - IPIP-15 **未發表重測信度**（§0.1 限制 5）。上線後應蒐集 4–6 週間隔的自有重測樣本，
    據以重算 F3 門檻與 F5 `anchor_coverage`——在此之前門檻採保守值

### F2 — 命盤象徵傾向（先驗）

- UI 一律稱「**命盤象徵傾向**」,**禁止**稱「命格人格」或任何斷言式命名
- 純規則對照表(主星／星座 → 五維調整),**LLM 不參與打分**
- 每條規則附**來源等級標注**:`classical`(古籍歸納)／`designer`(設計者判斷),前端可展開檢視
- 調整量 ±20／±10／0(100 分制),多條命中取平均,clamp [0,100]
- 兩盤並存時以紫微為主
- **無生辰時 F2 缺席,不阻擋 F1/F4/F5**

### F3 — 落差洞察

- 疊圖呈現實測與象徵兩條五維向量
- **F3 一律 opt-in**：填完生辰**不自動**顯示落差敘事，需使用者主動選擇查看，且進入前先說明
  「兩條線來源不同、不一致很常見、**不代表你有缺陷或你該改命**」
- **落差門檻為逐維 2×SEM，操作值統一取 20 分（0–100 量尺）**，低於門檻僅並列兩值、不敘事
- 落差達門檻才敘事,且措辭固定為**描述性**:「命盤象徵偏 X,你的實測偏 Y」
- **高神經質／低情緒穩定者，該維一律不敘事**（只並列數字）

> **rev.4 修訂（Grok 審 P0 → §0 路線 B 換表後重算）**：
>
> **原 15 分門檻在 TIPI 下太鬆。** 以 Gosling(2003) Table 3 的逐維重測 r（E .77／A .71／C .76／ES .70／**O .62**）
> 與 TIPI 常模 SD≈1.45（1–7 量尺）推算，SEM ≈ 11–14 分；15 分僅約 1.2 SEM，
> **即使真實落差為零，單維誤觸約 25%、五維至少一維觸發約 1−0.75⁵ ≈ 76%**——四分之三使用者會看到純噪聲的落差敘事。
> 另：TIPI 每維只有 13 個可能值，一個完整量尺點 = 16.67 分，15 分連一個量尺點都不到。
>
> **換用 IPIP-15 後以其自有 SD 與 α 重算**（SEM = SD·√(1−α)，原始分 3–15，×100/12 換算 0–100）：
>
> | 維度 | SD | α | SEM（原始） | 2×SEM（原始） | **2×SEM（0–100）** |
> |---|---|---|---|---|---|
> | 外向性 | 1.95 | .70 | 1.07 | 2.14 | **17.8** |
> | 友善性 | 1.70 | .67 | 0.98 | 1.95 | **16.3** |
> | 嚴謹性 | 1.99 | .74 | 1.01 | 2.03 | **16.9** |
> | 情緒穩定性 | 2.31 | .83 | 0.95 | 1.90 | **15.9** |
> | 智性／想像 | 2.07 | .80 | 0.93 | 1.85 | **15.4** |
>
> 換表把門檻從 TIPI 的 25–30 分壓到 **15.4–17.8 分**，五維間差距也從 TIPI 的 ~3 分收斂到 ~2.4 分。
> **但這組數字是下界**：α 為內部一致性，通常高於數週間隔的重測 r，而 F3 比較的是「此刻實測 vs 命盤先驗」，
> 應以重測信度而非 α 為準（§0.1 限制 5：IPIP-15 未發表重測信度）。
> **因此操作門檻統一取 20 分**（高於全部五維的 α 下界估計），待自有重測樣本到手後逐維重算。
>
> **這個門檻的效力範圍（Codex 審 P1 對帳後補，重要）**：2×SEM 只界定**實測側**的測量誤差。
> 命盤側的象徵向量來自 `classical`／`designer` 規則，**沒有任何誤差模型、沒有信度、沒有常模**，
> 兩條向量也**未經任何研究證明可共量**。因此 20 分**不是**「統計上偵測到真實落差」的檢定門檻，
> 它只回答一個較弱的問題：**這個差距是否大到不能單靠實測側的測量噪聲解釋**。
> 差距超過 20 分**不等於**兩者之間存在有意義的心理學落差——它只排除了一個競爭解釋。
> F3 的文案必須與此一致：只能描述「兩條線不一樣」，不得暗示這個不一樣本身有解釋力。
  - **禁止**價值判斷措辭(「你違背了命格」「你壓抑了本性」)
- 落差敘事是本產品最具差異化的內容,但也最容易變成傷人的斷言——措辭規範列為上線檢核項

### F4 — 情境輸入

- 五領域強度:`work / love / family / money / health`,各 0–3
- 選填自由文字 `target`(單一具體對象或事件)
- **摩擦上限:≤ 3 次點擊可送出**(預設全 0,只點有感的);回訪迴路的成敗取決於此

### F5 — 行為預測 ★核心

rev.3 只寫了「`prediction_text` + anchors」,**沒有定義預測的形狀、視野與粒度**,無法實作也無法評估。本節補齊:

- **時間視野**:未來 **7 天**(v1 固定;30 天版本列入後續候選)
- **粒度**:每個**強度 ≥ 1 的領域**產出 **1 條**預測,最多 5 條
- **每條預測的固定四段結構**(結構化欄位,非散文):

  | 欄位 | 內容 | 來源 |
  |---|---|---|
  | `tendency` | 你在這個領域傾向如何反應 | 規則錨點(trait × domain) |
  | `trigger` | 什麼情況會放大這個傾向 | 規則錨點 |
  | **`forecast`** | **一句 7 天後可被判定真假的陳述**(例:「這週你更可能在衝突當下先退開，而不是把話講清楚」) | 規則錨點 |
  | `experiment` | 選配。建議可以試的動作 | 規則錨點 + LLM 潤寫 |
  | `anchor_coverage` | `high` / `low` | 見下（**rev.4 更名**，原 `confidence`） |

> **rev.4 修訂（Grok 審 P0，已採納）**：原本的 `action`「一個具體、7 天內可執行的動作」是**建議（處方）而非預測**，
> 兩者的言語行為不同——預測有真值條件，建議沒有。若把建議送進 F6 的「準／不準／沒發生」，
> 「沒發生」會塌縮成「我沒照做」，「準」會塌縮成「事後覺得這建議有道理」，§1 的「可驗證」被這一個欄位整個取消。
> 故拆為 `forecast`（進 F6 計分）與 `experiment`（**不進** F6）。
>
> **`forecast` 的硬限制**：只能是本週**高基率、使用者自己可觀察**的行為（Epstein 1979 的加總原則：
> 特質預測的是加總後的趨勢，不是單次低基率事件）。禁止把辭職、分手、就醫這類低基率事件寫成 7 天 forecast。
>
> **負面效價的防護（rev.4，Codex 審 P1 對帳後補）**：本節原本的示例
> 「這週你更可能在衝突當下先退開，而不是把話講清楚」**本身就是一句負面、綁定身分的行為陳述**，
> 而 F8 原文只禁止**對照組**出現負面 forecast——真實使用者反而毫無防護。這個不對稱已在 F8 修掉，
> 但真實組同樣需要規範，否則本產品會在 health／money／love／family 四個最敏感的領域，
> 對使用者輸出高權威感的負面自我描述，構成病理化與自我實現預言的風險。故新增三條硬規則：
>
> 1. **`forecast` 一律寫成行為傾向的比較，不得寫成人格缺陷**：
>    可以說「更可能先退開」，**不得**說「你不擅長溝通」「你容易逃避」。
> 2. **health 領域禁止任何負面 `forecast`**（僅允許中性或正向的可觀察行為），
>    money 領域禁止涉及損失、負債、決策失誤的 forecast。
> 3. **每次產出的 forecast 集合中，負面效價不得超過半數**；超過則丟棄最負面的幾條，寧可少給。

- **`anchor_coverage` 的決定規則**(明訂,不由 LLM 決定)。**只看測量品質與錨點覆蓋,不看命盤**:

  | 條件 | `anchor_coverage` |
  |---|---|
  | 該領域命中 0 條錨點 | **不產出這條預測**(不是 low) |
  | 命中錨點互相矛盾(同維高／低規則同時命中) | `low`,且禁止輸出對立因果 |
  | 該維 IPIP-15 三題全距 ≥ 2 個量表點(內部不一致) | `low` |
  | 命中 1 條錨點 | `low` |
  | 命中 ≥ 2 條錨點,且該維測量品質達標 | `high` |

  「≥ 2 條」的分母須以 `rules` 目錄總數定義,規則目錄未定案前此門檻不可實作。

> **rev.4 修訂（Codex 審 P1／P2，已對帳採納）——欄位更名 `confidence` → `anchor_coverage`**：
> 兩個獨立問題：
> 1. **`medium` 不可達**：欄位宣告 `high`／`medium`／`low`，但規則表只產出 `high`、`low`、不產出三種結果。
>    （`medium` 是先前 D3 修訂移除的殘留。）**列舉值已縮為 `high`／`low`。**
> 2. **更嚴重：`confidence` 是一個測量宣稱，但決定它的東西不是測量**。
>    「命中 ≥ 2 條錨點」講的是**規則目錄的覆蓋密度**，與這條預測會不會成真**沒有已知關係**；
>    內部一致性門檻本身也自陳為「設計者判斷，非文獻推導」。
>    貼上 `high confidence` 等於對使用者宣稱一個 §0 明說尚未成立的效標效度。
>    **欄位更名為 `anchor_coverage`，UI 措辭同步改為「這條預測有幾條規則支撐」，禁止出現「信心」「把握」「準確度」字樣。**
>    真正的信心只能來自 F8 的對照差值，且是**產品層級**而非單條預測層級的。

> **rev.4 修訂（Grok 審 P0，已採納）**：原規則用 F3 落差決定 `high`／`medium`。這**違反 §1 自己訂的「命盤不參與打分」**——
> `confidence` 是預測的欄位之一，讓命盤決定它就是讓命盤打分。語義上也錯：F3 落差是**兩個不同來源不一致**，
> 不是量表重測不穩；拿「跟命盤不一致」去降低對實測預測的信心，等於在打分時承認命盤是效標。
>
> **rev.4 換表後補充**：內部不一致的判準隨量表改變。TIPI 每維 2 題、7 點，原訂「差距 ≥ 3 個量表點」；
> IPIP-15 每維 3 題、**5 點**，量尺更短，改以**三題全距 ≥ 2 個量表點**為準。此值為設計者判斷，非文獻推導，上線後應以自有資料校準。

- **LLM 的邊界**:只潤寫 `experiment` 的措辭。`tendency`／`trigger`／`forecast`／`anchor_coverage` **逐字來自規則**,
  過 schema 校驗,禁止新增錨點以外的因果宣稱。
  ~~「整體語氣」~~ 已刪除——語氣改寫可以在 schema 校驗抓不到的情況下竄改 `tendency`／`trigger` 的表面因果。
- **命盤在此的角色**:提供敘事外衣(「你的官祿宮…」)與領域→宮位映射,**不改變任何一個欄位的值**

### F6 — 預測準確度回饋 ★rev.4 新增

- 預測產生後 **7 天**(即視野結束),回訪時對每條預測收一次三選一:**準 / 不準 / 沒發生**
- 選填一行自由文字
- **產品內的用途**:回訪時顯示**已回饋的預測清單與各自的三選一結果**（逐條可回顧），
  **不顯示任何彙總命中率百分比**
- **產品外的用途**:F8 差值計算的唯一資料來源;**修規則錨點的觸發條件見 F8，不看原始命中率**

> **rev.4 修訂（Codex 審 P0，已對帳採納）**：本節原寫「回訪時顯示『你的預測命中率 68%』」＋
> 「命中率 < 60% 時據以回頭修規則錨點」，**與 §1.1 自己訂的「原始『準』的佔比不作為任何決策依據」直接矛盾**，
> 而且 60% 正是 §1.1 已刪除的那個門檻。兩處不能並存。
>
> 更嚴重的是產品內用途：§1.1 已論證原始自評命中率被巴納姆效應污染（Forer 1949 的空洞文本可得 85%），
> 把一個**自己宣告無證據力的數字**當成留存誘因秀給使用者，等於用已知失效的指標製造心理權威感，
> 放大確認偏誤。**彙總命中率百分比一律不對使用者呈現**；逐條結果可回顧（那是使用者自己的作答，非推論）。
- **刻意不做**:不用回饋自動調參。樣本量遠不足,自動調參會過擬合到噪聲。回饋只餵給人看、由人改規則。

### F8 — 對照條件（Barnum 基線）★rev.4 新增

> **為什麼這是 P0 而不是研究上的講究**：F1–F7 裡**沒有任何一項功能**能區分「真的有訊號」與
> 「巴納姆效應 + 確認偏誤」。對一個把「可驗證」寫進核心主張的產品，對照組不是奢侈品，是主張成立的**必要條件**。
> Forer(1949) 的原始材料就是從占星書抄的通用描述，而本產品正好同時具備占星敘事外衣與人格回饋兩個巴納姆棲地。

- **對照條件唯一且固定：洗牌人格**（隨機抽另一位使用者的 OCEAN 向量，餵進**完全相同**的規則管線）。
  **不使用「通用句」作為對照。**
- 指派：隨機 20–30% 的預測落入對照組；使用者不知道自己在哪一組
- **對照組與真實組必須逐項匹配**：領域分布、`forecast` 的**情緒效價分布**、句長、`anchor_coverage` 標籤分布。
  對照組**不做**任何真實組沒有的內容限制
- §1.1 的核心指標吃「**真實 − 對照**」的差，**不吃**原始「準」的佔比
- **判定規則（上線前預先登記，事後不得更改）**：
  - 分析單位 = **預測條目**，以使用者為隨機效應（同一人多條預測不得當獨立樣本）
  - 「沒發生」的編碼方式在資料蒐集前寫死，不得事後決定
  - 門檻 = **真實組 − 對照組 ≥ 5pp（r ≥ .10 / d ≥ 0.20）且 p < .05（雙尾）**（見 §5 D6 已定案）
  - **差值顯著為負** → 視同主張被推翻，觸發 §1 主張下修，不得只當「再調規則」
  - **必檢**：兩組的回饋率差異。若兩組回饋率本身顯著不同，差值反映的是誰願意回報，不是預測效度，該批資料作廢
- 倫理邊界：實驗說明放進 F7 同意文字；使用者可在結果頁事後得知本次是否為對照
- **沒有 F8 產出符合上述判定規則的正差之前，對外文案禁止出現「準確」**

> **rev.4 修訂（Codex 審 P0，已對帳採納）**：原文有三個獨立缺陷，任一個都足以讓 F8 無法證偽本產品的主張。
>
> 1. **對照條件二選一**：原寫「洗牌人格**或**通用句」。這是兩個**性質完全不同**的對照——
>    洗牌人格檢定的是「這個人格向量有沒有訊號」，通用句檢定的是「有沒有任何個人化」。
>    兩者混用時，差值可能來自對照組的選擇而非人格訊號。**已收斂為單一對照：洗牌人格。**
> 2. **效價不對稱**：原寫「對照預測**不得**含負面 `forecast`」，而真實組可以是負面的。
>    負面陳述的自評命中率本來就與正面不同，這使差值可被效價差異解釋。
>    **對照組改為與真實組效價分布匹配**，不再單方面禁止負面。
> 3. **「顯著 > 0」不是門檻**：樣本量夠大時任意微小差值都會顯著；且原規則只處理「不顯著」，
>    **沒有規定顯著為負時要做什麼**。已補效果量下限、負差處置、分析單位與回饋率差異檢查。

### F7 — 資料權利

- 檢視:使用者可看到自己的**原始十五題**答案
- 刪除:一鍵清除人格側寫／情境／預測／回饋(不連動刪除帳號與命盤)
- **原始十五題答案永不離開本站**——送給 LLM 的只有結構化錨點(OCEAN 分數 + 領域強度 + 規則命中),不含逐題作答
- 同意基礎:主動作答視為同意,作答前一行聲明用途
- 免責:「趨勢參考,非心理診斷、非醫療建議」,常駐於結果頁而非只在註冊頁

## 5. 待裁決的功能級決策

| # | 決策點 | 我的建議 | 理由 |
|---|---|---|---|
| **D1** | 現有 `/story`(紫微＋西洋合成故事)在 Big5 主軸下的去留 | **保留,但降為「敘事外衣」的素材來源,不列入 B1/B2 主線** | 已上線、有快取、成本已付。但它與「準確預測」的主張無關,不該佔首頁動線 |
| **D2** | 預測視野 7 天 vs 30 天 | **7 天** | 回饋迴路(F6)要在使用者還記得的時間內收斂;30 天會讓回饋率崩掉 |
| **D3** | 沒有生辰的使用者能否用 F5 預測 | **能,且 `anchor_coverage` 規則與有生辰者完全相同** | ~~medium 上限~~ 已撤銷：那等於讓命盤變成信心上限開關，與 §1 矛盾。生辰只解鎖 F2／F3 疊圖，**不買更高信心** |
| **D4** | 免費試用(30 天)的功能閘門 | **F1/F3 永久免費,F5 預測計次** | 人格結果是獲客誘因;預測是有邊際成本(LLM)的價值品 |
| **D5** | 亂答者(`skipped_prior_only`)能否看預測 | **不能產出掛在他名下的 F3／F5** | 見下方修訂 |
| **D6** ✅ 已定案 | F8 差值的**效果量下限** | **真實組命中率 − 對照組命中率 ≥ 5 個百分點**（`r ≥ .10` / `d ≥ 0.20`），且 `p < .05`（雙尾）才算過 | 見下方定案 |

> **rev.4 修訂（Grok 審 P0，我方原案被推翻，已採納）**：rev.4 初稿主張「標示比阻擋誠實」，讓亂答者仍看到全部標 `low` 的預測。
> **這個理由不成立**——個人行為預測**本身就是一種主張**，不是中性資訊。產品自己判定 `skipped_prior_only`（測量失效），
> 還輸出掛使用者名字的預測，等於在已知失效的輸入上販售核心宣稱，直接打臉 §1 的「量表測不準會讓整個產品失效」。
> 對照標準心理衡鑑實務：效度指標未過的作答**不做解釋**，而不是加註「低信心」後照發報告。
> 且「`low` 標示能保護使用者」對**命理取向使用者**這個族群偏樂觀——他們習慣忽略不確定用語。
>
> **改訂行為**：
> - `skipped_prior_only` → F1 顯示「本次無法計分」＋清楚的重測路徑
> - **F3 關閉、F5 不產出個人預測**；如需降低困惑，給**與個人資料脫鉤的示範預測**（明寫「這不是根據你的作答」）
> - 該狀態的任何回饋**排除於 F6／F8 統計之外**
> - 不消耗 D4 的計次額度

> **D6 — F8 效果量下限（rev.4 新增，Codex 審 P1 對帳後補）**
>
> F8 原本只寫「顯著 > 0」。這是**沒有門檻的門檻**：樣本量夠大時，真實組比對照組高 1 個百分點也會顯著。
> 而「真實組比洗牌人格高 1%」顯然不足以支撐一個以「可驗證行為預測」為主張的付費產品。
> 因此必須在**蒐集資料之前**先訂下效果量下限，事後不得更改（否則就是 p-hacking）。
>
> 這個數字是**產品主張的強度宣告**，不是技術參數——訂高了可能永遠達不到、產品得下修主張；
> 訂低了則「通過 F8」形同虛設。它決定了你願意用什麼標準說自己的產品有效，因此**應由你裁決**。
>
> **✅ 定案（Codex 審 + 對帳後，使用者已裁決）**：
> **真實組命中率 − 對照組命中率 ≥ 5 個百分點**（對應 `r ≥ .10` / `d ≥ 0.20`），且 `p < .05`（雙尾）才算 F8 通過。
> 這是心理學 small effect 門檻，也是三種推導收斂的結果：
>
> 1. **天花板折半法**：特質→單次行為理論天花板 `r=.20–.30` 經 BFI 信度 `~.80`、7 天單點取樣、3 選 1 自評噪音三重衰減，現實天花板剩 `r≈.12–.18`。`r=.10` 要求達到現實天花板的 60–80%，不灌水也不自殺。`r=.20` 則要求超越理論天花板的實務表現，必死。
> 2. **最小可感知差異**：`r=.05 / 2.5pp` 只要 N 夠大也能 `p<.05`，但用戶體感無差異，卻拿去行銷寫「科學驗證準確」，是付費產品的偽陽性自殺。`5pp` 是文獻上第一個能被稱為「小但非零」的群體差異。
> 3. **成本不對稱**：付費產品偽陽性成本（信任崩盤、行銷被打臉）遠大於偽陰性成本（晚點宣稱），寧可放掉一次 F8，也不能讓垃圾效應過關。
>
> **同時報告絕對命中率**（證明不是靠 Barnum 85% 的基礎命中率在混），**預先登記單尾/雙尾**（建議雙尾更抗質疑），**鎖死後跑完不得再改**（否則 p-hacking）。
> 第二檔「強驗證」`7.5pp / r=.15 / d=0.30` 可作加嚴參考，但解鎖 F8 的最低門檻就是 `5pp`。
>
> **在 D6 定案前，F8 不可實作**——現已定案，F8 可進入實作準備。

## 6. 非目標（本期）

- 臨床級人格量表(IPIP-15 是**簡版篩檢工具**,不假裝是 NEO-PI-R；每維僅 3 題、無 facet 分數)
- 用回饋資料自動調參／訓練模型
- 流年、流月、流曜等時間維度命理
- 宮位制選擇器、資料匯出端點、付費牆金流整合
- 多語系(先繁中)

---

# Part II — 技術設計（rev.4）

狀態：**待審（rev.4 已吸收 Grok 對抗式審查 + 本地實測對帳）**。撰寫於 Part I 的 §0 量表分歧定案之後。
rev.3 的技術內容（TypeScript / Hono / React）**整份作廢**，保留於本節末的摺疊區供審查歷史參考。

## 0. 硬約束（先講死，後面所有設計都由此推導）

### 0.1 ⚠️ Workers Paid 是 Rust 路線的**前提**，不是選配

| 限制 | Free | Paid |
|---|---|---|
| Worker 大小（**壓縮後**） | **3 MB** | **10 MB** |
| CPU／請求 | **10 ms** | 30 s（預設，上限 5 min）|
| 啟動 CPU（global scope） | 1 s | 1 s |
| Subrequests／次 | 50 | 10,000 |
| 記憶體／isolate | 128 MB（**含 WASM 配置**）| 同左 |

來源：Cloudflare Workers Platform Limits 官方文件。

> **rev.4 修訂（Grok 審 P1-3，已對帳採納——我原本的因果推論是錯的）**：
> 原文寫「**現行 TS 版之所以還活著，只是因為它的西洋引擎根本沒在算東西**」。**這句話不成立。**
> 現行方案的 CPU 重活是 **iztro**（`routes/charts.ts:142,280` 實際呼叫），西洋那 131 行只是查表，本來就不耗 CPU。
> 生產 API 活著，代表 **iztro 在現行方案的 CPU 預算內跑得動**——這反而是「紫微不一定超過 10 ms」的證據，
> 與我原本的推論相反。另：Durable Objects 自 2025-04 起 Free plan 亦可使用 SQLite backend，
> 因此「本專案有 DO 所以一定已是 Paid」也不能反推。
>
> **修正後的判斷**：Paid 是否為阻擋項，**目前未知，必須量過才知道**。真正可能撞牆的是
> VSOP87 級數展開的 CPU 與 WASM 的**啟動時間**（1 s 上限），不是紫微。

**Phase 0 必須先量三件事，再決定 Paid 是不是阻擋項**（不得跳過直接付錢或直接假設沒事）：

1. **現行帳號方案為何**（Free / Paid）——這是零成本的一次查詢，卻決定了後面兩項的判讀基準
2. **現網 `GET /api/charts/ziwei` 的實際 `cpuTime`**——若 iztro 已接近 10 ms，紫微換 Rust 只會更緊
3. **WASM 模組的實例化時間**（對 1 s startup 預算）——體積越大越接近這條線

> 若量測顯示現行仍在 Free 且 cpuTime 有餘裕，則 Paid 的唯一驅動剩下 §6 的體積與啟動時間，
> **而 §6 的實測結果（見下）已顯示體積遠比原估的寬鬆**——屆時 Paid 可能不是阻擋項。

### 0.2 其餘不可協商的專案既有約束

| 約束 | 出處 | 對本設計的影響 |
|---|---|---|
| **只寫整合測試，無單元測試、無 mock、無 stub、無假資料** | `.testing-rules` | §8 的驗證策略不能靠 mock 建構；differential test 必須打真的 iztro |
| **無 feature flag** | `CLAUDE.md` | 遷移**不能**用旗標雙寫切換，只能靠路由層級的分流（§7）|
| **無資料庫約束** | `CLAUDE.md` | D1 schema 不加 FK／CHECK，靠應用層驗證 |
| **不預設資料、不硬編碼、不作弊** | 全域 CLAUDE.md | 直接判定現行西洋引擎違規（§4.2）|
| **部署一律先 `unset CLOUDFLARE_API_TOKEN`** | `CLAUDE.md` | Rust 的建置腳本要沿用既有 `npm run deploy` 包裝 |
| **前端每次部署前必須 build** | `CLAUDE.md` | Rust/WASM 的 build 產物要進同一條 Pages 流程 |

## 1. 目標與非目標

### 目標
1. Worker 與前端**同一個 Cargo workspace**，單一語言、單一型別系統、共用 domain model
2. 消除 TS／Rust 之間的 schema 漂移——`ziwei-v3` 這類共用型別只定義一次
3. 紫微引擎輸出**與現行 iztro 逐欄位等價**（可驗證，不是宣稱）
4. 西洋引擎**首次**具備真實星曆（現況見 §4.2）

**必須逐項保全的現役行為（Grok 審 P1-6，原文漏列，Rust 重寫時最容易靜默消失）**：

| 行為 | 位置 |
|---|---|
| `GET /api/charts/story`、`POST /api/charts/story/generate`（合盤，Part I D1 已定保留）| `routes/charts.ts:84-216` |
| AIMutexDO：1 concurrent、`MAX_QUEUE_DEPTH=8`、`MAX_QUEUE_WAIT_MS=60000`、503 `AI_QUEUE_FULL`／`AI_QUEUE_TIMEOUT`、45 s abort、rpm/rpd、exresource、`ALL_PROVIDERS_FAILED` | `durable-objects/ai-mutex-do.ts` |
| SessionDO：7 天 TTL、`/create|/get|/refresh|/destroy` | `durable-objects/session-do.ts` |
| 30 天 trial `checkUserAccess`；`GET /me` 回 `billing` | `services/billing.ts` |
| ETag 304、per-route `Cache-Control`、`Vary: Authorization` | `middleware/cache.ts` |
| auth 與 interpret 端點 per-IP 10 req/min | `routes/auth.ts`、`routes/charts.ts` |
| security headers + CORS（localhost／`*.pages.dev`／`*.workers.dev` + credentials）| `index.ts`、`middleware/security.ts` |
| `birth_data_hash`（缺省 hour=12、tz=`Asia/Taipei`）；改生辰即刪該用戶全部 interpretations | `services/birth-hash.ts`、`routes/users.ts:92-95` |
| 409 `RECALC_REQUIRED` + 前端自動重算重試 | `routes/charts.ts:389-392`、`frontend/src/lib/api.ts:94-110` |
| Zod V3 response guard（前端目前**未**共用這份 schema——正是 §2 要修的漂移）| `routes/charts.ts:342-347` |

> **本 Part II 的範圍界線（Grok 審對帳後明確化）**：§2 的 workspace 畫了 `domain/big5`，
> 但 §7 分期表**沒有 Big5 的實作 slot**——這是**刻意的**：
> **Part II 只負責「把現有功能原樣搬到 Rust」，Part I 的 F1–F8 一律不在本期。**
> `domain/big5` 只是預留目錄位置，Phase A–D 都不會動它。
> 這樣寫死是為了避免 Part I 與 Part II 搶同一季的工期。

### 非目標（本期）
- **Part I 的 F1–F8 全部不在本期**（IPIP-15、F3 落差、F5 預測、F8 對照條件都等搬棧完成後另行排程）
- 不改 D1 schema 的既有欄位語義（`users` / `interpretations` 照舊）
- 不改 AI failover 的三家供應商與順序
- 不追求 WASM 效能極致，只求進得了 CPU 與體積預算
- 不做 SSR（前端維持 CSR，見 §5）

## 2. Cargo workspace 佈局

```
ft/
├── Cargo.toml                 # [workspace] members
├── crates/
│   ├── domain/                # 純邏輯，無 IO，wasm32 與 native 都能編
│   │   ├── ziwei/             #   紫微：x-iztro 封裝 + V3 shape 轉換
│   │   ├── western/           #   西洋：vsop87 封裝 + 宮位/相位
│   │   ├── big5/              #   IPIP-15 計分、SEM、F3 門檻、F5 規則引擎
│   │   └── schema/            #   共用 DTO（serde），前後端唯一真相來源
│   ├── worker/                # cdylib → wasm32-unknown-unknown（Cloudflare）
│   │   ├── routes/
│   │   ├── durable/           #   SessionDO / AIMutexDO
│   │   └── ai/
│   └── web/                   # cdylib → wasm32-unknown-unknown（Pages）
└── ...
```

> **⚠️ crate 切分是硬性要求（Grok 審 P1-4，已對帳採納）**：上面的目錄樹若做成
> `domain` 一個大 crate，則 `web` 依賴 `domain` 會把 `vsop87`（crate 原始碼 4.9 MB 係數表）
> 與 `x-iztro` **整包打進瀏覽器 bundle**。Worker 的 10 MB 與使用者手機的下載量是兩個獨立預算，
> 前者寬鬆不代表後者可以放任。因此：
>
> - `schema` **必須**是獨立的 workspace member（正文 `use ft_schema::ZiweiChartV3` 已假設如此）
> - `web` **只准**依賴 `schema`，**禁止**依賴 `domain/ziwei`、`domain/western`、`domain/big5`
> - 這條要用 CI 檢查（`cargo tree -p ft-web` 不得出現 `vsop87` / `x-iztro`），不能只靠自律

**`domain/` 必須能在 native target 下編譯與測試**——這是讓 differential test（§8）能跑得快的關鍵：
對拍 iztro 時不需要每次都過 wasm 工具鏈。

**`schema/` 是這次遷移最主要的收益**：現行 `backend/src/shared/schemas/ziwei-v3.ts`（72 行）
與前端的對應型別是兩份手寫定義，rev.3 審查紀錄裡 `DivinationPage.tsx:105 讀取後端不存在欄位` 就是這個漂移造成的。
Rust 下前後端 `use ft_schema::ZiweiChartV3;`，同一個 struct，漂移在編譯期就死。

## 3. 平台綁定對照（workers-rs 0.8.5）

已在 `docs.rs/worker/0.8.5` 逐項確認存在，非推測：

| 現行 TS | Rust（`worker` crate） | feature | 狀態 |
|---|---|---|---|
| `D1Database` | `worker::d1::D1Database`（`pub use crate::d1::*`）| **需 `d1`** | 上游標示 **alpha** ⚠️ |
| `DurableObject` | `#[durable_object]` + `worker::DurableObject` | — | ✅ |
| DO KV 儲存 | `state.storage()` | — | ✅ |
| DO SQLite | `worker::SqlStorage`（`state.storage().sql()`，需 `new_sqlite_classes` migration）| — | ✅ |
| R2 | `worker::Bucket` | — | ✅（README 未提，但 API 存在）|
| KV | `worker::KvStore` | — | ✅（本專案未用）|
| `fetch()` | `worker::Fetch` / `Request` / `Response` | — | ✅ |
| Hono router | `worker::Router` 或 `axum` | `http` / `axum` | 二選一，見下 |

- crate 授權 **Apache-2.0**；`worker` 自身 MSRV 1.75，但 **workspace 實際 MSRV 是 1.88**
  （由 `x-iztro` 0.4.0 的 `edition = "2024"` / `rust-version = "1.88"` 與 Leptos 0.8.20 的 MSRV 1.88 決定）
- `async`/`await` 可用；**不跑 Tokio runtime**（Workers 是 JS event loop）

> **rev.4 修訂（Grok 審 P1-2，已對帳採納）**：原文寫「**無 Tokio**」是**錯的事實陳述**。
> `worker` 0.8.5 的 **normal（非 optional）** 相依裡就有 `tokio ^1.28`（已在 crates.io dependencies API 核對）。
> 正確說法是：**應用程式不啟動 Tokio runtime**，真正的限制是不能在 wasm 上開 tokio 的 full runtime。
> 原文把限制寫反了。
- **Router 選型：用 `worker::Router`，不用 axum。** 現行 25 個 handler（實測 grep）雖不算少，但型態單純，
  axum 會為了 tower 生態多背數百 KB 進 §0.1 的體積預算，換不到對應價值

> **D1 alpha 的風險處置**：這是本次遷移**唯一**沒有退路的相依。
> Phase 0 要寫一支只碰 D1 的最小 Worker，把現行 `users`／`interpretations` 的
> 全部查詢型態跑過一遍，確認 alpha API 涵蓋所需。
> **這支探針不通過，整個 Rust 路線就地停止**（§7 Phase 0 的 exit criteria）。
>
> **rev.4 修訂（Grok 審 P1-2）**：原文寫探針要驗證「`PUT /me/birth` 的**批次刪除**」——
> 實際上 `routes/users.ts:92-95` 是**單條** `DELETE FROM interpretations WHERE user_id = ?`，
> 本 repo **沒有任何 `D1.batch()` 呼叫**。探針去驗一個不存在的 API 會測錯面，已更正為驗證實際使用的查詢型態。

## 4. 計算引擎選型

### 4.1 紫微 → `x-iztro` 0.4.0

| 項目 | 值 |
|---|---|
| 版本 / 授權 | 0.4.0 / **MIT** |
| 描述 | 「Zi Wei Dou Shu chart engine, **field-for-field identical to iztro**」 |
| 相依 | `chrono`、`lunar_rust ^1.0`、`serde`、`serde_json`（`pyo3`/`pythonize` 為 optional，**不啟用**）|
| edition / MSRV | **2024 / 1.88** ← 決定整個 workspace 的 MSRV |
| **總下載數** | **64** ⚠️ |
| 最後更新 | 2026-08-21（6 天前）|

**全部相依皆為純 Rust，無 C FFI。wasm32 可編——已實測，不是推論**：

```
cargo build --release --target wasm32-unknown-unknown   # 36s, 成功
x-iztro raw wasm = 1,286,889 bytes (1.23 MB)
x-iztro gzip -9  =   422,723 bytes (0.40 MB)
```

> **rev.4 對帳紀錄（Grok 審 P0-2 的子主張，經實測**推翻**）**：Grok 指出 `x-iztro` 的
> `chrono` 是以 `default-features = false, features = ["clock"]` 引入、**未開 `wasmbind`**，
> 並推論「Phase 0 native 全綠、Phase A 才發現編不過或 `Utc::now()` 炸」。
>
> **Cargo.toml 的事實陳述正確**（已核對 crate 內 `Cargo.toml`），**但推論不成立**：
> 1. **編得過。** 上面的建置是真的跑過的，36 秒成功。chrono 在 `wasm32-unknown-unknown` target 下
>    會**自動**拉進 `js-sys` / `wasm-bindgen`（`cargo tree -i js-sys` 顯示 `js-sys → chrono → x-iztro`），
>    不需要手動開 `wasmbind`。
> 2. **`now()` 根本不會被呼叫。** 全 crate 只有一處時鐘呼叫：`src/astro/horoscope.rs:725`
>    的 `chrono::Local::now()`，位於 `pub fn horoscope_now()` 之內——那是「以此刻算運限」的便利方法。
>    **Part I §6 非目標已排除「流年、流月、流曜等時間維度命理」**，本產品不會呼叫它；
>    要算運限有 `horoscope(&date, time_index)` 收明確日期的版本。
>
> 結論：這條風險**不存在**，但 Grok 指出的 Cargo.toml 事實值得記錄——若日後開放運限功能，
> `horoscope_now()` 在 Worker 內的行為需另外驗證。

**但 64 次下載意味著實質上沒有經過生產驗證。** 唯一能採信它的理由是那句
「field-for-field identical to iztro」——而這句話是**可被檢定的**，不必相信：

> **採用的唯一條件：differential test 通過。** 對同一組生辰輸入，`x-iztro` 與現行
> npm `iztro@2.6.0` 的輸出必須逐欄位相同。測試設計見 §8.1。
> **對拍不過就不採用**，改走 §4.1b。

> **⚠️ 最可能的對拍失敗點不在安星，在曆法（Grok 審 P1-7，已對帳採納）**：
> `x-iztro` 的農曆來自 **`lunar_rust` 1.0.1 — 發佈於 2023-12-07，此後未更新，總下載 4,096**（已核對 crates.io）。
> 而 npm `iztro@2.6.0` 用的是 `lunar-lite` / `lunar-typescript`——**兩套完全不同的曆法實作**。
> 「field-for-field identical」這句宣稱涵蓋的是**排盤邏輯**，不保證底下的農曆轉換一致。
> 因此 §8.1 的對拍案例**必須**把閏月、晚子時、月初月末排在最前面跑——
> 那裡先炸的機率遠高於十四主星安星。

**§4.1b 退路（對拍失敗時）**：保留 TS Worker 專責紫微排盤，Rust Worker 以 service binding 呼叫它。
代價是「單一 stack」的目標打折，但**正確性優先於棧的純度**——這一點先寫死，避免屆時為了棧的一致性而犧牲輸出正確。

### 4.2 西洋 → ⚠️ 重大發現：現行引擎是佔位程式，不是引擎

**我先前告知使用者「現行後端使用 npm `astronomy-engine` v2.1.19」——這是錯的，必須更正。**
`backend/package.json` 的 `dependencies` 只有 `hono`、`zod`、`iztro` 三項；
**`astronomy-engine` 從未安裝**。rev.3 只是「計畫」導入它，從未執行。

實際在生產跑的是 `backend/src/services/western/calculator.ts`（131 行，手寫）：

| 輸出 | 實際演算法 | 判定 |
|---|---|---|
| 太陽星座 | 日期區間查表（`month*100+day`）| 忽略年份、時刻、時區，**黃道點附近必錯** |
| 月亮星座 | 以 2000-01-06 為基準、除以 **27.3 天**取模 | 把**恆星月**（27.3d）當**朔望月**基準用；且完全忽略時刻 |
| 上升星座 | **`Math.floor(hour / 2) % 12`** | **完全忽略緯度與日期**——`latitude` 參數收下後未使用。這不是近似，是佔位 |
| 行星 | 只有 Sun 與 Moon，Moon 的 `degree` **硬編為 0** | 註解自陳「Real planetary positions require ephemeris data」|

**兩個直接後果**：

1. **違反全域 CLAUDE.md 的「不預設資料、不硬編碼、不作弊」。** 上升星座那行尤其嚴重——
   它收下 `latitude` 卻不用，對呼叫端偽裝成有在算。
2. **Part I 的 F2 命盤象徵向量若吃西洋側輸出，吃到的是噪聲。**
   Part I §1 說「命理算錯不會讓預測變差（只讓 F3 敘事與外衣變差）」——這個豁免對紫微成立，
   **對現行西洋不成立**，因為 F3 落差敘事會拿一條隨機向量去跟使用者的實測人格做比較並產出敘事。

**修正後的設計判斷（與我先前的說法相反）**：西洋引擎**沒有既有精度需要保全**。
換成 Rust `vsop87` 不是「換演算法、有損移植」，而是**首次真的實作**。這反而是本次遷移風險最低、收益最高的一塊。

#### ⚠️ 4.2.1 阻擋項：`vsop87` **沒有月球**（Grok 審 P0-1，實測確認成立）

我原本把 `vsop87` 寫成「✅ 採用」是**錯的選型**。已在本機解開 crate 逐檔確認：

```
vsop87-3.0.0/src/ → earth_moon.rs jupiter.rs mars.rs mercury.rs neptune.rs
                    saturn.rs uranus.rs venus.rs + vsop87{a,b,c,d,e}/
vsop87d 公開函式  → mercury venus earth mars jupiter saturn uranus neptune  （八顆，無月）
```

**`earth_moon` 不是月球，是地月質心。** crate 自己的 doc 註解寫得很清楚：
「the center of masses between the Earth and the Moon, **not exactly the center of the Earth**」。
VSOP87 是**行星理論**，本來就不含月球（月球要 ELP2000 之類的獨立理論），也不含冥王星。

**三個後果，每一個都足以擋住本節原方案**：

1. **本命盤第二重要的點是月亮。** 現行佔位程式再爛，至少還吐一個 `moonSign`
   （`western/calculator.ts:81-91`）。照原方案換成 VSOP87，月亮不是變準，是**直接消失**——這是回歸不是升級。
2. **Part I 的 F2 吃的是「星座」。** F2 規則表是「主星／**星座** → 五維調整」。
   西洋側若沒有月亮與上升星座，F2 的西洋向量要嘛缺維、要嘛繼續吃本節剛判定為噪聲的佔位輸出。
3. **§8.2 的驗證手段自我打臉。** 原文寫用「特定日期的**日食**」驗證——
   日食就是日月合相。**沒有月球理論，這條驗證在物理上做不到。**

**另外兩項原文的事實錯誤**（同樣已核對 crate）：

- **§6 的降級策略 (a)「只啟用 VSOP87D 子集」在 crate 層做不到**：
  `vsop87` 3.0.0 的 `[features]` 只有 `default = ["simd"]`、`no_std`、`simd`——**沒有按版本切分的 feature**。
  能省的只有 LTO/DCE（不呼叫就不進 bundle，這點實測有效，見 §6）。另注意 **預設開 `simd`**，wasm32 需確認 `simd128` 處置。
- **`astro` 2.0.0 不能當 2026 年的備案**：下載數 165,496 是對的，但它**自 2016 年起未更新**，pre-2018 edition。

#### 4.2.2 修正後的選型

| 需求 | 候選 | 狀態 |
|---|---|---|
| 行星黃經（水～海）| **`vsop87` 3.0.0** | ✅ 保留採用。已實測 wasm32 可編（見 §6）|
| **月球黃經** | **未定 — Phase 0 必須解決** | ⛔ **阻擋項**，見下 |
| 上升點／宮首（恆星時 + 真黃赤交角 + 緯度）| **未定 — Phase 0 必須解決** | ⛔ 原文只說「要另外算」，沒指定實作 |
| 冥王星 | 未定 | P2；現行也沒有，不阻擋 |

**月球與上升點的選項（Phase 0 探針要逐一評估，本節不預先拍板）**：
- 自行實作 **ELP2000-82B 截斷級數**（月球黃經取前數十項即可到分級精度，體積小、無授權問題）
- 自行實作上升點（GMST + 黃赤交角 + 緯度，是封閉公式，數十行）
- **不採用 Swiss Ephemeris 系**：`libswe-sys` 等是 C FFI（wasm32 不可用）；
  `swisseph-rs` 雖自稱純 Rust port，但**port 仍可能是 SE 的衍生作品，AGPL 對 SaaS 一樣咬**——
  原文用「下載數過低」排除它是**錯的理由**，正確理由是授權（Grok 審 P1-7，採納）

> **Phase 0 的西洋 exit criterion 因此改為**：月球黃經與上升點**都有可 wasm32 編譯、非 AGPL 的實作並通過精度驗證**，
> 才准進 Phase A。**在此之前，西洋引擎不動**——寧可繼續跑佔位程式並在 UI 標示「西洋盤為粗略近似」，
> 也不要換成一個**沒有月亮**的盤。

維持 rev.3 已定的 **Whole Sign** 宮位制——它只需要上升點落在哪一宮，對計算精度最寬容。

> **`ENGINE_VERSION_WESTERN` 必須 bump。** 現行值 `'3.0.0'` 與紫微並列，
> 暗示兩者成熟度相當，實際上一邊是 iztro、一邊是佔位程式。改為 `'4.0.0'` 並清空全部西洋快取。

### 4.3 曆法

`x-iztro` 已透過 `lunar_rust` 內建農曆轉換，**不另外引入曆法 crate**——
多一份實作就多一份不一致來源。現行 `backend/src/services/ziwei/lunar.ts`（101 行）連同
已成為死碼的 `calculator.ts`（242 行）在 Phase A 一併刪除。

（備查：`chinese-lunisolar-calendar` 50,984 下載、`lunar-lite` 1,476——若 §4.1b 退路啟用才需要重新評估。）

## 5. 前端框架 → **Leptos 0.8.20**

| 候選 | 最新版 | 近期下載 | 最後更新 | 判定 |
|---|---|---|---|---|
| **Leptos** | 0.8.20 | **1,246,773** | 2026-06-25 | ✅ **採用**（MSRV 1.88）|
| Dioxus | 0.7.10 | 860,536 | 2026-07-31 | 次選；強項在跨桌面／行動，本專案用不到 |
| Yew | 0.23.0 | 357,957 | **2026-03-10** | ✗ 動能明顯落後，近五個月無更新 |

**選 Leptos 的理由**：近期下載量領先、fine-grained reactivity 的心智模型與現行 React hooks 最接近、
CSR-only 模式可直接部署到既有的 Cloudflare Pages 流程（不需要 SSR runtime，§1 非目標）。

> **rev.4 修訂（Grok 審 P1-7）**：表中 Leptos「最後更新」原寫 2026-07-18，那其實是 **0.9.0-beta** 的發佈日；
> 穩定版 **0.8.20 發佈於 2026-06-25**（已核對 crates.io versions API）。已更正。

**遷移量**：`frontend/src` 共 16 檔，其中 `.ts`/`.tsx` **945 行**（另 `index.css` 48 行）（其中 `StoryPage.tsx` 128、`LoginPage.tsx` 91）。
這是整個計畫中唯一「小到可以整份重寫」的部分——不做漸進遷移，Phase C 一次換掉。

> **使用者曾指出「react 的測試比 rust 真的不行」。** 對帳現況：`frontend/` 目前**零測試檔**，
> 而 `.testing-rules` 又禁止單元測試與 mock。因此前端換 Rust 的實際收益**不在測試**
> （兩邊都只能寫整合測試），而在 §2 的 `schema/` 共用型別消除前後端漂移。這一點必須誠實記錄，
> 以免用「測試更好」當作決策理由，事後發現理由不成立。

## 6. WASM 體積與 CPU 預算

**體積上限 10 MB 壓縮後（Paid）**，兩個 Worker／前端各自獨立計算。分配：

> **rev.4：以下由估計值改為本機實測值。原估計錯得很離譜，記錄下來以免再犯。**

實測條件：`opt-level="z"` + `lto=true` + `codegen-units=1` + `panic="abort"` + `strip=true`，
target `wasm32-unknown-unknown`，Rust 1.95.0。

| 項目 | **實測 raw** | **實測 gzip -9** | 原估計 | 差距 |
|---|---|---|---|---|
| `vsop87`（vsop87d 八行星，只取 longitude）| 771,497 B (0.74 MB) | **527,110 B (0.50 MB)** | 2–3 MB | **低估風險 4–6×** |
| `x-iztro` + `lunar_rust` + `chrono` + `chrono-tz` | 1,286,889 B (1.23 MB) | **422,723 B (0.40 MB)** | 0.5 MB | 接近 |
| `worker` + workers-rs | 未測 | ~1.5 MB（仍為估計）| 1.5 MB | 待測 |
| **已測合計** | — | **~0.92 MB** | — | — |

**關鍵發現：`vsop87` 的 crate 原始碼是 4.9 MB，但編進 wasm 只有 0.50 MB（壓縮後）。**
原因是 LTO/DCE 會把沒呼叫到的 VSOP87A/B/C/E 係數表整批剝掉——
只 `use vsop87::vsop87d::*` 就只有 VSOP87D 的表進 bundle。
**這也是為什麼 §4.2 的降級策略 (a) 不需要 crate feature 就已經自動達成**（雖然原文說要靠 feature，那是錯的）。

**修正後的體積判斷**：加上 workers-rs 的估計 1.5 MB，Worker 總量約 **2.5 MB 壓縮後**。
- 對 Paid 的 **10 MB** 餘裕充足
- 對 Free 的 **3 MB** 也**沒有明顯超標**——這推翻了 §0.1 原本「體積必然逼 Paid」的推論

**仍未測、Phase 0 必須補的**：
1. `worker` + workers-rs 的實際體積（上表唯一還是估計的一項）
2. **WASM 實例化時間**對 1 s startup 預算——體積不是唯一風險，啟動才是
3. 月球與上升點實作的增量（§4.2.2 未定案，體積未知）

若日後真的超標，降級策略：(a) ~~只啟用 VSOP87D 子集~~ **已由 DCE 自動達成**、
(b) 月球改用更短的 ELP2000 截斷、(c) 西洋計算改非同步預計算存 D1。

> **注意 (c) 不是「開關」而是新增產品相依**（Grok 審 P1-6，採納）：
> 原文寫「改由 Queue 非同步預計算」——本專案 `wrangler.toml` **目前沒有 queue binding**，
> 這等於新引入 Cloudflare Queues。它是架構變更，不是降級旋鈕，不該寫成輕描淡寫的備案。

**CPU 預算 — Phase 0 實測（2026-08-27，wrangler tail --format json，SJC）**：

| 請求 | wallTime | cpuTime |
|---|---|---|
| `GET /health` | 1–5 ms | 0–5 ms |
| `POST /auth/register` | 82 ms | 0 ms |
| `GET /charts/ziwei`（iztro 重算，含 D1）| **391–747 ms** | **3–7 ms** |

**結論：`cpuTime` 僅 3–7 ms，遠低於 Free 的 10 ms 上限**。wallTime 的 400–700 ms 主要是 D1 與網路等待，不計入 CPU 預算。
**Paid 不是紫微的阻擋項**，與 §0.1 原本「紫微遠超 10 ms」的推論相反（已在 §0.1 更正為先量再決定）。
真正的風險是 **1 秒的啟動 CPU 上限**——WASM 實例化算在裡面，體積越大越接近這條線。

## 7. 遷移分期與回退

**無 feature flag（CLAUDE.md）**，因此分期靠**部署單位**切分，不靠旗標。

| Phase | 內容 | Exit criteria（未達成則停止，不進下一階段）|
|---|---|---|
| **0. 探針** | ① 量現行方案 + 現網 `cpuTime` + WASM 啟動時間（§0.1）② D1 alpha 最小 Worker 打通實際查詢型態 ③ **月球黃經與上升點的可 wasm32／非 AGPL 實作**（§4.2.2）④ `x-iztro` differential test（§8.1）⑤ workers-rs 實際體積 | **五項全過才進 Phase A。** 任一項失敗 → 回報並重新評估路線 |
| **A. 引擎** | `domain/ziwei` + `domain/western` 完成，以**獨立 Rust Worker + service binding** 提供；現行 TS Worker 改為呼叫它 | 紫微對拍 iztro 全通過；西洋（含月亮／上升）通過 §8.2 的事件表 |
| **B. Worker** | 路由、DO、AI failover 移到 Rust Worker；TS Worker 退場 | 整合測試（§8）全綠；生產健康檢查通過 |
| **C. 前端** | Leptos 重寫 945 行；Pages 部署流程接上 | 手動驗收全部頁面；`dist/.build-info` 機制保留 |
| **D. 清理** | 刪除 TS 殘留、`calculator.ts` 死碼、更新 CLAUDE.md | — |

> **rev.4 修訂（Grok 審 P0-3，已對帳採納）——Phase A 原方案的工具鏈不存在**：
> 原文寫 Phase A「仍由現行 TS Worker 呼叫（wasm 模組形式）」。這條路有兩個問題：
> 1. **`workers-rs` / `worker-build` 產出的是一整顆 Worker，不是給 JS `import` 的 library。**
>    要讓 TS Worker 載入 Rust 產物，需要的是 `wasm-bindgen` / `wasm-pack` 的 cdylib——
>    等於同時維護兩條 Rust→wasm 工具鏈，而原文一條都沒寫。
> 2. **Phase A 會是全計畫體積最大的部署**：現有 TS + npm `iztro`（磁碟 5.9 MB）+ 新 WASM 疊在同一顆 Worker。
>    §6 的合計是**終態**的量，不是 Phase A 的量。
>
> **已改為 service binding 雙 Worker**：Phase A 直接建立獨立的 Rust engine Worker，
> TS Worker 透過 service binding 呼叫。好處是它**與 §4.1b 的退路是同一個架構**——
> 若 `x-iztro` 對拍失敗，紫微留在 TS 側即可，不必改架構。
> 代價是 Phase A 期間確實是雙 Worker；**若 §4.1b 永久啟用，「單一 stack」的目標即打折，這點必須先向使用者講明**（見 §10 T2）。

### Phase 0 探針結果（2026-08-27）

| # | 探針 | 結果 | 判定 |
|---|---|---|---|
| P1 | 現網 `cpuTime`（`wrangler tail --format json`）| `GET /charts/ziwei` wall 391–747 ms / **cpu 3–7 ms**（`GET /health` 0–5 ms）| ✅ **通過** — 遠低於 Free 10 ms，Paid 非阻擋項 |
| P2 | D1 `alpha` 最小 Worker（`worker` 0.8.5 `d1` feature）| `wasm32` 編譯成功，`SELECT users` / `SELECT interpretations` / `DELETE` 語法正確；`release` **1.2 MB raw / 322 KB gzip** | ✅ **通過** |
| P3 | 西洋月球候選（`vsop87` 無月球）| `solar-ephemeris` 0.2.0（MIT/Apache-2.0，2026-07-20）內建 **ELP-MPP02** `moon_apparent_ecliptic` + `VSOP2013`，`wasm32` 編譯成功 **472 KB raw / 326 KB gzip** | ✅ **有解** — 不需立即手寫 ELP2000，改用此 crate，待精度驗證 |
| P4 | `x-iztro` vs `iztro@2.6.0` 對拍（6 組邊界：閏月、晚子時、跨世紀等）| `palaces=12` / `firstPalace` / `majorStars` 亮度 / `sihua` 四化 **逐欄一致** | ✅ **通過** |
| P5 | 體積合計（`vsop87` 0.50 + `x-iztro` 0.40 + `solar-ephemeris` 0.33 擇一 + `worker` 0.32）| **~1.2–1.5 MB gzip** | ✅ **通過** — 對 Paid 10 MB 與 Free 3 MB 皆有餘裕 |

**Phase 0 五道閘門全過，可進 Phase A。**

### Phase A 執行進度（2026-08-27，已部署並生產驗證）

| 項目 | 狀態 |
|---|---|
| Rust engine Worker `fortunet-engine` | ✅ 已部署 `fortunet-engine.yanggf.workers.dev`（1.4 MB gzip，startup 2ms），`worker-build` 打包 |
| `ft-schema`（V3 + Western types）| ✅ `skip_serializing_if` 修正 Zod null 相容 |
| `ft-ziwei`（x-iztro 封裝）| ✅ wasm 對拍生產 `iztro-adapter.ts` 逐欄一致；`hour_to_time_index` 移值生產；`majorLimits` 依陽男陰女順逆重排 |
| `ft-western`（solar-ephemeris + vsop87）| ✅ 太陽（geocentric）/ 月球（ELP-MPP02）/ 上升（GMST 封閉公式）/ Whole Sign 宮位 |
| `fortunet-api` service binding | ✅ `[[services]] FT_ENGINE → fortunet-engine`；`charts.ts` 兩處改用 `fetchEngineChart(env.FT_ENGINE)` |
| **端到端驗證** | ✅ 註冊 → 生辰 → `GET /api/charts/ziwei` 200，`土五局`、12 宮、`hourBranch=未` |
| TS Worker 保留 | auth / D1 快取 / 3-provider failover 仍在 `fortunet-api`，未動 |

**Phase A 期間的事實修正（與 spec 原判不同）**：

1. **`x-iztro` 的 DTO 是完整的**——有 `majorStars`/`minorStars`/`adjectiveStars`、`zh-TW` 繁體名、`decadal.range`。`spec §4.1` 說的「field-for-field」對 DTO 成立；先前 P4「6/6 通過」**假通過**，因當時兩邊都用 `zh-CN` + 只比 `majorStars`。改用 wasm 對拍生產 adapter 才浮出真 bug。
2. **`majorLimits` 需要陽男陰女順 / 陰男陽女逆**（iztro `decadalList()`），非固定 rotation。
3. **`engineVersion` 不一致（待決策）**：Rust engine 輸出 `meta.engineVersionZiwei = "4.0.0"`，但 `charts.ts` 用 `ENGINE_VERSION_ZIWEI`（`3.0.0`）覆寫。紫微側 `x-iztro` 對拍與 iztro **行為一致，無需 bump**；西洋側則因引擎真實化**應 bump**（見 R6）。**目前 `ENGINE_VERSION_ZIWEI`/`ENGINE_VERSION_WESTERN` 均維持 `3.0.0`，西洋的快取未清——這是在西洋引擎通過 §8.2 事件表前的暫態。**

**待辦**：
- 西洋引擎對 JPL 的精度對拍（§8.2 事件表）完成後：bump `ENGINE_VERSION_WESTERN` → `4.0.0` 並清西洋快取
- `scripts/deploy-engine.sh` 已固化部署；`fortunet-api` 用既有 `npm run deploy`
- `crates/web`（Leptos 前端）為 Phase C，尚未開始

### Phase B 執行進度（2026-08-27，已部署並生產驗證 / 單向門通過）

| 項目 | 狀態 |
|---|---|
| `ft-api` crate（routes/D1/middleware/AI failover 全遷）| ✅ `routes/auth.rs` `users.rs` `charts.rs`；`services/` billing、birth_hash、engine client、ai prompts+providers、clock、uuid、db helpers；`durable_objects/` SessionDO + AIMutexDO（`#[durable_object(fetch)]`）|
| **DO storage 相容契約** | ✅ `ft-schema::storage`（Session/ExResource/MinuteRecord，`f64` 處理 JS number）+ serde_wasm_bindgen 橋 → **位元相容** |
| **birth_hash 位元相容** | ✅ Rust `compute_birth_hash` 對 production 已知值 `-19bbe75a` **完全一致** |
| **單向門（覆蓋部署保 session）** | ✅ 同 script name（`fortunet-api`）+ 同 `SessionDO`/`AIMutexDO` class_name + 同 migration tag v1/v2 → **canary session 部署後 `GET /users/me` 200**，session/D1 讀回 |
| 部署量 | wasm 845KB（**330KB gzip**）+ index.js 7.6KB gzip，startup 3ms |
| `[services] FT_ENGINE` service binding | ✅ charts 兩處改用 engine service-binding client |
| `ENGINE_VERSION_WESTERN` | **已 bump → `4.0.0`**（引擎真實化 + 頂層 `sunSign`/`moonSign` 契約；舊 3.0.0 快取缺 `sunSign` 需重算）。`ZIWEI` 維持 `3.0.0`（x-iztro 對拍一致）|
| 整合測試 | `ziwei-iztro.test.ts` ✅ 全綠；`charts.test.ts` 15/17 綠（見下方 AI env 問題）|
| 引擎修復（Phase B 期間發現）| ① API `jd_from_birth` 原把 `Date` constructor 當 `Date.UTC` 呼叫 → NaN（改取 `Date.UTC` 靜態函數）② `Intl` 回 NaN 時 fallback offset=0 ③ engine `jdUtc` non-finite → 400（原會 1101 掛起）④ `ft-schema::WesternChartV3` 補 `sunSign`/`moonSign`（原缺頂層，前端契約不符）|
| AI interpret 503（環境問題，非回歸，待辦）| ⚠️ 三 provider 全失效：iflow `empty response`、groq `moonshotai/kimi-k2-instruct-0905` 404、cerebras `llama-3.3-70b` 404。model 一字不差來自 `ai-mutex-do.ts`（生產原設），**Phase B 前即已 503**（Phase A 只驗 `GET charts/ziwei`，未驗 interpret）。**非 Rust 移植回歸。** 待確認 provider model 正名或 key 失效後更新 |

> **Phase B 事實修正**：
> 1. 測試 `chart_data.sunSign` 斷言揭露：Phase A `ft-western` 輸出 V3 無頂層 `sunSign`/`moonSign`，前端契約（`WesternChart`）不符。Phase B 於 `ft-schema::WesternChartV3` 補上，由真實 Sun/Moon 黃經推導（非舊近似表）。
> 2. `Date.UTC` 為 `Date` 的靜態方法；`Reflect::apply(Date, ...)` 是呼叫建構子 → 回 Date object → `.as_f64()` = NaN。必須取 `Date.UTC` 屬性再 apply。

### Phase C 執行進度（2026-08-27，已部署並驗證）

| 項目 | 狀態 |
|---|---|
| Leptos 0.8.20 CSR 前端 | ✅ `crates/web` 取代 React `frontend/`（945 行），routes/pages/components 全遷，`ft-schema::api` 共用 wire type |
| build 工具鏈 | ✅ `scripts/build-web.sh`（cargo build + `wasm-bindgen --target web`，**不用 trunk**）；`scripts/deploy-web.sh`（+ `wrangler pages deploy`）。wasm 570KB gz（workspace `[profile.release]` opt-level=z+LTO）|
| E2E 驗證 | ✅ Playwright + prod API：register → 出生資料保存 → 紫微 12 宮渲染（土五局、命宮/身宮、紫微(得)）|
| CORS 修正 | ✅ preflight 缺 `cache-control`（前端 `no-cache` header）→ `crates/api/src/lib.rs` allow-headers 補上 |

### Phase D 執行進度（2026-08-27，清理完成）

| 項目 | 狀態 |
|---|---|
| 刪 TS 殘留 | ✅ `backend/`（TS Hono worker）與 `frontend/`（React）整個移除 |
| `schema.sql` 救回 | ✅ `backend/scripts/schema.sql` → `scripts/schema.sql`（D1 唯一權威 schema，含 story 型別、無 CHECK）|
| deploy scripts | ✅ 刪 `deploy-backend.sh`/`deploy-frontend.sh`；建 `deploy-web.sh`；`deploy-engine.sh` 保留 |
| CI | ✅ `deploy.yml` 重寫：fmt/clippy/build wasm（移除 API-token 部署，部署走 OAuth 手動）。pre-push hook 同步改 Rust 檢查 |
| CLAUDE.md | ✅ 全面更新為 Rust stack 描述 |
| `.testing-rules` | ✅ 改為 Rust 版（保留 integration-only / real-env 哲學）|

> **Phase C/D 事實修正**：`wasm-bindgen --target web` 不自動執行（需 index.html boot script `await init()`）；
> Leptos 的 `[lib] crate-type=["cdylib"]` + workspace 成員共用 root target 產物（`target/wasm32-unknown-unknown/release/ft_web.wasm`）。



### §8.2 西洋精度驗證（2026-08-27，對 JPL HORIZONS 事件表初測）

**事件表**（`docs/superpowers/specs/assets/western-ephemeris-event-table.json`）：
地心視黃經 ObsEcLon，QUANTITIES=31，CENTER=500@399，來源 NASA JPL HORIZONS（DE441）。

| 時間點（UTC）| 太陽誤差 | 月亮誤差 | 容差（≤0.05°）|
|---|---|---|---|
| 2026-01-15 00:00 | 0.00016° | 0.00002° | ✅ |
| 2026-03-21 06:00 | 0.00021° | 0.00004° | ✅ |
| 2026-06-21 12:00 | 0.00051° | 0.00001° | ✅ |
| 2026-09-23 03:00 | 0.00018° | 0.00007° | ✅ |
| 2026-11-07 18:00 | 0.00015° | 0.00007° | ✅ |
| 2026-08-27 12:00 | 0.00026° | 0.00008° | ✅ |

**結論**：
- **太陽（solar-ephemeris `sun_apparent_ecliptic`）與月亮（`elpmpp02`）通過 §8.2**——誤差
  0.000几度，比 ≤0.05° 容差低 ~100 倍。這正是 §4.2.2 的兩個 blocking 項（vsop87 無月、上升要另算），已解。
- **上升點**：時序基礎（`solar-ephemeris::time::gmst_deg`，Meeus 12.4）對教科書例 12.a **0.00000°**（可追溯）。
  章動對 Meeus 例 22.a。
- **行星修復（Phase A 遺留 bug，已解）**：vsop87 `*.longitude()` 是**日心黃經（J2000）**、
  非地心視黃經。HORIZONS 對拍原差 1–99°（Venus 99°、Mars 32°）。**改為 vendor
  `solar-ephemeris` 並把 `Body::elements()` 升 pub**，餵給 `planets::planet_apparent_ecliptic`
  （地心視：光行差 + Meeus-21 歲差 + 章動）；移除 vsop87 依賴。修後 9 天體對拍：
  Sun 0.00026° / Moon 0.00008° / Mercury 0.00011° / Venus 0.00042° / Mars 0.00002° /
  Jupiter 0.00002° / Saturn 0.00002° / Uranus 0.00025° / Neptune 0.00000° —— **全 ≤0.05°**。
- **上升點修復（Phase A 遺留 bug，已解）**：原公式
  `atan2(cos θ, −den)` 用了**負號在分子** → 給的是**降點 DSC**；且 probe 的赤緯用了
  `cos ε`（Meeus 13.4 應 `sin ε`）。Grok 審確認正確式為
  **`λ_ASC = atan2(cos θ, −(sin θ cos ε + tan φ sin ε))`**（θ=RAMC，|φ|<90°−ε 時即東昇點、
  南北半球同一式、**不需選支**）。修後 6 點 **alt≈0（≤0.0019°）且全東升**（az 69–115°）。

**§8.2 定論**：西洋引擎（月/日/行星/上升）**全數對 JPL HORIZONS 通過**，spec 的
`ENGINE_VERSION_WESTERN=4.0.0` 提升即告確定。

**回退路徑**：每個 Phase 都是獨立部署。Worker 層用 `wrangler rollback`；前端 Pages 保留前一次部署。

> **rev.4 修訂（Grok 審 P1-1，已對帳採納）——原文把單向門指錯地方**：
> 原文說「Phase B 是唯一的單向門，因為 DO migration（`new_sqlite_classes`）一旦套用無法還原」。
> **這扇門早就過了。** `backend/wrangler.toml` 已有：
>
> ```toml
> [[migrations]]
> tag = "v1"
> new_sqlite_classes = ["SessionDO"]
> [[migrations]]
> tag = "v2"
> new_sqlite_classes = ["AIMutexDO"]
> ```
>
> 生產的 DO **已經是 SQLite backend**（雖然程式仍走 KV API `storage.put/get/delete`，未用 `.sql()`）。
> `new_sqlite_classes` 也不能對既有 class 重跑。
>
> **Phase B 真正的單向門是：同一個 `class_name` 底下把 JS DO 換成 Rust DO。**
> 這要求 storage 的**鍵名與序列化格式位元相容**，否則後果是
> **全站使用者登出**（SessionDO）與 **AI 計量歸零**（AIMutexDO 的 rpm/rpd/exresource）。
> 處置：Phase B 前必須先寫出並驗證 **session storage 相容契約**；
> 若無法相容，就改用新 class 名並接受狀態重置（需明確告知使用者會登出）。
>
> **另注意**：原文假設有 staging 可演練。`CLAUDE.md` 明載 CI **沒有 staging 環境**，
> `test:integration:staging` 指向的主機是否還活著**未經證實**。演練前要先確認。

## 8. 驗證策略（受 `.testing-rules` 約束：只寫整合測試、不得 mock）

### 8.1 紫微 differential test（Phase 0 的關鍵閘門）

> **⚠️ rev.4 修訂（Grok 審 P0-2，已對帳採納）——原設計與 `.testing-rules` 直接衝突，且測錯目標**：
>
> **衝突**：`.testing-rules` 寫死「只准整合測試、禁單元測試／mock／stub／假資料、必須打**已部署**服務」。
> 而原 §8.1 設計的是「Node 跑 iztro、Rust native 跑 x-iztro、離線 JSON diff」——
> **不經過 Worker、不經過部署**，這就是離線雙引擎對拍，不是整合測試。
> 因此 Phase 0 的第四項 exit criterion 原本只有兩條路：違反 `.testing-rules`，或沒寫測試就宣稱通過。
> **兩者都不是閘門。**
>
> **測錯目標**：原文說對拍在 native target 跑「求速度」。但要上線的是 `wasm32-unknown-unknown`。
> native 全綠不能證明 wasm 產物正確。
>
> **原文還把 `ziwei-iztro.test.ts` 說成「種子」——它不是。** 該檔（84 行）打的是 production HTTP，
> 只驗 V3 形狀、`palaces=12`、`timeIndex===12`、`isLeap` 為 boolean、interpret 允許 200/409/503。
> **它從未與 npm iztro 逐欄比對過。** 當靈感來源可以，當「已有 differential test」不行。

**修正後的 §8.1 設計**：

- **已核准一條 `.testing-rules` 窄例外**（見 §10 T5 已定案）：引擎對拍是**跨語言等價性驗證**，
  性質上無法打已部署服務完成。**不核准則 Phase 0 無法關門，Rust 路線就地停止。**
- **對拍的 Rust 側必須是 `wasm32-unknown-unknown` 產物**（用 wasmtime 或 node 載入），**不是 native**
- **兩側都跑真實引擎、真實輸入**，無 mock 無假資料——這一點與 `.testing-rules` 的精神一致
- **輸入優先序（依 §4.1 的曆法風險排列）**：① 閏月 ② 晚子時（`dayDivide='forward'`）
  ③ 月初月末 ④ 跨世紀 1900／2000／2100 ⑤ 南北半球時區
- **比對**：序列化 JSON 逐欄位 diff，**零容差**——宣稱是 field-for-field identical

### 8.2 其餘
- 現有 `charts.test.ts`（175 行）在 Phase B 後**原樣重跑打 Rust Worker**——
  它打的是真實部署，不需改寫就能當回歸測試，這是既有測試哲學的紅利

> **但它抓不到西洋算錯（Grok 審 P1-5，已對帳採納）**：`charts.test.ts:102-112` 對西洋
> 只斷言 `chart_data` 存在且 `sunSign` 有值——**佔位引擎也會綠**。
> `ENGINE_VERSION` bump 後快取失效重算，測試照樣過。它是回歸測試，不是正確性測試。
>
> **且它完全沒有涵蓋 `/story`**（`routes/charts.ts:84-216` 的兩條路由）。
> Part I D1 已定「保留 `/story` 當敘事外衣」，Phase B 的驗收必須補上 story 的整合測試。

- **西洋正確性驗證**：原文寫「已知天象事件（例：日食、行星合相）」——**規格不足以執行**，
  且日食在 §4.2.1 解決月球之前**物理上做不到**。已改為必須先產出一張**可執行的事件表**，
  每一列都要具備：
  | 欄位 | 為什麼必要 |
  |---|---|
  | 日期時刻（**UT**，非本地時）| 沒有時區基準就無法比對 |
  | 座標系（地心視黃經／真黃經／J2000）| 不同座標系差可達數十角分 |
  | 待驗天體（日／月／上升／哪些行星）| 逐點驗，不是整盤「看起來對」 |
  | 期望值與**容差** | 沒有容差就不是判定條件 |
  | 資料來源（NASA JPL HORIZONS 或 IERS）| 期望值必須可追溯，不能自己編——違反「不作弊」 |
  **事件表產出前，西洋不得進 Phase A。** 「人工抽樣核對」沒有樣本清單，不算閘門。

## 9. 風險

**rev.4 重排（Grok 審對帳後）**：原表有兩項等級訂錯、一項描述錯誤，並漏掉五項實質風險。

| # | 風險 | 等級 | 處置 |
|---|---|---|---|
| **R9** | **`vsop87` 無月球理論** → 西洋盤沒有月亮，§8.2 日食驗證不可執行 | **阻擋** | §4.2.2；月球＋上升實作是 Phase 0 exit criterion。**未解決前西洋引擎不動** |
| **R8** | **`.testing-rules` 與 §8.1 引擎對拍不相容** → Phase 0 關不了門 | **已解** | §10 T5 已核准窄例外（見條文） |
| R2 | `worker` 的 **D1 為 alpha** | **高** | Phase 0 探針；不通過即停止整條路線 |
| R3 | `x-iztro` 僅 64 下載，對拍可能失敗 | **高** | §4.1b 退路已定案。**最可能先炸在 `lunar_rust` 曆法**（2023 年後未更新、4,096 下載），不是安星 |
| **R13** | **JS DO → Rust DO 同名 class 的 storage 相容性** | **高** | 不相容 = 全站登出 + AI 計量歸零。Phase B 前須寫出並驗證相容契約 |
| **R12** | **Leptos 的 Pages 建置管線未設計**（Vite → trunk/wasm-pack、MIME、SPA fallback、`dist/.build-info`）| 中 | Phase C 前補設計 |
| **R10** | Phase A 雙 Rust→wasm 工具鏈 | 中 | 已改 service binding 架構迴避（§7）|
| R6 | 西洋輸出形狀全變 → **Phase A 就會把新 JSON 打到現役 React** | 中 | 見 R14；bump `ENGINE_VERSION_WESTERN` 4.0.0 並清快取 |
| **R14** | **`CHART_SCHEMA_VERSION`（現值 3）與前端契約未納入分期** | 中 | `DivinationPage.tsx:147-157` 的 `WesternDisplay` 已把 `sunSign` 當 string 讀（後端給的是物件）。schema bump 與前端修正是同一扇門，不能偷渡進 Phase C |
| **R11** | 若退回 Swiss Ephemeris 系，`swisseph-rs` 等 port 可能是 SE **衍生作品，AGPL 對 SaaS 一樣咬** | 中 | 排除理由改為授權而非下載數（§4.2.2）|
| ~~R1~~ | ~~Workers Paid 未升級~~ | **降為「待測」** | §0.1：因果推論原本是錯的。先量方案 + cpuTime + 啟動時間，再決定是不是阻擋項 |
| ~~R4~~ | ~~`vsop87` 體積超出預算~~ | **降為低** | §6 實測 **0.50 MB gzip**（原估 2–3 MB）。DCE 自動剝除未用版本。體積不是問題，**缺月才是**（R9）|
| ~~R5~~ | ~~DO migration 單向不可還原~~ | **改寫為 R13** | `new_sqlite_classes` 早已套用（wrangler.toml v1/v2），這扇門已過 |
| R7 | Leptos 生態較 React 小 | 低 | 前端僅 945 行，重寫成本可承受 |

## 10. 待裁決（技術層）

| # | 決策點 | 我的建議 |
|---|---|---|
| **T1** | Workers Paid $5/mo 是否現在升級 | **現在升級**——§0.1 已證明它是前提而非選配 |
| **T2** | `x-iztro` 對拍失敗時，走 §4.1b（保留 TS 紫微 Worker）還是放棄 Rust 路線 | **走 §4.1b**，不放棄；紫微是唯一有此問題的模組 |
| **T3** | 現行西洋佔位引擎的處置時機 | **Phase A 一併換掉**——它同時是正確性問題與 CLAUDE.md 違規，不宜留到 Phase D |
| **T4** | `backend/src/services/ziwei/calculator.ts`（242 行死碼）現在刪還是留到 Phase D | **現在刪**——route 已無引用（`ziwei/index.ts:1` 仍 re-export，一併處理）。注意 `ziwei/constants.ts` **不是**死碼，`iztro-adapter.ts:4` 仍用 `EARTHLY_BRANCHES` |
| **T5** ✅ 已定案 | **是否核准一條 `.testing-rules` 例外給引擎對拍** | **核准窄例外**——見下方條文 |
| **T6** ✅ 已定案 | 月球／上升點 | **Hybrid：Phase 0 先做 3–5 天探針，逐一試 permissive 授權的 Rust 候選（如 solar-ephemeris / OxiEphemeris，需獨立驗證 wasm32 與授權），都不過才手寫 ELP2000-82B 截斷 + 上升點封閉公式** | 見下方定案 |

> **T5 說明 — ✅ 已定案（Codex 審 + 對帳後，使用者已裁決：核准窄例外）**：
> `.testing-rules` 要求測試必須打**已部署服務**。但 `x-iztro` vs npm `iztro` 的
> 跨語言等價性驗證，性質上就不是「打服務」能完成的——它要在同一組輸入上比對兩個引擎的完整輸出。
> **不核准例外，Phase 0 的第四項閘門在規則內寫不出來**，只剩「違規」或「不寫測試卻宣稱通過」兩條路，
> 兩者都不可接受。
>
> **核准條文（Codex 擬定，已對帳採納）**：
> > Engine-equivalence gates may execute offline only to compare pinned production engines on identical valid domain inputs, with canonical JSON and zero field/value tolerance, no mocks, stubs, or substitute implementations; Rust must execute the release `wasm32-unknown-unknown` artifact. This exception supplements and never replaces `RUN_INTEGRATION=true` tests.
>
> 範圍嚴格限縮為：**僅限引擎等價性對拍；兩側都必須是真實引擎與真實輸入（無 mock／無 stub／無假資料）；Rust 側必須是 wasm32 產物**。這樣仍守住 `.testing-rules` 的實質精神（不作弊、不造假），只放寬「必須打已部署服務」這一條形式要求。
>
> **不核准的備案**（若日後收回例外）：部署兩個受保護的 staging Worker（Node/iztro 2.6.0 oracle vs Rust/x-iztro wasm）走 HTTP 比對，字面上符合 `.testing-rules`，代價約 2–5 人天。

> **T6 說明 — ✅ 已定案（Codex 審 + 對帳後，使用者已裁決：Hybrid）**：
> `vsop87` 沒有月球（`earth_moon` 是地月質心，已解包 crate 實測確認），Whole Sign 雖只需知道上升點落在哪一宮，但仍需**算對上升點**。
> 體積不是問題（實測 `vsop87` gzip 僅 0.50 MB，Paid 上限 10 MB），`astro` 2016 後未更新不能當備案，Swiss 系 AGPL 對 SaaS 會咬——這三條先前已對帳成立。
>
> **定案採 Hybrid，不直接二選一**：
> - **Phase 0 先做 3–5 天探針**：逐一試 permissive 授權的 Rust 候選的 wasm32 編譯、授權、精度（對 JPL HORIZONS 等權威星曆，月球 ≤0.05°、上升點 ≤0.1°）。
> - **都不過才手寫**：ELP2000-82B 截斷級數（月球 5–10 天）+ UTC/TT/UT1 管線（2–4 天）+ 上升點封閉公式（1–2 天）+ 驗證（3–6 天），合計有經驗者 10–20 人天、一般者 15–30 人天（1900–2100 範圍假設）。
> - **在月球與上升點皆通過驗證前，西洋引擎不動**——寧可繼續跑佔位程式並在 UI 標示「西洋盤為粗略近似」，也不要換成沒有月亮的盤。
> - **縮範圍的代價**（若最終走 b）：西洋盤失去 Big Three 中的兩個（月亮、上升）與全部 12 宮位詮釋，AI 輸出少掉情感需求與人設敘事錨點，API 需標示「partial western profile」並 bump 版本清快取。
>
> **在 T6 探針完成前，西洋不得進 Phase A。**

---

<details>
<summary>以下為 rev.3 技術設計原文（已作廢，保留供審查歷史參考。§4.2 方法論已提煉進 Part I；附錄 A／B 為審查裁決紀錄）</summary>

## 0. 決策紀錄（已與使用者確認）

| 決定點 | 選擇 |
|---|---|
| 引擎架構 | 導入 **iztro**（紫微）＋ **astronomy-engine**（西洋），自製版保留做對照基準（設 sunset 條件，§3.1） |
| 紫微 API 回應 | 升級為**完整盤**（四化、亮度、大限、三方四正） |
| 西洋範圍 | **完整本命盤**（十大行星＋ASC/MC＋主要相位，Whole Sign 宮位） |
| 時辰處理 | 接 `users.timezone` ＋ 真太陽時（僅用於斗數時辰判定，§3.3 切分原則）；晚子時採 iztro `dayDivide='forward'`（已對源碼證實） |
| Big5 測量 | TIPI 十題中性題幹；命理包裝只出現在結果呈現 |
| TIPI 融合 | 先驗＋校正：預測以實測向量為準；落差分析當洞察 |
| 預測模型 | 人格 × 情境交互 |
| API 型別契約 | **新端點採 Zod schema**（V3 圖型/personality/predict）：schema 放 `shared/schemas/`，後端 zValidator、前端 import 型別；既有端點不動，漸進採納 |

## 1. 目標與非目標

### 目標
A1. 紫微核心改由 iztro 計算，輸出完整盤（星曜＋亮度＋生年四化＋大限＋三方四正）
A2. 西洋核心改由 astronomy-engine：十大行星黃道經度（UT）、ASC/MC、主要相位
A3. 出生時間管線升級：前端表單補收 分鐘／時區／城市座標；timezone→UTC；斗數時辰採真太陽時（邊界才生效）
B1. TIPI 十題 → OCEAN 實測向量＋命盤象徵傾向（先驗）
B2. 人格×情境預測端點（新表、新 DO 請求形、規則錨點先行＋LLM 潤寫）

### 非目標
- 臨床級量表；宮位制選擇器；流年流曜細部暴露；付費牆整合
- **資料可攜（匯出）端點**——本期僅提供刪除權；匯出列為後續候選
- 圖表函式庫引入（雷達圖手繪 SVG：雷達圖僅五軸靜態，自繪約百行，避免在 gzip 預算敏感的 Workers bundle 上疊依賴；若實作工時超出預估一倍再重議）

## 2. 架構總覽與分期

```
A1 紫微換庫（含前端紫微盤重寫——現有顯示讀取不存在的欄位，無回歸風險）
A2 西洋換庫（行星 UT ＋ ASC/MC；前端本命盤渲染）
A3 時間管線（表單欄位 → UTC → 斗數 TST；per-type 版本遞增）
B1 人格側寫（schema + TIPI + 先驗 + 落差）
B2 預測（情境 + predict 端點 + 規則庫 + predictions 表）
```

每期獨立交付、獨立部署驗證。**舊前端相容**：PUT /me/birth 對缺席的 minute/timezone/lat/lng 一律走 assumed 預設路徑，永不回 400。

## 3. Phase A：引擎現代化

### 3.1 紫微 → iztro（v2.6.0，MIT）

adapter：`backend/src/services/ziwei/iztro-adapter.ts`

- API（已對 2.6.0 d.ts 核實）：`bySolar(solarDate: string, timeIndex: number, gender: GenderName, fixLeap?: boolean, language?: Language)`；
  `isLeapMonth` 僅存在於 `byLunar`。國曆輸入一律直接 `bySolar`，由 iztro 內部 lunar-lite 換農曆，
  **禁止**先過自製 solarToLunar 再餵 byLunar（兩套曆法疊加會使閏月邊界與 iztro 盤不一致）
- `timeIndex`：hour+minute → 0–12（早子 0…晚子 12）
- **晚子時**：iztro 內建預設 `dayDivide='forward'`（源碼 astro.js:39 `_dayDivide = 'forward'`，晚子按次日安星）。
  adapter 啟動時以 `config({ dayDivide: 'forward' })` **顯式設定**（不依賴隱含預設），
  `meta.dayDivide` 回報；23:00 案例列入 §5 測試。此決定使部分 23:00 出生用戶的盤與自製版不同屬預期修正
- `fixLeap = true`（前十五日本月、後半月次月，與自製版一致）
- `language = 'zh-TW'`；locale 不具 tree-shakable 性質，體積以 dry-run 實測為準
- 序列化：iztro Functional* 類別帶實例級 `toJSON()`（自訂 serialize，不觸發巢狀呼叫）；
  但 V3 形狀與 iztro 不同，adapter **一律逐欄映射**，不直接 stringify 類實例
- **座標系**：iztro 宮位陣列寅起 0；V3 一律地支序（子=0）。映射 helper：
  `branchIndexOf(palaceIndex) = (palaceIndex + 2) % 12`（寅=2），以 adapter 錨點測試釘死
- 映射到 `ZiWeiChartV3`：palaces[12]（地支序；每星 name/brightness/化耀）、majorLimits[]、
  三方四正索引、fourPillars（取自 iztro）、meta（dayDivide/isLeap/fixLeap/timeIndex/hourShifted/assumed）
- 自製 calculator 保留匯出；**sunset 條件**：A1/A2 錨點測試全綠且上線滿兩週無 parity 異常後，
  自製版退出 bundle（保留於 git 歷史供對照）

### 3.2 西洋 → astronomy-engine（v2.1.19，MIT）

新檔：`backend/src/services/western/natal.ts`

- 行星：`GeoVector(body, t_UT, true)` —— 第三參數是 **aberration**（光行差，apparent 位置），非座標系旗標；
  回傳 J2000 赤道直角座標，再 `Ecliptic(vec)` 得真黃道經緯度。月亮直用 `EclipticGeoMoon(t_UT)`
- 角距優先用 **`PairLongitude(body1, body2, date)`**（d.ts:1321 已核實）；逆行判定：相鄰 Δt=1 天經度比較
- ASC/MC（施工級規格）：
  ```
  GAST = SiderealTime(t_UT)                       // Greenwich *apparent* sidereal time，小時 [0,24)
  LST  = (GAST + eastLongitudeDeg/15) mod 24      // 東經為正
  RAMC = LST × 15°                                // 度
  ε    = 23.4392911° − 0.0130042·T − 1.64e-7·T²   // IAU 低階多項式，T = 自 J2000 起的儒略世紀數
  MC   = atan2( sin RAMC, cos RAMC · cos ε )
  ASC  = atan2( cos RAMC, −(sin RAMC · cos ε + tan φ · sin ε) )   // φ = 緯度
  ```
- 宮位：Whole Sign（ASC 所在星座為第一宮）
- 相位容許度：{合0±8, 六合60±6, 刑90±7, 三合120±8, 沖180±8}，常數集中定義
- 星座判定一律黃道經度 ÷30；廢除固定日期表與舊近似函式

### 3.3 時間管線（P0 切分原則：TST 只服務斗數時辰，絕不進入西洋星曆）

```
birth y/m/d/h/min + timezone(IANA)
  ├─→ UT（一次，Intl 換算）──────────────→ 西洋 GeoVector / EclipticGeoMoon / SiderealTime（星曆以 UT 為準）
  └─→ 斗數時辰：LMT = UT + lon×4min
              TST = LMT + EoT           // EoT = 視太陽時 − 平太陽時（NOAA 閉式近似公式實作於 services/western/eot.ts）
              僅當 TST 與鐘錶時間跨過時辰邊界 ±20 分內才採 TST 定 timeIndex
```

- 缺資料決策表（禁止「一律台北」）：

| 有 timezone？ | 有經緯度？ | 行為 |
|---|---|---|
| ✓ | ✓ | 全功能：UT＋TST＋ASC/MC |
| ✓ | ✗ | 西洋：行星可算、**ASC/MC 降級不可算**（null＋assumed 標記）；斗數：跳過 TST，用鐘錶時辰＋assumed:true。**不得拿台北經度配外國時區** |
| ✗ | — | 預設 `Asia/Taipei`，全鏈 assumed:true 揭露 |

- 城市座標：台灣**鄉鎮市區**級靜態對照表放前端、隨請求上送（鄉鎮中心誤差 ≈ 時間 2 分內，遠小於 ±20 分邊界窗，可接受）
- **A3 前置＝前端 BirthDataForm 升級**（分鐘／時區下拉／鄉鎮座標或 opt-in 定位）；未升級前 TST/ASC/MC 不得上線

### 3.4 版本、快取與相容（rev.3：per-type 版本）

- **分類版本**：`ENGINE_VERSION_ZIWEI` 與 `ENGINE_VERSION_WESTERN` 各自內嵌於所屬 chart_data；
  bump 只失效對應類型，避免 A2 上線把全站紫微盤連帶重算（並連帶清掉有效的 AI 解讀）
- 初值皆 `'3.0.0'`；語意決策樹：**計算演算法變** → bump 該類 ENGINE_VERSION；
  **回應 JSON 形狀變** → bump 頂層 `chartSchemaVersion`（前端適配用，不觸發重算）；兩者同時變就都 bump
- `POST /:type/interpret` 守衛：解析既有 `chart_data.engineVersion` ≠ 當前該類版本 → `409 RECALC_REQUIRED`
  ；ETag 摻入對應版本。**前端行為定義**：收 409 → 自動 GET /:type 重算一次 → 成功後重送 interpret；
  再失敗才 toast 錯誤（不自動循環）
- 回應採加欄位策略：V3 新欄位全新增、V2 欄位保留過渡期、頂層 `chartSchemaVersion: 3`

## 4. Phase B：Big5 人格×情境預測

### 4.1 資料模型（D1，no-constraints 慣例）

users 表欄位已齊備（birth_minute/timezone/latitude/longitude 均存在於 schema.sql，無需 ALTER）。新增三表：

```sql
CREATE TABLE IF NOT EXISTS personality_profiles (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  tipi_answers TEXT NOT NULL,        -- JSON [10] int 1–7
  ocean_measured TEXT NOT NULL,      -- JSON 五維 0–100，(mean−1)/6×100
  ocean_prior TEXT,                  -- JSON 同尺度「命盤象徵傾向」
  prior_source TEXT,                 -- 'ziwei'|'western'|null
  measurement_status TEXT NOT NULL DEFAULT 'complete',  -- 'complete'|'skipped_prior_only'
  item_duration_ms INTEGER,
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS situation_checks (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  domains TEXT NOT NULL,             -- JSON {work,love,family,money,health} 0–3
  target TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS predictions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  profile_id TEXT NOT NULL,
  situation_id TEXT NOT NULL,
  divination_type TEXT NOT NULL,     -- 'ziwei'|'western'
  anchors TEXT NOT NULL,
  prediction_text TEXT NOT NULL,
  cache_key TEXT NOT NULL,
  rules_version TEXT NOT NULL DEFAULT 'rules-1',
  prompt_version TEXT NOT NULL DEFAULT 'prompt-1',
  created_at TEXT DEFAULT (datetime('now'))
);
```

遷移路徑：`backend/scripts/migrations/*.sql`，以 `unset CLOUDFLARE_API_TOKEN && npx wrangler d1 execute fortunet-db --file <file> --remote|--local` 施行。
`interpretations` 的 CHECK 殘留 `'bazi'`：predictions 有獨立表不受影響；SQLite 移除 CHECK 需重建表，風險大於收益，**保留殘留並記錄**。

版本遞增策略：RULES_VERSION/PROMPT_VERSION 手動語意遞增（規則集或 prompt 模板實質變更才動）；舊 predictions 保留但被新版本取代。

### 4.2 方法論（雙審修訂版）

- **題幹中性**：TIPI 十題公開繁中版語意對，作答畫面零命盤資訊（防 priming）；命理包裝只在結果頁
- **先驗命名與出處**：UI 稱「**命盤象徵傾向**」（不稱命格人格）；`priors.ts` 每條規則附來源等級標注
  （古典文本歸納／設計者判斷），透明可 review
- **落差門檻**：TIPI test-retest 信度有限（±15 分量級噪声），落差 <15 分不呈現差異敘事，僅顯示兩向量
- **亂答偵測**：作答時長／全端作答／正反題矛盾三項；觸發行為＝提示重測一次，仍失敗才存為
  `measurement_status='skipped_prior_only'`；使用者主動跳過亦同。predict 對 skipped 狀態回 `409 MEASUREMENT_REQUIRED`
- **量尺**：每維兩題均分（反向翻轉）→ `(mean−1)/6×100`；先驗調整量 ±20/±10/0（100 分制），多星命中取平均，clamp [0,100]；
  兩盤並存時 prior_source 以 ziwei 為準

### 4.3 先驗規則表與領域映射（純規則，LLM 不打分）

- `services/personality/priors.ts`：主星/星座 → 特質調整透明對照表（含口訣出處）
- `services/personality/rules.ts`：trait×domain 錨點（高N×money高壓→壓力放大等），`RULES_VERSION` 常數；
  領域→宮位固定映射：work→官祿、love→夫妻、family→田宅、money→財帛、health→疾厄

### 4.4 API 與 AI 接線

| 端點 | 方法 | 備註 |
|---|---|---|
| `/api/personality/tipi` | POST | 驗證：長度恰 10、整數 1–7、伺服器端最短作答 ≥5s |
| `/api/personality/me` | GET | 最新側寫 |
| `/api/personality/me` | DELETE | 個資刪除權：清 personality_profiles/situation_checks/predictions 三表 |
| `/api/personality/situation` | POST | domains 0–3 校驗 |
| `/api/charts/:type/predict` | POST | authMiddleware＋獨立 rate limit（10/min/IP） |

predict 流程：錨點 JSON（OCEAN＋domains＋宮位活化＋規則命中；**不上傳原始十題**）→ AIMutexDO 新增
`{ kind:'predict', payload }` 分派（現有 handleRequest 只認 interpretRequest）。**佇列策略明示**：
v1 共享單一 FIFO、不設優先級；DO metrics 加 per-kind queue depth；p95 等待超標再議分流。
潤寫輸出過 schema 校驗（禁止新增錨點外因果）→ 存 predictions。
快取鍵 = hash(ocean_measured + domains + target + divination_type + birth_data_hash + RULES_VERSION + PROMPT_VERSION + 對應 ENGINE_VERSION)；
`birth_data_hash` 由 predict 當下讀 users 表後呼叫**共用的 computeBirthHash**（自 routes/users.ts 抽至 `services/birth-hash.ts`，兩處共用同一實作）取得，禁止重寫第二份演算法。

個資合規（上線前檢核）：同意基礎（主動作答視為同意，UI 聲明目的）、保存期限（跟隨刪除權即時清除）、
LLM 傳輸揭露（僅傳錨點不含原始答案）。註：TIPI 答案未必落入台灣個資法 §6 特種個資法定類別，
惟產品自我要求按高標準處理。

### 4.5 前端

- `/personality`＋ProtectedRoute：問卷（中性題幹）→ 結果頁（SVG 雷達疊加實測/象徵傾向＋落差文字；落差<15 分僅並列）
- DivinationPage 重寫：紫微十二宮卡片（亮度/四化徽章/大限列）、西洋行星表＋相位列表；情境勾選→預測區塊
- 統一錯誤處理：409 RECALC_REQUIRED（自動重算流程）、409 MEASUREMENT_REQUIRED（導向問卷）、429/400 toast 對照表
- BirthDataForm 升級（§3.3）；隱私文案（目的限定、刪除入口、「趨勢參考非診斷」免責）

## 5. 驗證策略

- **已提交 integration tests**（RUN_INTEGRATION=true 才跑）：
  - iztro 錨點：出版範例盤（固定輸入→預期命宮主星＋四化），不用名人生辰
  - 西洋錨點：astro.com 對照，鎖 tropical/geocentric/apparent；行星 ±1°、ASC/MC ±2°；**不比宮頭**（Placidus≠Whole Sign）
  - 23:00 案例（dayDivide='forward' 行為）、閏月案例、interpret 對 stale engineVersion 的 409 流程
- 拋棄式腳本續用於開發期曆法錨點（JDN 法）；擋迴歸靠上述已提交 tests
- 上線門檻：`wrangler deploy --dry-run` gzip<3MB；CPU 以實測為準（目標 <10ms/request），超標拆 DO 或升付費
- 部署前 commit；`wrangler dev` 本地驗證需使用者確認

## 6. 已知差異清單（自製 vs iztro）

| 情境 | 自製版 | iztro | 備註 |
|---|---|---|---|
| 23:00 出生 | 子時、當日日柱 | dayDivide='forward'：次日安星 | 已定案，§3.1 |
| 閏月 | 十五日界 | fixLeap=true 相同 | 應一致 |
| 四化/亮度/大限 | 無 | 有 | 功能增量非衝突 |

## 7. 風險與緩解

| 風險 | 緩解 |
|---|---|
| iztro 依賴鏈超過 gzip 上限 | dry-run 量測門檻；Workers 不支援動態 import、單 bundle 無 code-split 可救——超標即啟用自製引擎 fallback（架構保險，非可選項） |
| Workers 10ms CPU 免費層 | 上線前基準測試 cache-miss 全路徑；超標拆計算到 DO 或升付費 |
| 真太陽時讓老用戶的盤變了 | meta.hourShifted 揭露＋release note；TST 只在邊界 ±20 分生效 |
| LLM 幻覺 | 錨點先行＋schema 校驗＋prompt 只含結構化錨點 |
| 心理資料保護 | 刪除端點、目的限定、原始答案不出本地、UI 免責 |
| 前後端切換窗口 | 加欄位策略＋chartSchemaVersion |

---

## 附錄 A：rev.2 修訂紀錄（Grok 審查裁決）

30 條發現：28 採納、2 留驗。P0 三項全數證實並修入：真太陽時時間軸切分（§3.3）、缺輸入資料決策表與表單前置（§3.3/A3）、
TIPI 去 priming（§4.2）。我方獨立驗證四項事實：wrangler.toml 無 nodejs_compat ✓；DivinationPage.tsx:105 讀取後端不存在欄位 ✓；
schema.sql:37 CHECK 殘留 'bazi' ✓；BirthDataForm 未收集 minute/lat/lng ✓。

## 附錄 B：rev.3 修訂紀錄（Qwen 3.8 Max 盲審 × 套件實物驗證的三方裁決）

Qwen 30 條：約 20 條採納/部分採納、2 條 **P0 級幻覺被實物證據駁回**、其餘由原始碼探針解案。

**實物驗證結果**（npm pack 拆 2.6.0/2.1.19 tarball 直查源碼）：
| 待定點 | 結論 | 裁決 |
|---|---|---|
| iztro bySolar 簽名（Qwen 稱 rev.2 寫錯，P0） | d.ts:63 與 rev.2 完全一致 | **Qwen 幻覺，駁回** |
| users 表缺 timezone（Qwen 稱需 ALTER，P0） | schema.sql:18-21 timezone/latitude/longitude/birth_minute 全都在 | **Qwen 幻覺，駁回** |
| dayDivide 預設值（懸案） | astro.js:39 `_dayDivide='forward'` | **定案：採 forward、顯式設定** |
| toJSON 存在性 | FunctionalStar/FunctionalSurpalaces 帶實例級 toJSON | 定案：存在，但仍逐欄映射 |
| PairLongitude 存在性 | d.ts:1321 `PairLongitude(body1, body2, date)` | 定案：採用（Qwen 給的四參簽名也不對） |
| obliquity 來源（Qwen 提 e.Tilt(t)） | astronomy-engine 無地球 obliquity API | 改 IAU 多項式寫死 §3.2 |

**Qwen 實質貢獻（採納）**：per-type 引擎版本號避免跨類型無謂重算（§3.4，Grok 與我都漏了）、先驗來源等級標注與
「命盤象徵傾向」命名（§4.2）、TIPI 噪声→落差<15 分不敘事（§4.2）、skipped 狀態機觸發條件（§4.2）、
409 前端行為與統一錯誤處理（§3.4/§4.5）、佇列 FIFO 明示＋queue depth 監控（§4.4）、cache_key 共用 computeBirthHash（§4.4）、
自製引擎 sunset 條件（§3.1）、版本語意決策樹（§3.4）、gzip 無 code-split 出路的風險措辭（§7）、資料可攛列非目標（§1）。

**部分駁回**：個資法特種個資分類（法定類別不含心理測驗答案，採高標準自我要求但不引用錯誤法條）、
雷達圖改用圖表庫（bundle 預算理由成立，維持手繪並設重議門檻）、bazi CHECK 重建清理（SQLite 需重建表，風險>收益，記錄保留）。

教訓：Qwen 的兩條 P0 若未經拆包驗證直接採信，會改錯正確的 API 呼叫並寫出多餘的資料庫遷移——
**外部審查的事實性主張必須以實物證據裁決，方法論主張按品質裁決**。


</details>
