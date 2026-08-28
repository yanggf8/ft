#!/bin/bash
# Big5 F1 production integration verification. Spec: .../2026-08-28-big5-f1-design.md §9.3
# 註：每次執行留下一個孤兒測試帳號（@verify.local）；清掉了 personality 資料但
# users row 留存——可接受，帳號本身無功能影響（Grok 審 #22）。
set -e
API_URL="${API_URL:-https://fortunet-api.yanggf.workers.dev}"
GREEN='\033[0;32m'; RED='\033[0;31m'; NC='\033[0m'
fail() { echo -e "${RED}✗ $1${NC}"; echo "Last response: $2"; exit 1; }
pass() { echo -e "${GREEN}✓ $1${NC}"; }

EMAIL="big5-verify-$(date +%s)@verify.local"
R=$(curl -s -X POST "$API_URL/api/auth/register" -H "Content-Type: application/json" -d "{\"email\":\"$EMAIL\"}")
SID=$(echo "$R" | grep -o '"sessionId":"[^"]*"' | cut -d'"' -f4)
[ -n "$SID" ] || fail "register" "$R"
pass "register"
AUTH="Authorization: Bearer $SID"; CT="Content-Type: application/json"

# 2. 正常作答：[5,5,5,4,4,4,3,3,3,2,2,2,1,1,1] → E=100 A=75 C=50 ES=75(反向2→4) I=0
#    regex 精確匹配（Grok 審 #20）：100.0 過、100.5 不過——前綴匹配會誤綠。
Q=$(curl -s -X POST "$API_URL/api/personality/quiz" -H "$AUTH" -H "$CT" \
  -d '{"skip":false,"answers":[5,5,5,4,4,4,3,3,3,2,2,2,1,1,1],"durationMs":42000}')
echo "$Q" | grep -Eq '"status":"complete"([\.,}]|$)'   || fail "quiz complete" "$Q"
echo "$Q" | grep -Eq '"extraversion":100(\.0+)?[,}]'   || fail "quiz E=100" "$Q"
echo "$Q" | grep -Eq '"agreeableness":75(\.0+)?[,}]'   || fail "quiz A=75" "$Q"
echo "$Q" | grep -Eq '"conscientiousness":50(\.0+)?[,}]' || fail "quiz C=50" "$Q"
echo "$Q" | grep -Eq '"emotionalStability":75(\.0+)?[,}]' || fail "quiz ES=75" "$Q"
echo "$Q" | grep -Eq '"intellectImagination":0(\.0+)?[,}]' || fail "quiz I=0" "$Q"
pass "quiz scoring (fixed input, hand-computed, exact)"

M=$(curl -s "$API_URL/api/personality/me" -H "$AUTH")
echo "$M" | grep -q '"status":"complete"' || fail "me after quiz" "$M"
pass "GET me returns latest profile + status"

# 4. 亂答 #1（全同＋超快）→ 422 CARELESS_SUSPECTED；GET 應回 carelessSuspected
#    且 profile 仍為步驟 2 的 complete（讀模型——Grok 二審 R2-2）。
#    斷言用順序無關結構式（serde_json Map=BTreeMap 鍵字母序，status 排最後——baicodex F1）
C1=$(curl -s -X POST "$API_URL/api/personality/quiz" -H "$AUTH" -H "$CT" \
  -w "\n%{http_code}" -d '{"skip":false,"answers":[4,4,4,4,4,4,4,4,4,4,4,4,4,4,4],"durationMs":3000}')
[ "$(echo "$C1" | tail -n1)" = "422" ] || fail "careless #1 expected 422" "$C1"
echo "$C1" | grep -q 'CARELESS_SUSPECTED' || fail "careless #1 code" "$C1"
M1=$(curl -s "$API_URL/api/personality/me" -H "$AUTH")
echo "$M1" | grep -Eq '"status":"carelessSuspected"' || fail "me after careless #1" "$M1"
echo "$M1" | grep -Eq '"profile":\{' || fail "me has profile" "$M1"
echo "$M1" | grep -Eq '"oceanMeasured":\{[^{]*"extraversion"' || fail "last-complete survives careless" "$M1"
pass "careless #1 -> 422 + GET carelessSuspected, last-complete survives"

# 5. 亂答 #2（連續第二次，latest=careless_suspected）-> 升級 skippedPriorOnly（200）
#    採 Grok 對抗審 #2 的「連續次數」閘（丟 5 分鐘窗——窗會罰慢而誠實的重測）。
#    D5 紅線：skipped 記錄不得帶分數（baicodex F10a）
C2=$(curl -s -X POST "$API_URL/api/personality/quiz" -H "$AUTH" -H "$CT" \
  -w "\n%{http_code}" -d '{"skip":false,"answers":[4,4,4,4,4,4,4,4,4,4,4,4,4,4,4],"durationMs":3000}')
[ "$(echo "$C2" | tail -n1)" = "200" ] || fail "careless #2 expected 200" "$C2"
echo "$C2" | grep -q 'skippedPriorOnly' || fail "careless #2 status" "$C2"
case "$C2" in *oceanMeasured*) fail "skipped row must carry no scores" "$C2";; esac
pass "careless #2 -> skippedPriorOnly, no scores on skipped row"

