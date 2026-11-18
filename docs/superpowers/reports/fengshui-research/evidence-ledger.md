# 空間風水 v1 證據帳本（回原文核對）

日期：2026-11-17
對象：`docs/superpowers/specs/2026-11-17-fengshui-room-v1-design.md` §2.2–§2.4（徽章、帳本規則、§2.3 初值表）
性質：敘述與核對筆記（spec §2.4：`evidence.rs` 才是唯一來源；本檔供其取值與審稿）。

## 0. 判讀規則

- `full_text_read: true` 只在**本輪親自讀到全文主體**（方法＋結果＋討論）時標記；經 WebFetch 代讀、只讀摘要、只讀新聞稿，一律 `false` 並註明。
- 「讀取管道」欄記錄實際讀到的檔案來源，方便下次重核。
- 「支撐判定」三級：**撐得住**（卡片若照本帳本寫法引用，句句對得到原文）、**部分**（只撐得住較窄的說法，需改寫）、**撐不住**（不應出現在該卡研究欄）。
- 效果量、樣本數一律照原文抄；原文未報者寫「原文未報」，不推算。

---

## 1. 必核清單（15 篇）

### 1.1 `dosen2016` — 瞭望－庇護理論證據彙整

- **書目**：Dosen, A. S., & Ostwald, M. J. (2016). Evidence for prospect-refuge theory: a meta-analysis of the findings of environmental preference research. *City, Territory and Architecture*, 3, 4. DOI 10.1186/s40410-016-0033-1
- **URL**：https://link.springer.com/article/10.1186/s40410-016-0033-1
- **讀取管道**：Springer 官方 PDF（CC BY，全 14 頁）
- **研究設計**：自稱 meta-analysis，實為**計票式文獻分類**（vote counting）：把 34 篇量化研究的結論逐項歸為支持／中性／反對，**沒有合併效果量**。作者明說不評估各研究的嚴謹度，結論「prima facie」接受。
- **樣本**：34 篇研究（都市 3、地景 10、室內 21）；參與者總數原文未報。作者另指出多數問卷研究參與者人數偏少。
- **實際測量的結果變項**（被納入研究）：多為照片／場景的**偏好、舒適、寬敞等評分**（問卷），以及 isovist 等**幾何計算**；沒有睡眠、壓力荷爾蒙或工作表現。
- **主要發現**：
  - 全部 82 筆判定：支持 36、中性 34、反對 12（Table 4）。
  - 瞭望（prospect）：支持 19／中性 10／反對 2。
  - 庇護（refuge）：支持 8／中性 14／反對 5；室內有 15 篇測庇護，「mostly with relatively neutral findings」，只有 2 篇支持部分圍合，2 篇反對。
  - 結論原文：「The quantitative evidence for prospect-refuge theory remains inconsistent」；「the evidence for the latter [refuge] is far weaker than for the former, and especially so for interiors」。
- **限制**：非統計合併；不評估研究品質；刺激多為照片與電腦分析；最常被建築界引用的是自然地景研究，不是室內。
- `full_text_read: true`
- **支撐的卡**：`bed_back`、`seat_back`（庇護一半）、`bed_door_view`／`seat_door_view` hidden（瞭望一半）。
- **支撐判定**：**撐得住**「瞭望－庇護研究測的是景觀與空間偏好評分，結果不一致；『背後有遮蔽』那一半在室內研究中多為中性」。**撐不住**任何關於睡眠、壓力、專注的說法。spec 寫「統合分析」應改為「文獻彙整（計票）」較精確。

### 1.2 `psathiti2017` — 休息室座位使用率

- **書目**：Psathiti, C., & Sailer, K. (2017). A prospect-refuge approach to seat preference: Environmental psychology and spatial layout. *Proceedings of the 11th International Space Syntax Symposium*, Lisbon, paper #137.
- **URL**：https://discovery.ucl.ac.uk/1568213/
- **讀取管道**：UCL Discovery 作者 PDF（全 15 頁）
- **研究設計**：觀察研究。英國 3 個咖啡廳式客戶休息室（共 6 層），9 天觀察；每個座位 17 次快照計算使用率；以 space syntax（isovist、connectivity）量化座位空間屬性，做單變項相關與多元迴歸。
- **樣本**：158 組家具座位；時間序列觀察共記錄 526 人。
- **實際測量的結果變項**：**座位使用率**（被坐的次數／17 次快照）。沒有測壓力、專注、舒適感。
- **主要發現**：
  - 單一空間變項解釋力都很低（R² 0–0.10）；isovist 面積與使用率無關（R²=0.00，p=0.63）。
  - 合成指標「可看範圍／可被接近程度」比值與使用率小幅正相關（R²=0.06–0.08）。
  - 最佳模型（空間比值＋家具類型＋座向）解釋 41% 變異（R²=0.414／0.410）；扶手椅最受歡迎。
  - **座向**：D0「背向走道（面朝遠離動線）」的座位最受歡迎，D1「正面直視主要走道」、D2、D3 的係數皆為負（B=−0.12 至 −0.19）。
  - 文中**沒有**「背靠牆」這個變項。（注意：Sailer et al. 2021 引用本篇時寫成「背後受保護、例如靠牆的座位較受偏好」，與本篇實際變項不符。）
- **限制**：作者自述觀察數有限、只有三個場所；只能描述集體使用型態，不看個人差異；座位是否靠窗、靠電視等「吸引物」強烈干擾空間效果。
- `full_text_read: true`
- **支撐的卡**：`seat_back`、`seat_door_view`。
- **支撐判定**：**部分**。只撐得住「一項休息室觀察研究測的是座位使用率；空間位置的解釋力小，家具與座向影響更大」。**撐不住**「人偏好背後靠牆」——本篇沒測；而且背向走道的座位反而最常被坐，與風水「背後不要是走道」方向相反。

### 1.3 `barneta2006` — 曲線物件偏好

