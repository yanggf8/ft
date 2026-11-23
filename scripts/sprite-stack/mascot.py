"""吉祥物月亮貓 sprite stacking 切片產生器（2026-11-23）。

程式化體素月亮貓（長靴、肚肚、圍巾、月牙、尾巴）× 三個姿勢：
  wave（首頁打招呼，拿星星魔杖）／wait（塔羅翻牌前，手放胸前期待）／joy（翻牌後，雙手舉高、笑瞇眼）。
輸出 crates/web/assets/mascot/stack/<pose>/NN.webp（由下而上），層數與格數寫成產生檔
crates/web/src/components/mascot_stacks.rs（勿手改，components/mascot.rs 讀它）。
正面＝切片的 +y（圖下緣），元件以左右擺動代替整圈自轉，臉一直朝前。
改模型後重跑：python3 scripts/sprite-stack/mascot.py；
預覽合成圖（與 CSS 同款疊法）：python3 scripts/sprite-stack/mascot.py --preview <輸出目錄>。
顏色是點陣圖內容，不受 :root token 規則管轄。"""
import math
import os
import subprocess
import sys
from PIL import Image

N = 32        # 平面格數
SCALE = 8     # 每格輸出像素
C = (N - 1) / 2
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "crates", "web", "assets", "mascot", "stack")
RUST_OUT = os.path.join(HERE, "..", "..", "crates", "web", "src", "components", "mascot_stacks.rs")
POSES = ("wave", "wait", "joy")

LAV = (204, 188, 244); LAV_D = (180, 162, 228)
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


