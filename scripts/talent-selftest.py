"""上手小實驗（2026-11-22）自測：建立→冪等→OPEN_TRIAL_EXISTS→REQUEST_ID_CONFLICT→錯誤碼→回報→
重回報→列表差距／傾向門檻→沒做不計入→達門檻看到傾向→過期 410／expired／可再建立→刪單筆→刪全部。
spec docs/superpowers/specs/2026-11-22-talent-tryout-design.md §11、§13。
自造測試帳號（DB 直插 login_tokens，同 shop-selftest.py）；結束時（含中途失敗）SQL 硬清
talent_trials（本帳號）、users、login_tokens。
前提：migration 2026-11-22-talent-trials.sql 已套用到 --db 且 API 已部署；本機 turso 已登入。
寫入約 25 次，計入本帳號 talent:user 限流（60 次／小時；帳號每次新建，不影響真實使用者）。
用法：python3 -I scripts/talent-selftest.py [--api URL] [--db NAME]
  staging：--api https://fortunet-api-staging.yanggf.workers.dev --db fortunet-staging"""
import argparse, hashlib, json, secrets, subprocess, sys, urllib.request, urllib.error, uuid
from datetime import datetime, timedelta, timezone

ap = argparse.ArgumentParser()
ap.add_argument("--api", default="https://api.ahexagram.com")
ap.add_argument("--db", default="fortunet")
args = ap.parse_args()
API = args.api.rstrip("/")
DB = args.db
EMAIL = "ft-selftest-talent@example.invalid"
UA = ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
      "Chrome/130.0 Safari/537.36 FortuneT-selftest")
TOKEN = None
results = []

def call(method, path, body=None, token=None):
  req = urllib.request.Request(API + path, method=method,
    data=json.dumps(body).encode() if body is not None else None)
  req.add_header("Content-Type", "application/json")
  req.add_header("Origin", "https://ahexagram.com")
  req.add_header("User-Agent", UA)
  if token or TOKEN:
    req.add_header("Authorization", f"Bearer {token or TOKEN}")
  try:
    with urllib.request.urlopen(req, timeout=60) as r:
      t = r.read().decode()
      return r.status, (json.loads(t) if t else None)
  except urllib.error.HTTPError as e:
    t = e.read().decode()
    try: return e.code, json.loads(t)
    except Exception: return e.code, t

def sql(q):
  p = subprocess.run(["turso", "db", "shell", DB, q], capture_output=True, text=True, timeout=30)
  if p.returncode != 0: sys.exit("turso fail: " + p.stderr[:200])
  return p.stdout

def iso_ms(dt):
  return dt.strftime("%Y-%m-%dT%H:%M:%S.") + f"{dt.microsecond // 1000:03d}Z"

def step(name, ok, detail=""):
  print(("✅ " if ok else "❌ ") + name + (f" — {detail}" if detail else ""))
  results.append(ok)

def code_of(b):
  return b.get("code") if isinstance(b, dict) else None

def rid():
  return "st-" + uuid.uuid4().hex

def create(item, predicted, request_id):
  return call("POST", "/api/talent/trials", {"itemId": item, "predicted": predicted, "requestId": request_id})

def report(trial_id, body):
  return call("POST", f"/api/talent/trials/{trial_id}/report", body)

def listing():
  s, b = call("GET", "/api/talent/trials")
  if s != 200 or not isinstance(b, dict):
    return s, [], {}
  return s, b.get("trials", []), b.get("tendency", {})

