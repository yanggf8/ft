"""空間風水結果頁「你的房間」縮景零件產生器（2026-11-22，spec
docs/superpowers/specs/2026-11-22-fengshui-diorama-design.md，§7 優先）。

每個可畫的答案選項＝一個零件（程式化體素盒子，同 rooms.py 風格），三房型共用格數 N=40、
同一套層高；前端（components/sprite_stack.rs SpriteStackParts）把選中的零件按層疊在同一格裡。

輸出：
  crates/web/assets/stack/fs/<room>/<part>/NN.webp   只輸出有像素的層，NN＝全域層號
  crates/schema/src/fengshui/diorama_parts.rs        DIORAMA_GRID／DioramaPart／DIORAMA_PARTS（產生檔，勿手改）

用法：
  python3 scripts/sprite-stack/fengshui_parts.py           重新產生切片與 Rust 常數
  python3 scripts/sprite-stack/fengshui_parts.py --check   不寫檔，檢查：檔案 ⇄ 常數一致（含像素）、
      不同題目且可同時選中的零件兩兩無共用體素、每個零件都接地（不懸空）、沒有零件超出格子。
  改零件、改 ruleset 選項後必跑一次產生＋--check（CI 沒有 PIL，--check 是手動步驟）；
  改零件或對照表要 bump crates/schema/src/fengshui/diorama.rs 的 DIORAMA_VERSION。

明暗：每層亮度 f = 0.86 + 0.14 * z / TOP（TOP 全域固定，三房型一致）；邊緣加深只看同零件內鄰格，
零件交界的細線是預期內的。顏色是點陣圖內容，不受 :root token 規則管轄；只用中性粉彩，
樑、柱角、雜物不用深色或紅色（§7.10：縮景不暗示好壞）。只畫使用者描述的東西，不畫任何風水物件。

座標：x 0→40 左到右、y 0→40 後到前（y 小＝後）、z 0 地板、1 起是地板上的第一層。
box(x0, y0, z0, x1, y1, z1) 含頭不含尾。牆高 z1–10、牆頂 z10；門框到 z15；樑在 z15–17。

零件位置表（同題互斥選項可共用位置；其餘兩兩不重疊，由 --check 把關）：

臥室 bedroom（床頭靠後牆 y=0，人躺著面向 y 大）
  base            地板全幅；走道帶 x<4、y≥34；後牆 x4–8、x28–40；左牆 x4–6 y18–34
  bed             床 x11–25 y2–22（床頭板 y2–4）＋床頭櫃 x29–34 y2–7
  bed-floating    同上整組往前移到 y7–27（bed_back=floating，取代 bed）
  bed-back-wall／-window／-door   床頭後牆段 x8–28 y0–2（門框到 z15）
  door-diag       前牆段 x26–40 y32–34，門 x29–37      （bed_door_view=visible）
  door-foot       前牆段 x9–26  y32–34，門 x13–22      （facing）
  door-behind     左牆段 x4–6   y2–18，門 y6–14         （hidden）
  door-opposite-diag／-foot／-behind  房門對面另一扇門：y37–39 對齊前兩者；左側 x0–2 y2–18
  beam            柱 x6–8、x33–35，y12–15；樑 x6–35 z15–17
  shelf-over      吊櫃 x8–28 y2–6 z12–15，側柱 x8–10、x26–28 y2–5（bed_overhead=heavy）
  mirror          立鏡 x7–9 y19–27
  corner          方柱 5×5 中心 (31,18)
  window-glow     右側矮牆＋窗 x38–40 y10–26；地面淡光 x35–38 y11–25
  clutter-some    A x26–30 y23–27、B x14–18 y28–32
  clutter-lots    A、B ＋ C x7–11 y28–32、D x31–35 y25–29、E x20–24 y28–31

書房 study（人面向 y 小；椅背後＝y 大）
  base            地板全幅；左牆 x0–2 y2–26
  desk            桌 x12–28 y14–21（桌面 z8）＋筆電、書
  chair           椅 x17–23 y23–30
  back-wall／-window／-door   椅後牆段 x4–40 y36–38
  door-diag       前右牆段 x28–40 y2–4，門 x30–38      （seat_door_view=visible）
  door-front      前牆段 x10–30 y2–4，門 x16–24        （facing）
  door-behind     左牆段 x0–2 y26–38，門 y28–36         （hidden）
  beam            柱 x4–6、x34–36，y22–25；樑 z15–17
  front-open      桌前淺色地墊 x12–28 y6–12（只 z1）
  front-wall      桌前高牆 x8–32 y11–13
  corner          方柱 5×5 中心 (33,17)
  sun-patch       桌面光斑 x22–27 y15–19（z9，靠在桌上）＋地面光斑 x5–11 y13–21
  clutter-some    A x24–28 y24–28、B x8–12 y26–30
  clutter-lots    A、B ＋ C x29–33 y27–31、D x12–16 y31–35、E x25–29 y31–35

辦公室座位 office（無牆，格紋地毯；人面向 y 小）
  base            格紋地毯全幅
  desk            桌 x10–30 y14–20（桌面 z8）＋螢幕、鍵盤
  chair           椅 x17–23 y22–28
  back-wall       高櫃 x8–32 y34–38
  back-aisle      走道線 x0–40 y31–33 ＋路人 x30–33 y34–37
  back-low        矮隔板 x6–34 y31–33 ＋後方空桌 x12–28 y34–39
  door-front      出入口牆段 x8–32 y1–3，門 x15–25    （office_door=entrance）
  boss-desk       主管大桌 x8–32 y3–8 ＋主管椅 x17–23 y0–3（boss）
  front-open      桌前淺色地墊 x12–28 y9–13（只 z1）
  front-partition 緊貼高隔板 x8–32 y11–13
  beam            柱 x3–5、x35–37，y23–26；樑 z15–17
  lamp-tube       同柱位＋頂架 z17，燈管 x8–32 y24–25 z15
  corner          方柱 5×5 中心 (34,17)
  window-side     左側矮牆＋窗 x0–2 y6–28
  clutter-some    A x24–28 y22–26、B x6–10 y22–26
  clutter-lots    A、B ＋ C x30–34 y27–31、D x10–14 y27–31、E x5–9 y14–18
"""
import argparse
import io
import os
import subprocess
import sys
from collections import deque

