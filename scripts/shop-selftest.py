"""開運小物 v1（2026-11-22）正式站自測：匿名登記／重複／匿名轉登入不重算／me 聯集／取消／錯誤碼。
自造測試帳號（DB 直插 login_tokens，同 subunits-selftest.py）；結束時（含中途失敗）SQL 硬清
shop_interest（本次 anon_id／user_id）、users、login_tokens。
前提：migration 2026-11-22-shop-interest.sql 已套用且 API 已部署；本機 turso 已登入。
會對呼叫端 IP 計約 12 次 shop 限流（30 次／小時）。
用法：python3 -I scripts/shop-selftest.py"""
import hashlib, json, secrets, subprocess, sys, urllib.request, urllib.error, uuid
from datetime import datetime, timedelta, timezone

API = "https://fortunet-api.yanggf.workers.dev"
DB = "fortunet"
EMAIL = "ft-selftest-shop@example.invalid"
ITEM_A, ITEM_B, ITEM_D = "tarot-postcards", "zodiac-bookmark", "desk-plant"
ANON1, ANON2, ANON3 = (str(uuid.uuid4()) for _ in range(3))
results = []

def call(method, path, body=None, token=None):
  req = urllib.request.Request(API + path, method=method,
    data=json.dumps(body).encode() if body is not None else None)
  req.add_header("Content-Type", "application/json")
  req.add_header("Origin", "https://ahexagram.com")
  req.add_header("User-Agent", "Mozilla/5.0 (FortuneT selftest)")
  if token:
    req.add_header("Authorization", f"Bearer {token}")
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

def count(where):
  return sql(f"SELECT COUNT(*) FROM shop_interest WHERE {where}").split()[-1]

def step(name, ok, detail=""):
  print(("✅ " if ok else "❌ ") + name + (f" — {detail}" if detail else ""))
  results.append(ok)

def code_of(b):
  return b.get("code") if isinstance(b, dict) else None

def me(token=None, anon=None):
  s, b = call("GET", "/api/shop/interest/me" + (f"?anonId={anon}" if anon else ""), token=token)
  return s, (b or {}).get("itemIds") if isinstance(b, dict) else b

