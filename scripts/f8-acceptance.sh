#!/bin/bash
# f8-acceptance.sh — F8 對照組驗收自動化（production E2E，全自動登入）
#
# 全自動模式（預設）：本機 turso write 憑證 = DB 擁有者，直接 INSERT 一列
# login_tokens（magic-link 只存 SHA-256，明碼由本腳本自造）→ POST /api/auth/verify
# 換 sessionId — 不寄信、不用信箱、不改 worker 程式。測試帳號是 ops 縫隙，
# 不是產品功能：凡是拿得到 turso write 的人本來就能寫任何表。
#     自舉鏈：verify 建帳 →（無 complete 側寫則自動作答 15 題，避開 careless
#     三訊號）→ generate → 事前盲檢 → checks → feedback → 事後解盲交叉比對。
#
# 交叉比對的承重假設：事後解盲的 API isControl 必須逐列等於
# f8_assignments.drawn_arm 對應的 predictions.is_control — API 與 DB 兩條路徑
# 同一事實才算通過。
#
# ⚠️ checks/feedback 是不可逆 F6 測量寫入 — 用測試帳號；--cleanup 驗收後
#    F7 刪除 + SQL 清users/login_tokens/f8_assignments（測試列硬清，零污染）。
#
# 用法:
#   ./scripts/f8-acceptance.sh [選項]
# 選項:
#       --email <addr>      測試帳號 email（預設 f8-acceptance@ahexagram.com；
#                           帳號可重複自舉，cleanup 會刪）
#   -t, --token <t>         改用既有 session token（跳過自動登入）
#       --base-url <url>    API 基底（預設 https://fortunet-api.yanggf.workers.dev）
#       --db <name>         turso DB 名稱（預設 fortunet）
#       --situation <s>     Stage 1 回答：absent|occurred（預設 occurred）
#       --response <r>      Stage 2 回答：hit|miss|other（預設 hit）
#       --dry-run           只登入+自舉+generate+事前盲檢，不寫 checks/feedback
#       --cleanup           驗收後 F7 刪除 + SQL 硬清測試列（不可逆）
#   -h, --help              顯示說明

set -euo pipefail

API="${FORTUNET_API:-https://fortunet-api.yanggf.workers.dev}"
TOKEN="${FORTUNET_SESSION:-}"
EMAIL="f8-acceptance@ahexagram.com"
DB_NAME="fortunet"
SITUATION="occurred"
RESPONSE="hit"
DRY_RUN=0
CLEANUP=0

usage() {
  sed -n '2,29p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    --email) EMAIL="$2"; shift 2 ;;
    -t|--token) TOKEN="$2"; shift 2 ;;
    --base-url) API="$2"; shift 2 ;;
    --db) DB_NAME="$2"; shift 2 ;;
    --situation) SITUATION="$2"; shift 2 ;;
    --response) RESPONSE="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --cleanup) CLEANUP=1; shift ;;
    -h|--help) usage ;;
    *) echo "未知參數: $1"; usage ;;
  esac
done

case "$SITUATION" in absent|occurred) ;; *) echo "❌ --situation 必須是 absent|occurred"; exit 2 ;; esac
case "$RESPONSE" in hit|miss|other) ;; *) echo "❌ --response 必須是 hit|miss|other"; exit 2 ;; esac
command -v turso >/dev/null || { echo "❌ 需要 turso CLI（自動登入 + 帳本比對）"; exit 2; }
if [ -z "$TOKEN" ] && [ -z "$EMAIL" ]; then
  echo "❌ 需要 --email（自動登入）或 -t <token>"; exit 2
fi

export FORTUNET_API="$API" FORTUNET_TOKEN="$TOKEN" FORTUNET_EMAIL="$EMAIL" \
       FORTUNET_DB="$DB_NAME" FORTUNET_SITUATION="$SITUATION" \
       FORTUNET_RESPONSE="$RESPONSE" FORTUNET_DRY_RUN="$DRY_RUN" \
       FORTUNET_CLEANUP="$CLEANUP"

