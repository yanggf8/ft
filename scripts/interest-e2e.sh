#!/bin/bash
# interest-e2e.sh — 興趣探索 v1 正式站端到端（真登入、真寫入，結束必硬清）
#
# 登入同 f8-acceptance.sh：本機 turso write 憑證直接 INSERT 一列 login_tokens
# （只存 SHA-256，明碼由本腳本自造）→ POST /api/auth/verify 換 sessionId。
# 測試帳號是 ops 縫隙，不是產品功能。
#
# 兩段，各自登入、各自在 finally 硬清（interest_results / users / login_tokens）：
#   1. API：建立、伺服器重算分數、冪等重送、409 IDEMPOTENCY_CONFLICT /
#      STALE_INTEREST_STATE / STALE_INSTRUMENT、400 INVALID_INTEREST、overview、
#      history、DB 交叉比對、predictions/feedback/f8 隔離、刪除與 epoch、未登入 401。
#   2. 瀏覽器（scripts/interest-e2e-browser.cjs，不攔截 API）：我的命格作答 48 題 →
#      結果 → 重新整理讀回 → 歷史 → 刪除 → 再作答；CSP 違規檢查。
# 興趣資料可完整刪除，故 cleanup 無條件執行（不同於 f8 的 F6 不可逆寫入）。
#
# 用法:
#   ./scripts/interest-e2e.sh [選項]
# 選項:
#       --email <addr>      測試帳號 email（預設 interest-acceptance@ahexagram.com）
#       --base-url <url>    API 基底（預設 https://fortunet-api.yanggf.workers.dev）
#       --web-url <url>     web 基底（預設 https://ahexagram.com；其 API 由 web 自身決定）
#       --db <name>         turso DB 名稱（預設 fortunet）
#       --api-only          只跑 API 段
# 環境變數（瀏覽器段）:
#   PLAYWRIGHT_MODULE       playwright 套件路徑（未設則找全域 npm 的 playwright、
#                           @playwright/mcp/node_modules/playwright）
#   CHROME_PATH             chromium 執行檔（未設則用 playwright 預設）
#   -h, --help              顯示說明

set -euo pipefail

API="${FORTUNET_API:-https://fortunet-api.yanggf.workers.dev}"
WEB="https://ahexagram.com"
EMAIL="interest-acceptance@ahexagram.com"
DB_NAME="fortunet"
API_ONLY=0

usage() {
  sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    --email) EMAIL="$2"; shift 2 ;;
    --base-url) API="$2"; shift 2 ;;
    --web-url) WEB="$2"; shift 2 ;;
    --db) DB_NAME="$2"; shift 2 ;;
    --api-only) API_ONLY=1; shift ;;
    -h|--help) usage ;;
    *) echo "未知參數: $1"; usage ;;
  esac
done

command -v turso >/dev/null || { echo "❌ 需要 turso CLI（自動登入 + DB 比對 + 清理）"; exit 2; }
[ "$API_ONLY" = 1 ] || command -v node >/dev/null || { echo "❌ 瀏覽器段需要 node（或加 --api-only）"; exit 2; }

export FORTUNET_API="$API" FORTUNET_WEB="$WEB" FORTUNET_EMAIL="$EMAIL" FORTUNET_DB="$DB_NAME"

echo "── 1. API（$API）──"
python3 - <<'PY_EOF'
import hashlib, json, os, secrets, subprocess, sys, urllib.error, urllib.request, uuid
from datetime import datetime, timedelta, timezone

API = os.environ["FORTUNET_API"].rstrip("/")
DB = os.environ["FORTUNET_DB"]
EMAIL = os.environ["FORTUNET_EMAIL"]
TOKEN = None