from PIL import Image

N = 40       # 平面格數（三房型共用）
SCALE = 8    # 每格輸出像素
TOP = 20     # 全域明暗基準層（§7.6）
HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.normpath(os.path.join(HERE, "..", ".."))
ASSETS = os.path.join(REPO, "crates", "web", "assets", "stack", "fs")
RUST_OUT = os.path.join(REPO, "crates", "schema", "src", "fengshui", "diorama_parts.rs")

# ── 調色盤（中性粉彩）──
FLOOR = (238, 222, 204); FLOOR2 = (230, 212, 192)
HALL = (232, 230, 240); HALL2 = (224, 222, 234)
WALL = (232, 226, 246); WALL_TOP = (214, 204, 238)
WOOD = (214, 176, 140); WOOD_D = (196, 158, 124); WOOD_L = (230, 204, 172)
DOOR = (226, 196, 168); KNOB = (250, 222, 150)
WHITE = (250, 248, 252); LAV = (204, 188, 240); PINK = (244, 190, 206); MINT = (176, 220, 200)
YELLOW = (250, 222, 150); BLUE = (190, 212, 240); PANE = (206, 226, 246); SCREEN = (168, 192, 226)
GREY = (200, 196, 216); KEY = (220, 218, 230); SKIN = (246, 216, 196)
BEAM = (226, 214, 234); BEAM_D = (212, 198, 224)
PILLAR = (240, 234, 246); PILLAR_E = (218, 208, 236)
LIGHT = (246, 242, 228); SUN = (252, 238, 196)
MAT = (220, 236, 226)
KRAFT = (236, 214, 186)
BOXES = [PINK, MINT, BLUE, KRAFT, LAV, YELLOW]
CARPET = (226, 224, 236); CARPET2 = (234, 232, 242)
CABINET = (222, 214, 238); CABINET_D = (206, 196, 228)
PART_BLUE = (196, 214, 238); PART_BLUE_T = (176, 198, 232)
LINE = (252, 236, 178)

WALL_Z = 10   # 牆身 z1..WALL_Z，牆頂一層在 WALL_Z
DOOR_Z = 15   # 門框頂