uid = None
try:
  # 0. login（DB 直插 login token）
  plain = secrets.token_hex(32)
  h = hashlib.sha256(plain.encode()).hexdigest()
  exp = datetime.now(timezone.utc) + timedelta(minutes=10)
  sql("INSERT INTO login_tokens (token_hash, email, expires_at, used_at, pending_full_name, pending_invite_code) "
      f"VALUES ('{h}', '{EMAIL}', '{iso_ms(exp)}', NULL, 'Selftest', NULL)")
  s, b = call("POST", "/api/auth/verify", {"token": plain})
  if s != 200: sys.exit(f"verify {s} {b}")
  TOKEN = b["sessionId"]
  s, u = call("GET", "/api/users/me")
  uid = (u or {}).get("id") or (u or {}).get("userId")
  step("login", bool(uid), uid)
  if not uid: sys.exit("no uid")

  s, trials, tend = listing()
  step("初始列表為空、need=3", s == 200 and trials == [] and tend.get("need") == 3, f"{s} {tend}")

  # 1. 建立 → 同 requestId 冪等 → 第二筆 409 → 同 requestId 不同內容 409
  RA = rid()
  s, t1 = create("juggle", 1, RA)
  ok = s == 201 and isinstance(t1, dict) and t1.get("status") == "open" and t1.get("predicted") == 1
  step("建立 T1（juggle，猜 1）→ 201 open", ok, f"{s} {t1}")
  if not ok: sys.exit("cannot continue")
  T1 = t1["id"]
  s, b = create("juggle", 1, RA)
  step("同 requestId 重送 → 200 原筆", s == 200 and (b or {}).get("id") == T1, f"{s} {b}")
  s, b = create("crane", 2, rid())
  step("已有進行中 → 409 OPEN_TRIAL_EXISTS", s == 409 and code_of(b) == "OPEN_TRIAL_EXISTS", f"{s} {b}")
  s, b = create("juggle", 2, RA)
  step("同 requestId 不同內容 → 409 REQUEST_ID_CONFLICT", s == 409 and code_of(b) == "REQUEST_ID_CONFLICT", f"{s} {b}")
  n = sql(f"SELECT COUNT(*) FROM talent_trials WHERE user_id='{uid}'").split()[-1]
  step("DB：只有一列", n == "1", n)

  # 2. 錯誤碼
  for body, code, name in [
    ({"itemId": "nope", "predicted": 1, "requestId": rid()}, "INVALID_ITEM", "未知 itemId"),
    ({"itemId": "crane", "predicted": 1.5, "requestId": rid()}, "INVALID_PREDICTION", "predicted=1.5"),
    ({"itemId": "crane", "predicted": -1, "requestId": rid()}, "INVALID_PREDICTION", "predicted=-1"),
    ({"itemId": "crane", "predicted": 4, "requestId": rid()}, "INVALID_PREDICTION", "predicted=4"),
    ({"itemId": "crane", "predicted": 1, "requestId": "short"}, "INVALID_REQUEST_ID", "requestId 太短"),
  ]:
    s, b = call("POST", "/api/talent/trials", body)
    step(f"POST {name} → 400 {code}", s == 400 and code_of(b) == code, f"{s} {b}")
  s, b = report(T1, {"requestId": rid()})
  step("report 缺 reached → 400 INVALID_REACHED", s == 400 and code_of(b) == "INVALID_REACHED", f"{s} {b}")
  s, b = report(T1, {"reached": None, "stuck": "time", "requestId": rid()})
  step("report reached=null 帶 stuck → 400 INVALID_STUCK", s == 400 and code_of(b) == "INVALID_STUCK", f"{s} {b}")
  s, b = report(T1, {"reached": 1, "stuck": "lazy", "requestId": rid()})
  step("report 未知 stuck → 400 INVALID_STUCK", s == 400 and code_of(b) == "INVALID_STUCK", f"{s} {b}")
  s, b = report(str(uuid.uuid4()), {"reached": 1, "requestId": rid()})
  step("report 不存在 → 404 NOT_FOUND", s == 404 and code_of(b) == "NOT_FOUND", f"{s} {b}")

  # 3. 回報 → 同 requestId 200 → 不同 requestId 409
  R1 = rid()
  s, b = report(T1, {"reached": 3, "stuck": "none", "requestId": R1})
  step("回報 T1 做到 3 → 200 reported gap=2",
       s == 200 and (b or {}).get("status") == "reported" and b.get("gap") == 2 and b.get("stuck") == "none", f"{s} {b}")
  s, b = report(T1, {"reached": 3, "stuck": "none", "requestId": R1})
  step("同 requestId 重回報 → 200 原結果", s == 200 and (b or {}).get("gap") == 2, f"{s} {b}")
  s, b = report(T1, {"reached": 1, "requestId": rid()})
  step("不同 requestId 重回報 → 409 ALREADY_REPORTED", s == 409 and code_of(b) == "ALREADY_REPORTED", f"{s} {b}")

  # 4. 列表：差距與門檻
  s, trials, tend = listing()
  t = next((x for x in trials if x.get("id") == T1), {})
  step("列表 T1 gap=2", s == 200 and t.get("gap") == 2 and t.get("status") == "reported", f"{s} {t}")
  step("tendency need=2、無 kind", tend.get("need") == 2 and tend.get("reportedCount") == 1 and tend.get("kind") is None, f"{tend}")

  # 5. T2 沒做（不計入）
  s, t2 = create("crane", 2, rid())
  step("建立 T2（crane，猜 2）", s == 201, f"{s} {t2}")
  T2 = (t2 or {}).get("id")
  s, b = report(T2, {"reached": None, "requestId": rid()})
  step("T2 這次沒做 → skipped、無 gap", s == 200 and (b or {}).get("status") == "skipped" and b.get("gap") is None, f"{s} {b}")
  s, trials, tend = listing()
  step("沒做不計入傾向（need 仍 2）", tend.get("need") == 2 and tend.get("reportedCount") == 1, f"{tend}")
  sk = sql(f"SELECT skipped FROM talent_trials WHERE id='{T2}'").split()[-1]
  step("DB：T2 skipped=1", sk == "1", sk)

  # 6. 再兩筆達門檻：T3 gap 0、T4 gap 1 → gaps [2,0,1] 平均 1 → conservative
  s, t3 = create("hangul", 2, rid())
  T3 = (t3 or {}).get("id")
  s3, b3 = report(T3, {"reached": 2, "requestId": rid()})
  step("T3 猜 2 做到 2 → gap 0", s == 201 and s3 == 200 and (b3 or {}).get("gap") == 0, f"{s} {s3} {b3}")
  s, t4 = create("digits", 1, rid())
  T4 = (t4 or {}).get("id")
  s4, b4 = report(T4, {"reached": 2, "stuck": "memory", "requestId": rid()})
  step("T4 猜 1 做到 2 → gap 1", s == 201 and s4 == 200 and (b4 or {}).get("gap") == 1, f"{s} {s4} {b4}")
  s, trials, tend = listing()
  step("達門檻：need=0、kind=conservative、under=2 exact=1 over=0",
       tend.get("need") == 0 and tend.get("kind") == "conservative" and tend.get("reportedCount") == 3
       and (tend.get("under"), tend.get("exact"), tend.get("over")) == (2, 1, 0), f"{tend}")
  ids = [x.get("id") for x in trials]
  step("列表新到舊", ids[:4] == [T4, T3, T2, T1], f"{ids}")

  # 7. 過期：DB 直插 8 天前未回報的一列
  TX = str(uuid.uuid4())
  old = iso_ms(datetime.now(timezone.utc) - timedelta(days=8))
  sql("INSERT INTO talent_trials (id, user_id, item_id, catalog_version, predicted, created_at, create_request_id) "
      f"VALUES ('{TX}', '{uid}', 'song', '1', 1, '{old}', '{rid()}')")
  s, b = report(TX, {"reached": 1, "requestId": rid()})
  step("回報 8 天前的實驗 → 410 EXPIRED", s == 410 and code_of(b) == "EXPIRED", f"{s} {b}")
  s, trials, tend = listing()
  tx = next((x for x in trials if x.get("id") == TX), {})
  step("列表顯示 expired", tx.get("status") == "expired" and tx.get("gap") is None, f"{tx}")
  step("過期不計入傾向", tend.get("reportedCount") == 3, f"{tend}")
  s, t5 = create("knots", 0, rid())
  step("過期不擋新實驗 → 201", s == 201 and (t5 or {}).get("status") == "open", f"{s} {t5}")

  # 8. 刪單筆 → 重刪 404 → 刪全部 → 列表空
  s, b = call("DELETE", f"/api/talent/trials/{T1}")
  step("刪 T1 → 200 deleted=1", s == 200 and (b or {}).get("deleted") == 1, f"{s} {b}")
  s, b = call("DELETE", f"/api/talent/trials/{T1}")
  step("重刪 T1 → 404 NOT_FOUND", s == 404 and code_of(b) == "NOT_FOUND", f"{s} {b}")
  s, b = call("DELETE", "/api/talent/trials")
  step("刪全部 → 200 deleted=5", s == 200 and (b or {}).get("deleted") == 5, f"{s} {b}")
  s, trials, tend = listing()
  step("列表空、need=3", s == 200 and trials == [] and tend.get("need") == 3, f"{s} {trials} {tend}")
finally:
  if uid:
    sql(f"DELETE FROM talent_trials WHERE user_id='{uid}'")
    sql(f"DELETE FROM users WHERE id='{uid}'")
  sql(f"DELETE FROM login_tokens WHERE email='{EMAIL}'")
  left = sql(f"SELECT COUNT(*) FROM talent_trials WHERE user_id='{uid}'").split()[-1] if uid else "0"
  print(f"\nuid={uid}\ncleanup done（talent_trials 殘留 {left} 列）")

print("\n" + ("全部通過" if results and all(results) else f"有失敗：{results.count(False)} 項"))
sys.exit(0 if results and all(results) else 1)
