"""首頁「月亮王座」吉祥物（2026-11-23 站長：吉祥物不用動太多，只簡單轉頭；椅背月亮↔太陽循環；
星星以吉祥物為中心公轉、只出現在椅背；頭永遠是月牙（月相改由椅背的月亮表現）；尾巴要看得到）。

組成（由後往前）：
  1. 一疊 sprite stack＝圓形展示底座＋石造王座（椅背花窗只有鉛條，不畫月亮）＋坐姿月亮貓的身體——靜止不轉；
  2. 同一疊的每一層再放一張「頭」圖集，只有頭繞自己的中心慢慢左右轉（CSS `.throne-head`）；
  3. 花窗上疊 2D 像素圖：當晚月相的月亮與太陽交替淡入淡出（位置由投影算好，並檢查不會蓋到前方體素，
     貓頭左右轉的範圍也算）；
  4. 星星繞著吉祥物頭部中心公轉：星星層套一張遮罩（orbit-mask），只留「椅背正面、且沒被前方體素擋住」的
     區域——星星繞到貓背後就被擋住、離開椅背就消失。
貓本體重用 mascot.py 的模型（月牙頭；新月夜睡覺用暗色月牙＋閉眼）。站姿尾巴往後伸會埋進椅背，坐姿改由左側
繞出、翹在扶手上。
輸出：crates/web/assets/mascot/throne/*.webp（檔名帶內容 hash）、crates/web/src/components/throne_stacks.rs（勿手改）。
改模型後重跑：python3 scripts/sprite-stack/throne.py；預覽：--preview <輸出目錄>。顏色是點陣圖內容，不受 :root token 規則管轄。"""
import hashlib
import importlib.util
import math
import os
import subprocess
import sys
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("mascot_model", os.path.join(HERE, "mascot.py"))
mg = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mg)

N = 64              # 平面格數
C = (N - 1) / 2
SHEET_SCALE = 2     # 圖集每格像素（pixelated 放大）
COLS = 8
SQUASH = mg.SQUASH
LIFT = mg.LIFT
HEAD_TURN = 16      # 轉頭角度（±度），與 style.css .throne-head 的 keyframes 一致
ROOT = os.path.join(HERE, "..", "..", "crates", "web", "assets", "mascot", "throne")
RUST_OUT = os.path.join(HERE, "..", "..", "crates", "web", "src", "components", "throne_stacks.rs")

STONE = (214, 208, 232); STONE_D = (186, 178, 214); PANEL = (226, 140, 176)
BASE = (64, 74, 140); BASE_E = (96, 110, 176)
MOON_C = (252, 236, 170); MOON_E = (214, 178, 96)
SUN_C = (252, 206, 96); SUN_E = (236, 150, 60)
STAR_C = (250, 206, 110); STAR_E = (214, 158, 60)

SZ = 15                         # 座面頂
PANEL_Y = int(C - 11) - 1       # 花窗面（椅背最前一層）
OX, OY, OZ = C - mg.C, (C - 4) - mg.C, SZ - 5   # 貓模型平移：坐在座面上、背靠椅背
HEAD_PIVOT = (C, C - 4)         # 頭的轉軸（切片座標）
# 花窗上的疊加物：(名稱, 中心 x, 中心 z, 寬, 高)（體素格；中心落在格中央）
MOON_AT = (C - 2, 71, 21, 21)
STAR_SIZE = 9
ORBIT_R = 0.30      # 星星公轉半徑（層寬倍數）
ORBIT_N = 6         # 星星數
MASK_PX = 4         # 遮罩每格像素
PHASE_KEYS = ("new", "crescent", "quarter", "gibbous", "full")   # ＝ft_schema::moon::MoonPhase::key()


def put(v, x, y, z, c, keep=False):
  x, y, z = int(math.floor(x + 0.5)), int(math.floor(y + 0.5)), int(math.floor(z + 0.5))
  if 0 <= x < N and 0 <= y < N and z >= 0:
    if keep and (x, y, z) in v:
      return
    v[(x, y, z)] = c


