#!/bin/bash
# predictions-e2e.sh — F5 本週預測半自動 E2E
#
# 貼一次 session token（localStorage 的 `sessionId`，magic-link 登入後取得），
# 自動跑完整條鏈：
#     GET /api/predictions →（空則 generate）→ 逐 trigger PUT checks →
#     GET（驗證遮罩解除）→ 逐條 POST feedback → 最終 GET 總結
# 並在過程中驗證兩道測量閘門：①未收齊時 forecast 為 null；②收齊後已揭露。
#
# ⚠️ 注意：checks / feedback 是 **不可逆的 F8 測量寫入**（一次性、鎖定、無撤銷 API）。
#    建議丟在測試帳號、或確實想填的週次執行；不想寫入請改用 `--situation absent`
#    （不進 Stage 2）或 `--dry-run`（完全不寫 checks/feedback）。
#
# 用法:
#   ./scripts/predictions-e2e.sh -t <SESSION_TOKEN> [選項]
# 選項:
#   -t, --token <t>         session token（或環境變數 FORTUNET_SESSION）
#       --base-url <url>    API 基底（預設 https://fortunet-api.yanggf.workers.dev）
#       --situation <s>     Stage 1 回答：absent|occurred（預設 occurred）
#       --response <r>      Stage 2 回答：hit|miss|other（預設 hit；僅 occurred 生效）
#       --dry-run           只 GET + generate，不寫 checks/feedback
#   -h, --help              顯示說明

set -euo pipefail

API="${FORTUNET_API:-https://fortunet-api.yanggf.workers.dev}"
TOKEN="${FORTUNET_SESSION:-}"
SITUATION="occurred"
RESPONSE="hit"
DRY_RUN=0

usage() {
  sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    -t|--token) TOKEN="$2"; shift 2 ;;
    --base-url) API="$2"; shift 2 ;;
    --situation) SITUATION="$2"; shift 2 ;;
    --response) RESPONSE="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage ;;
    *) echo "未知參數: $1"; usage ;;
  esac
done

[ -n "$TOKEN" ] || { echo "❌ 需要 session token（-t <token> 或設定 FORTUNET_SESSION）"; exit 2; }
case "$SITUATION" in absent|occurred) ;; *) echo "❌ --situation 必須是 absent|occurred"; exit 2 ;; esac
case "$RESPONSE" in hit|miss|other) ;; *) echo "❌ --response 必須是 hit|miss|other"; exit 2 ;; esac

export FORTUNET_API="$API" FORTUNET_TOKEN="$TOKEN" FORTUNET_SITUATION="$SITUATION" \
       FORTUNET_RESPONSE="$RESPONSE" FORTUNET_DRY_RUN="$DRY_RUN"

exec python3 - <<'PY_EOF'
import json, os, sys, urllib.error, urllib.request

API = os.environ["FORTUNET_API"].rstrip("/")
TOKEN = os.environ["FORTUNET_TOKEN"]
SITUATION = os.environ["FORTUNET_SITUATION"]
RESPONSE = os.environ["FORTUNET_RESPONSE"]
DRY_RUN = os.environ["FORTUNET_DRY_RUN"] == "1"