exec python3 - <<'PY_EOF'
import hashlib, json, os, secrets, subprocess, sys, urllib.error, urllib.request
from datetime import datetime, timedelta, timezone

API = os.environ["FORTUNET_API"].rstrip("/")
TOKEN = os.environ["FORTUNET_TOKEN"]
EMAIL = os.environ["FORTUNET_EMAIL"]
DB = os.environ["FORTUNET_DB"]
SITUATION = os.environ["FORTUNET_SITUATION"]
RESPONSE = os.environ["FORTUNET_RESPONSE"]
DRY_RUN = os.environ["FORTUNET_DRY_RUN"] == "1"
CLEANUP = os.environ["FORTUNET_CLEANUP"] == "1"

def call(method, path, body=None, token=None):
    req = urllib.request.Request(API + path, method=method)
    req.add_header("Authorization", f"Bearer {token or TOKEN}")
    # Cloudflare 擋非瀏覽器 UA（1010）— 須帶瀏覽器指紋（同 predictions-e2e.sh）
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

def turso_exec(sql):
    p = subprocess.run(["turso", "db", "shell", DB, sql],
                       capture_output=True, text=True, timeout=30)
    if p.returncode != 0:
        sys.exit(f"❌ turso 寫入失敗: {p.stderr.strip()[:200]}")

def turso_rows(sql):
    """turso 表格輸出 → list[rows]。表頭標籤可含空白（如 DRAWN ARM），欄位數
    以第一個「資料列」為準；0 列回 []（查詢成功但無資料）。可 NULL 欄用
    COALESCE(x,'-') 哨兵，在此還原 None。解析不對回 None（呼叫端記失敗）。"""
    p = subprocess.run(["turso", "db", "shell", DB, sql],
                       capture_output=True, text=True, timeout=30)
    if p.returncode != 0:
        return None
    lines = [l for l in p.stdout.splitlines() if l.strip()]
    if not lines:
        return []
    if len(lines) == 1:  # 只有表頭 = 0 列
        return []
    ncols = len(lines[1].split())
    rows = []
    for l in lines[1:]:
        parts = l.split()
        if len(parts) != ncols:
            return None
        rows.append([None if v == "-" else v for v in parts])
    return rows

def q(s):
    return s.replace("'", "''")

steps = []
def step(name, ok, detail=""):
    print(f"{'✅' if ok else '❌'} {name}" + (f" — {detail}" if detail else ""))
    steps.append(ok)

# ── 0. 登入 ──
if TOKEN:
    print("ℹ️  使用既有 session token（-t）。\n")
else:
    plain = secrets.token_hex(32)  # 256-bit，同 login_token::new_token 形狀
    token_hash = hashlib.sha256(plain.encode()).hexdigest()
    exp = datetime.now(timezone.utc) + timedelta(minutes=10)
    iso = exp.strftime("%Y-%m-%dT%H:%M:%S.") + f"{exp.microsecond // 1000:03d}Z"
    turso_exec(
        "INSERT INTO login_tokens (token_hash, email, expires_at, used_at, "
        "pending_full_name, pending_invite_code) VALUES "
        f"('{token_hash}', '{q(EMAIL)}', '{iso}', NULL, 'F8 acceptance test', NULL)")
    status, body = call("POST", "/api/auth/verify", {"token": plain})
    if status != 200 or "sessionId" not in (body or {}):
        sys.exit(f"❌ verify → {status} {body}（邀請閘被重開？INVITE_REQUIRED=true？）")
    TOKEN = body["sessionId"]
    print(f"ℹ️  自動登入成功 — 帳號 {EMAIL}（session {TOKEN[:8]}…）\n")

# ── 1. 身分 ──
status, me = call("GET", "/api/users/me")
if status == 401:
    sys.exit("❌ session 無效或過期")
if status != 200:
    sys.exit(f"❌ GET /api/users/me → {status} {me}")
user_id = me.get("id") or me.get("userId") or me.get("user_id")
step("GET /api/users/me", bool(user_id), f"user={user_id}")
uid = q(user_id)