- **書目**：Bar, M., & Neta, M. (2006). Humans prefer curved visual objects. *Psychological Science*, 17(8), 645–648. DOI 10.1111/j.1467-9280.2006.01759.x
- **URL**：https://pubmed.ncbi.nlm.nih.gov/16913943/（作者 PDF：https://moshebar.org/_files/ugd/a77db7_5598c791a95a4436960eb2fe671ca1e5.pdf）
- **讀取管道**：作者實驗室網站 PDF（全 4 頁）
- **研究設計**：受試者內實驗。140 對日常物件圖片（同物件的尖角版／曲線版）、140 對無意義圖形、80 件混合輪廓的對照物件；每張圖呈現 84 毫秒，按「喜歡／不喜歡」。
- **樣本**：14 人（18–40 歲）。
- **實際測量的結果變項**：**短暫一瞥後的喜歡比例**（二選一）。
- **主要發現**：曲線物件喜歡比例 67.2%，尖角 50.6%；曲線 vs 尖角 t(13)=3.53，d=0.94。無意義圖形同向（37.9% vs 24.8%）。「尖角傳遞威脅」是作者提出的解釋假說，本篇未直接測威脅或杏仁核。
- **限制**：樣本 14 人；84 毫秒呈現；物件為去脈絡圖片；後續統合分析顯示效果隨刺激類型差異很大（見 1.4）。
- `full_text_read: true`
- **支撐的卡**：`bed_corner`、`seat_corner`。
- **支撐判定**：**部分**。撐得住「看圖片一瞬間，人平均較喜歡曲線版本的物件」；**撐不住**「尖角讓人有威脅感／不安」（未測），也撐不住任何睡眠或專注說法。

### 1.4 `chuquichambi2022` — 曲線偏好統合分析

- **書目**：Chuquichambi, E. G., Vartanian, O., Skov, M., Corradi, G. B., Nadal, M., Silvia, P. J., & Munar, E. (2022). How universal is preference for visual curvature? A systematic review and meta-analysis. *Annals of the New York Academy of Sciences*, 1518(1), 151–165. DOI 10.1111/nyas.14919
- **URL**：https://pmc.ncbi.nlm.nih.gov/articles/PMC10091794
- **讀取管道**：PMC 全文 HTML
- **研究設計**：系統性回顧＋三層隨機效應統合分析。
- **樣本**：61 篇研究、106 個獨立樣本、309 個效果量、11,023 名參與者。
- **實際測量的結果變項**：**偏好／喜歡評分**（連續量表或喜歡／不喜歡）。不含行為、生理、睡眠。
- **主要發現**：
  - 整體 g=0.39（95% CI 0.25–0.52）；異質性大（I² 第三層 77.67%）。
  - 依刺激類型：無意義圖形 g=0.56；真實物件 g=0.42；符號設計 g=0.30（不顯著）；**空間設計（室內、建築立面）g=−0.04，p=0.80，不顯著**。
  - 呈現時間短者效果較大（g=0.75 vs 0.32）；專家 g=0.13（不顯著，只有 6 篇招募專家）。
  - 未見小樣本偏誤（Egger 型檢定 p=0.32）。
- **限制**：只合併連續量表；仍有大量未解釋異質性；28 篇資料缺失；只收英文、神經典型成人。
- `full_text_read: true`
- **支撐的卡**：`bed_corner`、`seat_corner`。
- **支撐判定**：**撐得住**「曲線偏好主要出現在抽象圖形與物件圖片；換成室內空間的圖片，統合分析裡效果是零」。spec §2.3 只寫「主要用抽象圖形」，應補上「室內空間類效果為零」這個更直接相關的數字。

### 1.5 `chuquichambi2025` — 真實桌椅的曲線偏好

- **書目**：Chuquichambi, E. G., Trawinski, T., Munar, E., & Palumbo, L. (2025). Reassessing the curvature effect in tables and chairs. *i-Perception*, 16(3), 1–20. DOI 10.1177/20416695251341682
- **URL**：https://pmc.ncbi.nlm.nih.gov/articles/PMC12099157/（Liverpool Hope 典藏 PDF：https://hira.hope.ac.uk/id/eprint/4786/）
- **讀取管道**：機構典藏 PDF（全 20 頁）＋ Europe PMC 全文 XML
- **研究設計**：兩個實驗。80 張灰階桌椅照片（40 桌、40 椅，曲線／直線成對、經基本視覺屬性比對）。實驗 1 實驗室眼動＋喜歡評分；實驗 2 線上，84 毫秒 vs 看到作答為止。
- **樣本**：實驗 1：40 人（另 35 人做刺激評定）；實驗 2：50 人（Prolific）。
- **實際測量的結果變項**：**喜歡評分（1–100）**、凝視次數、平均凝視時間。
- **主要發現**：兩個實驗都**沒有**曲線偏好（直線 M=35.9 vs 曲線 M=35，不顯著）；84 毫秒條件也沒有。曲線桌子的平均凝視時間較長（196 vs 188 ms），但不影響喜歡程度。新奇與「輕」的觀感預測較低喜歡度。
- **限制**：只有兩類物件；許多物件同時有曲直部件；照片非實物。
- `full_text_read: true`
- **支撐的卡**：`bed_corner`、`seat_corner`。
- **支撐判定**：**撐得住**「換成真實桌椅的照片，近期研究沒有重現曲線偏好」。

### 1.6 `vartanian2013` — 曲線室內空間與美感判斷（fMRI）

- **書目**：Vartanian, O., Navarrete, G., Chatterjee, A., Fich, L. B., Leder, H., Modroño, C., Nadal, M., Rostrup, N., & Skov, M. (2013). Impact of contour on aesthetic judgments and approach-avoidance decisions in architecture. *PNAS*, 110(Suppl 2), 10446–10453. DOI 10.1073/pnas.1301227110
- **URL**：https://pmc.ncbi.nlm.nih.gov/articles/PMC3690611/
- **讀取管道**：PMC 全文 HTML
- **研究設計**：fMRI 受試者內實驗。200 張室內空間照片（曲線／直線各半，並在各半內平衡天花板高低與開放／封閉）；一輪判斷「美／不美」，一輪判斷「進入／離開」；掃描後另評愉悅度與美感（5 點）。
- **樣本**：18 人（12 女）。
- **實際測量的結果變項**：二元**美感判斷**、二元**進入／離開決定**、愉悅與美感評分、腦區活化。
- **主要發現**：曲線空間較常被判為美（Wilcoxon Z=−2.13，p<0.05）；對**進入／離開決定沒有影響**（Z=−1.27，p=0.21）。愉悅度解釋美感評分 58% 變異。直線空間**沒有**引發預期中的杏仁核活化，作者據此推論在建築中尖銳輪廓未必是威脅訊號。天花板高度、開放度只作控制變項，本篇未報其主效果。
- **限制**：作者自述二元作答、刺激為照片，生態效度有限；樣本 18 人。
- `full_text_read: true`
- **支撐的卡**：`bed_corner`、`seat_corner`（延伸）。
- **支撐判定**：**部分**。撐得住「一項腦造影研究中，曲線室內照片較常被判為美，但不影響要不要進去」；並且**反向削弱**「尖角＝威脅」的說法。不撐任何「尖角朝著人」的說法（照片是整體空間輪廓，不是尖角指向觀看者）。