def call(method, path, body=None):
    req = urllib.request.Request(API + path, method=method)
    req.add_header("Authorization", f"Bearer {TOKEN}")
    # Cloudflare 會擋非瀏覽器 UA（1010）；curl 可過、urllib 預設不行，須帶瀏覽器指紋
    req.add_header("User-Agent",
                   "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                   "(KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
    req.add_header("Content-Type", "application/json")
    data = json.dumps(body).encode() if body is not None else None
    try:
        with urllib.request.urlopen(req, data=data) as r:
            text = r.read().decode()
            return r.status, (json.loads(text) if text else {})
    except urllib.error.HTTPError as e:
        text = e.read().decode()
        try:
            return e.code, json.loads(text)
        except Exception:
            return e.code, {"error": text}
    except Exception as e:
        sys.exit(f"❌ 網路錯誤: {e}")

def check_auth(status, body):
    if status == 401:
        sys.exit("❌ session 無效或過期 — 重新取得 magic-link 並貼上新 token（-t）")

steps = []
def step(name, ok, detail=""):
    print(f"{'✅' if ok else '❌'} {name}" + (f" — {detail}" if detail else ""))
    steps.append(ok)

if not DRY_RUN:
    print("⚠️  即將對你的帳號寫入不可逆的 F8 測量資料（situation_checks / prediction_feedback）。")
    print("    不想污染資料：用 --situation absent（不進 Stage 2）或 --dry-run（完全不寫）。\n")
else:
    print("ℹ️  --dry-run：只讀 + generate，不寫 checks/feedback。\n")

# ── 1. GET（當週）──
status, body = call("GET", "/api/predictions")
check_auth(status, body)
if status != 200:
    sys.exit(f"❌ GET /api/predictions → {status} {body}")
cycle = body.get("cycleId", "?")
preds = body.get("predictions", [])
checks = body.get("checks", [])
step("GET 當週列表", True, f"cycleId={cycle} predictions={len(preds)} checks={len(checks)}")

# ── 2. 空則 generate（冪等）──
if not preds:
    status, body = call("POST", "/api/predictions/generate")
    check_auth(status, body)
    if status == 409 and (body or {}).get("code") == "PROFILE_INCOMPLETE":
        sys.exit("❌ 尚無 complete 人格測驗 — 請先到 /personality 完成測驗再跑")
    if status not in (200, 409):
        sys.exit(f"❌ POST generate → {status} {body}")
    step("POST generate", status == 200, f"code={status}")
    status, body = call("GET", "/api/predictions")
    preds = body.get("predictions", [])
    checks = body.get("checks", [])

if not preds:
    print("\nℹ️  本週沒有預測（誠實空週）— 鏈結到此結束，此為正常空狀態而非錯誤。")
    sys.exit(0)

distinct = {p["trigger"] for p in preds}
answered = {c["trigger"] for c in checks}
unanswered = sorted(distinct - answered, key=lambda t: int(t[1:]))

# ── 3. 閘門①：未收齊時 forecast 必須為 null ──
if unanswered:
    redacted_ok = all(p.get("forecast") is None for p in preds)
    step("遮罩閘門：未收齊時 forecast 為 null", redacted_ok,
         f"未答 trigger={','.join(unanswered)}")

# ── 4. Stage 1：逐 trigger PUT checks ──
if unanswered:
    if DRY_RUN:
        print(f"ℹ️  dry-run：跳過 {len(unanswered)} 個 checks（{','.join(unanswered)}）")
    else:
        for t in unanswered:
            status, body = call("PUT", "/api/predictions/checks",
                                {"trigger": t, "situation": SITUATION})
            if status == 200:
                step(f"check {t} → {SITUATION}", True)
            elif status == 409 and (body or {}).get("code") == "SITUATION_LOCKED":
                step(f"check {t}", True, "已鎖定（略）")
            else:
                step(f"check {t}", False, f"{status} {body}")
        status, body = call("GET", "/api/predictions")
        checks = body.get("checks", [])
        preds = body.get("predictions", [])

answered = {c["trigger"] for c in checks}
unanswered = sorted(distinct - answered, key=lambda t: int(t[1:]))

# ── 5. 閘門②：收齊後 forecast 必須已揭露 ──
if not unanswered:
    lifted = bool(preds) and all(p.get("forecast") is not None for p in preds)
    step("遮罩閘門：收齊後 forecast 已揭露", lifted)
else:
    step("遮罩閘門：收齊後 forecast 已揭露", False,
         f"仍有未答 trigger={','.join(unanswered)}")

# ── 6. Stage 2：occurred 且無 feedback 的預測 → POST feedback ──
if not DRY_RUN and SITUATION == "occurred":
    occurred_triggers = {c["trigger"] for c in checks if c["situation"] == "occurred"}
    fb_ids = {f["predictionId"] for f in body.get("feedback", [])}
    pending = [p for p in preds
               if p["trigger"] in occurred_triggers and p["id"] not in fb_ids]
    if pending:
        for p in pending:
            status, body = call("POST", f"/api/predictions/{p['id']}/feedback",
                                {"response": RESPONSE})
            if status == 200:
                step(f"feedback {p['domain']}/{p['trigger']} → {RESPONSE}", True)
            elif status == 409 and (body or {}).get("code") == "FEEDBACK_EXISTS":
                step(f"feedback {p['domain']}/{p['trigger']}", True, "已存在（略）")
            else:
                step(f"feedback {p['domain']}/{p['trigger']}", False, f"{status} {body}")
    else:
        print("ℹ️  Stage 2：沒有待回饋的 occurred 預測（全部已回饋或無 occurred）。")
elif SITUATION != "occurred":
    print(f"ℹ️  situation={SITUATION} → 不進 Stage 2（無 feedback 寫入）。")

# ── 7. 最終总结 ──
status, body = call("GET", "/api/predictions")
print("\n—— 總結 ——")
print(f"cycleId: {body.get('cycleId', '?')}")
print(f"predictions: {len(body.get('predictions', []))}")
print(f"checks: {len(body.get('checks', []))}")
print(f"feedback: {len(body.get('feedback', []))}")
for p in body.get("predictions", []):
    fcast = (p.get("forecast") or "（遮罩）").replace("\n", " ")[:64]
    print(f"  - {p.get('domain')}/{p.get('trigger')} "
          f"coverage={p.get('anchorCoverage')} | {fcast}")

ok = all(steps)
print("\n" + ("✅ E2E 全數通過" if ok else "❌ E2E 有失敗項目（見上方 ❌）"))
sys.exit(0 if ok else 1)
PY_EOF