# 5b. 單訊號分例（baicodex F10b）：too_fast 單獨觸發；straight-lining 單獨觸發
#    ⚠️ 連續次數 escalation 是**有狀態**的：S1 把 latest 設成 careless_suspected，
#    對齊後的 S2 會直接被升級為 skipped 而非 422。要在 S1 後插入重測→complete 回位。
S1=$(curl -s -X POST "$API_URL/api/personality/quiz" -H "$AUTH" -H "$CT" \
  -w "\n%{http_code}" -d '{"skip":false,"answers":[1,2,3,4,5,1,2,3,4,5,1,2,3,4,5],"durationMs":3000}')
[ "$(echo "$S1" | tail -n1)" = "422" ] || fail "too_fast-only expected 422" "$S1"
R_B=$(curl -s -X POST "$API_URL/api/personality/quiz" -H "$AUTH" -H "$CT" \
  -d '{"skip":false,"answers":[5,5,5,4,4,4,3,3,3,2,2,2,1,1,1],"durationMs":42000}')
echo "$R_B" | grep -q '"status":"complete"' || fail "retake-reset before straight-lining" "$R_B"
S2=$(curl -s -X POST "$API_URL/api/personality/quiz" -H "$AUTH" -H "$CT" \
  -w "\n%{http_code}" -d '{"skip":false,"answers":[2,2,2,2,2,2,2,2,2,2,2,2,2,2,2],"durationMs":60000}')
[ "$(echo "$S2" | tail -n1)" = "422" ] || fail "straight-lining-only expected 422" "$S2"
pass "too_fast-only and straight-lining-only each trigger 422"

# 6. skipped 後主動重測通過 -> complete（D5：skipped 非永久）
#    createdAt 必須不同於步驟 2 那筆（確認是真的重新落庫計分——baicodex F10c）
Q_T=$(echo "$Q" | grep -o '"createdAt":"[^"]*"')
C3=$(curl -s -X POST "$API_URL/api/personality/quiz" -H "$AUTH" -H "$CT" \
  -d '{"skip":false,"answers":[5,5,5,4,4,4,3,3,3,2,2,2,1,1,1],"durationMs":42000}')
echo "$C3" | grep -q '"status":"complete"' || fail "retake after skipped" "$C3"
C3_T=$(echo "$C3" | grep -o '"createdAt":"[^"]*"')
[ -n "$Q_T" ] && [ -n "$C3_T" ] && [ "$Q_T" != "$C3_T" ] || fail "retake createdAt must differ" "$C3"
pass "retake after skipped -> complete (fresh row)"

# 7. 14 題 -> 400 VALIDATION_FAILED；{} 漏欄 -> 400 SKIP_ANSWERS_CONFLICT（Grok #12）
B=$(curl -s -X POST "$API_URL/api/personality/quiz" -H "$AUTH" -H "$CT" \
  -w "\n%{http_code}" -d '{"skip":false,"answers":[3,3,3,3,3,3,3,3,3,3,3,3,3,3],"durationMs":42000}')
[ "$(echo "$B" | tail -n1)" = "400" ] || fail "bad length expected 400" "$B"
echo "$B" | grep -q 'VALIDATION_FAILED' || fail "bad length code" "$B"
E=$(curl -s -X POST "$API_URL/api/personality/quiz" -H "$AUTH" -H "$CT" \
  -w "\n%{http_code}" -d '{}')
[ "$(echo "$E" | tail -n1)" = "400" ] || fail "empty body expected 400" "$E"
echo "$E" | grep -q 'SKIP_ANSWERS_CONFLICT' || fail "empty body code" "$E"
pass "validation -> 400 (VALIDATION_FAILED / SKIP_ANSWERS_CONFLICT)"

# 8. 主動跳過 -> skippedPriorOnly；讀模型：profile 仍為最新 complete（Grok #3）
#    D5：主動 skip 的本筆也不得帶分數；GET retry（D1 read-after-write 跨 isolate——baicodex F16.5）
S=$(curl -s -X POST "$API_URL/api/personality/quiz" -H "$AUTH" -H "$CT" \
  -d '{"skip":true}')
echo "$S" | grep -q 'skippedPriorOnly' || fail "explicit skip" "$S"
case "$S" in *oceanMeasured*) fail "skipped row must carry no scores" "$S";; esac
M3=""
for i in 1 2 3; do
  M3=$(curl -s "$API_URL/api/personality/me" -H "$AUTH")
  echo "$M3" | grep -Eq '"status":"skippedPriorOnly"' && break
  sleep 1
done
echo "$M3" | grep -Eq '"status":"skippedPriorOnly"' || fail "me status after skip" "$M3"
echo "$M3" | grep -Eq '"oceanMeasured":\{[^{]*"extraversion"' || fail "last-complete profile must survive skip" "$M3"
pass "skip -> not-scored view, last-complete profile survives, no scores on skipped row"

# 9. DELETE（無 body）-> success；GET -> 兩欄皆 null
D=$(curl -s -X DELETE "$API_URL/api/personality/me" -H "$AUTH" -H "$CT")
echo "$D" | grep -q '"success":true' || fail "delete" "$D"
M2=$(curl -s "$API_URL/api/personality/me" -H "$AUTH")
echo "$M2" | grep -Eq '"profile":null' || fail "me profile after delete" "$M2"
echo "$M2" | grep -Eq '"status":null' || fail "me status after delete" "$M2"
pass "delete -> data cleared"

echo ""; echo -e "${GREEN}✅ Big5 F1 integration verification passed${NC}"