### 1.7 `fich2014` — 封閉 vs 有開口的虛擬房間與壓力反應

- **書目**：Fich, L. B., Jönsson, P., Kirkegaard, P. H., Wallergård, M., Garde, A. H., & Hansen, Å. (2014). Can architectural design alter the physiological reaction to psychosocial stress? A virtual TSST experiment. *Physiology & Behavior*, 135, 91–97. DOI 10.1016/j.physbeh.2014.05.034
- **URL**：https://pubmed.ncbi.nlm.nih.gov/24907691/
- **讀取管道**：只讀摘要（Europe PMC）。OpenAlex 標示 closed、無典藏全文；Lund／AAU／KU 典藏頁皆無檔案。
- **研究設計**：虛擬版 Trier 社會壓力測驗，房間為電腦生成；組間比較封閉房間 vs 有開口房間。
- **樣本**：**原文摘要未報，本輪無法確認**。
- **實際測量的結果變項**：唾液皮質醇、心率變異。
- **主要發現（摘要）**：封閉房間組在壓力誘發時皮質醇反應較明顯，恢復期仍較高；自律神經指標無差異。
- **限制**：單一 VR 研究；操弄的是牆面開口，不是天花板或頭頂物；樣本數未確認。
- `full_text_read: false`（只讀摘要）
- **支撐的卡**：`bed_overhead`、`seat_overhead`（spec 已列為延伸閱讀）。
- **支撐判定**：**撐不住**上方樑或吊櫃的任何說法；只能作「封閉感」延伸閱讀，且須標明只讀摘要。

### 1.8 `meyerslevy2007` — 天花板高度與思考方式

- **書目**：Meyers-Levy, J., & Zhu, R. (2007). The influence of ceiling height: The effect of priming on the type of processing that people use. *Journal of Consumer Research*, 34(2), 174–186. DOI 10.1086/519146
- **URL**：https://doi.org/10.1086/519146
- **讀取管道**：網路上的全文 PDF 複本（清華大學經管學院網站，版面為期刊原排版，14 頁）；非官方來源，引用數字前建議再對一次出版社版本。
- **研究設計**：三個實驗＋註腳中的補充研究。房間天花板 10 呎 vs 8 呎，用三個吊掛的彩色燈籠提高天花板的顯著性（實驗 2 另操弄燈籠位置）。
- **樣本**：實驗 1：32 人；實驗 2：100 人；實驗 3：34 人；補充情緒研究 64 人；另一補充研究 60 人（皆為 Rice 大學學生）。
- **實際測量的結果變項**：自評「自由／受限」身體狀態、相關字謎反應時間、產品評價與分類（關聯式 vs 單項式處理）、回憶群聚。
- **主要發現**：高天花板（且被注意到時）使人較偏向關聯、抽象的處理；低天花板偏向具體、單項處理。實驗 1：高天花板自評自由感較高（5.11 vs 4.29）、受限感較低（1.89 vs 3.00）。**補充研究中，情緒正負向指標與六項舒適／愉悅評分在各條件間皆無差異**。
- **限制**：學生樣本、小房間短時停留；測的是認知處理方式，不是壓力或睡眠。
- `full_text_read: true`（非官方複本，見讀取管道）
- **支撐的卡**：`bed_overhead`、`seat_overhead`。
- **支撐判定**：**撐不住**「頭上有東西會讓人有壓迫感」。本篇反而顯示低天花板（含頭頂吊燈籠）**沒有**造成情緒或舒適感差異。最多只能作延伸閱讀：「天花板高度研究測的是思考方式，沒有測到舒適感差異」。

### 1.9 `saxbe2010` — 家庭導覽用語、皮質醇與情緒

- **書目**：Saxbe, D. E., & Repetti, R. (2010). No place like home: Home tours correlate with daily patterns of mood and cortisol. *Personality and Social Psychology Bulletin*, 36(1), 71–81. DOI 10.1177/0146167209352864
- **URL**：https://pubmed.ncbi.nlm.nih.gov/19934011/
- **讀取管道**：UCLA CELF 網站的掃描版 PDF（https://www.celf.ucla.edu/2010_conference_articles/Saxbe_Repetti_2010b.pdf，11 頁），本輪 OCR 後逐段閱讀。
- **研究設計**：相關研究。30 個雙薪中產家庭的夫妻各自錄影導覽自家；以 LIWC 計算「雜亂」「房子沒完成（待修繕）」「放鬆」「自然」用語頻率，主成分合併為「壓力型家」與「修復型家」；之後 3 個平日每天 4 次唾液採樣與情緒自評；多層次模型。
- **樣本**：60 人（30 對夫妻）。
- **實際測量的結果變項**：**導覽時的用字**（不是實際雜亂程度）、日間皮質醇斜率、憂鬱情緒日間變化。
- **主要發現**：
  - **只在妻子**：壓力型家分數高者，日間皮質醇斜率較平（Table 3：時間斜率上的效果 0.08，t≈3.35）；修復型家分數高者斜率較陡；控制婚姻滿意度與神經質後仍成立。妻子壓力型家分數高者，一天中憂鬱情緒上升。
  - 丈夫：壓力型家分數只與皮質醇起始值有關，與斜率無關。
  - 用字分數與房屋面積、屋齡、改建次數等**客觀特徵都不相關**。
- **限制（作者自述）**：樣本小、只限雙薪父母；不清楚用字反映的是客觀還是主觀感受；新編字典未驗證；**因果方向不明**（是壓力讓人覺得家亂，還是家亂造成壓力）。
- `full_text_read: true`
- **支撐的卡**：`bed_clutter`、`seat_clutter`。
- **支撐判定**：**撐得住** spec 現有寫法「研究測的是對整個家的描述用語與壓力荷爾蒙型態、情緒；只有相關、不能推論因果；沒有測床底或桌面」，並建議補「只在妻子身上看到」與「『雜亂』與『房子沒完成』合併計分」。

### 1.10 `roster2016` — 雜亂與主觀幸福感

- **書目**：Roster, C. A., Ferrari, J. R., & Jurkat, M. P. (2016). The dark side of home: Assessing possession 'clutter' on subjective well-being. *Journal of Environmental Psychology*, 46, 32–41. DOI 10.1016/j.jenvp.2016.03.003
- **URL**：https://doi.org/10.1016/j.jenvp.2016.03.003
- **讀取管道**：無。全文 closed；ScienceDirect 403；OpenAlex／Europe PMC 無摘要；只有二手報導。
- **研究設計**：問卷調查（二手報導）。
- **樣本**：二手報導稱約 1,489 名美加成人，**未經原文核對**。
- **實際測量的結果變項**：二手報導稱為雜亂、「心理上的家」、主觀幸福感；量表名稱未核實。
- **主要發現**：二手報導稱雜亂與「心理上的家」及幸福感負相關。**本輪未能核實任何數字**。
- **限制**：未讀；推測為橫斷面自陳，未核實。
- `full_text_read: false`（連摘要都未從原始來源讀到）
- **支撐的卡**：`bed_clutter`、`seat_clutter`。
- **支撐判定**：**撐不住**（未讀）。在讀到原文前，卡片研究欄不應引用本篇。

