# 興趣探索理論基礎改向研究:從 RIASEC 特質測量到預測處理(Predictive Processing)

日期:2026-11-17 | 階段:研究(設計之前)| 觸發:站長指示改掉 RIASEC 路線,
改以「大腦預測/預測誤差最小化」的假設為基礎,重新構思自我探索功能。

## 0. 範圍與結論先行

- 站長所稱「預測最小偏差的假設」即認知科學的 **prediction error minimization(PEM,
  預測誤差最小化)**,屬預測處理(Predictive Processing,PP)理論家族。可能出處:
  Andy Clark《The Experience Machine》(中譯《體驗機器:我們的大腦如何預測和塑造現實》,
  簡體 2025)或 Jakob Hohwy《The Predictive Mind》(中譯《預測心智》)。
- **研究結論:改向站得住。** 但有三個必須帶進設計的修正:(1) 驅動力不是「最小化
  不確定性」而是「學習進度」;(2) PP 是啟發不是教條,理論本身在學界仍有強弱版之爭;
  (3) 與本站 guardrail(不借腦科學背書命理)的邊界要先劃清楚。
- 已部署的 interest v1(RIASEC)處置待站長裁示,見 §7。

## 1. 理論核心:大腦是預測機器

PP 主張:大腦不是被動接收感覺,而是持續用**分層生成模型**預測感覺輸入;只有
「預測誤差」往上傳;模型靠最小化誤差(貝葉斯更新)與調整**精確度權重**(哪些誤差
值得聽)來學習;行動本身也是預測的一種(主動推論 active inference——改變世界去
符合預測,而非只改預測去符合世界)。知覺因此被稱為「受控幻覺」。

理論地位(誠實面):Clark 的 embodied/行動版 vs Hohwy/Friston 的強版(自由能原理)
仍在爭辯;「萬物皆 PEM」被批為不可證偽。**產品上把它當設計隱喻與啟發,不當科學認證。**

## 2. 關鍵修正:驅動力是「學習進度」,不是「最小化誤差」

這是本研究最重要的發現,直接反直覺:

- 若人只想最小化預測誤差,最優策略是躲在可預測舒適區或盯著純噪音(兩者都「解決」
  了誤差問題但學不到東西)——即 noisy TV problem。
- 計算與實證的共識:好奇的內在獎勵來自 **learning progress(預測誤差的下降率)**,
  不是誤差本身的高低(Oudeyer 等的 intrinsic motivation 框架;行為實證 Ten et al.
  2021、Poli et al. 2022)。
- **Goldilocks effect**(Kidd, Piantadosi & Aslin 2012,PLoS ONE):嬰兒對「太可預測」
  與「太不可預測」的事件都移開視線,注意力峰值在中度驚喜區——倒 U 曲線。
- Loewenstein 資訊落差理論(1994):好奇 = 察覺已知與想知之間的落差;**先給一小口
  (priming dose)會增加而非減少好奇**。

對產品的含義:直譯「最小化誤差」會做出推薦繭房(給你已喜歡的);正確的形態是
**維持「中度誤差 + 可學習」的探索迴圈**——預測、驗證、更新,誤差下降時最爽。

## 3. 興趣的過程觀:Hidi & Renninger 四階段 × PP 對照

教育心理學的興趣發展模型(Hidi & Renninger 2006,引用 8800+)把興趣當**過程**而非
類型,與 PP 對接得非常乾淨:

| 階段 | 教育心理學描述 | PP 語言 |
|------|----------------|---------|
| 1 觸發的情境興趣 | 新奇/驚訝/外部提示引起注意 | 預測誤差事件 |
| 2 維持的情境興趣 | 持續有意義的投入 | learning progress 迴圈 |
| 3 萌芽的個人興趣 | 自發重訪、產生自己的問題 | 能自我生成預測與誤差的模型 |
| 4 成熟的個人興趣 | 深度、自續、有價值感 | 穩定但持續更新的生成模型 |

**本體論差異**:RIASEC 問「你是哪一型」(特質、單次測量、貼標籤);過程觀問
「什麼觸發你、什麼讓你想回去」(事件、縱向、可更新)。這是站長想改掉的深層理由。

## 4. RIASEC 這邊的證據(誠實盤點)

- Nauta 2010 權威綜評:congruence(人境匹配)假說有支持但**效果量 modest**——
  「測出興趣碼→預測滿意度」的實用鏈條比宣稱的弱。
