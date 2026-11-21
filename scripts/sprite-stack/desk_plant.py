"""開運小物「桌面小盆栽」sprite stacking 切片產生器（2026-11-22）。

程式化體素模型（盆＋三圈多肉葉片）→ 由下而上每層一張透明 webp，
輸出 crates/web/assets/shop/stack/desk-plant/NN.webp；/shop 卡片把各層疊起來旋轉。
改模型後重跑：python3 scripts/sprite-stack/desk_plant.py；層數變了要同步
crates/web/src/pages/shop.rs 的 STACKS。顏色是點陣圖內容，不受 :root token 規則管轄。"""
import math, os, sys
from PIL import Image
N = 48            # 平面格數
H = 40            # 層數
SCALE = 8         # 每格輸出像素
C = (N - 1) / 2
vox = {}          # (x,y,z) -> rgb
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "crates", "web", "assets", "shop", "stack", "desk-plant")
os.makedirs(OUT, exist_ok=True)

def put(x, y, z, rgb):
  if 0 <= x < N and 0 <= y < N and 0 <= z < H: vox[(x, y, z)] = rgb

CREAM = (248, 234, 222); RIM = (236, 214, 198)
DOTS = [(244, 170, 190), (176, 214, 196), (196, 178, 236), (250, 214, 150)]
SOIL = (150, 110, 96)
LEAF_IN = (170, 204, 176); LEAF_OUT = (140, 182, 156); TIP = (236, 160, 178)

# 盆：下窄上寬的圓盆，頂部加一圈盆緣
for z in range(0, 13):
  r = 9.5 + z * 0.35
  for x in range(N):
    for y in range(N):
      d = math.hypot(x - C, y - C)
      if d <= r:
        shell = d > r - 1.6
        if z == 12 or shell or z == 0:
          col = RIM if z >= 11 else CREAM
          # 點點花紋：依角度與高度散佈
          a = math.atan2(y - C, x - C)
          k = int((a + math.pi) / (2 * math.pi) * 14)
          if shell and 2 <= z <= 9 and (k + z // 3) % 2 == 0 and z % 3 == 1:
            col = DOTS[(k + z) % 4]
          put(x, y, z, col)
        elif z == 11:
          put(x, y, z, SOIL)

# 多肉蓮座：三圈葉片，每片是傾斜的扁橢球
def leaf(angle, tilt_r, base_z, length, width, lift, tip=False):
  ca, sa = math.cos(angle), math.sin(angle)
  for x in range(N):
    for y in range(N):
      for z in range(base_z, H):
        dx, dy, dz = x - C, y - C, z - base_z
        u = dx * ca + dy * sa            # 沿葉片
        v = -dx * sa + dy * ca           # 側向
        w = dz - u * lift                # 葉面往上翹
        if u < 0: continue
        if (u / length) ** 2 + (v / width) ** 2 + (w / 2.2) ** 2 <= 1:
          t = u / length
          col = TIP if (t > 0.9 and tip) else (LEAF_IN if t < 0.5 else LEAF_OUT)
          put(x, y, z, col)

rings = [(10, 17.0, 5.0, 12, 0.22, 0.0, True), (8, 12.5, 4.4, 14, 0.5, 0.33, True), (6, 8.0, 3.4, 16, 0.9, 0.7, False)]
for n, length, width, bz, lift, off, tip in rings:
  for i in range(n):
    leaf(2 * math.pi * i / n + off, 0, bz, length, width, lift, tip)
# 中心芽
for x in range(N):
  for y in range(N):
    for z in range(17, 24):
      if math.hypot(x - C, y - C) <= 2.2 - (z - 17) * 0.25: put(x, y, z, LEAF_IN)

def shade(rgb, f): return tuple(max(0, min(255, int(c * f))) for c in rgb)
top = max(z for (_, _, z) in vox)
for z in range(top + 1):
  im = Image.new("RGBA", (N, N), (0, 0, 0, 0))
  f = 0.90 + 0.10 * z / top                 # 越低越暗，產生立體感
  for (x, y, zz), rgb in vox.items():
    if zz != z: continue
    edge = any((x + dx, y + dy, z) not in vox for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
    leafy = rgb in (LEAF_IN, LEAF_OUT, TIP)
    im.putpixel((x, y), shade(rgb, f * (0.88 if (edge and leafy) else 1.0)) + (255,))
  im.resize((N * SCALE, N * SCALE), Image.NEAREST).save(os.path.join(OUT, f"{z:02d}.webp"), lossless=True)
print("layers", top + 1, "voxels", len(vox))
