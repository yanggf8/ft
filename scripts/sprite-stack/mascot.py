"""吉祥物月亮貓 sprite stacking 切片產生器（2026-11-23）。

程式化體素月亮貓（頭本身是月牙、長靴、肚肚、圍巾、尾巴；照 2026-11-19 原設計平面圖；大頭短身呆萌比例）× 三個姿勢：
  wave（首頁打招呼，右手魔杖）／wait（塔羅翻牌前，雙手握魔杖在胸前）／joy（翻牌後，雙手舉高、張嘴笑）。
舞台高度與層底位置由 stage_box 算出寫進產生檔，元件以 --mh／--mb 行內帶給 CSS。
輸出圖集 crates/web/assets/mascot/stack/<pose>-<phase>-<hash>.webp（姿勢 × 月相頭形，新月另有 sleep）（一個姿勢一張、一個請求），層數／格數／hash 寫成產生檔
crates/web/src/components/mascot_stacks.rs（勿手改，components/mascot.rs 讀它）。
正面＝切片的 +y（圖下緣），元件以左右擺動代替整圈自轉，臉一直朝前。
改模型後重跑：python3 scripts/sprite-stack/mascot.py；
預覽合成圖（與 CSS 同款疊法）：python3 scripts/sprite-stack/mascot.py --preview <輸出目錄>。
顏色是點陣圖內容，不受 :root token 規則管轄。"""
import hashlib
import math
import os
import subprocess
import sys
from PIL import Image

N = 48        # 平面格數（2026-11-23 由 32 提高：臉要放得下正圓大眼）
SCALE = 8     # 預覽每格像素
SHEET_SCALE = 3   # 圖集每格像素（顯示 --mw 150px、2x 螢幕約 6 px／格，pixelated 放大仍銳利）
COLS = 8      # 圖集欄數
SQUASH = 0.5  # 與 style.css .mascot-stack 的 --squash 一致（低俯角，臉看得到）
C = (N - 1) / 2
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "crates", "web", "assets", "mascot", "stack")
RUST_OUT = os.path.join(HERE, "..", "..", "crates", "web", "src", "components", "mascot_stacks.rs")
POSES = ("wave", "wait", "joy")
# 月相頭形（2026-11-23「引路貓」設計稿）：挖口圓 (x 偏移, z 偏移, 半徑)；半徑 0＝滿月圓頭。
# wire 值＝ft_schema::moon::MoonPhase::key()，是圖集檔名的一部分，不可改名。
PHASES = {
  "new": (2.2, 12.6, 10.4),       # 新月：頭形同眉月、頭色轉暗（DARK）
  "crescent": (2.2, 12.6, 10.4),  # 眉月：標準造型
  "quarter": (2.0, 15.5, 9.0),
  "gibbous": (1.5, 18.0, 7.5),
  "full": (0.0, 0.0, 0.0),
}
# (姿勢, 月相) 組合：三個姿勢 × 五種月相，加新月夜睡覺（wait 手勢＋閉眼）
COMBOS = [(p, ph) for ph in PHASES for p in POSES] + [("sleep", "new")]

LAV = (204, 188, 244); LAV_D = (180, 162, 228); DARK = (150, 140, 204)
PINK = (246, 184, 206); MOON = (244, 166, 196)
CREAM = (252, 244, 232); MINT = (150, 214, 208)
TEAL = (110, 196, 200); INK = (92, 78, 128); WHITE = (255, 255, 255)
BLUSH = (248, 160, 186); STAR = (130, 214, 212); STICK = (96, 170, 180)