### 1.11 `stevenson2018` — 注意力恢復理論系統性回顧

- **書目**：Stevenson, M. P., Schilhab, T., & Bentsen, P. (2018). Attention Restoration Theory II: A systematic review to clarify attention processes affected by exposure to natural environments. *Journal of Toxicology and Environmental Health, Part B*, 21(4), 227–268. DOI 10.1080/10937404.2018.1505571
- **URL**：https://pubmed.ncbi.nlm.nih.gov/30130463/
- **讀取管道**：只讀摘要（Europe PMC）。OpenAlex 標示 closed；T&F 頁面被擋。
- **研究設計**：系統性回顧＋隨機效應統合分析（2013/7–2017/11 文獻）。
- **樣本**：42 篇文章；8 個認知領域、49 個結果指標。總人數摘要未報。
- **實際測量的結果變項**：接觸自然環境（實地或虛擬／圖片）後的**客觀認知測驗**表現。
- **主要發現（摘要）**：工作記憶、認知彈性改善，注意力控制改善較不穩定；效果量「low to moderate」。實地接觸可能比虛擬接觸效果大，但也可能是接觸時間較長所致。
- **限制**：摘要未提供異質性數字；spec 寫的「異質大」本輪**無法從摘要核實**。
- `full_text_read: false`（只讀摘要）
- **支撐的卡**：`seat_front`。
- **支撐判定**：**部分**。撐得住「接觸自然環境後，部分認知測驗小到中度改善」；**撐不住**「異質大」（未核）與任何「面牆」說法（未測）。

### 1.12 `ulrich1984` — 病房窗景與術後恢復

- **書目**：Ulrich, R. S. (1984). View through a window may influence recovery from surgery. *Science*, 224(4647), 420–421. DOI 10.1126/science.6143402
- **URL**：https://pubmed.ncbi.nlm.nih.gov/6143402/（全文複本：https://www.wrrc.arizona.edu/sites/default/files/ulrich-view%20through%20a%20window.pdf）
- **讀取管道**：Gale 重排的全文複本（文字完整，圖未附）
- **研究設計**：回溯病歷配對研究。賓州一家醫院 1972–1981 年 5–10 月膽囊切除病人；同一側病房窗外為小片落葉樹或褐色磚牆；依性別、年齡、樓層等配對；不知窗景的護理師抽取病歷。
- **樣本**：46 人（23 對；15 對女性、8 對男性）。
- **實際測量的結果變項**：術後住院天數、護理紀錄中負面／正面評語數、止痛藥與鎮靜藥劑量、輕微併發症。
- **主要發現**：樹景組住院 7.96 天 vs 磚牆組 8.70 天（Wilcoxon，p=0.025）；負面護理評語較少（p<0.001）；術後第 2–5 天強效止痛藥較少。
- **限制**：回溯、非隨機；護理站離磚牆病房較近（作者自述）；單一醫院、單一手術；磚牆是窗外景物，不是貼近眼前的牆。
- `full_text_read: true`
- **支撐的卡**：`seat_front`。
- **支撐判定**：**部分**。撐得住「一項病歷研究中，窗外看到樹的術後病人比看到磚牆的住院略短」；**撐不住**「書桌面牆影響專注」——測的是術後恢復，不是工作或注意力。

### 1.13 `boubekri2014` — 辦公室有無窗戶與睡眠

- **書目**：Boubekri, M., Cheung, I. N., Reid, K. J., Wang, C.-H., & Zee, P. C. (2014). Impact of windows and daylight exposure on overall health and sleep quality of office workers: A case-control pilot study. *Journal of Clinical Sleep Medicine*, 10(6), 603–611. DOI 10.5664/jcsm.3780
- **URL**：https://pmc.ncbi.nlm.nih.gov/articles/PMC4031400/
- **讀取管道**：PMC 全文 HTML
- **研究設計**：病例對照前導研究（橫斷面比較）。伊利諾大學校園日班行政人員，無窗工作環境 vs 有窗（工作位日光係數 >2%）。
- **樣本**：49 人（無窗 27、有窗 22）；其中 21 人戴腕動計 2 週（無窗 10、有窗 11）。
- **實際測量的結果變項**：SF-36 生活品質、PSQI 睡眠品質（自陳）、腕動計測光照量、活動量、睡眠時間。
- **主要發現**：有窗組工作時段光照較多（3.00 vs 2.58 log lux，p=0.02）；腕動計睡眠時間平均每晚多 46 分鐘；PSQI 總分與「睡眠干擾」分項較好；SF-36 活力與「身體問題導致角色受限」兩項較好。
- **限制（作者自述）**：樣本小、便利抽樣；未測咖啡因、壓力、時型；非隨機。
- `full_text_read: true`
- **支撐的卡**：`seat_light`。
- **支撐判定**：**撐得住**「一項小樣本研究：在有窗辦公室工作的人，光照較多、睡得較長；測的是睡眠與自評生活品質，不是專心」。

### 1.14 `zijlstra2024` — 黃金比例、風水、實證設計病房 RCT

- **書目**：Zijlstra, E., van der Zwaag, B., Kullak, S., Rogers, A., Walker, D., van Dellen, S., & Mobach, M. (2024). A randomized controlled trial of Golden Ratio, Feng Shui, and evidence based design in healthcare. *PLOS ONE*, 19(6), e0303032. DOI 10.1371/journal.pone.0303032
- **URL**：https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0303032
- **讀取管道**：PLOS 官方全文 XML（表格內容以文字段落為準）
- **研究設計**：線上四臂平行隨機對照試驗（1:1:1:1）。受試者看荷蘭某醫院單人病房的數位分身：2 分鐘影片、圖片、平面圖與文字說明（**不是 VR**，排除聲音），想像自己術後住在這間房。回溯註冊 ISRCTN10480033。
- **樣本**：558 人完成（對照 137、黃金比例 138、風水 140、實證設計 143）；平均約 66 歲；經荷蘭病人聯盟招募。
- **風水組操弄**：只用形勢派與五行，共改 15 處，同時改動——包括床後的「靠」色塊、圓角家具、移開反光的電視位置、移除時鐘、**沒有頭頂結構物**、綠植遮擋動線、米黃配色等。
- **實際測量的結果變項**：STAI-6 焦慮（主要結果）；控制感、社會支持、正向分心（醫院環境量表）；房間愉悅度（單題 10 點）。
- **主要發現**：
  - 風水組**對焦慮沒有整體效果**（原文：「lacked an overall effect of FS on anxiety」）。
  - 風水組房間愉悅度較高（B=1.09，p<0.001）、預期社會支持較高（B=1.32，p=0.001）；經由這兩者對焦慮有間接效果（B=−0.85；B=−0.33）。
  - 實證設計組焦慮下降（平均差 −1.35，95% CI −2.15 至 −0.55，Cohen's d=0.40）。黃金比例組無效果。