- 興趣碼有 rank-order 穩定性,但會因教育、工作經驗、重大生活事件而移動;六角結構
  「部分支持」。RIASEC 沒有被推翻——是**本體論不同**,加上本站脈絡(非職業媒介、
  無中文常模)本來就用不上它的長處(職業媒合、常模百分位)。
- 附帶:RIASEC 穩定性研究甚至顯示興趣比人格更穩——「改掉」的理由不是「RIASEC 錯了」,
  而是「它回答的問題(靜態分型)不是本站想回答的(動態探索)」。設計文件裡要這樣寫,
  避免過度宣稱。

## 5. 與 FortuneT 現有結構的對接(repo 內部)

- **F5 本週預測已經是 PP 迴圈**:generate(模型預測)→ 週遮罩 → 一次性回饋(誤差
  信號)→ 情境鎖 → 只寫當週。新理論不是外掛,是把既有結構說清楚、並把「興趣探索」
  也做成同構的預測-驗證迴圈。
- **guardrail 同向**:GUARDRAIL_ZH/EN 的「傾向非注定」與 PP 的核心(先驗可被誤差
  更新;你不是你的先驗)方向一致,理論換軌反而強化既有立場。
- **「信則靈」的 PP 解釋與對沖**:精確度加權偏誤讓強先驪壓制證據(PP 版確認偏誤;
  極端即妄想的預測誤差模型,Corlett)。命理產品的固有風險正好被 PP 說明——設計上
  要「記錄誤差,不只記錄命中」(例如預測命中率誠實呈現),這是別家命理產品不會做、
  本站理論上應該做的差異點。
- **紅線**:命理解讀 system prompt 不得借腦科學背書命理。新功能的 PP 是**設計基礎**,
  文案不得宣稱「因為你的大腦是預測機器所以這個功能準/科學」。兩者的界線:PP 解釋
  「為什麼這樣的探索過程有效」,不背書「命盤內容為真」。

## 6. 產品方向假設(候選,設計階段展開,尚未決定)

- A. **誤差日誌**:記錄「什麼讓你驚訝」(觸發事件),而非「你喜歡什麼」(自陳特質)。
- B. **小型預測實驗**:使用者先猜「我會喜歡 X 嗎」→ 實際體驗 → 對比預測與實際,
  誤差就是內容。與 F5 同構,但以興趣/日常為題。
- C. **四階段階梯**:按 Hidi & Renninger 設計內容(觸發卡→微任務→自己出題→自我引導),
  功能目標從「測量」變「推進階段」。
- D. **priming dose 式揭露**:結果不一次給全,留「剛好的缺口」驅動下一步。
- 既有資產:interest v1 的 API 骨架(冪等寫入、epoch、歷史、刪除、E2E 腳本)可承接
  內容層替換;`interest_results` 表的六域欄位綁 RIASEC,需遷移或新表。

## 7. 待站長裁示(設計前必須定)

1. **v1 處置**:下架(移除入口+停用端點)?保留入口但凍結(不再推廣、授權信作廢)?
   授權信確認作廢?
2. **PP 的顯隱**:當「看不見的設計基礎」(使用者只感受到探索迴圈),還是當「顯性
   賣點」(教使用者預測處理的觀念)?影響文案深度與 guardrail 邊界。
3. **對頻確認**:站長讀的是《體驗機器》(Clark)還是《預測心智》(Hohwy)?兩者
   對「行動」的 emphasis 不同,影響設計敘事。

## 8. 主要來源

- Clark, *The Experience Machine*(2023;簡中《體驗機器》2025);*Surfing Uncertainty*(2017)
- Hohwy, *The Predictive Mind*(2013;中譯《預測心智》)
- Oudeyer, Gottlieb & Lopes (2016), Computational models of intrinsic motivation,
  *Trends in Cognitive Science*;Oudeyer (2018) arXiv:1802.10546
- Kidd, Piantadosi & Aslin (2012), The Goldilocks Effect, *PLoS ONE*
- Ten, Min, Siegel et al. (2021), Humans monitor learning progress;Poli et al. (2022),
  *Acta Psychologica*
- Loewenstein (1994), The Psychology of Curiosity, *Psychological Bulletin*
- Hidi & Renninger (2006), The Four-Phase Model of Interest Development,
  *Educational Psychologist*
- Nauta (2010), Holland's theory: development, evolution, status, *J. of Vocational Behavior*
- Kidd & Hayden (2015), The psychology and neuroscience of curiosity, *Nature Neuroscience*
- Corlett 等對妄想/剛性信念的預測誤差模型;Kaltenbock (2016) rigid beliefs