uid = None
try:
  # 0. login（DB 直插 login token）
  plain = secrets.token_hex(32)
  h = hashlib.sha256(plain.encode()).hexdigest()
  exp = datetime.now(timezone.utc) + timedelta(minutes=10)
  iso = exp.strftime("%Y-%m-%dT%H:%M:%S.") + f"{exp.microsecond // 1000:03d}Z"
  sql("INSERT INTO login_tokens (token_hash, email, expires_at, used_at, pending_full_name, pending_invite_code) "
      f"VALUES ('{h}', '{EMAIL}', '{iso}', NULL, 'Selftest', NULL)")
  s, b = call("POST", "/api/auth/verify", {"token": plain})
  if s != 200: sys.exit(f"verify {s} {b}")
  TOKEN = b["sessionId"]
  s, u = call("GET", "/api/users/me", token=TOKEN)
  uid = (u or {}).get("id") or (u or {}).get("userId")
  step("login", bool(uid), uid)

  # 1. 匿名登記 → 重複 already
  s, b = call("POST", "/api/shop/interest", {"itemId": ITEM_A, "anonId": ANON1})
  step("匿名登記 A", s == 200 and b.get("registered") is True and b.get("already") is False, f"{s} {b}")
  s, b = call("POST", "/api/shop/interest", {"itemId": ITEM_A, "anonId": ANON1})
  step("匿名重複 A → already", s == 200 and b.get("already") is True, f"{s} {b}")
  step("DB：A 只有一列", count(f"item_id='{ITEM_A}' AND anon_id='{ANON1}'") == "1")

  # 2. 匿名轉登入：同 anonId 帶 token → already，不新增列，補上 user_id
  s, b = call("POST", "/api/shop/interest", {"itemId": ITEM_A, "anonId": ANON1}, token=TOKEN)
  step("登入後同 anonId 登記 A → already", s == 200 and b.get("already") is True, f"{s} {b}")
  n = count(f"item_id='{ITEM_A}' AND (anon_id='{ANON1}' OR user_id='{uid}')")
  step("DB：匿名轉登入不新增列", n == "1", n)
  n = count(f"item_id='{ITEM_A}' AND anon_id='{ANON1}' AND user_id='{uid}'")
  step("DB：該列已補上 user_id", n == "1", n)

  # 3. 登入登記 B（另一個 anonId）；無效 token 視同匿名
  s, b = call("POST", "/api/shop/interest", {"itemId": ITEM_B, "anonId": ANON2}, token=TOKEN)
  step("登入登記 B", s == 200 and b.get("already") is False, f"{s} {b}")
  s, b = call("POST", "/api/shop/interest", {"itemId": ITEM_D, "anonId": ANON3}, token="not-a-real-session")
  step("無效 token＋anonId → 視同匿名登記 D（不回 401）", s == 200 and b.get("already") is False, f"{s} {b}")

  # 4. me 聯集
  s, ids = me(anon=ANON1)
  step("me 只帶 ANON1 → [A]", s == 200 and ids == [ITEM_A], f"{s} {ids}")
  s, ids = me(token=TOKEN)
  step("me 只帶 token → A,B", s == 200 and sorted(ids or []) == sorted([ITEM_A, ITEM_B]), f"{s} {ids}")
  s, ids = me(token=TOKEN, anon=ANON3)
  step("me token ∪ ANON3 → A,B,D", s == 200 and sorted(ids or []) == sorted([ITEM_A, ITEM_B, ITEM_D]), f"{s} {ids}")

  # 5. 登入取消 B
  s, b = call("DELETE", f"/api/shop/interest/{ITEM_B}", {"itemId": ITEM_B}, token=TOKEN)
  step("登入取消 B", s == 200 and b.get("registered") is False, f"{s} {b}")
  step("DB：B 已刪", count(f"item_id='{ITEM_B}' AND (user_id='{uid}' OR anon_id='{ANON2}')") == "0")
  s, ids = me(token=TOKEN)
  step("me 不再含 B", s == 200 and ITEM_B not in (ids or []), f"{s} {ids}")

  # 6. 匿名取消 D
  s, b = call("DELETE", f"/api/shop/interest/{ITEM_D}", {"itemId": ITEM_D, "anonId": ANON3})
  step("匿名取消 D", s == 200 and b.get("registered") is False, f"{s} {b}")
  step("DB：D 已刪", count(f"item_id='{ITEM_D}' AND anon_id='{ANON3}'") == "0")

  # 7. 錯誤碼
  s, b = call("POST", "/api/shop/interest", {"itemId": "nope", "anonId": ANON1})
  step("未知 itemId → 400 INVALID_ITEM", s == 400 and code_of(b) == "INVALID_ITEM", f"{s} {b}")
  s, b = call("POST", "/api/shop/interest", {"itemId": ITEM_A, "anonId": "bad id!"})
  step("壞 anonId → 400 INVALID_ANON_ID", s == 400 and code_of(b) == "INVALID_ANON_ID", f"{s} {b}")
  s, b = call("GET", "/api/shop/interest/me?anonId=short")
  step("me 壞 anonId → 400 INVALID_ANON_ID", s == 400 and code_of(b) == "INVALID_ANON_ID", f"{s} {b}")
  s, b = call("GET", "/api/shop/interest/me")
  step("me 無身分 → 400 NO_IDENTITY", s == 400 and code_of(b) == "NO_IDENTITY", f"{s} {b}")
  s, b = call("POST", "/api/shop/interest", {"itemId": ITEM_A})
  step("POST 無身分 → 400 NO_IDENTITY", s == 400 and code_of(b) == "NO_IDENTITY", f"{s} {b}")
  s, b = call("DELETE", f"/api/shop/interest/{ITEM_A}", {"itemId": ITEM_A})
  step("DELETE 無身分 → 400 NO_IDENTITY", s == 400 and code_of(b) == "NO_IDENTITY", f"{s} {b}")
finally:
  anons = ", ".join(f"'{a}'" for a in (ANON1, ANON2, ANON3))
  user_clause = f" OR user_id='{uid}'" if uid else ""
  sql(f"DELETE FROM shop_interest WHERE anon_id IN ({anons}){user_clause}")
  if uid:
    sql(f"DELETE FROM users WHERE id='{uid}'")
  sql(f"DELETE FROM login_tokens WHERE email='{EMAIL}'")
  left = count(f"anon_id IN ({anons})" + user_clause)
  print(f"\nuid={uid}\ncleanup done（shop_interest 殘留 {left} 列）")

print("\n" + ("全部通過" if results and all(results) else f"有失敗：{results.count(False)} 項"))