- **限制（作者自述）**：多項設計同時改動、無法分離單一特徵；只有單人房；模擬而非真實住院；排除聲音；高齡樣本。**利益揭露**：共同作者 Kullak 隸屬國際風水協會。
- `full_text_read: true`
- **支撐的卡**：說明卡（spec §2.5 第 1 點）；不支撐任何單卡。
- **支撐判定**：**撐得住** spec §2.5 的寫法。建議兩處微調：「線上虛擬病房」宜寫成「線上看病房的影片與圖片」，避免讀者以為是 VR；可補一句「15 處同時改動，無法看出是哪一項起作用」。

### 1.15 `tamaki2016` — 陌生環境首夜效應

- **書目**：Tamaki, M., Bang, J. W., Watanabe, T., & Sasaki, Y. (2016). Night watch in one brain hemisphere during sleep associated with the first-night effect in humans. *Current Biology*, 26(9), 1190–1194. DOI 10.1016/j.cub.2016.02.063
- **URL**：https://pmc.ncbi.nlm.nih.gov/articles/PMC4864126/（作者稿：https://par.nsf.gov/servlets/purl/10067764）
- **讀取管道**：NSF PAR 作者稿 PDF＋PMC HTML
- **研究設計**：實驗室睡眠研究，三個實驗，比較第 1 晚與第 2 晚。MEG＋多頻道睡眠紀錄；實驗 2、3 在睡眠中播放單耳罕見聲音。
- **樣本**：主文未直接列人數；由自由度推得實驗 1 為 11 人、實驗 2 為 13 人、實驗 3 為 11 人，合計 35 人（與校方新聞稿一致）。
- **實際測量的結果變項**：慢波睡眠中左右腦慢波活動不對稱（預設模式網路）、對罕見聲音的誘發反應、被聲音喚醒的比例與反應時間。
- **主要發現**：第 1 晚左側預設模式網路慢波活動較弱（睡得較淺，t10=2.59，d=0.8），不對稱程度與入睡潛伏期相關（r=0.68）；左腦對罕見聲音反應較大、較易被喚醒、反應較快；第 2 晚這些不對稱都消失。
- **限制**：小樣本、年輕健康者；機制推論（「守夜」）是解釋；與房間佈置無關，全文**沒有提到鏡子或房間格局**。
- `full_text_read: true`
- **支撐的卡**：研究報告原把它列為 `bed_mirror` 的間接證據。
- **支撐判定**：**撐不住** `bed_mirror`。它測的是「換到陌生地方的第一晚」，與鏡子無關；放在鏡子卡上會變成用不相干研究替風水補故事，違反 spec §0 與 §2.1 第 2 點。建議不引用。

---

## 2. 補找（一）：夜間光照與睡眠（`bed_light`）

spec §2.3 註記 `bed_light`「夜間光照與睡眠的文獻待 ledger 補具名」。本輪找到下列具名研究：

### 2.1 `mason2022` — 睡眠時開燈的實驗

- **書目**：Mason, I. C., Grimaldi, D., Reid, K. J., Warlick, C. D., Malkani, R. G., Abbott, S. M., & Zee, P. C. (2022). Light exposure during sleep impairs cardiometabolic function. *PNAS*, 119(12), e2113290119. DOI 10.1073/pnas.2113290119
- **URL**：https://pmc.ncbi.nlm.nih.gov/articles/PMC8944904/
- **讀取管道**：Europe PMC 全文 XML
- **研究設計**：實驗室隨機分組平行設計。開燈組：第 1 晚昏暗（<3 lx）、第 2 晚室內燈 100 lx；昏暗組：兩晚都 <3 lx。
- **樣本**：20 名健康年輕成人（每組 10 人）。
- **實際測量的結果變項**：多頻道睡眠紀錄（睡眠階段）、夜間心率與心率變異、隔天早上胰島素阻抗、褪黑激素、主觀嗜睡。
- **主要發現**：開燈那晚 N2 比例較高、慢波睡眠（p=0.017）與快速動眼期（p=0.033）比例較低；整夜心率較高（p<0.0001）；隔天胰島素阻抗指標上升（HOMA-IR 約 +15% vs −4%，p=0.018）。**褪黑激素無差異**；總睡眠時間、入睡潛伏期、睡眠效率等未見差異；主觀嗜睡無差異。
- **限制**：20 人、單晚、年輕健康者；100 lx 是頭頂室內燈的亮度，遠亮於一般「窗外透進來的光」。
- `full_text_read: true`
- **支撐判定**：**部分**。撐得住「一項 20 人的實驗：開著室內燈睡一晚，深睡與快速動眼期比例下降、夜間心率上升」。**文案紅線**：胰島素阻抗屬代謝健康結果，按 spec §2.6 禁「有損健康」語意，卡片只寫睡眠階段與心率即可。

### 2.2 `sweeney2023` — Sister Study 臥室光源與睡眠（橫斷面）