def capsule(v, a, b, r, col):
  (ax, ay, az), (bx, by, bz) = a, b
  dx, dy, dz = bx - ax, by - ay, bz - az
  ll = dx * dx + dy * dy + dz * dz or 1
  for x in range(int(min(ax, bx) - r) - 1, int(max(ax, bx) + r) + 2):
    for y in range(int(min(ay, by) - r) - 1, int(max(ay, by) + r) + 2):
      for z in range(max(0, int(min(az, bz) - r) - 1), int(max(az, bz) + r) + 2):
        t = max(0, min(1, ((x - ax) * dx + (y - ay) * dy + (z - az) * dz) / ll))
        if (x - ax - dx * t) ** 2 + (y - ay - dy * t) ** 2 + (z - az - dz * t) ** 2 <= r * r:
          if 0 <= x < N and 0 <= y < N:
            v[(x, y, z)] = col(z) if callable(col) else col


def throne():
  """底座＋王座（椅背花窗只有放射狀鉛條；月亮／太陽／星星是 2D 疊加）。"""
  v = {}
  for x in range(N):
    for y in range(N):
      d = math.hypot(x - C, y - C)
      if d <= 30:
        for z in range(0, 3):
          v[(x, y, z)] = BASE_E if d > 28.5 else BASE
  for x in range(int(C - 15), int(C + 15)):
    for y in range(int(C - 12), int(C + 8)):
      for z in range(3, SZ):
        v[(x, y, z)] = STONE
  for sx in (-1, 1):
    x0 = int(C + 11) if sx > 0 else int(C - 15)
    for x in range(x0, x0 + 4):
      for y in range(int(C - 12), int(C + 8)):
        for z in range(SZ, SZ + 8):
          v[(x, y, z)] = STONE_D
  for x in range(int(C - 25), int(C + 26)):
    for z in range(SZ, 84):
      top = 56 + 26 * math.sqrt(max(0, 1 - ((x - C) / 25.5) ** 2))
      if z > top:
        continue
      inner = abs(x - C) <= 21.5 and SZ + 3 <= z <= top - 3
      for y in range(int(C - 16), PANEL_Y + 1):
        c = STONE
        if inner and y == PANEL_Y:
          c = PANEL
          a = math.degrees(math.atan2(z - 52, x - C))
          if 20 < a < 160 and int(a) % 30 < 2:
            c = STONE_D
        v[(x, y, z)] = c
  return v


