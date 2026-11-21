"""子單元（2026-11-21）正式站自測：自造測試帳號（DB 直插 login_tokens，同 f8-acceptance.sh）→
紫微流年大運／今年運勢 → 塔羅感情事業 → 擋板與快取清除；預設結束時 SQL 硬清測試列。
需本機 turso 已登入。會消耗 1 次今年運勢 AI＋1 次塔羅 AI。
用法：python3 -I scripts/subunits-selftest.py [--keep]（--keep 保留帳號並印 session，供瀏覽器自玩）"""
import hashlib, json, secrets, subprocess, sys, urllib.request, urllib.error, uuid
from datetime import datetime, timedelta, timezone

API = "https://fortunet-api.yanggf.workers.dev"
DB = "fortunet"
EMAIL = "ft-selftest-subunits@example.invalid"
KEEP = "--keep" in sys.argv
TOKEN = None
results = []

def call(method, path, body=None, token=None):
  req = urllib.request.Request(API + path, method=method,
    data=json.dumps(body).encode() if body is not None else None)
  req.add_header("Content-Type", "application/json")
  req.add_header("Origin", "https://ahexagram.com")
  req.add_header("User-Agent", "Mozilla/5.0 (FortuneT selftest)")
  if token or TOKEN:
    req.add_header("Authorization", f"Bearer {token or TOKEN}")
  try:
    with urllib.request.urlopen(req, timeout=120) as r:
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

def step(name, ok, detail=""):
  print(("✅ " if ok else "❌ ") + name + (f" — {detail}" if detail else ""))
  results.append(ok)

# 0. login
plain = secrets.token_hex(32)
h = hashlib.sha256(plain.encode()).hexdigest()
exp = datetime.now(timezone.utc) + timedelta(minutes=10)
iso = exp.strftime("%Y-%m-%dT%H:%M:%S.") + f"{exp.microsecond // 1000:03d}Z"
sql("INSERT INTO login_tokens (token_hash, email, expires_at, used_at, pending_full_name, pending_invite_code) "
    f"VALUES ('{h}', '{EMAIL}', '{iso}', NULL, 'Selftest', NULL)")
s, b = call("POST", "/api/auth/verify", {"token": plain})
if s != 200: sys.exit(f"verify {s} {b}")
TOKEN = b["sessionId"]
s, me = call("GET", "/api/users/me")
uid = me.get("id") or me.get("userId")
step("login", bool(uid), uid)

# 1. 擋板：無時辰
s, b = call("PUT", "/api/users/me/birth", {"birth_year": 1990, "birth_month": 5, "birth_day": 15, "birth_hour": None, "gender": "male"})
step("PUT birth（無時辰）", s == 200, str(s) if s != 200 else "")
s, b = call("GET", "/api/ziwei/horoscope")
step("無時辰 → horoscope 400 NO_BIRTH_HOUR", s == 400 and "NO_BIRTH_HOUR" in json.dumps(b), f"{s} {b}")
s, b = call("POST", "/api/ziwei/year/interpret")
step("無時辰 → interpret 400 NO_BIRTH_HOUR", s == 400 and "NO_BIRTH_HOUR" in json.dumps(b), f"{s} {b}")

# 2. 有時辰
s, b = call("PUT", "/api/users/me/birth", {"birth_year": 1990, "birth_month": 5, "birth_day": 15, "birth_hour": 12, "gender": "male"})
step("PUT birth（午時）", s == 200, str(s) if s != 200 else "")
s, b = call("GET", "/api/ziwei/horoscope?year=2026")
hz = (b or {}).get("horoscope", {})
y = hz.get("yearly", {})
ok = (s == 200 and y.get("stem") == "丙" and y.get("branch") == "午" and hz.get("nominalAge") == 37
      and hz.get("decadal", {}).get("branch") == "寅" and hz.get("age", {}).get("branch") == "辰"
      and [y.get("mutagen", {}).get(k) for k in ("lu", "quan", "ke", "ji")] == ["天同", "天機", "文昌", "廉貞"])
step("horoscope 2026 golden", ok, "" if ok else f"{s} {json.dumps(b, ensure_ascii=False)[:300]}")
step("horoscope 帶版本", "engineVersionZiweiHoroscope" in json.dumps(b), "")
s, b = call("GET", "/api/ziwei/horoscope")
step("horoscope 省略 year → 今天 luckYear 2026", s == 200 and (b or {}).get("horoscope", {}).get("luckYear") == 2026, f"{s}")
s, b = call("GET", "/api/ziwei/horoscope?year=1800")
step("year 越界 → 400 INVALID_YEAR", s == 400 and "INVALID_YEAR" in json.dumps(b), f"{s} {b}")
s, b = call("GET", "/api/ziwei/horoscope?year=1993")
step("1993 童限", s == 200 and b["horoscope"]["decadal"].get("isChildhood") is True, json.dumps(b.get("horoscope", {}).get("decadal") if isinstance(b, dict) else b, ensure_ascii=False))