- **書目**：Sweeney, M. R., Nichols, H. B., Jones, R. R., Olshan, A. F., Keil, A. P., Engel, L. S., James, P., Sandler, D. P., White, A. J., & Jackson, C. L. (2024). Exposure to indoor light at night in relation to multiple dimensions of sleep health: findings from the Sister Study. *Sleep*, 47(2), zsad100. DOI 10.1093/sleep/zsad100（2023 線上刊出）
- **URL**：https://pmc.ncbi.nlm.nih.gov/articles/PMC10851850/
- **讀取管道**：PMC 全文經 WebFetch 代讀（含 Table 2 數字）；PMC 對 curl 擋驗證頁，本輪**未能自行逐字閱讀**。
- **研究設計**：美國 Sister Study 基線資料橫斷面分析。
- **樣本**：47,765 名女性。
- **實際測量的結果變項**：**自陳**睡覺時臥室光源（電視開著／房內開燈／**房外的光，含其他房間或窗外**／小夜燈／無），與自陳睡眠指標（短睡、失眠症狀、午睡、作息不規律、睡眠債、安眠藥、綜合差睡分數）。
- **主要發現（工具回報，待自核）**：開電視睡覺者差睡分數 ≥3 的盛行率比 1.58（1.48–1.68）；房內開燈 1.13（1.01–1.27）。**「房外的光」對綜合差睡分數無關（0.99，0.93–1.04）**，但半夜醒來後難再睡 1.21（1.13–1.30）等少數單項有小幅關聯；討論段原文稱房外光與小夜燈「not associated with most sleep health dimensions」。
- **限制**：橫斷面、無法判定時序；光源與睡眠皆自陳；無亮度、遮光簾資料；樣本以白人、高教育女性為主。
- `full_text_read: false`（工具代讀，未親自逐字核對）
- **支撐判定**：**部分**。這是目前找到唯一把「外面照進來的光」單獨分類的研究，但結果偏弱且混合。升級 `direct` 前必須自行讀全文核對 Table 2。

### 2.3 `xu2023` — 夜間光照與睡眠問題統合分析

- **書目**：Xu, Y.-X., Zhang, J.-H., Tao, F.-B., & Sun, Y. (2023). Association between exposure to light at night (LAN) and sleep problems: A systematic review and meta-analysis of observational studies. *Science of the Total Environment*, 857(Pt 1), 159303. DOI 10.1016/j.scitotenv.2022.159303
- **URL**：https://pubmed.ncbi.nlm.nih.gov/36228789/
- **讀取管道**：只讀摘要（Europe PMC）
- **研究設計／樣本**：7 篇橫斷面研究、577,932 人；隨機效應合併。
- **實際測量的結果變項**：各研究定義的睡眠問題（多為自陳）；光照含室內與室外（衛星夜光）。
- **主要發現（摘要）**：高夜間光照者睡眠問題盛行率較高（SOR 1.22，1.13–1.33）；室內光照 SOR 1.74（1.27–2.37）高於室外 1.19（1.11–1.29）。作者自評證據「scarce and inconsistent」。
- `full_text_read: false`（只讀摘要）
- **支撐判定**：**部分**（僅可作延伸閱讀）：全為橫斷面相關。

### 2.4 其他找到但只讀摘要者

- `kretzschmar2026`：Kretzschmar, J., Rath, N., Starke, K. R., Seidler, A., & Freiberg, A. (2026). The effect of exposure to artificial light at night (ALAN) on sleep disturbance: a systematic review and meta-analysis. *Environmental Research*, 123689. DOI 10.1016/j.envres.2026.123689。15 篇研究（765,838 人），10 篇合併：最高暴露組睡眠干擾 PR 1.27（1.12–1.45）；多數研究偏誤風險高（暴露評估）。`full_text_read: false`。
- `obayashi2022`：Obayashi, K., Tai, Y., Yamagami, Y., & Saeki, K. (2022). Associations between indoor light pollution and unhealthy outcomes in 2,947 adults: Cross-sectional analysis in the HEIJO-KYO cohort. *Environmental Research*, 215, 114350. DOI 10.1016/j.envres.2022.114350。以**照度計實測**臥室夜間光（中位數 1.0 lux）；≥3 lux 與 ≥10 lux 組睡眠干擾勝算較高；平均 69.3 歲。`full_text_read: false`。

### 2.5 `bed_light` 小結

具名文獻已補齊：有一個小型實驗（Mason 2022，開室內燈睡）與數個大型橫斷面研究。但 spec 題目問的是「外面的光照進來」，最貼近的分類（Sister Study「房外的光」）結果偏弱且混合；強的結果都來自「房內開燈／開電視」。

---

## 3. 補找（二）：直接測過床位、靠牆、門、鏡子、橫樑、桌面雜物的研究

### 3.1 找到的

#### `sporrle2010` — 平面圖擺床實驗（床位、看得到門、靠牆、窗）

- **書目**：Spörrle, M., & Stich, J. (2010). Sleeping in safe places: An experimental investigation of human sleeping place preferences from an evolutionary perspective. *Evolutionary Psychology*, 8(3), 405–419. DOI 10.1177/147470491000800308
- **URL**：https://pmc.ncbi.nlm.nih.gov/articles/PMC10426856/
- **讀取管道**：PMC 全文 HTML
- **研究設計**：2（對門牆上有窗／無窗）×2（門往左／右開）受試者間實驗；受試者在**紙本平面圖**上擺放比例家具（床、衣櫃、桌、椅）。床刻意做得比房間寬度長，只能沿長軸擺——床頭朝門或朝對面牆二選一，**無法斜擺或擺在門的同側牆**。
- **樣本**：138 人（14–73 歲，50% 女性；61% 學生）。
- **實際測量的結果變項**：床頭方向、床與對門牆的距離、床在房間左右哪一側——即**想像中的擺床偏好**，沒有測睡眠或舒適感（作者自述「These feelings were not measured」）。
- **主要發現**：
  - 83% 把床擺成躺著能看到門（各條件 76–94%；二項檢定 p<.001）。在此平面圖限制下，這等於**床頭靠對面牆、床尾朝向門所在的那面牆**。
  - 70% 把床頭貼緊對門的牆（離門最遠）；但對門牆上有窗時降為 59%（無窗 81%；χ²=8.4，w=0.25）。
  - 門往哪邊開，多數人就把床擺在那一側（74%／64%；χ²=20.3，w=0.39）。
- **限制（作者自述）**：只在想像中佈置、沒有真的睡；平面圖限制擺法；2D；可能受「避開門口的風」或床頭板習慣影響。
- `full_text_read: true`
- **支撐的卡與判定**：
  - `bed_door_view` hidden：**撐得住**「一項 138 人的擺床實驗中，83% 選擇躺著看得到門的位置；測的是在平面圖上擺床的偏好，不是睡眠」。這是第一篇**直接以床位與門的關係為操弄**的研究。
  - `bed_door_view` facing：**部分且方向相反**——多數人選的位置正是床尾朝向門那面牆；但平面圖不允許斜擺，無法區分「正對床尾」與「斜前方」。
  - `bed_back` window：**部分**——對門牆上有窗時，較多人把床拉離那面牆（81%→59%）。測的仍是擺放偏好。

#### `bonin2023` — 上篇的重現與延伸