class Part:
  def __init__(self, room, pid, group, supports=()):
    self.room = room
    self.id = pid
    self.group = group          # 題目 id；固定零件為 "fixed"
    self.supports = supports    # 允許靠在其上的其他零件 id（接地檢查用）
    self.v = {}

  def box(self, x0, y0, z0, x1, y1, z1, rgb):
    for x in range(x0, x1):
      for y in range(y0, y1):
        for z in range(z0, z1):
          self.v[(x, y, z)] = rgb

  def pillar(self, cx, cy, z0, z1):
    """5×5 方柱（中心 cx, cy）：柱身淺奶白、底座與柱頂一圈淡紫。轉動時方角輪流朝向床或座位。"""
    self.box(cx - 2, cy - 2, z0, cx + 3, cy + 3, z1, PILLAR)
    self.box(cx - 2, cy - 2, z0, cx + 3, cy + 3, z0 + 1, PILLAR_E)
    self.box(cx - 2, cy - 2, z1 - 1, cx + 3, cy + 3, z1, PILLAR_E)

  # ── 牆段（含門／窗）──
  def wall_x(self, x0, x1, y0, y1, cap=True):
    """沿 x 方向的牆段。"""
    self.box(x0, y0, 1, x1, y1, WALL_Z, WALL)
    if cap:
      self.box(x0, y0, WALL_Z, x1, y1, WALL_Z + 1, WALL_TOP)

  def wall_y(self, x0, x1, y0, y1, cap=True):
    self.box(x0, y0, 1, x1, y1, WALL_Z, WALL)
    if cap:
      self.box(x0, y0, WALL_Z, x1, y1, WALL_Z + 1, WALL_TOP)

  def carve(self, pred):
    for k in [k for k in self.v if pred(*k)]:
      del self.v[k]

  def window_x(self, x0, x1, y0, y1, wx0, wx1):
    """x 向牆段開窗：窗台 z4、玻璃 z4–8、窗框白。"""
    self.wall_x(x0, x1, y0, y1)
    self.carve(lambda x, y, z: wx0 <= x < wx1 and y0 <= y < y1 and 4 <= z < 9)
    self.box(wx0, y0, 4, wx1, y1, 9, WHITE)
    self.box(wx0 + 1, y0, 5, wx1 - 1, y1, 8, PANE)
    mid = (wx0 + wx1) // 2
    self.box(mid, y0, 5, mid + 1, y1, 8, WHITE)

  def window_y(self, x0, x1, y0, y1, wy0, wy1):
    self.wall_y(x0, x1, y0, y1)
    self.carve(lambda x, y, z: wy0 <= y < wy1 and x0 <= x < x1 and 4 <= z < 9)
    self.box(x0, wy0, 4, x1, wy1, 9, WHITE)
    self.box(x0, wy0 + 1, 5, x1, wy1 - 1, 8, PANE)
    mid = (wy0 + wy1) // 2
    self.box(x0, mid, 5, x1, mid + 1, 8, WHITE)

  def door_x(self, x0, x1, y0, y1, dx0, dx1, wall=True):
    """x 向牆段＋門：白門框到 DOOR_Z、淺木門板、門把。"""
    if wall:
      self.wall_x(x0, x1, y0, y1)
    self.carve(lambda x, y, z: dx0 <= x < dx1 and y0 <= y < y1)
    self.box(dx0, y0, 1, dx0 + 1, y1, DOOR_Z, WHITE)
    self.box(dx1 - 1, y0, 1, dx1, y1, DOOR_Z, WHITE)
    self.box(dx0, y0, DOOR_Z, dx1, y1, DOOR_Z + 1, WHITE)
    self.box(dx0 + 1, y0, 1, dx1 - 1, y1, DOOR_Z, DOOR)
    self.box(dx0 + 2, y0, 11, dx1 - 2, y1, 13, WOOD_L)
    self.box(dx1 - 3, y0, 7, dx1 - 2, y1, 8, KNOB)

  def door_y(self, x0, x1, y0, y1, dy0, dy1, wall=True):
    if wall:
      self.wall_y(x0, x1, y0, y1)
    self.carve(lambda x, y, z: dy0 <= y < dy1 and x0 <= x < x1)
    self.box(x0, dy0, 1, x1, dy0 + 1, DOOR_Z, WHITE)
    self.box(x0, dy1 - 1, 1, x1, dy1, DOOR_Z, WHITE)
    self.box(x0, dy0, DOOR_Z, x1, dy1, DOOR_Z + 1, WHITE)
    self.box(x0, dy0 + 1, 1, x1, dy1 - 1, DOOR_Z, DOOR)
    self.box(x0, dy0 + 2, 11, x1, dy1 - 2, 13, WOOD_L)
    self.box(x0, dy1 - 3, 7, x1, dy1 - 2, 8, KNOB)

  def gantry(self, xa, xb, y0, y1, top_rgb, deep=True):
    """兩根柱＋頂上橫樑（樑、燈架共用）：柱 x[xa,xa+2)、x[xb,xb+2)。"""
    for x in (xa, xb):
      self.box(x, y0, 1, x + 2, y1, 15 if deep else 16, BEAM_D)
    if deep:
      self.box(xa, y0, 15, xb + 2, y1, 17, top_rgb)
      self.box(xa, y0, 15, xb + 2, y0 + 1, 16, BEAM_D)
    else:
      self.box(xa, y0, 16, xb + 2, y1, 17, top_rgb)

  def piles(self, spots):
    """雜物：每個點一疊兩三個淺色紙箱。"""
    for i, (x0, y0, x1, y1) in enumerate(spots):
      a, b = BOXES[i % len(BOXES)], BOXES[(i + 2) % len(BOXES)]
      h1 = 3 + i % 2
      self.box(x0, y0, 1, x1, y1, 1 + h1, a)
      self.box(x0, y0 + 1, 1 + h1 - 1, x1, y0 + 2, 1 + h1, (a[0] - 14, a[1] - 14, a[2] - 14))  # 封箱帶
      self.box(x0 + 1, y0, 1 + h1, x1 - 1, y1 - 1, 3 + h1, b)
      if i % 3 == 0:
        self.box(x0 + 1, y0 + 1, 3 + h1, x1 - 2, y1 - 2, 4 + h1, WHITE)


