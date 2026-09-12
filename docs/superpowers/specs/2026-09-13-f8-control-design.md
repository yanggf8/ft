# F8 對照組 — 技術設計切片(rev.2)

- 日期:2026-09-13(rev.2 — Kimi 審查 5 major + 2 minor 全採,見文末對帳)
- 上游規格:`docs/superpowers/specs/2026-08-26-engine-modernization-big5-design.md`
  §P0 表(:171)、對照組設計(:345-376)、D6 定案(:396-430,✅ 已定案)

## 產品裁決(2026-09-13,owner 核准)

1. **逐列 25%** 入對照組;洗牌向量抽「其他使用者的**最新** complete profile」,
   **池中無他人 → 誠實跳過對照列**(該槽回真實組,不造假)。
2. **解盲 = 週事後摘要行**:該週回饋兩段收齊後,卡片底部 muted 顯示
   「本週有 M 條為對照樣本(研究用)」;不逐條指認。

**rev.2 對帳(Kimi #6)**:上游 :365「使用者可在結果頁事後得知**本次是否為對照**」
被裁決降級為週級聚合摘要。理由:逐條指認會給使用者辨識對照列的線索,污染其
回饋行為(把對照列答成 missing 的誘因),破壞 D6 差值的可信度;週級聚合仍滿足
「事後可得知」的倫理目的。此降級為對上游條款的明白覆寫,非默認。

## 0. 紅線與不變式

1. 對照列的一切行為真實列一致:同一錨點目錄、同一 `filter_negative_half`、
   同一 `source='rule_anchor'`(讀回路徑對其他 source fail-closed,**不改**)、
   同一 F6 動線(`TriggerClass::question()` 逐字同問法、遮罩、一次性回饋)。
2. D6 判定參數以**預先登記檔**固化(`docs/preregistration/f8-d6.md`),於任何
   `is_control=1` 列產生**之前** commit,登記後不得更改(內容見 §3,逐字轉錄)。
3. 對照列的 `user_id` = 觀看者本人;洗牌來源者的 OCEAN **不落庫**(只產出預測文字)。
4. 統計分析工具**不在本切片**(N4:v1 = 資料結構;資料累積至樣本充足後另案)。
5. freeze(`prediction_generations`)之後**不得報錯**:一切對照生成失敗
   (池空、資料損壞、crypto 不可用)一律降級為該槽真實組(Kimi #2/#7)。

## 1. 生成規則(`services/predictions.rs` generate())

- 對每個 domain 槽位(Work、Money),在真實選則**之前**擲 25% 籤:
  `uuid::secure_bytes(1)` → **None(Crypto 不可用)= 該槽回真實組**(fail-closed,
  Kimi #7);`byte % 4 == 0` 為中籤(256 % 4 == 0,無模偏誤)。
- 中籤槽位:
  1. 抽樣 SQL — **每使用者取最新 complete 列,再對使用者隨機**(Kimi #1,
     對齊真實路徑的「最新」語法,避免重測使用者的舊側寫被過度加權):
     ```sql
     SELECT p.user_id, p.ipip_answers, p.ocean_measured FROM personality_profiles p
     WHERE p.measurement_status = 'complete' AND p.user_id != ?me
       AND p.id = (SELECT q.id FROM personality_profiles q
                   WHERE q.user_id = p.user_id AND q.measurement_status = 'complete'
                   ORDER BY q.created_at DESC, q.rowid DESC LIMIT 1)
     ORDER BY RANDOM() LIMIT 1
     ```
     無列 = 誠實跳過(該槽回真實組)。
  2. **先驗後用**(Kimi #2):被抽中列的 `ipip_answers` JSON 與 `dim_ranges` 若
     損壞/驗證失敗 → 有界重抽(最多 3 次)→ 仍敗 → 該槽回真實組。
     freeze 之後任何一路失敗都不報錯、不 500。
  3. 以通過驗證的列走**完全相同**的管線:`dim_ranges(ipip_answers)`、
     `display_rounded(ocean_measured)`、`select_for_domain`、
     `filter_negative_half`(對最終整組列集合套用,真實/對照對稱)。
  4. 洗牌向量對該 domain **零命中**(`select_for_domain` 回 None)→ 該槽誠實空
     (不硬湊);此情形使對照占比統計上略低於 25%,接受(登記於 §3)。
  5. INSERT 帶 `is_control = 1`;其餘欄位(來源、週期、規則版本)與真實列相同。
- 未中籤槽位:照現行真實路徑。
- `filter_negative_half` 對「真實+對照混合的最終列集合」套用:D2-A 語意
  (全負面週保留較佳 1 條)對兩組對稱作用於**輸入**;但被 D2-A 壓掉的列
  (含其對照指派)不落庫 — 此「存活者語意」登記於 §3(Kimi #5a)。

## 2. 讀回、揭露與同意

- 讀回:零改動(`source` 恆 `rule_anchor`;wire `isControl` 已存在)。
- 前端 `PredictionsCard`:該週回饋兩段收齊後,卡片底部 muted 行:
  `本週有 {M} 條為對照樣本(研究用)`(M = `isControl` 真的列數;0 時不顯示)
  — 見文末 rev.2 對帳(逐條指認降級為週級摘要)。
- **作答前聲明**(Kimi #6 — 上游 :384-385 同意基礎):測驗頁作答區前加 muted 行:
  「你的作答與回饋可能以匿名方式用於研究對照;可隨時刪除全部資料。」
- F7 刪除確認文字保留研究揭露:「人格資料將用於匿名研究對照(可隨時刪除)」。

## 3. 預先登記內容(`docs/preregistration/f8-d6.md`,逐字固化,登記後不得更改)

- **分析單位**:預測條目;使用者 = 隨機效應(同一人多條不當獨立樣本)。
- **命中率** = `occurred+hit / occurred`。`absent` 不進分母;`occurred` 且未交
  第 2 段 = 缺測(不進分母、不當 miss);**`occurred+other` 只進分母、不算命中**
  (上游 §5.4.1 逐字)。
- **通過門檻**:真實組 − 對照組命中率 **≥ 5 個百分點(對應 r ≥ .10 / d ≥ 0.20)**
  **且 p < .05(雙尾)**。加嚴參考檔:**7.5pp / r=.15 / d=0.30**。
- 同時報告兩組絕對命中率(證明非 Barnum 基礎率);**兩組回饋率差異必檢**
  — 回饋率 = 該組 `occurred` 列中完成第 2 段提交的比例(`absent` 不進分母);
  差異顯著 → 該批資料作廢。
- **「批」的定義**:自然曆週;分析以**全部累積資料**執行;不許期中停樣或
  挑選分析時點。首批分析於兩組合計 `occurred` ≥ 50 列時。
- **指派機制登記**:逐槽 25%(`secure_bytes(1)[0] % 4 == 0`,CSPRNG,
  crypto 不可用 → 該槽真實);來源 = 其他使用者最新 complete profile 隨機抽;
  逃生口(池不足/資料損壞/零命中/crypto 不可用 → 該槽真實)使上線初期
  對照占比低於 25% — **所有可指派週一律 ITT 全數計入**,不排除低密度週。
- **D2-A 存活者語意**:全負面週被 `filter_negative_half` 壓掉的槽位(含其
  對照指派)不落庫、分析不可恢復 — 此為登記過的設計特性;兩組對稱承受。
- **D5**:`skipped_prior_only` 狀態的一切回饋排除於 F6/F8 統計。
- 對外文案在正差達標前禁用「準確」;負差顯著 → 主張推翻,觸發 §1 下修。
- 分析工具與最小 N:另案(樣本充足後);首批分析門檻如上。

## 4. 測試(TDD)

- 生成:`is_control` 旗標正確落庫;籤注入點可替換(生產 `secure_bytes`,
  測試注入固定序列);25% 分布統計測試(大樣本比例 0.20–0.30);池不足回
  真實列;洗牌向量零命中 → 該槽無列;`filter_negative_half` 對混合集合對稱。
- 讀回:`isControl` wire 布林正確;遮罩對對照列同樣作用。
- 既有 F5 golden 測試不得改動(紅線:真實路徑輸出不變 — golden 注入
  「永不中籤」序列)。

## 5. 不做的事

統計分析工具、簽名層置換對照(N8,複用本切片機制,另案)、三領域擴充、
D2-A 廢除(觸發器=三領域落地)、`source` 切 `personal_record`(門票=F8 對照達標)、
F8 揭露的逐條指認(rev.2 對帳,見文首)。

## rev.2 對帳(Kimi 審查,2026-09-13:5 major + 2 minor 全採)

#1 抽樣改最新-per-user;#2 洗牌列先驗後用 + 有界重抽 + freeze 後不報錯;
#3 D6 三指標逐字登記;#4 coverage/句長改登記平衡檢定承諾(同目錄同管線近似
匹配的論證一併登記);#5 D2-A 存活者語意 + `other` 計分 + 回饋率公式登記;
#6 揭露移至作答前聲明 + 逐條指認降級的 rev. 對帳;#7 `secure_bytes` None 分支。
minor:#8 批定義、ITT 處理、D5 重申併入 §3。原文見對話記錄。