- **書目**：Bonin, P., Thiebaut, G., Didierjean, A., Fančovičová, J., Kubjatková, N., Prokop, P., & Méot, A. (2023). "Good night, sleep tight": Do we have an "evolutionary preference" for placing beds in sleeping rooms? A replication and extension of Spörrle and Stich (2010). *Evolutionary Psychological Science*, 9(4), 463–476. DOI 10.1007/s40806-023-00377-w
- **URL**：https://link.springer.com/article/10.1007/s40806-023-00377-w（HAL 綠色開放版：hal-04321992，本輪被反機器人頁擋下）
- **讀取管道**：無全文；只見搜尋摘要片段（法國 2D 平面圖、斯洛伐克 3D 房間圖兩研究；結論稱支持原研究）。
- `full_text_read: false`
- **支撐判定**：**待讀**。若全文確認重現成功，`bed_door_view` hidden 的證據會更穩；在此之前不引用數字。

#### `sailer2021` — 辦公桌位置與自評專注

- **書目**：Sailer, K., Koutsolampros, P., & Pachilova, R. (2021).（作者依 Europe PMC 書目紀錄） Differential perceptions of teamwork, focused work and perceived productivity as an effect of desk characteristics within a workplace layout. *PLOS ONE*, 16(4), e0250058. DOI 10.1371/journal.pone.0250058
- **URL**：https://pmc.ncbi.nlm.nih.gov/articles/PMC8081188/
- **讀取管道**：Europe PMC 全文 XML
- **研究設計**：單一倫敦科技公司開放式辦公室的橫斷面研究；員工問卷＋座位 isovist 分析；多元序位迴歸。
- **樣本**：167 個座位／受訪者（5 份缺座位資料排除）。
- **實際測量的結果變項**：**自評**工作空間對「專注」「有效率工作」與團隊合作各項的支持程度。
- **主要發現**：「control」（面前 170° 可見面積／四周 360° 可見面積，越接近 1 越像「面向整個房間」）越高，自評專注與有效率越好（專注 OR=31.9，以 0–1 全距計）；面前看得到的桌數越多，自評專注越差（OR=0.982／桌）。座位在「牆邊」者（n=11）自評有效率工作比「窗邊」者低（OR=0.230）。
- **限制（作者自述）**：單一案例；平面同質性高；橫斷面、自評。
- `full_text_read: true`
- **支撐的卡與判定**：`seat_door_view`（hidden／facing）、`seat_back`：**部分**——測的是「面對整個房間」的可見範圍比例與自評專注，**不是門的可見性、也不是背後靠牆**；`seat_front`：**部分**——「牆邊座位」不等於「面對很近的牆」。

### 3.2 只找到線索、未讀全文

- `vohs2013`：Vohs, K. D., Redden, J. P., & Rahinel, R. (2013). Physical order produces healthy choices, generosity, and conventionality, whereas disorder produces creativity. *Psychological Science*, 24(9), 1860–1867. DOI 10.1177/0956797613480186。實驗操弄**整齊 vs 凌亂的辦公室（含桌面）**；結果變項為點心選擇、捐款、點子創意（凌亂組點子被評為較有創意）。這是最接近 `seat_clutter`「桌面雜物」的直接操弄研究，但方向不是單純「雜亂＝不好」。作者 PDF 下載被 403 擋；`full_text_read: false`。
- `thacher2015`：St. Lawrence University 的 SLEEP 2015 年會摘要（Thacher 等），Mechanical Turk 樣本，囤積風險組（83 人）vs 非風險組（198 人），PSQI 等自陳睡眠較差。只有學會新聞稿與摘要，未見同儕審查全文；`full_text_read: false`。不建議作 `bed_clutter` 引用。

### 3.3 搜尋後未找到

| 特徵 | 結論 | 用過的查詢詞（WebSearch／Europe PMC） |
|---|---|---|
| 鏡子照到床 | **搜尋後未找到**任何同儕審查研究；結果全為居家風水文章與零售商說法 | `mirror in bedroom sleep study empirical OR "mirror" "bedroom" sleep quality research peer-reviewed` |
| 外露橫樑／頭頂吊櫃 | **搜尋後未找到**；最接近者為天花板高度（Meyers-Levy & Zhu 2007，未測舒適差異）與封閉感（Fich 2014，只讀摘要） | `exposed ceiling beam OR "overhead beam" perception oppressive study participants ceiling element psychological effect experiment` |
| 門對門 | **搜尋後未找到** | `"facing doors" OR "doors facing each other" interior layout psychological study privacy perception experiment` |
| 床位對睡眠（實際睡眠結果） | **搜尋後未找到**以睡眠為結果的床位研究；只有擺床偏好實驗（Spörrle 2010、Bonin 2023）與無法追溯的業者調查 | `study bed position against wall OR mirror in bedroom OR exposed ceiling beam OR "bed placement" sleep quality experiment environmental psychology`；`Spörrle Stich 2010 "Sleeping in safe places" ...`；`Bonin ... "Good Night, Sleep Tight" ...` |
| 書桌面牆 vs 面窗與專注 | **只找到**會議摘要等級線索（ANFA 2016 摘要，40 人，檔案無法下載）與 Sailer 2021（非面牆變項） | `desk facing wall versus window view office seat orientation experiment attention performance` |
| 桌面／床底雜物 | 桌面：Vohs 2013（未讀）；床底：**搜尋後未找到** | `Vohs Redden Rahinel 2013 "Physical order produces healthy choices" messy room replication`；`clutter sleep quality hoarding risk Clutter Image Rating Thacher sleep study` |
| 夜間臥室光 | 已找到（§2） | `bedroom light at night sleep meta-analysis OR cohort "light at night" bedroom objective measured sleep quality`；`Mason Zee 2022 PNAS "Light exposure during sleep impairs cardiometabolic function"`；Europe PMC：`TITLE:"artificial light at night" AND TITLE:"sleep" AND (meta-analysis)`、`AUTH:"Obayashi K" AND (bedroom light) AND (sleep)` |

---

## 4. 逐卡建議（對照 spec §2.3 初值表）

標 ★ 者與初值不同。limitation 句一律寫出「測的是什麼結果」。