def floor_wood(p):
  for x in range(N):
    for y in range(N):
      hall = x < 4 or y >= 34
      if hall:
        p.v[(x, y, 0)] = HALL if (x + y) % 2 else HALL2
      else:
        p.v[(x, y, 0)] = FLOOR if (x // 3 + y) % 2 else FLOOR2


def bed_set(p, dy):
  """床＋床頭櫃；dy＝整組往前位移。"""
  p.box(11, 2 + dy, 1, 25, 22 + dy, 3, WOOD)
  p.box(11, 2 + dy, 3, 25, 4 + dy, 9, WOOD_D)          # 床頭板
  p.box(12, 3 + dy, 9, 24, 4 + dy, 10, WOOD)           # 床頭板頂飾
  p.box(12, 4 + dy, 3, 24, 21 + dy, 5, WHITE)          # 床墊
  p.box(13, 5 + dy, 5, 23, 9 + dy, 6, PINK)            # 枕頭
  p.box(12, 11 + dy, 5, 24, 21 + dy, 6, LAV)           # 被子
  p.box(12, 11 + dy, 6, 24, 12 + dy, 7, LAV)
  p.box(29, 2 + dy, 1, 34, 7 + dy, 6, WOOD)            # 床頭櫃
  p.box(30, 3 + dy, 6, 33, 6 + dy, 7, WOOD_L)
  p.box(31, 4 + dy, 7, 32, 5 + dy, 9, GREY)            # 小燈
  p.box(30, 3 + dy, 9, 33, 6 + dy, 11, YELLOW)


def bedroom():
  R = "bedroom"
  P = []
  base = Part(R, "base", "fixed"); floor_wood(base)
  base.wall_x(4, 8, 0, 2); base.wall_x(28, 40, 0, 2)
  base.wall_y(4, 6, 18, 34)
  P.append(base)

  bed = Part(R, "bed", "fixed"); bed_set(bed, 0); P.append(bed)

  w = Part(R, "bed-back-wall", "bed_back"); w.wall_x(8, 28, 0, 2); P.append(w)
  w = Part(R, "bed-back-window", "bed_back"); w.window_x(8, 28, 0, 2, 11, 25); P.append(w)
  w = Part(R, "bed-back-door", "bed_back"); w.door_x(8, 28, 0, 2, 13, 22); P.append(w)
  w = Part(R, "bed-floating", "bed_back"); bed_set(w, 5); P.append(w)

  d = Part(R, "door-diag", "bed_door_view"); d.door_x(26, 40, 32, 34, 29, 37); P.append(d)
  d = Part(R, "door-foot", "bed_door_view"); d.door_x(9, 26, 32, 34, 13, 22); P.append(d)
  d = Part(R, "door-behind", "bed_door_view"); d.door_y(4, 6, 2, 18, 6, 14); P.append(d)

  d = Part(R, "door-opposite-diag", "bed_door_door"); d.door_x(27, 39, 37, 39, 29, 37); P.append(d)
  d = Part(R, "door-opposite-foot", "bed_door_door"); d.door_x(10, 25, 37, 39, 13, 22); P.append(d)
  d = Part(R, "door-opposite-behind", "bed_door_door"); d.door_y(0, 2, 2, 18, 6, 14); P.append(d)

  b = Part(R, "beam", "bed_overhead"); b.gantry(6, 33, 12, 15, BEAM); P.append(b)
  s = Part(R, "shelf-over", "bed_overhead")
  s.box(8, 2, 1, 10, 5, 12, WOOD_D); s.box(26, 2, 1, 28, 5, 12, WOOD_D)
  s.box(8, 2, 12, 28, 6, 15, WOOD_L)
  s.box(8, 5, 12, 28, 6, 15, WOOD)
  for x in (12, 17, 22):
    s.box(x, 5, 13, x + 3, 6, 14, WOOD_D)        # 櫃門把手線
  P.append(s)

  m = Part(R, "mirror", "bed_mirror")
  m.box(7, 19, 1, 9, 20, 2, WOOD_D); m.box(7, 26, 1, 9, 27, 2, WOOD_D)   # 腳座
  m.box(7, 19, 2, 9, 27, 13, WHITE)
  m.box(8, 20, 3, 9, 26, 12, PANE)
  m.box(7, 19, 13, 9, 27, 14, PINK)
  P.append(m)

  c = Part(R, "corner", "bed_corner"); c.pillar(31, 18, 1, 16); P.append(c)

  g = Part(R, "window-glow", "bed_light")
  g.window_y(38, 40, 10, 26, 13, 23)
  g.box(35, 13, 1, 38, 23, 2, LIGHT)
  P.append(g)

  spots = [(26, 23, 30, 27), (14, 28, 18, 32), (7, 28, 11, 32), (31, 25, 35, 29), (20, 28, 24, 31)]
  k = Part(R, "clutter-some", "bed_clutter"); k.piles(spots[:2]); P.append(k)
  k = Part(R, "clutter-lots", "bed_clutter"); k.piles(spots); P.append(k)
  return P


def desk_study(p):
  for (x, y) in ((12, 14), (26, 14), (12, 19), (26, 19)):
    p.box(x, y, 1, x + 2, y + 2, 8, WOOD_D)
  p.box(12, 14, 8, 28, 21, 9, WOOD)
  p.box(14, 15, 9, 20, 19, 10, GREY)        # 筆電底
  p.box(14, 15, 10, 20, 16, 14, GREY)       # 螢幕（面向椅子）
  p.box(15, 15, 11, 19, 16, 13, SCREEN)
  p.box(15, 16, 9, 19, 18, 10, KEY)         # 鍵盤區（覆蓋筆電底）
  p.box(22, 19, 9, 26, 21, 10, PINK)        # 書
  p.box(22, 19, 10, 25, 21, 11, MINT)


def chair(p, x0, y0, back_y):
  p.box(x0 + 2, y0 + 2, 1, x0 + 4, y0 + 4, 4, GREY)
  p.box(x0, y0, 4, x0 + 6, y0 + 6, 5, LAV)
  p.box(x0, back_y, 5, x0 + 6, back_y + 2, 11, LAV)


def study():
  R = "study"
  P = []
  base = Part(R, "base", "fixed")
  for x in range(N):
    for y in range(N):
      base.v[(x, y, 0)] = FLOOR if (x // 3 + y) % 2 else FLOOR2
  base.wall_y(0, 2, 2, 26)
  P.append(base)
  d = Part(R, "desk", "fixed"); desk_study(d); P.append(d)
  c = Part(R, "chair", "fixed"); chair(c, 17, 23, 28); P.append(c)

  w = Part(R, "back-wall", "seat_back"); w.wall_x(4, 40, 36, 38); P.append(w)
  w = Part(R, "back-window", "seat_back"); w.window_x(4, 40, 36, 38, 12, 30); P.append(w)
  w = Part(R, "back-door", "seat_back"); w.door_x(4, 40, 36, 38, 16, 25); P.append(w)

  d = Part(R, "door-diag", "seat_door_view"); d.door_x(28, 40, 2, 4, 30, 38); P.append(d)
  d = Part(R, "door-front", "seat_door_view"); d.door_x(10, 30, 2, 4, 16, 24); P.append(d)
  d = Part(R, "door-behind", "seat_door_view"); d.door_y(0, 2, 26, 38, 28, 36); P.append(d)

  b = Part(R, "beam", "seat_overhead"); b.gantry(4, 34, 22, 25, BEAM); P.append(b)

  f = Part(R, "front-open", "seat_front"); f.box(12, 6, 1, 28, 12, 2, MAT)
  f.box(13, 7, 1, 27, 8, 2, WHITE); f.box(13, 10, 1, 27, 11, 2, WHITE); P.append(f)
  f = Part(R, "front-wall", "seat_front"); f.box(8, 11, 1, 32, 13, 16, WALL)
  f.box(8, 11, 16, 32, 13, 17, WALL_TOP); P.append(f)

  c = Part(R, "corner", "seat_corner"); c.pillar(33, 17, 1, 16); P.append(c)

  s = Part(R, "sun-patch", "seat_light", supports=("desk",))
  s.box(22, 15, 9, 27, 19, 10, SUN)
  s.box(24, 15, 9, 25, 19, 10, LIGHT)
  s.box(5, 13, 1, 11, 21, 2, SUN)
  s.box(5, 16, 1, 11, 17, 2, LIGHT)
  s.box(7, 13, 1, 8, 21, 2, LIGHT)
  P.append(s)

  spots = [(24, 24, 28, 28), (8, 26, 12, 30), (29, 27, 33, 31), (12, 31, 16, 35), (25, 31, 29, 35)]
  k = Part(R, "clutter-some", "seat_clutter"); k.piles(spots[:2]); P.append(k)
  k = Part(R, "clutter-lots", "seat_clutter"); k.piles(spots); P.append(k)
  return P


def desk_office(p, x0, x1, y0, y1, monitor=True):
  for (x, y) in ((x0, y0), (x1 - 2, y0), (x0, y1 - 2), (x1 - 2, y1 - 2)):
    p.box(x, y, 1, x + 2, y + 2, 8, GREY)
  p.box(x0, y0, 8, x1, y1, 9, WHITE)
  if monitor:
    p.box(15, 15, 9, 25, 16, 15, GREY)
    p.box(16, 15, 10, 24, 16, 14, SCREEN)
    p.box(19, 16, 9, 21, 17, 11, GREY)
    p.box(16, 17, 9, 24, 19, 10, KEY)
    p.box(26, 16, 9, 28, 18, 12, MINT)        # 杯子


def office():
  R = "office"
  P = []
  base = Part(R, "base", "fixed")
  for x in range(N):
    for y in range(N):
      base.v[(x, y, 0)] = CARPET if (x // 4 + y // 4) % 2 else CARPET2
  P.append(base)
  d = Part(R, "desk", "fixed"); desk_office(d, 10, 30, 14, 20); P.append(d)
  c = Part(R, "chair", "fixed"); chair(c, 17, 22, 26); P.append(c)

  w = Part(R, "back-wall", "office_back")
  w.box(8, 34, 1, 32, 38, 14, CABINET)
  for z in (5, 9):
    w.box(8, 34, z, 32, 35, z + 1, CABINET_D)
  for x in (15, 24):
    w.box(x, 34, 1, x + 1, 35, 14, CABINET_D)
  w.box(8, 34, 14, 32, 38, 15, CABINET_D)
  P.append(w)
  w = Part(R, "back-aisle", "office_back")
  w.box(0, 31, 1, 40, 33, 2, LINE)
  w.box(30, 34, 1, 33, 37, 3, GREY)                 # 腳
  w.box(30, 34, 3, 33, 37, 10, MINT)                # 身體
  w.box(30, 34, 10, 33, 37, 13, SKIN)               # 頭
  w.box(30, 34, 13, 33, 37, 14, WOOD_D)             # 頭髮
  P.append(w)
  w = Part(R, "back-low", "office_back")
  w.box(6, 31, 1, 34, 33, 7, PART_BLUE); w.box(6, 31, 7, 34, 33, 8, PART_BLUE_T)
  desk_office(w, 12, 28, 34, 39, monitor=False)
  P.append(w)

  d = Part(R, "door-front", "office_door"); d.door_x(8, 32, 1, 3, 15, 25); P.append(d)
  b = Part(R, "boss-desk", "office_door")
  desk_office(b, 8, 32, 3, 8, monitor=False)
  b.box(9, 4, 9, 13, 7, 10, KRAFT)
  chair(b, 17, 0, 0)                                # 主管椅（椅背 y0–2，面向使用者）
  P.append(b)

  f = Part(R, "front-open", "office_front"); f.box(12, 9, 1, 28, 13, 2, MAT)
  f.box(13, 10, 1, 27, 11, 2, WHITE); P.append(f)
  f = Part(R, "front-partition", "office_front")
  f.box(8, 11, 1, 32, 13, 15, PART_BLUE); f.box(8, 11, 15, 32, 13, 16, PART_BLUE_T); P.append(f)

  b = Part(R, "beam", "office_overhead"); b.gantry(3, 35, 23, 26, BEAM); P.append(b)
  t = Part(R, "lamp-tube", "office_overhead")
  t.gantry(3, 35, 23, 26, BEAM_D, deep=False)
  for x in (10, 29):
    t.box(x, 24, 15, x + 1, 25, 16, GREY)           # 吊線
  t.box(8, 23, 14, 32, 26, 15, KEY)                 # 燈座
  t.box(9, 24, 13, 31, 25, 14, WHITE)               # 燈管
  P.append(t)

  c = Part(R, "corner", "office_corner"); c.pillar(34, 17, 1, 16); P.append(c)
  g = Part(R, "window-side", "office_light"); g.window_y(0, 2, 6, 28, 9, 25); P.append(g)

  spots = [(24, 22, 28, 26), (6, 22, 10, 26), (30, 27, 34, 31), (10, 27, 14, 31), (5, 14, 9, 18)]
  k = Part(R, "clutter-some", "office_clutter"); k.piles(spots[:2]); P.append(k)
  k = Part(R, "clutter-lots", "office_clutter"); k.piles(spots); P.append(k)
  return P


# 互斥（不會同時被選中）但分屬不同題目的零件對：床離牆時 bed-floating 取代 bed。
EXCLUSIVE = {("bedroom", "bed", "bed-floating")}
ROOMS = [("bedroom", "Bedroom", bedroom), ("study", "Study", study), ("office", "Office", office)]


def build():
  return [(wire, variant, fn()) for wire, variant, fn in ROOMS]


def layer_image(part, z):
  """回傳 z 層影像（N×N 的 RGBA，未放大）；空層回 None。"""
  cells = [(x, y, rgb) for (x, y, zz), rgb in part.v.items() if zz == z]
  if not cells:
    return None
  im = Image.new("RGBA", (N, N), (0, 0, 0, 0))
  f = 0.86 + 0.14 * min(z, TOP) / TOP
  for x, y, rgb in cells:
    edge = any((x + dx, y + dy, z) not in part.v for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
    k = f * (0.9 if edge else 1.0)
    im.putpixel((x, y), tuple(min(255, int(c * k)) for c in rgb) + (255,))
  return im


def part_layers(part):
  return sorted({z for (_, _, z) in part.v})


def rust_source(rooms):
  lines = [
    "//! 空間風水結果頁縮景零件清單——由 `scripts/sprite-stack/fengshui_parts.py` 產生，**勿手改**。",
    "//!",
    "//! 切片在 `crates/web/assets/stack/fs/<room>/<id>/NN.webp`（只有 `layers` 列出的層）。",
    "//! 改零件後重跑產生器並跑 `--check`；答案→零件對照在 `diorama.rs`。",
    "",
    "use super::Room;",
    "",
    "/// 零件切片的平面格數（寬＝高）。",
    f"pub const DIORAMA_GRID: usize = {N};",
    "",
    "/// 一個零件：房型、id（目錄名）、有切片的全域層號（由低到高）。",
    "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
    "pub struct DioramaPart {",
    "    pub room: Room,",
    "    pub id: &'static str,",
    "    pub layers: &'static [u8],",
    "}",
    "",
    "/// 全部零件；同房型內的順序即同層內的繪製順序。",
    "pub const DIORAMA_PARTS: &[DioramaPart] = &[",
  ]
  for wire, variant, parts in rooms:
    for p in parts:
      ls = ", ".join(str(z) for z in part_layers(p))
      lines += [
        "    DioramaPart {",
        f"        room: Room::{variant},",
        f"        id: \"{p.id}\",",
        f"        layers: &[{ls}],",
        "    },",
      ]
  lines += ["];", ""]
  src = "\n".join(lines)
  out = subprocess.run(["rustfmt", "--edition", "2021", "--emit", "stdout"], input=src.encode(),
                       capture_output=True, check=True).stdout.decode()
  return out


def write_all(rooms):
  for wire, _, parts in rooms:
    room_dir = os.path.join(ASSETS, wire)
    if os.path.isdir(room_dir):
      for root, dirs, files in os.walk(room_dir, topdown=False):
        for f in files:
          os.remove(os.path.join(root, f))
        for d in dirs:
          os.rmdir(os.path.join(root, d))
    for p in parts:
      out = os.path.join(room_dir, p.id)
      os.makedirs(out, exist_ok=True)
      for z in part_layers(p):
        im = layer_image(p, z)
        im.resize((N * SCALE, N * SCALE), Image.NEAREST).save(os.path.join(out, f"{z:02d}.webp"), lossless=True)
      print(f"{wire}/{p.id}: {len(part_layers(p))} layers")
  with open(RUST_OUT, "w") as fh:
    fh.write(rust_source(rooms))
  print("wrote", os.path.relpath(RUST_OUT, REPO))


def check(rooms):
  errs = []
  # 1. 格子範圍
  for wire, _, parts in rooms:
    for p in parts:
      for (x, y, z) in p.v:
        if not (0 <= x < N and 0 <= y < N and 0 <= z < 100):
          errs.append(f"{wire}/{p.id}: voxel out of grid {(x, y, z)}")
          break
  # 2. 重疊：不同題目、可同時選中的零件兩兩無共用體素（同題互斥選項不檢查）
  for wire, _, parts in rooms:
    for i, a in enumerate(parts):
      for b in parts[i + 1:]:
        if a.group == b.group and a.group != "fixed":
          continue
        if (wire, a.id, b.id) in EXCLUSIVE or (wire, b.id, a.id) in EXCLUSIVE:
          continue
        shared = set(a.v) & set(b.v)
        if shared:
          errs.append(f"{wire}: {a.id} ∩ {b.id} share {len(shared)} voxels, e.g. {sorted(shared)[0]}")
  # 3. 接地：每個體素經同零件（或其 supports 零件）六鄰連到 z≤1
  for wire, _, parts in rooms:
    by_id = {p.id: p for p in parts}
    for p in parts:
      if p.id == "base":
        continue
      solid = set(p.v)
      for s in p.supports:
        solid |= set(by_id[s].v)
      seen = deque(k for k in solid if k[2] <= 1)
      reach = set(seen)
      while seen:
        x, y, z = seen.popleft()
        for d in ((1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)):
          n = (x + d[0], y + d[1], z + d[2])
          if n in solid and n not in reach:
            reach.add(n); seen.append(n)
      floating = [k for k in p.v if k not in reach]
      if floating:
        errs.append(f"{wire}/{p.id}: {len(floating)} floating voxels, e.g. {sorted(floating)[0]}")
  # 4. 檔案 ⇄ 模型（層號集合＋像素）
  for wire, _, parts in rooms:
    room_dir = os.path.join(ASSETS, wire)
    want_dirs = {p.id for p in parts}
    have_dirs = set(os.listdir(room_dir)) if os.path.isdir(room_dir) else set()
    for extra in sorted(have_dirs - want_dirs):
      errs.append(f"{wire}: stray part dir {extra}")
    for p in parts:
      d = os.path.join(room_dir, p.id)
      want = {f"{z:02d}.webp" for z in part_layers(p)}
      have = set(os.listdir(d)) if os.path.isdir(d) else set()
      if want != have:
        errs.append(f"{wire}/{p.id}: files {sorted(have ^ want)} differ from model")
        continue
      for z in part_layers(p):
        exp = layer_image(p, z).resize((N * SCALE, N * SCALE), Image.NEAREST)
        got = Image.open(os.path.join(d, f"{z:02d}.webp")).convert("RGBA")
        if got.tobytes() != exp.tobytes():
          errs.append(f"{wire}/{p.id}/{z:02d}.webp: pixels differ from model (regenerate)")
  # 5. 常數 ⇄ 模型
  want_src = rust_source(rooms)
  have_src = open(RUST_OUT).read() if os.path.exists(RUST_OUT) else ""
  if want_src != have_src:
    errs.append(f"{os.path.relpath(RUST_OUT, REPO)} differs from generator output (regenerate)")
  for e in errs:
    print("FAIL", e)
  total = sum(len(parts) for _, _, parts in rooms)
  print(f"{'FAIL' if errs else 'PASS'}: {total} parts, {len(errs)} problems")
  return not errs


if __name__ == "__main__":
  ap = argparse.ArgumentParser()
  ap.add_argument("--check", action="store_true")
  args = ap.parse_args()
  rooms = build()
  if args.check:
    sys.exit(0 if check(rooms) else 1)
  write_all(rooms)