# 3. 今年運勢
s, b = call("GET", "/api/ziwei/year")
step("GET year（未產生）reading=null", s == 200 and b.get("reading") is None and b.get("summary"), f"{s} {json.dumps(b, ensure_ascii=False)[:200]}")
s, b = call("POST", "/api/ziwei/year/interpret")
text = json.dumps(b, ensure_ascii=False)
step("POST interpret", s == 200, f"{s} {text[:300]}")
if s == 200:
  reading = b.get("reading") or b
  t = reading.get("aiInterpretation") or reading.get("interpretation") or ""
  print("—— AI 今年運勢 ——\n" + t + "\n——")
  import re
  step("輸出無月份／注定／預測", not re.search(r"[一二三四五六七八九十０-９0-9]+月|一定|注定|必然|保證|預測|本週", t.replace("流年是傳統的觀察框架，不是預測。", "")), "")
  step("結尾提醒句", "流年是傳統的觀察框架，不是預測" in t, "")
  s2, b2 = call("POST", "/api/ziwei/year/interpret")
  step("再次 POST → 快取命中", s2 == 200 and json.dumps(b2.get("reading") if isinstance(b2, dict) else b2, ensure_ascii=False)[:80] == json.dumps(b.get("reading"), ensure_ascii=False)[:80], f"{s2}")
  rows = sql(f"SELECT COUNT(*) FROM ziwei_year_readings WHERE user_id='{uid}'")
  step("DB 一列快取", "1" in rows.split()[-1], rows.strip().split()[-1])

# 4. 塔羅感情
rid = str(uuid.uuid4())
s, b = call("POST", "/api/tarot/draw", {"spread": "three", "question": "最近和他的相處", "requestId": rid, "topic": "love"})
step("tarot draw love", s == 201 and b.get("topic") == "love", f"{s} {json.dumps(b, ensure_ascii=False)[:200]}")
if s == 201:
  print("—— 規則解牌（感情）——\n" + b.get("interpretation", "") + "\n——")
  s2, b2 = call("POST", "/api/tarot/draw", {"spread": "three", "question": "最近和他的相處", "requestId": rid, "topic": "career"})
  step("同 requestId 換 topic → 409", s2 == 409, f"{s2}")
  s3, b3 = call("POST", "/api/tarot/draw", {"spread": "three", "question": "最近和他的相處", "requestId": rid, "topic": "love"})
  step("同 requestId 同 topic 重送 → 200 replay", s3 == 200, f"{s3}")
  s4, b4 = call("POST", f"/api/tarot/readings/{b['id']}/interpret")
  step("tarot interpret love", s4 == 200 and (b4.get("aiStatus") == "done"), f"{s4} {json.dumps(b4, ensure_ascii=False)[:200]}")
  if s4 == 200: print("—— AI 塔羅（感情）——\n" + (b4.get("aiInterpretation") or "") + "\n——")
s, b = call("POST", "/api/tarot/draw", {"spread": "three", "requestId": str(uuid.uuid4()), "topic": "career"})
step("tarot draw career", s == 201 and b.get("topic") == "career", f"{s}")
s, b = call("POST", "/api/tarot/draw", {"spread": "three", "requestId": str(uuid.uuid4())})
step("舊客戶端不帶 topic → general", s == 201 and b.get("topic") == "general", f"{s} {b.get('topic') if isinstance(b, dict) else b}")
s, b = call("GET", "/api/tarot/readings")
topics = [r.get("topic") for r in (b or {}).get("items", b.get("readings", []) if isinstance(b, dict) else [])]
step("歷史列出 topic", set(topics) >= {"love", "career", "general"}, str(topics))

# 5. 改生辰 → 今年運勢快取被刪
s, b = call("PUT", "/api/users/me/birth", {"birth_year": 1990, "birth_month": 5, "birth_day": 15, "birth_hour": 12, "gender": "male"})
rows = sql(f"SELECT COUNT(*) FROM ziwei_year_readings WHERE user_id='{uid}'")
step("改生辰 → ziwei_year_readings 清空", s == 200 and rows.split()[-1] == "0", rows.split()[-1])

print(f"\nuid={uid}")
if KEEP:
  print(f"SESSION={TOKEN}")
else:
  for t in ("tarot_readings", "ziwei_year_readings", "interpretations"):
    sql(f"DELETE FROM {t} WHERE user_id='{uid}'")
  sql(f"DELETE FROM users WHERE id='{uid}'"); sql(f"DELETE FROM login_tokens WHERE email='{EMAIL}'")
  print("cleanup done")
print("\n" + ("全部通過" if all(results) else f"有失敗：{results.count(False)} 項"))