| 卡 | 初值 | 建議 reach | 建議 limitation（一句） | 依據文獻 |
|---|---|---|---|---|
| `bed_back` window | related | related | 研究測的是空間偏好評分與在平面圖上擺床的偏好，不是睡眠；擺床實驗中，對面牆有窗時較多人把床拉離那面牆 | dosen2016、sporrle2010 |
| `bed_back` passage | related | related | 瞭望－庇護研究測的是空間偏好評分，「背後有遮蔽」在室內研究多為中性；沒有研究測過床頭靠門或走道 | dosen2016 |
| `bed_back` floating | related | related | 同上；擺床實驗中多數人把床頭貼牆，但測的是想像中的擺法，不是睡眠 | dosen2016、sporrle2010 |
| `bed_door_view` hidden | related | ★ **direct**（限偏好） | 有一項 138 人的實驗直接測過：83% 選擇躺著看得到門的位置；測的是在平面圖上擺床的偏好，不是睡眠或壓力 | sporrle2010（true）、dosen2016 |
| `bed_door_view` facing | none | ★ **related** | 唯一的擺床實驗中，多數人選的位置正是床尾朝向門那面牆；但平面圖不能斜擺，測的是偏好不是睡眠 | sporrle2010 |
| `bed_overhead` beam／heavy | none | none | 沒有研究測過橫樑或床上方重物；天花板高度研究測的是思考方式，也沒有測到舒適感差異 | meyerslevy2007（延伸）、fich2014（延伸，只讀摘要） |
| `bed_mirror` | none | none | 目前沒有研究測過鏡子與睡眠（不引用 Tamaki 2016，它測的是換地方睡的第一晚） | — |
| `bed_corner`／`seat_corner` | related | related | 曲線偏好來自看圖片的喜歡評分；統合分析裡室內空間類圖片效果為零，真實桌椅研究（2025）也沒有偏好；沒有研究測過尖角朝著人 | chuquichambi2022、chuquichambi2025、barneta2006、vartanian2013 |
| `bed_door_door` | none | none | 目前沒有研究直接測過（已搜尋） | — |
| `bed_light` | related | related | 研究測的是開燈或開電視睡覺與睡眠：20 人實驗中深睡比例下降；但「外面照進來的光」在最大型問卷研究中與多數睡眠指標無關 | mason2022（true）、sweeney2023（false）、xu2023（false） |
| `bed_clutter`／`seat_clutter` | related | related | 研究測的是導覽自家時的用字與妻子的皮質醇型態、情緒；只有相關、不能推論因果；沒有測床底或桌面 | saxbe2010（true）；roster2016 未讀不引用 |
| `seat_back` window／passage | related | related | 研究測的是休息室座位使用率與辦公室自評專注，不是壓力；觀察研究中背向走道的座位反而最常被坐 | psathiti2017、sailer2021、dosen2016 |
| `seat_door_view` hidden | related | related | 研究測的是座位使用率與「面向整個房間」的自評專注，不是門本身；結果方向不一 | psathiti2017、sailer2021 |
| `seat_door_view` facing | none | ★ **related** | 一項觀察研究中，正面直視主要走道的座位比背向走道的少人坐；測的是使用率，對象是走道不是門 | psathiti2017 |
| `seat_overhead` | none | none | 同 `bed_overhead` | — |
| `seat_front` | related | related | 研究測的是接觸自然景觀後的認知測驗與術後住院天數，效果小到中等；沒有測「書桌直接面牆」 | stevenson2018（false）、ulrich1984（true）、sailer2021 |
| `seat_light` | related | related | 一項 49 人的研究：在有窗辦公室的人光照較多、睡得較長；測的是睡眠與自評生活品質，不是專心 | boubekri2014（true） |

### 4.1 與初值不同之處與理由

1. **`bed_door_view` hidden：related → direct（限偏好）**。Spörrle & Stich 2010 直接以「床與門的相對位置」為操弄、以擺床選擇為結果，符合 §2.2「有研究直接測過這個特徵」的字面定義，且全文已讀（滿足 T6）。但這會是全站第一張 `direct` 卡，與 spec §2.3 末句「`direct` 目前沒有任何一張卡達到」衝突，且使用者可能把徽章讀成「研究支持風水」。**建議交站長裁示**：若採用，limitation 必須寫明「測的是擺床偏好，不是睡眠」；若不採用，維持 related 並在「看研究」展開區列出本篇。Bonin 2023 全文讀完後再決定較穩。
2. **`bed_door_view` facing：none → related**。Spörrle 2010 的平面圖下，多數人的選擇等於床尾朝向門那面牆——與風水「床尾不對門」方向相反。這不是「沒有研究」，而是「唯一相近的研究指向另一邊，且無法區分正對與斜對」。照實寫比寫「沒有研究」更準確。
3. **`seat_door_view` facing：none → related**。Psathiti & Sailer 2017 的座向變項中，正面直視主要走道的座位使用率低於背向走道者。對象是走道而非門，因此只到 related。
4. **`bed_corner`／`seat_corner` limitation 補強**（reach 不變）：統合分析中「空間設計」類效果 g=−0.04（不顯著），比 spec 現寫的「主要用抽象圖形」更直接相關，應寫進 limitation。
5. **`bed_light` limitation 改寫**（reach 不變）：具名文獻已補（Mason 2022 為主）；同時要寫出 Sister Study 中「外面照進來的光」與多數睡眠指標無關，避免高估。升 `direct` 前須親讀 Sister Study 全文。
6. **`seat_back`／`seat_door_view` hidden limitation 改寫**（reach 不變）：Psathiti 2017 沒有「背靠牆」變項，且背向走道的座位最受歡迎——與風水 `passage` 變體方向相反，limitation 應照實寫，不應寫成「研究支持背後要有靠」。
7. **`bed_mirror` 移除 Tamaki 2016**：首夜效應與鏡子無關，引用它等於替風水補機制故事。
8. **`seat_front` limitation 刪「異質大」**：Stevenson 2018 只讀到摘要，摘要沒有異質性數字；在讀到全文前只寫「效果小到中等」。
9. **`bed_clutter`／`seat_clutter` 暫不引用 Roster 2016**：本輪連原始摘要都讀不到。Vohs 2013（凌亂桌面→較有創意）若日後讀到全文，`seat_clutter` 應補上「雜亂不一定只有壞處」。

### 4.2 說明卡（spec §2.5）

Zijlstra 2024 的現有寫法經核對成立。建議：「線上虛擬病房」改為「線上看病房的影片與圖片」（原文明說不是 VR）；可補「15 處同時改動、無法看出是哪一項起作用」與「共同作者之一隸屬國際風水協會」。

---

## 5. 下一輪待辦

- 親讀 Sweeney 2023（Sister Study）全文 Table 2；讀 Bonin 2023（HAL 開放版）全文。兩者決定 `bed_light` 與 `bed_door_view` 能否升級。
- 取得 Fich 2014、Stevenson 2018、Roster 2016、Vohs 2013 全文（需圖書館管道）。
- 研究報告 §2 提到但本輪未核的：Frontiers 2025 圖書館座位研究、KAIST 閱覽室研究、Kryžanowski 2019 的 118 條明細。