class Model:
  def __init__(self):
    self.v = {}

  def put(self, x, y, z, rgb, keep=False):
    if 0 <= x < N and 0 <= y < N and z >= 0:
      if keep and (x, y, z) in self.v:
        return
      self.v[(x, y, z)] = rgb

  def ellipsoid(self, cx, cy, cz, rx, ry, rz, rgb, keep=False):
    for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
      for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for z in range(max(0, int(cz - rz) - 1), int(cz + rz) + 2):
          if ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 + ((z - cz) / rz) ** 2 <= 1:
            self.put(x, y, z, rgb, keep)

  def capsule(self, a, b, r, rgb, keep=False):
    """a→b 的圓柱＋圓頭。"""
    (ax, ay, az), (bx, by, bz) = a, b
    dx, dy, dz = bx - ax, by - ay, bz - az
    ll = dx * dx + dy * dy + dz * dz or 1
    for x in range(int(min(ax, bx) - r) - 1, int(max(ax, bx) + r) + 2):
      for y in range(int(min(ay, by) - r) - 1, int(max(ay, by) + r) + 2):
        for z in range(max(0, int(min(az, bz) - r) - 1), int(max(az, bz) + r) + 2):
          t = max(0, min(1, ((x - ax) * dx + (y - ay) * dy + (z - az) * dz) / ll))
          px, py, pz = ax + dx * t, ay + dy * t, az + dz * t
          if (x - px) ** 2 + (y - py) ** 2 + (z - pz) ** 2 <= r * r:
            self.put(x, y, z, rgb, keep)

  def front(self, x, z):
    """(x, z) 這一直線上最靠前（y 最大）的體素 y；沒有則 None。"""
    ys = [y for y in range(N) if (x, y, z) in self.v]
    return max(ys) if ys else None

  def paint_front(self, x, z, rgb, depth=1, ymin=None):
    """在正面表面上色（往內 depth 格，讓斜看也有厚度）。ymin：只畫到正面夠前的地方（避開手臂後面）。"""
    y = self.front(x, z)
    if y is None or (ymin is not None and y < ymin):
      return
    for d in range(depth):
      if (x, y - d, z) in self.v:
        self.v[(x, y - d, z)] = rgb


def star_pts(cx, cz, r):
  """十字星（x, z）格點。"""
  pts = {(cx, cz)}
  for i in range(1, r + 1):
    pts |= {(cx + i, cz), (cx - i, cz), (cx, cz + i), (cx, cz - i)}
  return pts


LIFT = 0.85   # 每層往上位移（格），與 sprite_stack.rs layer_step 一致；畫面上的圓要把 z 半徑除以它