def call(method, path, body=None):
    req = urllib.request.Request(API + path, method=method)
    if TOKEN:
        req.add_header("Authorization", f"Bearer {TOKEN}")
    # Cloudflare 擋非瀏覽器 UA（1010）— 須帶瀏覽器指紋（同 predictions-e2e.sh）
    req.add_header("User-Agent",
                   "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                   "(KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
    req.add_header("Content-Type", "application/json")
    data = json.dumps(body).encode() if body is not None else None
    try:
        with urllib.request.urlopen(req, data=data, timeout=30) as r:
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

def turso(sql):
    p = subprocess.run(["turso", "db", "shell", DB, sql], capture_output=True, text=True, timeout=30)
    if p.returncode != 0:
        sys.exit(f"❌ turso 失敗: {p.stderr.strip()[:200]}")
    return p.stdout

def last_cell(out):
    lines = out.strip().splitlines()
    return lines[-1].strip() if lines else ""

def q(s):
    return s.replace("'", "''")

steps = []
def step(name, ok, detail=""):
    print(f"{'✅' if ok else '❌'} {name}" + (f" — {detail}" if detail else ""))
    steps.append(ok)

uid = None
try:
    # ── 0. 登入 ──
    plain = secrets.token_hex(32)  # 256-bit，同 login_token::new_token 形狀
    exp = datetime.now(timezone.utc) + timedelta(minutes=10)
    iso = exp.strftime("%Y-%m-%dT%H:%M:%S.") + f"{exp.microsecond // 1000:03d}Z"
    turso("INSERT INTO login_tokens (token_hash, email, expires_at, used_at, "
          "pending_full_name, pending_invite_code) VALUES "
          f"('{hashlib.sha256(plain.encode()).hexdigest()}', '{q(EMAIL)}', '{iso}', NULL, "
          "'Interest acceptance test', NULL)")
    status, body = call("POST", "/api/auth/verify", {"token": plain})
    if status != 200 or "sessionId" not in (body or {}):
        sys.exit(f"❌ verify → {status} {body}（邀請閘被重開？INVITE_REQUIRED=true？）")
    TOKEN = body["sessionId"]
    status, me = call("GET", "/api/users/me")
    uid = me.get("id")
    step("登入測試帳號", status == 200 and bool(uid), EMAIL)
    if not uid:
        sys.exit(1)

    # ── 1. 初始狀態 ──
    status, ov = call("GET", "/api/interest")
    e0 = ov.get("interestEpoch")
    step("GET /api/interest 初始", status == 200 and isinstance(e0, int)
         and ov.get("totalResults") == 0 and ov.get("latest") is None, json.dumps(ov))
    if not isinstance(e0, int):
        sys.exit(1)

    # ── 2. 送出：R5 I4 A3 S3 E2 C1（題庫索引 t*8..t*8+7 為第 t 型）──
    answers = [v for v in (5, 4, 3, 3, 2, 1) for _ in range(8)]
    rid = str(uuid.uuid4())
    req = {"instrumentVersion": "1", "form": "A", "answers": answers,
           "expectedEpoch": e0, "requestId": rid}
    status, r1 = call("POST", "/api/interest/results", req)
    step("POST 建立 → 201", status == 201 and bool(r1.get("id")), str(status))
    step("伺服器重算分數", r1.get("scores") ==
         {"R": 5.0, "I": 4.0, "A": 3.0, "S": 3.0, "E": 2.0, "C": 1.0}, json.dumps(r1.get("scores")))

    # ── 3. 冪等 ──
    status, r = call("POST", "/api/interest/results", req)
    step("同 requestId 重送 → 200 同一筆", status == 200 and r.get("id") == r1.get("id"), str(status))
    status, r = call("POST", "/api/interest/results", {**req, "answers": [3] * 48})
    step("同 requestId 不同作答 → 409 IDEMPOTENCY_CONFLICT",
         status == 409 and r.get("code") == "IDEMPOTENCY_CONFLICT", f"{status} {r}")

    # ── 4. 過期／驗證 ──
    status, r = call("POST", "/api/interest/results",
                     {**req, "requestId": str(uuid.uuid4()), "expectedEpoch": e0 + 7})
    step("錯 epoch → 409 STALE_INTEREST_STATE",
         status == 409 and r.get("code") == "STALE_INTEREST_STATE", f"{status} {r}")
    status, r = call("POST", "/api/interest/results",
                     {**req, "requestId": str(uuid.uuid4()), "instrumentVersion": "0"})
    step("舊版題庫 → 409 STALE_INSTRUMENT",
         status == 409 and r.get("code") == "STALE_INSTRUMENT", f"{status} {r}")
    status, r = call("POST", "/api/interest/results",
                     {**req, "requestId": str(uuid.uuid4()), "answers": answers[:47]})
    step("47 題 → 400 INVALID_INTEREST",
         status == 400 and r.get("code") == "INVALID_INTEREST", f"{status} {r}")
    status, r = call("POST", "/api/interest/results",
                     {**req, "requestId": str(uuid.uuid4()), "answers": answers[:47] + [6]})
    step("作答值 6 → 400 INVALID_INTEREST",
         status == 400 and r.get("code") == "INVALID_INTEREST", f"{status} {r}")

    # ── 5. 讀取 ──
    status, ov = call("GET", "/api/interest")
    step("GET overview：1 筆、latest 為剛送出那筆", status == 200 and ov.get("totalResults") == 1
         and (ov.get("latest") or {}).get("id") == r1.get("id"), str(status))
    status, h = call("GET", "/api/interest/history")
    step("GET history：1 筆、無下一頁", status == 200 and len(h.get("results", [])) == 1
         and h.get("nextCursor") is None, str(status))

    # ── 6. DB 交叉比對 ──
    out = turso(f"SELECT answers, request_id FROM interest_results WHERE user_id = '{q(uid)}'")
    step("DB：一列、原始作答依題庫索引、requestId 已存",
         out.count(rid) == 1 and "[5,5,5,5,5,5,5,5,4," in out, last_cell(out)[:80])
    u = q(uid)
    out = turso(f"SELECT (SELECT COUNT(*) FROM predictions WHERE user_id='{u}') + "
                "(SELECT COUNT(*) FROM prediction_feedback f JOIN predictions p "
                f"ON p.id = f.prediction_id WHERE p.user_id='{u}') + "
                f"(SELECT COUNT(*) FROM f8_assignments WHERE user_id='{u}') AS n")
    step("隔離：predictions/feedback/f8 無此帳號列", last_cell(out) == "0", last_cell(out))

    # ── 7. 刪除 ──
    status, d = call("DELETE", "/api/interest")
    step("DELETE → deleted + epoch 加一", status == 200 and d.get("deleted") is True
         and d.get("interestEpoch") == e0 + 1, json.dumps(d))
    status, ov = call("GET", "/api/interest")
    step("刪除後 overview 歸零", status == 200 and ov.get("totalResults") == 0
         and ov.get("latest") is None and ov.get("interestEpoch") == e0 + 1, json.dumps(ov))
    out = turso(f"SELECT COUNT(*) FROM interest_results WHERE user_id = '{u}'")
    step("DB：原始作答已刪", last_cell(out) == "0", last_cell(out))
    status, r = call("POST", "/api/interest/results", req)
    step("刪除後重送舊 requestId/epoch → 409 STALE_INTEREST_STATE",
         status == 409 and r.get("code") == "STALE_INTEREST_STATE", f"{status} {r}")

    # ── 8. 未登入 ──
    saved, TOKEN = TOKEN, None
    status, _ = call("GET", "/api/interest")
    TOKEN = saved
    step("未登入 → 401", status == 401, str(status))
finally:
    # ── 9. 硬清測試帳號 ──
    if uid:
        turso(f"DELETE FROM interest_results WHERE user_id = '{q(uid)}'")
        turso(f"DELETE FROM users WHERE id = '{q(uid)}'")
    turso(f"DELETE FROM login_tokens WHERE email = '{q(EMAIL)}'")
    left = last_cell(turso(f"SELECT COUNT(*) FROM users WHERE email = '{q(EMAIL)}'"))
    print(f"ℹ️  cleanup：測試帳號殘留 {left} 列")

ok = bool(steps) and all(steps)
print(f"\n{'✅' if ok else '❌'} API 段 {sum(steps)}/{len(steps)}")
sys.exit(0 if ok else 1)
PY_EOF

[ "$API_ONLY" = 1 ] && exit 0

echo
echo "── 2. 瀏覽器（$WEB）──"
node "$(dirname "$0")/interest-e2e-browser.cjs"