def build(pose):
  m = Model()
  # 靴子＋腿
  for sx in (-1, 1):
    m.capsule((C + sx * 3, C + 0.5, 1.2), (C + sx * 3, C + 0.5, 3), 2.6, CREAM)
    m.capsule((C + sx * 3, C, 4), (C + sx * 2.6, C, 8), 2.3, LAV)
  # 身體＋肚肚
  m.ellipsoid(C, C, 13, 6.2, 5.2, 7, LAV)
  for x in range(N):
    for z in range(8, 18):
      if ((x - C) / 3.6) ** 2 + ((z - 12.5) / 4.6) ** 2 <= 1:
        m.paint_front(x, z, PINK, 2)
  # 尾巴：從背後彎上來，末端薄荷色條紋
  pts = [(C + 2 + 3.5 * math.sin(t * 1.6), C - 4.5 - 4.5 * t, 7 + 11 * t) for t in [i / 14 for i in range(15)]]
  for i, (x, y, z) in enumerate(pts):
    m.ellipsoid(x, y, z, 1.7, 1.7, 1.7, MINT if i >= 10 and i % 2 == 0 else LAV)
  # 頭
  HZ = 30
  m.ellipsoid(C, C, HZ, 9.2, 7.6, 8.4, LAV)
  # 耳朵（內側粉紅）
  for sx in (-1, 1):
    for k in range(7):
      r = 2.8 * (1 - k / 7) + 0.4
      m.ellipsoid(C + sx * (5.6 + k * 0.25), C, 36 + k, r, r * 0.8, 0.8, LAV)
    for k in range(4):
      r = 1.4 * (1 - k / 5)
      m.ellipsoid(C + sx * (5.7 + k * 0.25), C + 1.4, 37 + k, r, 0.6, 0.6, PINK)
  # 月牙：頭後方立起的粉紅新月（x–z 平面，y 在頭後）
  for x in range(N):
    for z in range(HZ, 50):
      outer = (x - (C - 1)) ** 2 + (z - 37) ** 2 <= 10.5 ** 2
      inner = (x - (C + 3.5)) ** 2 + (z - 40) ** 2 <= 9.5 ** 2
      if outer and not inner:
        for y in range(int(C - 6), int(C - 3)):
          m.put(x, y, z, MOON, keep=True)
  # 臉：眼睛、亮點、腮紅、嘴、額頭星
  EZ = HZ - 0.5
  for sx in (-1, 1):
    ex = C + sx * 3.7
    for x in range(N):
      for z in range(HZ - 6, HZ + 5):
        d = ((x - ex) / 2.4) ** 2 + ((z - EZ) / 2.6) ** 2
        if pose == "joy":
          # 笑瞇眼：倒 U 弧線
          dd = math.hypot(x - ex, (z - (EZ - 1.2)) * 1.1)
          if 1.6 <= dd <= 2.6 and z >= EZ - 1.2:
            m.paint_front(x, z, INK)
        elif d <= 1:
          m.paint_front(x, z, INK if d > 0.55 else TEAL)
    if pose != "joy":
      m.paint_front(round(ex - 0.6), round(EZ + 0.6), WHITE)
      m.paint_front(round(ex + 0.7), round(EZ - 0.9), WHITE)
    for x in range(N):
      for z in range(HZ - 6, HZ):
        if ((x - (C + sx * 6.4)) / 1.7) ** 2 + ((z - (HZ - 3.4)) / 1.0) ** 2 <= 1:
          m.paint_front(x, z, BLUSH)
  for x, z in ((C - 0.5, HZ - 3), (C + 0.5, HZ - 3), (C - 1.5, HZ - 2.4), (C + 1.5, HZ - 2.4)):
    m.paint_front(round(x), round(z), INK)
  for x, z in star_pts(round(C), HZ + 5, 1):
    m.paint_front(x, z, STAR)
  # 圍巾：脖子一圈薄荷／奶油條紋＋垂下的一角
  for x in range(N):
    for y in range(N):
      a = math.atan2(y - C, x - C)
      d = math.hypot(x - C, y - C)
      for z in range(19, 22):
        if 3.2 <= d <= 6.4 - abs(z - 20.5) * 0.5:
          m.put(x, y, z, MINT if int((a + math.pi) / (2 * math.pi) * 12) % 2 else CREAM)
  for z in range(13, 20):
    for x in range(int(C - 5), int(C - 2)):
      m.put(x, int(C + 5), z, MINT if (z // 2) % 2 else CREAM)
      m.put(x, int(C + 4), z, MINT if (z // 2) % 2 else CREAM)
  # 手臂（依姿勢）
  sh = lambda sx: (C + sx * 5.2, C + 0.5, 17.5)

  def arm(sx, hand, pad=False):
    m.capsule(sh(sx), hand, 1.9, LAV)
    m.ellipsoid(*hand, 2.2, 2.2, 2.2, LAV)
    if pad:
      hx, hy, hz = hand
      m.ellipsoid(hx, hy + 1.6, hz, 1.2, 0.8, 1.2, PINK)

  if pose == "wave":
    arm(-1, (C - 11, C + 1, 27), pad=True)
    hand = (C + 9, C + 3, 15)
    arm(1, hand)
    m.capsule((C + 9, C + 3.5, 12), (C + 12, C + 3.5, 27), 1.1, STICK, keep=True)
    for x, z in star_pts(round(C + 12), 28, 2):
      for y in (int(C + 3), int(C + 4)):
        m.put(x, y, z, STAR)
  elif pose == "wait":
    arm(-1, (C - 2, C + 6, 17))
    arm(1, (C + 2, C + 6, 17))
  else:  # joy
    arm(-1, (C - 11.5, C + 1, 33), pad=True)
    arm(1, (C + 11.5, C + 1, 33), pad=True)
  return m


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


def save(pose, layers):
  out = os.path.join(ROOT, pose)
  os.makedirs(out, exist_ok=True)
  for f in os.listdir(out):
    if f.endswith(".webp"):
      os.remove(os.path.join(out, f))
  for z, im in enumerate(layers):
    im.resize((N * SCALE, N * SCALE), Image.NEAREST).save(os.path.join(out, f"{z:02d}.webp"), lossless=True)
  print(pose, "layers", len(layers))


SQUASH = 0.5   # 與 style.css .mascot-stack 的 --squash 一致（低俯角，臉看得到）


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


def write_rust(counts):
  rows = "\n".join(f'    ("{p}", {n}),' for p, n in counts)
  src = f"""//! 吉祥物月亮貓 sprite stack 的切片層數——由 `scripts/sprite-stack/mascot.py` 產生，**勿手改**。
//!
//! 切片在 `crates/web/assets/mascot/stack/<pose>/NN.webp`。

/// 切片的平面格數（寬＝高）。
pub const MASCOT_STACK_GRID: usize = {N};

/// 各姿勢的層數。
pub const MASCOT_STACK_LAYERS: [(&str, usize); {len(counts)}] = [
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
    for p in POSES:
      layers = render(build(p))
      row = [preview(layers, a) for a in (0, -25, 25, 90)]
      sheet = Image.new("RGBA", (sum(i.width for i in row), max(i.height for i in row)), (255, 255, 255, 255))
      x = 0
      for i in row:
        sheet.alpha_composite(i, (x, sheet.height - i.height)); x += i.width
      sheet.save(os.path.join(sys.argv[2], f"{p}.png"))
    print("preview →", sys.argv[2])
  else:
    counts = []
    for p in POSES:
      layers = render(build(p))
      save(p, layers)
      counts.append((p, len(layers)))
    write_rust(counts)