def build(pose, phase="crescent"):
  """呆萌比例（2026-11-23 站長回饋）：大頭短身（頭約佔全身一半）、正圓大眼放低分開、小 ω 嘴、圓腮紅。
  頭本身是月牙（照原設計）：大圓上方被挖掉一塊、兩角朝上，挖口內緣粉紅月牙帶只畫在前後表面。"""
  m = Model()
  # 靴子＋短腿
  for sx in (-1, 1):
    m.capsule((C + sx * 4.2, C + 0.8, 2), (C + sx * 4.2, C + 0.8, 3.5), 3.4, CREAM)
    m.capsule((C + sx * 4, C, 5), (C + sx * 3.6, C, 8), 3.0, LAV)
  # 胖胖的身體＋肚肚
  m.ellipsoid(C, C, 14, 8.2, 7.2, 8.6, LAV)
  for x in range(N):
    for z in range(6, 22):
      if ((x - C) / 4.8) ** 2 + ((z - 13) / 5.4) ** 2 <= 1:
        m.paint_front(x, z, PINK, 2)
  # 尾巴：從背後彎上來，末端薄荷色條紋
  pts = [(C + 3 + 5 * math.sin(t * 1.6), C - 6 - 6 * t, 8 + 16 * t) for t in [i / 20 for i in range(21)]]
  for i, (x, y, z) in enumerate(pts):
    m.ellipsoid(x, y, z, 2.3, 2.3, 2.3, MINT if i >= 14 and i % 3 == 0 else LAV)
  # 頭＝胖月牙：厚度＝離外緣、離挖口兩邊都做圓角（不會像被切平的碗）
  HZ = 38.5
  R = 15.0
  bx, bz, BR = PHASES[phase]
  BX, BZ = C + bx, HZ + bz
  HEAD = DARK if phase == "new" else LAV
  sleep = pose == "sleep"
  D = 10.0
  for x in range(N):
    for z in range(int(HZ - R) - 1, int(HZ + R) + 2):
      d = math.hypot(x - C, z - HZ)
      db = math.hypot(x - BX, z - BZ)
      if d > R or db < BR:
        continue
      outer = D * math.sqrt(max(0.0, 1 - (d / R) ** 2))
      inner = 2.2 + 2.4 * math.sqrt(db - BR)
      ry = max(1.6, min(outer, inner))
      band = db < BR + 2.8
      for y in range(N):
        if abs(y - C) <= ry:
          m.put(x, y, z, MOON if BR > 0 and band and abs(y - C) > ry - 1.6 else HEAD)
  # 臉：正圓大眼（畫面上的圓：z 半徑＝x 半徑 / LIFT）、大反光點、圓腮紅、小 ω 嘴、額頭小星
  EZ = HZ - 4.6
  ER = 4.3
  for sx in (-1, 1):
    ex = C + sx * 6.9
    for x in range(N):
      for z in range(int(EZ - ER / LIFT) - 1, int(EZ + ER / LIFT) + 2):
        d = math.hypot((x - ex) / ER, (z - EZ) * LIFT / ER)
        if sleep:
          # 閉眼：下彎弧線
          dd = math.hypot((x - ex) / ER, (z - (EZ + 1)) * LIFT / ER)
          if 0.72 <= dd <= 1 and z < EZ + 1:
            m.paint_front(x, z, INK)
        elif d <= 1:
          m.paint_front(x, z, INK if (d > 0.78 or z > EZ + 0.6) else TEAL)
    # 反光：左上 2×2 大亮點＋右下 1 點（睡著不畫）
    for dx, dz in (() if sleep else ((-1.6, 1.8), (-0.6, 1.8), (-1.6, 2.8), (-0.6, 2.8), (1.6, -1.9))):
      m.paint_front(round(ex + dx), round(EZ + dz), WHITE)
    for x in range(N):
      for z in range(int(HZ) - 14, int(HZ) - 6):
        if math.hypot((x - (C + sx * 10.6)) / 2.6, (z - (HZ - 9.4)) * LIFT / 1.7) <= 1:
          m.paint_front(x, z, BLUSH)
  if pose == "joy":
    for x in range(N):
      for z in range(int(HZ) - 13, int(HZ) - 7):
        d = math.hypot((x - C) / 1.9, (z - (HZ - 10)) * LIFT / 1.9)
        if d <= 1:
          m.paint_front(x, z, PINK if d < 0.55 and z < HZ - 10 else INK)
  else:
    for x, z in ((C - 1.5, HZ - 9.2), (C - 0.5, HZ - 10.2), (C + 0.5, HZ - 10.2), (C + 1.5, HZ - 9.2)):
      m.paint_front(round(x), round(z), INK)
  for x, z in star_pts(round(C), round(HZ - 1), 1):
    m.paint_front(x, z, STAR)
  # 圍巾：脖子一圈薄荷／奶油條紋＋垂下的一角
  for x in range(N):
    for y in range(N):
      a = math.atan2(y - C, x - C)
      d = math.hypot(x - C, y - C)
      for z in range(21, 25):
        if 4.5 <= d <= 9.2 - abs(z - 23) * 0.6:
          m.put(x, y, z, MINT if int((a + math.pi) / (2 * math.pi) * 14) % 2 else CREAM)
  for z in range(13, 22):
    for x in range(int(C - 7), int(C - 3)):
      for y in (int(C + 7), int(C + 8)):
        m.put(x, y, z, MINT if (z // 2) % 2 else CREAM)
  # 短短的手（依姿勢）
  sh = lambda sx: (C + sx * 7.2, C + 1, 19)

  def arm(sx, hand, pad=False):
    m.capsule(sh(sx), hand, 2.6, LAV)
    m.ellipsoid(*hand, 3.0, 3.0, 3.0, LAV)
    if pad:
      hx, hy, hz = hand
      m.ellipsoid(hx, hy + 2.2, hz, 1.6, 1.0, 1.6, PINK)

  def wand(a, b, top):
    """魔杖（≥2 格粗，轉動時不閃爍）＋杖頭星星。"""
    m.capsule(a, b, 1.2, STICK, keep=True)
    tx, ty, tz = top
    for x, z in star_pts(round(tx), round(tz), 3):
      for y in (int(ty) - 1, int(ty)):
        m.put(x, y, z, STAR)

  if pose == "wave":
    arm(-1, (C - 15, C + 1, 30), pad=True)
    arm(1, (C + 12, C + 4, 16))
    wand((C + 12, C + 5, 11), (C + 17, C + 5, 33), (C + 17, C + 5.5, 35))
  elif pose in ("wait", "sleep"):
    # 雙手握魔杖擋在胸前，杖頭星星在右臉旁
    arm(-1, (C + 0.5, C + 9, 15))
    arm(1, (C + 4.5, C + 9, 18.5))
    wand((C - 2, C + 10, 9), (C + 12, C + 10, 33), (C + 12.5, C + 10.5, 35))
  else:  # joy：雙手舉高、張嘴笑，右手魔杖往右上
    arm(-1, (C - 16, C + 1, 36), pad=True)
    arm(1, (C + 15.5, C + 1, 35))
    wand((C + 15.5, C + 2, 31), (C + 19, C + 2, 47), (C + 19, C + 2.5, 49))
  return m


def stage_box(models, angles=(-24, 0, 24)):
  """舞台高度與層底位置（以層寬 W 為單位），照 style.css 疊法算出所有姿勢、擺動角度的外框：
  回傳 (mh, mb)：舞台高 = mh·W、.stack-layer bottom = -mb·W，最低點貼舞台底、最高點貼舞台頂。"""
  lo, hi = 1e9, -1e9
  for m in models:
    for (x, y, z) in m.v:
      for a in angles:
        t = math.radians(a)
        # 切片繞中心轉（CSS rotate 順時針為正、圖座標 y 向下）
        yr = (x - C) * math.sin(t) + (y - C) * math.cos(t) + C
        base = (1 + SQUASH) / 2 - SQUASH * (yr + 0.5) / N
        lo = min(lo, base - SQUASH * 0.5 / N + z * LIFT / N)
        hi = max(hi, base + SQUASH * 0.5 / N + (z + 0.5) * LIFT / N)
  return round(hi - lo + 0.02, 3), round(lo - 0.01, 3)


def render(m):
  """每層一張 RGBA（N×N）。越低越暗、輪廓與背面略暗，產生立體感。"""
  top = max(z for (_, _, z) in m.v)
  out = []
  for z in range(top + 1):
    im = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    f = 0.9 + 0.1 * z / top
    for (x, y, zz), rgb in m.v.items():
      if zz != z:
        continue
      edge = any((x + dx, y + dy, z) not in m.v for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
      k = f * (0.9 if edge and rgb in (LAV, LAV_D) else 1.0)
      im.putpixel((x, y), tuple(min(255, int(c * k)) for c in rgb) + (255,))
    out.append(im)
  return out


def save(name, layers):
  """合成一張圖集：COLS 欄、由左而右由上而下排（第 0 格＝最底層），回傳內容 hash（快取鍵）。"""
  rows = -(-len(layers) // COLS)
  cell = N * SHEET_SCALE
  sheet = Image.new("RGBA", (COLS * cell, rows * cell), (0, 0, 0, 0))
  for z, im in enumerate(layers):
    sheet.paste(im.resize((cell, cell), Image.NEAREST), ((z % COLS) * cell, (z // COLS) * cell))
  # 檔名帶內容 hash（<pose>-<hash>.webp）：舊版程式只會 404（吉祥物空白），不會拿到新圖集配舊格數而錯位
  os.makedirs(ROOT, exist_ok=True)
  tmp = os.path.join(ROOT, f".{name}.tmp.webp")
  sheet.save(tmp, lossless=True)
  with open(tmp, "rb") as fh:
    digest = hashlib.sha256(fh.read()).hexdigest()[:12]
  path = os.path.join(ROOT, f"{name}-{digest}.webp")
  os.replace(tmp, path)
  print(name, "layers", len(layers), "sheet", sheet.size, os.path.getsize(path), "bytes")
  return digest


def preview(layers, angle, width=240):
  """與 style.css 同款疊法：每半層位移 0.85 格、壓扁 SQUASH、繞中心自轉。"""
  step = width * 0.85 / N
  h = int(width * SQUASH + step * len(layers) + 8)
  canvas = Image.new("RGBA", (width, h), (255, 255, 255, 255))
  for k in range(len(layers) * 2):
    pos = k // 2 + (0.5 if k % 2 else 0)
    im = layers[k // 2].resize((N * SCALE, N * SCALE), Image.NEAREST).rotate(angle, Image.NEAREST)
    im = im.resize((width, int(width * SQUASH)), Image.NEAREST)
    canvas.alpha_composite(im, (0, int(h - width * SQUASH - pos * step)))
  return canvas


def write_rust(counts, box):
  rows = "\n".join(f'    ("{p}", "{ph}", {n}, "{h}"),' for p, ph, n, h in counts)
  src = f"""//! 吉祥物月亮貓 sprite stack 的圖集常數——由 `scripts/sprite-stack/mascot.py` 產生，**勿手改**。
//!
//! 每個（姿勢, 月相）一張圖集 `crates/web/assets/mascot/stack/<pose>-<phase>-<hash>.webp`：{COLS} 欄、由左而右由上而下，第 0 格＝最底層。

/// 切片的平面格數（寬＝高）。
pub const MASCOT_STACK_GRID: usize = {N};

/// 圖集欄數。
pub const MASCOT_SHEET_COLS: usize = {COLS};

/// 舞台高（層寬的倍數，CSS `--mh`）與 `.stack-layer` 底部位移（`bottom: -mb × 層寬`，CSS `--mb`）：
/// 由 `stage_box` 依所有姿勢、±24° 擺動的外框算出，最低點貼舞台底、最高點貼舞台頂。
pub const MASCOT_STAGE_H: f32 = {box[0]};
pub const MASCOT_STAGE_B: f32 = {box[1]};

/// (姿勢, 月相 wire 值, 層數, 圖集內容 hash——在檔名裡，舊程式拿不到新圖集、zone 4h 快取也不會配到舊圖)。
pub const MASCOT_STACK_LAYERS: [(&str, &str, usize, &str); {len(counts)}] = [
{rows}
];
"""
  out = subprocess.run(["rustfmt", "--edition", "2021", "--emit", "stdout"], input=src.encode(),
                       capture_output=True, check=True).stdout.decode()
  with open(RUST_OUT, "w") as fh:
    fh.write(out)
  print("wrote", os.path.normpath(RUST_OUT))


if __name__ == "__main__":
  if len(sys.argv) == 3 and sys.argv[1] == "--preview":
    os.makedirs(sys.argv[2], exist_ok=True)
    for p, ph in COMBOS:
      layers = render(build(p, ph))
      row = [preview(layers, a) for a in (0, -25, 25, 90)]
      sheet = Image.new("RGBA", (sum(i.width for i in row), max(i.height for i in row)), (255, 255, 255, 255))
      x = 0
      for i in row:
        sheet.alpha_composite(i, (x, sheet.height - i.height)); x += i.width
      sheet.save(os.path.join(sys.argv[2], f"{p}-{ph}.png"))
    print("preview →", sys.argv[2])
  else:
    counts, models = [], []
    os.makedirs(ROOT, exist_ok=True)
    for f in os.listdir(ROOT):
      if f.endswith(".webp"):
        os.remove(os.path.join(ROOT, f))
    for p, ph in COMBOS:
      m = build(p, ph)
      models.append(m)
      layers = render(m)
      counts.append((p, ph, len(layers), save(f"{p}-{ph}", layers)))
    box = stage_box(models)
    print("stage", box)
    write_rust(counts, box)