# ── 2. 人格側寫自舉（generate 的前置）──
status, prof = call("GET", "/api/personality/me")
if status != 200 or prof.get("status") != "complete":
    # 答 15 題：E/O 中庸、A/C/ES 全 Low（反向題 10–12 答 5 → 翻轉後 1）—
    # 極化 profile 保證錨點命中（work-t1-agr-lo / work-t2-con-lo / emo-lo 等）；
    # 非 straight-lining（3↔1↔5 有變化）、各維 range=0 無端點衝突、45s > 20s —
    # careless 三訊號全避開
    answers = [3, 3, 3, 1, 1, 1, 1, 1, 1, 5, 5, 5, 3, 3, 3]
    status, body = call("POST", "/api/personality/quiz",
                        {"skip": False, "answers": answers, "durationMs": 45000})
    if status != 200:
        sys.exit(f"❌ POST quiz → {status} {body}")
    status, prof = call("GET", "/api/personality/me")
    step("人格測驗自舉", status == 200 and prof.get("status") == "complete",
         f"status={prof.get('status')}")
else:
    print("ℹ️  側寫已 complete，跳過測驗自舉。")

# ── 3. 當週狀態 + generate ──
status, body = call("GET", "/api/predictions")
if status != 200:
    sys.exit(f"❌ GET /api/predictions → {status} {body}")
cycle = body.get("cycleId", "?")
preds = body.get("predictions", [])
checks = body.get("checks", [])
fbs = body.get("feedback", [])
step("GET 當週列表", True, f"cycle={cycle} preds={len(preds)} checks={len(checks)} fb={len(fbs)}")

if not preds:
    # F4 起必帶 strengths（work/money/love 閘門 ≥1；family/health 目錄恆 0）。
    # pre-F4 API（2026-09-13 的 production）不讀 body，無害。
    strengths = {"work": 2, "money": 2, "love": 2, "family": 0, "health": 0}
    status, body = call("POST", "/api/predictions/generate", {"strengths": strengths})
    if status == 409 and (body or {}).get("code") == "PROFILE_INCOMPLETE":
        sys.exit("❌ 服務端仍視為無 complete 側寫 — 檢查 personality/me")
    if status not in (200, 409):
        sys.exit(f"❌ POST generate → {status} {body}")
    step("POST generate", status == 200, f"code={status}")
    status, body = call("GET", "/api/predictions")
    preds = body.get("predictions", [])
    checks = body.get("checks", [])
    fbs = body.get("feedback", [])

if not preds:
    rows = turso_rows(f"SELECT COUNT(*) FROM f8_assignments WHERE cycle_id = '{q(cycle)}'")
    n = rows[0][0] if rows else None
    step("誠實空週 → 帳本本週無列", n == "0", f"rows={n}")
    print("\nℹ️  空週是有效結果（凍結冪等），但本輪沒有對照樣本。")
    sys.exit(0 if steps and all(steps) else 1)

# ── 4. 事前盲：回饋未收齊前 isControl 恆 false（spec §0/Codex #2）──
accounted = all(
    any(c["trigger"] == p["trigger"] for c in checks
        if c["situation"] == "absent") or
    any(f["predictionId"] == p["id"] for f in fbs)
    for p in preds)
if accounted:
    print("ℹ️  本週已達解盲條件 — 事前盲不可重演，略過（重驗請 --cleanup 後重跑）。\n")
else:
    blind_ok = all(p.get("isControl") is False for p in preds)
    step("事前盲：收齊前 isControl 全 false", blind_ok,
         f"isControl={[p.get('isControl') for p in preds]}")

# ── 5. F6 鏈 ──
distinct = {p["trigger"] for p in preds}
answered = {c["trigger"] for c in checks}
unanswered = sorted(distinct - answered, key=lambda t: int(t[1:]))

if unanswered:
    redacted_ok = all(p.get("forecast") is None for p in preds)
    step("遮罩閘門：未收齊時 forecast 為 null", redacted_ok)