def seated_cat(pose, phase):
  """坐姿月亮貓：拿掉站姿的腿與靴（z<6）、移到座面；大腿向前、小腿垂下、條紋長襪＋靴子。
  回傳 (身體體素, 頭體素)。"""
  m = mg.build(pose, phase)
  head_keys = {k for k, c in m.head.items() if m.v.get(k) == c}
  body, head = {}, {}
  for (x, y, z), c in m.v.items():
    if z < 6 or (x, y, z) in m.tail:
      continue
    k = (int(x + OX), int(y + OY), int(z + OZ))
    (head if (x, y, z) in head_keys else body)[k] = c
  sock = lambda z: mg.MINT if (z // 2) % 2 else mg.CREAM
  for sx in (-1, 1):
    hip = (C + sx * 4, C - 2, SZ + 3)
    knee = (C + sx * 4.6, C + 8, SZ + 2)
    foot = (C + sx * 5, C + 10, SZ - 5)
    capsule(body, hip, knee, 3.0, mg.LAV)
    capsule(body, knee, foot, 2.6, sock)
    capsule(body, (foot[0], foot[1] + 0.5, foot[2] - 1), (foot[0], foot[1] + 2.5, foot[2] - 2), 3.4, mg.CREAM)
  # 尾巴：從左後腰繞出、越過左扶手甩到外側再往上翹，末端薄荷條紋（站姿往後伸的尾巴會埋進椅背）
  path = [(C - 6, C - 6, SZ + 3), (C - 12, C - 4, SZ + 4), (C - 17, C - 2, SZ + 7),
          (C - 19, C - 1, SZ + 12), (C - 18.5, C, SZ + 17), (C - 20.5, C + 0.5, SZ + 21)]
  pts = []
  for (a, b) in zip(path, path[1:]):
    for i in range(6):
      t = i / 6
      pts.append(tuple(a[j] + (b[j] - a[j]) * t for j in range(3)))
  pts.append(path[-1])
  for i, (x, y, z) in enumerate(pts):
    r = 2.2 if i < len(pts) - 4 else 1.9
    col = mg.MINT if i >= len(pts) - 12 and (i // 3) % 2 == 0 else mg.LAV
    for xx in range(int(x - r) - 1, int(x + r) + 2):
      for yy in range(int(y - r) - 1, int(y + r) + 2):
        for zz in range(int(z - r) - 1, int(z + r) + 2):
          if (xx - x) ** 2 + (yy - y) ** 2 + ((zz - z) * LIFT) ** 2 <= r * r:
            body[(xx, yy, zz)] = col
  return body, head


def rotate_part(part, deg):
  """把頭體素繞 HEAD_PIVOT 轉 deg 度（只用於碰撞檢查）。"""
  t = math.radians(deg)
  px, py = HEAD_PIVOT
  out = {}
  for (x, y, z), c in part.items():
    dx, dy = x - px, y - py
    out[(px + dx * math.cos(t) - dy * math.sin(t), py + dx * math.sin(t) + dy * math.cos(t), z)] = c
  return out


def layers_of(part, top, full):
  """每層一張 RGBA（N×N）。明暗與輪廓依整個模型（full）判斷，身體與頭分開畫也接得起來。"""
  out = []
  for z in range(top + 1):
    im = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    f = 0.88 + 0.12 * z / top
    for (x, y, zz), c in part.items():
      if zz != z:
        continue
      edge = any((x + dx, y + dy, z) not in full for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
      k = f * (0.9 if edge and c in (mg.LAV, mg.LAV_D, mg.DARK, STONE) else 1.0)
      im.putpixel((x, y), tuple(min(255, int(ch * k)) for ch in c) + (255,))
    out.append(im)
  return out


def save_sheet(name, layers):
  rows = -(-len(layers) // COLS)
  cell = N * SHEET_SCALE
  sheet = Image.new("RGBA", (COLS * cell, rows * cell), (0, 0, 0, 0))
  for z, im in enumerate(layers):
    sheet.paste(im.resize((cell, cell), Image.NEAREST), ((z % COLS) * cell, (z // COLS) * cell))
  return save_img(name, sheet)


def save_img(name, im):
  tmp = os.path.join(ROOT, f".{name}.tmp.webp")
  im.save(tmp, lossless=True)
  with open(tmp, "rb") as fh:
    digest = hashlib.sha256(fh.read()).hexdigest()[:12]
  os.replace(tmp, os.path.join(ROOT, f"{name}-{digest}.webp"))
  print(name, im.size, digest)
  return digest


# ── 2D 疊加：像素畫（每格一像素，CSS pixelated 放大；高度乘 LIFT 對上椅背的層距） ──
def outline(im, fill, edge):
  w, h = im.size
  px = im.load()
  src = [[px[x, y][3] > 0 for x in range(w)] for y in range(h)]
  for y in range(h):
    for x in range(w):
      if src[y][x]:
        on_edge = any(not (0 <= x + dx < w and 0 <= y + dy < h and src[y + dy][x + dx])
                      for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
        px[x, y] = (edge if on_edge else fill) + (255,)
  return im


def draw_moon(phase="crescent", size=21):
  """椅背的月亮＝當晚月相。眉月沿用斜掛的月牙；弦月／凸月／滿月以明暗界線（亮面在左）；新月只留淡輪廓。"""
  im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
  c = (size - 1) / 2
  R = 9.6
  tk = {"quarter": 0.0, "gibbous": 0.5, "full": 1.01}.get(phase)
  for x in range(size):
    for y in range(size):
      if math.hypot(x - c, y - c) > R:
        continue
      if phase == "crescent":
        lit = math.hypot(x - (c + 5), y - (c - 4)) > 8.4
      elif phase == "new":
        lit = math.hypot(x - c, y - c) > R - 1.2
      else:
        yr = (y - c) / R
        lit = (x - c) / R <= tk * math.sqrt(max(0.0, 1 - yr * yr)) + 1e-9
      if lit:
        im.putpixel((x, y), MOON_C + (255,))
  im = outline(im, MOON_C, MOON_E)
  if phase == "new":
    px = im.load()
    for y in range(size):
      for x in range(size):
        r, g, b, a = px[x, y]
        if a:
          px[x, y] = (r, g, b, 150)
  return im


def draw_sun(size=21):
  im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
  c = (size - 1) / 2
  for x in range(size):
    for y in range(size):
      d = math.hypot(x - c, y - c)
      a = math.degrees(math.atan2(y - c, x - c)) % 45
      if d <= 5.6 or (7.2 <= d <= 10.2 and (a < 7 or a > 38)):
        im.putpixel((x, y), SUN_C + (255,))
  return outline(im, SUN_C, SUN_E)


def draw_star(size=STAR_SIZE):
  im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
  c = size // 2
  r = 2
  for dx in range(-c, c + 1):
    for dz in range(-c, c + 1):
      if abs(dx) + abs(dz) <= r or (abs(dx) <= 0 and abs(dz) <= c) or (abs(dz) <= 0 and abs(dx) <= c):
        im.putpixel((c + dx, c + dz), STAR_C + (255,))
  return outline(im, STAR_C, STAR_E)


def screen_h(y, z):
  """體素中心在畫面上、離層框底的高度（層寬 W 為單位；角度 0，與 style.css 疊法一致）。"""
  return (1 + SQUASH) / 2 - SQUASH * (y + 0.5) / N + (z + 0.5) * LIFT / N


def stage_box(parts):
  lo, hi = 1e9, -1e9
  for part in parts:
    for (x, y, z) in part:
      b = screen_h(y, z)
      lo = min(lo, b - SQUASH * 0.5 / N - 0.5 * LIFT / N)
      hi = max(hi, b + SQUASH * 0.5 / N + 0.5 * LIFT / N + 0.5 * LIFT / N)
  return round(hi - lo + 0.02, 3), round(lo - 0.01, 3)


def overlay_rect(cx, cz, w, h):
  """花窗上一個疊加物的畫面矩形 (左, 右, 下, 上)，W 為單位。"""
  mid = screen_h(PANEL_Y, cz)
  return ((cx + 0.5 - w / 2) / N, (cx + 0.5 + w / 2) / N, mid - h / 2 * LIFT / N, mid + h / 2 * LIFT / N)


def check_clear(rects, front_parts):
  """疊加物不得蓋到花窗前方的體素（貓、扶手、魔杖…）。"""
  for name, (l, r, b, t) in rects:
    for part in front_parts:
      for (x, y, z) in part:
        if y <= PANEL_Y + 0.5:
          continue
        h = screen_h(y, z)
        if (x + 1) / N > l and x / N < r and h + 0.5 * LIFT / N > b and h - 0.5 * LIFT / N < t:
          raise SystemExit(f"疊加物 {name} 會蓋到前方體素 {(x, y, z)}：請移動位置")


def pct(v):
  return round(v * 100, 3)


def head_center(head):
  """頭部的畫面中心 (x, h)，W 為單位（星星公轉中心）。"""
  xs = [(x + 0.5) / N for (x, _, _) in head]
  hs = [screen_h(y, z) for (_, y, z) in head]
  return sum(xs) / len(xs), sum(hs) / len(hs)


def orbit_mask(th, fronts, mh, mb):
  """星星層遮罩：椅背正面（y==PANEL_Y、座面以上）減去所有在它前方的體素（含轉頭範圍）。"""
  from PIL import ImageDraw
  wpx = N * MASK_PX
  hpx = int(round(mh * wpx))
  m = Image.new("L", (wpx, hpx), 0)
  d = ImageDraw.Draw(m)

  def box(x, y, z):
    h = screen_h(y, z)
    lo, hi = h - SQUASH * 0.5 / N, h + SQUASH * 0.5 / N + 0.5 * LIFT / N
    return (x * MASK_PX, (mh - (hi - mb)) * wpx, (x + 1) * MASK_PX - 1, (mh - (lo - mb)) * wpx)

  for (x, y, z) in th:
    if y == PANEL_Y and z >= SZ:
      d.rectangle(box(x, y, z), fill=255)
  for part in fronts:
    for (x, y, z) in part:
      if y > PANEL_Y + 0.5:
        x0, y0, x1, y1 = box(int(math.floor(x + 0.5)), y, z)
        d.rectangle((x0 - 1, y0 - 1, x1 + 1, y1 + 1), fill=0)   # 外擴 1px：星星不從前方體素的縫漏出
  out = Image.new("RGBA", (wpx, hpx), (255, 255, 255, 0))
  out.putalpha(m)
  return out


def build_all():
  th = throne()
  bodies, variants = {}, {}
  for pose, head_key in (("wave", "crescent"), ("sleep", "new")):
    body, head = seated_cat(pose, head_key)
    bodies[pose] = body
    variants["sleep" if pose == "sleep" else "crescent"] = head
  top = max(z for part in [th, *bodies.values(), *variants.values()] for (_, _, z) in part)
  fronts = [th, *bodies.values()]
  for h in variants.values():
    fronts += [rotate_part(h, a) for a in range(-HEAD_TURN, HEAD_TURN + 1, 4)]
  moon_rect = overlay_rect(MOON_AT[0], MOON_AT[1], MOON_AT[2], MOON_AT[3])
  check_clear([("moon", moon_rect)], fronts)
  mh, mb = stage_box([th, *bodies.values(), *variants.values()])
  return th, bodies, variants, top, fronts, moon_rect, mh, mb


def main():
  os.makedirs(ROOT, exist_ok=True)
  th, bodies, variants, top, fronts, moon_rect, mh, mb = build_all()
  layers = top + 1
  for f in os.listdir(ROOT):
    if f.endswith(".webp"):
      os.remove(os.path.join(ROOT, f))
  body_rows, head_rows, moon_rows = [], [], []
  for pose, body in bodies.items():
    head = variants["sleep" if pose == "sleep" else "crescent"]
    full = {**th, **body, **head}
    body_rows.append((pose, save_sheet(f"body-{pose}", layers_of({**th, **body}, top, full))))
  for name, head in variants.items():
    full = {**th, **bodies["sleep" if name == "sleep" else "wave"], **head}
    head_rows.append((name, save_sheet(f"head-{name}", layers_of(head, top, full))))
  for ph in PHASE_KEYS:
    moon_rows.append((ph, save_img(f"moon-{ph}", draw_moon(ph))))
  sun_h = save_img("sun", draw_sun())
  star_h = save_img("star", draw_star())
  mask_h = save_img("orbit-mask", orbit_mask(th, fronts, mh, mb))

  l, r, b, t = moon_rect
  sky = (pct(l), pct((b - mb) / mh), pct(r - l), pct((t - b) / mh))
  hx, hh = head_center(variants["crescent"])
  orbit = (pct(hx), pct((hh - mb) / mh), ORBIT_R, round(STAR_SIZE / N, 4))
  origin = (pct((HEAD_PIVOT[0] + 0.5) / N), pct((HEAD_PIVOT[1] + 0.5) / N))

  def rows(rs):
    return "\n".join(f'    ("{p}", "{h}"),' for p, h in rs)

  src = f"""//! 首頁月亮王座吉祥物的圖集與疊加物常數——由 `scripts/sprite-stack/throne.py` 產生，**勿手改**。
//!
//! 圖集在 `crates/web/assets/mascot/throne/`：`body-<pose>-<hash>.webp`（底座＋王座＋貓身，靜止）、
//! `head-<crescent|sleep>-<hash>.webp`（只有頭）；兩者層數相同、逐層交錯疊。

/// 切片的平面格數（寬＝高）。
pub const THRONE_GRID: usize = {N};
/// 圖集欄數。
pub const THRONE_COLS: usize = {COLS};
/// 層數（身體與頭的圖集相同）。
pub const THRONE_LAYERS: usize = {layers};
/// 舞台高（層寬倍數，CSS `--mh`）與層底位移（CSS `--mb`）。
pub const THRONE_STAGE_H: f32 = {mh};
pub const THRONE_STAGE_B: f32 = {mb};
/// 頭的轉軸（層框內 x%, y%）＝ CSS `transform-origin`。
pub const THRONE_HEAD_ORIGIN: (f32, f32) = ({origin[0]}, {origin[1]});

/// 身體圖集：(姿勢, hash)。
pub const THRONE_BODY: [(&str, &str); {len(body_rows)}] = [
{rows(body_rows)}
];
/// 頭圖集：("crescent" 或 "sleep", hash)。
pub const THRONE_HEAD: [(&str, &str); {len(head_rows)}] = [
{rows(head_rows)}
];
/// 椅背月亮（當晚月相，鍵＝`MoonPhase::key()`）：(月相, hash)。
pub const THRONE_MOONS: [(&str, &str); {len(moon_rows)}] = [
{rows(moon_rows)}
];
/// 太陽、星星、星星層遮罩的檔名 hash。
pub const THRONE_SUN: &str = "{sun_h}";
pub const THRONE_STAR: &str = "{star_h}";
pub const THRONE_ORBIT_MASK: &str = "{mask_h}";
/// 月亮／太陽在舞台上的位置：(left%, bottom%, width%, height%)（%＝舞台寬／高）。
pub const THRONE_SKY_RECT: (f32, f32, f32, f32) = {sky};
/// 星星公轉：(中心 left%, 中心 bottom%, 半徑（層寬倍數）, 星星寬（層寬倍數）)。
pub const THRONE_ORBIT: (f32, f32, f32, f32) = {orbit};
/// 星星數。
pub const THRONE_ORBIT_STARS: usize = {ORBIT_N};
"""
  out = subprocess.run(["rustfmt", "--edition", "2021", "--emit", "stdout"], input=src.encode(),
                       capture_output=True, check=True).stdout.decode()
  with open(RUST_OUT, "w") as fh:
    fh.write(out)
  print("layers", layers, "stage", (mh, mb), "orbit", orbit, "wrote", os.path.normpath(RUST_OUT))


def preview(out_dir):
  """舞台座標的合成預覽（與 CSS 同一套幾何）：月亮＋公轉星星（套遮罩），頭轉 -HEAD_TURN／0／+HEAD_TURN。"""
  os.makedirs(out_dir, exist_ok=True)
  th, bodies, variants, top, fronts, moon_rect, mh, mb = build_all()
  W = 384
  Hs = int(round(mh * W))
  mask = orbit_mask(th, fronts, mh, mb).resize((W, Hs), Image.NEAREST).getchannel("A")
  hx, hh = head_center(variants["crescent"])
  star = draw_star()
  sp = int(STAR_SIZE / N * W)
  for name, head_key, pose, phase in (("crescent", "crescent", "wave", "full"), ("sleep", "sleep", "sleep", "new")):
    for a in (-HEAD_TURN, 0, HEAD_TURN):
      vox = {**th, **bodies[pose]}
      for (x, y, z), c in rotate_part(variants[head_key], a).items():
        vox[(int(math.floor(x + 0.5)), int(math.floor(y + 0.5)), z)] = c
      ls = layers_of(vox, max(z for (_, _, z) in vox), vox)
      cv = Image.new("RGBA", (W, Hs), (255, 245, 250, 255))
      for k in range(len(ls) * 2):
        pos = k // 2 + (0.5 if k % 2 else 0)
        top_h = -mb + (1 - SQUASH) / 2 + SQUASH + pos * LIFT / N
        cv.alpha_composite(ls[k // 2].resize((W, int(W * SQUASH)), Image.NEAREST), (0, int(Hs - top_h * W)))
      l, r, b, t = moon_rect
      cv.alpha_composite(draw_moon(phase).resize((int((r - l) * W), int((t - b) * W)), Image.NEAREST),
                         (int(l * W), int(Hs - (t - mb) * W)))
      layer = Image.new("RGBA", (W, Hs), (0, 0, 0, 0))
      for i in range(ORBIT_N):
        ang = math.radians(i * 360 / ORBIT_N + a * 2)
        cx, cy = hx * W + ORBIT_R * W * math.cos(ang), Hs - (hh - mb) * W - ORBIT_R * W * math.sin(ang)
        layer.alpha_composite(star.resize((sp, sp), Image.NEAREST), (int(cx - sp / 2), int(cy - sp / 2)))
      la = layer.getchannel("A")
      layer.putalpha(Image.composite(la, Image.new("L", la.size, 0), mask))
      cv.alpha_composite(layer)
      cv.save(os.path.join(out_dir, f"throne-{name}-{a}.png"))
  print("preview →", out_dir)


if __name__ == "__main__":
  if len(sys.argv) == 3 and sys.argv[1] == "--preview":
    preview(sys.argv[2])
  else:
    main()
