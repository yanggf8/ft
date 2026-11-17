# 空間風水 v1 spec v2 — Fable 5.2 獨立審查（2026-11-17）

審查對象：spec v2（commit 0818b84）。審查者未被告知 Kimi 結論。verdict：**有條件通過**（Batch 1＋2 修完 F01–F05 可開工；Batch 3 先處理 F06–F08）。
Claude 核實：程式碼引用全數屬實（`lib.rs:71 decorate`、`clock.rs` 全走 `js_sys`、`cycle.rs:101/111` 日期算術非 pub、`tarot.rs:33 USER_LIMIT=20`、`tarot.rs:447` 非 UUID 回 400、`tarot.rs:539 {deleted:true}`、`common.rs:397 valid_uuid_shape` 私有、workspace 無 chrono/time）；小出入：`civil_from_days` 為 `pub(crate)` 而非私有。文獻判讀（Fich 2014 操弄開口非頭頂物件；Saxbe & Repetti 2010 測全家描述用語非床底雜物；研究報告無具名夜間光照文獻）與研究報告一致。

| # | 級別 | 摘要 | 處置 |
|---|---|---|---|
| F01 | Blocking | 「結果大致一致」無比較對象，會讀成「與風水一致」；ok 卡＋徽章＝背書 | 收：標籤自帶結果變項；ok 卡不顯示徽章 |
| F02 | Blocking | 初值過高：clutter 非 direct、樑借 Fich 2014、`bed_light` 無具名文獻卻標 consistent | 收：樑兩變體皆 none；clutter→related；初值取下限，升級需 ledger |
| F03 | Should | 明廳暗房、採光被改寫成睡眠衛生題，最強徽章落在改寫題上 | 收：風水欄照實寫「問的是相近但不同的事」，reach 降 related |
| F04 | Should | 漏了唯一直接檢驗風水佈置的 RCT（Zijlstra 2024） | 收：說明卡加入；固定句改「多數不是為了檢驗風水而做」 |
| F05 | Should | 欄名「你的大腦怎麼看」名實不符、殘留腦科學光環 | **待站長裁示**（動到站長定的並列框架） |
| F06 | Should | 風水欄可寫的後果層級未定義；詞表缺「磁場」「能量」「財運」等；substring 誤殺「不能證明」 | 收：溫和後果引述層級；擴詞表；改 token／白名單片語 |
| F07 | Should（Batch 3 前 Blocking） | 時窗錨在 created_at，不是實際調整日；`adherence=no` 對答案未定義 | 收 |
| F08 | Should | 伺服端嚴謹度無保護對象；先做 localStorage 版驗好玩度 | **待站長裁示**（Batch 3 拆 3a/3b） |
| F09 | Should | 1–5 分預測誤差多為 0/±1，回饋弱 | **待站長裁示**（0–10 或區間預測＋校準紀錄） |
| F10 | Should | `adjustmentId` 與語料（單段文字）對不上；mild/flag 分不出 | 收：`adjustments:[{id,text}]`、body 加 `state` |
| F11 | Should | e2e 會撞 uid 10/24h 限流；等號邊界 e2e 打不到 | 收：限流 20（塔羅同值）；邊界重用同列，等號只留 T9 |
| F12 | Nit | 回報 affected=0 回查少「未回報且在窗內」分支 | 收：回 500，靠重試收斂 |
| F13 | Should | no-store 放 route helper 會漏 401／page_query 400 | 收：在 `decorate` 依 `/api/fengshui/` 前綴補；`REPORT_TOO_EARLY` 自訂 body |
| F14 | Should | 時間算術無現成函式、T9 native 測不到 | 收：`ft_schema` 新增 `iso_shift_days`，重用 cycle.rs |
| F15 | Nit | 與塔羅兩處偏離未註明；`valid_uuid_shape` 私有 | 收：改對齊塔羅（`{deleted:true}`、非 UUID 400）；開放 `valid_uuid_shape` |
| F16 | Should | Batch 2「試試看」按鈕與 Batch 1 wire/T7 超前閘門 | 收 |
| F17 | Should | 有變體題目的 ok 卡徽章未定義 | 收（F01 一併解決） |
| F18 | Nit | ledger 與 evidence.rs 雙來源 | 收：機器可讀欄位放 evidence.rs，`consistent` 需 `full_text_read` |
| F19 | Nit | `state` 混三維度 | 收：徽章只留 reach＋一句限制說明 |
| F20 | Nit | `bed_back` 三種 flag 共用一卡 | 收：拆變體 |
| F21 | Should | 閘門驗的是最像問卷的一半；建議 Batch 2 加「翻卡前先猜」即時預測迴圈 | **待站長裁示**（Claude 強烈建議收） |
| F22 | Nit | `.ics` 在 iOS 不穩 | 收：列入實機驗收，不行就只顯示日期 |

## 總評（摘錄）

工程面紮實：條件式 INSERT／UPDATE 在 libSQL 單寫者下原子性成立。科學誠實差最後一步（F01–F04）。最大盲點在產品：預測處理在吸收 K02 後被清出逐卡內容，差異化只剩一張說明卡和一個要等 7 天、被閘門擋住的記錄型功能；建議把預測體驗前移到 Batch 2 的「翻卡前先猜」，先猜後試先做 localStorage 版。