if DRY_RUN:
    print(f"ℹ️  dry-run：跳過 {len(unanswered)} 個 checks 與 Stage 2。")
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
    if {c["trigger"] for c in checks} >= distinct:
        step("遮罩閘門：收齊後 forecast 已揭露",
             all(p.get("forecast") is not None for p in preds))

    occurred = {c["trigger"] for c in checks if c["situation"] == "occurred"}
    fb_ids = {f["predictionId"] for f in body.get("feedback", [])}
    for p in [p for p in preds if p["trigger"] in occurred and p["id"] not in fb_ids]:
        status, body = call("POST", f"/api/predictions/{p['id']}/feedback",
                            {"response": RESPONSE})
        if status == 200:
            step(f"feedback {p['domain']}/{p['trigger']} → {RESPONSE}", True)
        elif status == 409 and (body or {}).get("code") == "FEEDBACK_EXISTS":
            step(f"feedback {p['domain']}/{p['trigger']}", True, "已存在（略）")
        else:
            step(f"feedback {p['domain']}/{p['trigger']}", False, f"{status} {body}")

# ── 6. 最終 GET + 帳本交叉比對（承重檢查）──
status, body = call("GET", "/api/predictions")
preds = body.get("predictions", [])

ledger = turso_rows(
    f"SELECT id, domain, drawn_arm, suppressed, COALESCE(prediction_id, '-') "
    f"FROM f8_assignments WHERE user_id = '{uid}' AND cycle_id = '{q(cycle)}'")
dbpreds = turso_rows(
    f"SELECT id, domain, is_control FROM predictions "
    f"WHERE user_id = '{uid}' AND cycle_id = '{q(cycle)}'")

if ledger is None or dbpreds is None:
    step("turso 帳本查詢", False, "表格輸出解析失敗")
else:
    assigned = [r for r in ledger if r[3] == "0"]
    suppressed = [r for r in ledger if r[3] == "1"]
    dbmap = {r[0]: r for r in dbpreds}
    step("帳本查詢", True,
         f"ledger={len(ledger)}（assigned={len(assigned)} suppressed={len(suppressed)}）"
         f" predictions={len(dbpreds)}")
    for r in assigned:
        dp = dbmap.get(r[4])
        ok = dp is not None and dp[2] == ("1" if r[2] == "control" else "0")
        step(f"帳本 assigned {r[1]}/{r[2]}", ok,
             "" if ok else f"prediction_id={r[4]} 不存在或 is_control 不符")
    for r in suppressed:
        step(f"帳本 suppressed {r[1]}", r[4] is None, "prediction_id 應為 NULL")

    mism = [(p["domain"], p["trigger"]) for p in preds
            if p["id"] in dbmap
            and ("1" if p.get("isControl") else "0") != dbmap[p["id"]][2]]
    n_ctrl = sum(1 for p in preds if p.get("isControl"))
    step("事後解盲：API isControl ≡ DB is_control", not mism,
         (f"不符={mism}" if mism else f"controls={n_ctrl}/{len(preds)}"))

print("\n—— 總結 ——")
print(f"cycleId: {body.get('cycleId', '?')}  account: {EMAIL}")
for p in preds:
    fcast = (p.get("forecast") or "（遮罩）").replace("\n", " ")[:56]
    print(f"  - {p.get('domain')}/{p.get('trigger')} "
          f"isControl={p.get('isControl')} | {fcast}")

# ── 7. cleanup：F7 API 刪除 → SQL 硬清測試列 ──
if CLEANUP:
    status, body = call("DELETE", "/api/personality/me")
    step("F7 刪除（--cleanup）", status == 200, f"code={status}" if status == 200 else str(body))
    if status == 200:
        turso_exec(f"DELETE FROM f8_assignments WHERE user_id = '{uid}'")
        turso_exec(f"DELETE FROM users WHERE id = '{uid}'")
        turso_exec(f"DELETE FROM login_tokens WHERE email = '{q(EMAIL)}'")
        print("ℹ️  SQL 硬清完成：users / login_tokens / f8_assignments 測試列已除名。")

ok = bool(steps) and all(steps)
print("\n" + ("✅ F8 驗收全數通過" if ok else "❌ F8 驗收有失敗項目（見上方 ❌）"))
sys.exit(0 if ok else 1)
PY_EOF
