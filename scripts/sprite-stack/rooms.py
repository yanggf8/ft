"""空間風水房型縮景 sprite stacking 切片產生器（2026-11-22）。

三個程式化體素縮景（臥室／書房（在家）／辦公室座位）→ 每層一張透明 webp，
輸出 crates/web/assets/stack/room-<wire>/NN.webp（由下而上）。/fengshui 選房間按鈕把各層疊起來自轉。
改模型後重跑：python3 scripts/sprite-stack/rooms.py；層數與格數同時寫成產生檔
crates/web/src/pages/room_stacks.rs（勿手改，fengshui.rs 讀它）。顏色是點陣圖內容，不受 :root token 規則管轄。
只用盒子拼家具——示意空間配置，不畫任何風水物件或宣稱效果。"""
import os
import subprocess
from PIL import Image

N = 36       # 平面格數
SCALE = 8    # 每格輸出像素
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "crates", "web", "assets", "stack")
RUST_OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "crates", "web", "src", "pages", "room_stacks.rs")

FLOOR = (238, 222, 204); FLOOR2 = (230, 212, 192)
WALL = (232, 226, 246); WALL_TOP = (214, 204, 238)
WOOD = (214, 176, 140); WOOD_D = (190, 150, 116)
WHITE = (250, 248, 252); LAV = (204, 188, 240); PINK = (244, 182, 202); MINT = (176, 220, 200)
YELLOW = (250, 222, 150); GREY = (120, 116, 138); SCREEN = (150, 176, 214); BLUE = (190, 212, 240)
LEAF = (150, 196, 160)


class Model:
  def __init__(self):
    self.v = {}

  def box(self, x0, y0, z0, x1, y1, z1, rgb):
    """含頭不含尾的長方體；後畫覆蓋先畫。"""
    for x in range(x0, x1):
      for y in range(y0, y1):
        for z in range(z0, z1):
          if 0 <= x < N and 0 <= y < N and z >= 0:
            self.v[(x, y, z)] = rgb

  def room_shell(self):
    # 地板（木紋交錯）＋後牆（y 小）＋左牆（x 小），矮牆避免旋轉時擋住家具
    for x in range(N):
      for y in range(N):
        self.v[(x, y, 0)] = FLOOR if (x // 3 + y) % 2 else FLOOR2
    self.box(0, 0, 1, N, 2, 9, WALL)
    self.box(0, 0, 1, 2, N, 9, WALL)
    self.box(0, 0, 9, N, 2, 10, WALL_TOP)
    self.box(0, 0, 9, 2, N, 10, WALL_TOP)

  def save(self, name):
    out = os.path.join(ROOT, name)
    os.makedirs(out, exist_ok=True)
    for f in os.listdir(out):
      if f.endswith(".webp"):
        os.remove(os.path.join(out, f))
    top = max(z for (_, _, z) in self.v)
    for z in range(top + 1):
      im = Image.new("RGBA", (N, N), (0, 0, 0, 0))
      f = 0.86 + 0.14 * z / top          # 越低越暗，產生立體感
      for (x, y, zz), rgb in self.v.items():
        if zz != z:
          continue
        edge = any((x + dx, y + dy, z) not in self.v for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
        k = f * (0.9 if edge else 1.0)
        im.putpixel((x, y), tuple(min(255, int(c * k)) for c in rgb) + (255,))
      im.resize((N * SCALE, N * SCALE), Image.NEAREST).save(os.path.join(out, f"{z:02d}.webp"), lossless=True)
    print(name, "layers", top + 1)
    return top + 1


def bedroom():
  m = Model(); m.room_shell()
  # 床：床頭靠後牆
  m.box(8, 3, 1, 24, 26, 4, WOOD)
  m.box(8, 3, 4, 24, 6, 9, WOOD_D)          # 床頭板
  m.box(9, 6, 4, 23, 25, 6, WHITE)           # 床墊
  m.box(10, 7, 6, 22, 11, 7, PINK)           # 枕頭
  m.box(9, 13, 6, 23, 25, 7, LAV)            # 被子
  # 床頭櫃＋小燈
  m.box(26, 3, 1, 32, 9, 7, WOOD)
  m.box(28, 5, 7, 30, 7, 10, GREY)
  m.box(27, 4, 10, 31, 8, 13, YELLOW)
  # 地毯
  m.box(10, 28, 1, 26, 34, 2, MINT)
  return m.save("room-bedroom")


def study():
  m = Model(); m.room_shell()
  # 書桌：靠左牆，面向房間
  m.box(4, 10, 1, 6, 12, 9, WOOD_D); m.box(4, 24, 1, 6, 26, 9, WOOD_D)
  m.box(13, 10, 1, 15, 12, 9, WOOD_D); m.box(13, 24, 1, 15, 26, 9, WOOD_D)
  m.box(3, 9, 9, 16, 27, 10, WOOD)
  m.box(5, 12, 10, 10, 15, 12, PINK)         # 書
  m.box(5, 16, 10, 9, 22, 11, WHITE)         # 筆記本
  m.box(11, 22, 10, 13, 24, 15, GREY)        # 檯燈
  m.box(10, 21, 15, 14, 25, 16, YELLOW)
  # 椅子（坐在桌子右側，背後是房間）
  m.box(18, 14, 1, 24, 20, 5, LAV)
  m.box(22, 14, 5, 24, 20, 11, LAV)
  # 書櫃靠後牆
  m.box(22, 2, 1, 33, 6, 16, WOOD)
  for i, c in enumerate((PINK, MINT, BLUE, YELLOW, LAV)):
    m.box(23 + i * 2, 3, 5, 24 + i * 2, 7, 9, c)
    m.box(23 + i * 2, 3, 11, 24 + i * 2, 7, 15, (MINT, LAV, PINK, BLUE, YELLOW)[i])
  # 盆栽
  m.box(28, 26, 1, 32, 30, 4, WHITE); m.box(28, 26, 4, 32, 30, 8, LEAF)
  return m.save("room-study")


def office():
  m = Model()
  for x in range(N):
    for y in range(N):
      m.v[(x, y, 0)] = (226, 224, 236) if (x // 4 + y // 4) % 2 else (234, 232, 242)
  # 矮隔板（座位前方與側邊）
  m.box(4, 6, 1, 32, 8, 13, BLUE)
  m.box(4, 6, 1, 6, 26, 13, BLUE)
  m.box(4, 6, 13, 32, 8, 14, (170, 196, 232)); m.box(4, 6, 13, 6, 26, 14, (170, 196, 232))
  # 桌子貼隔板
  m.box(6, 8, 8, 30, 18, 9, WHITE)
  m.box(7, 16, 1, 9, 18, 8, GREY); m.box(27, 16, 1, 29, 18, 8, GREY)
  # 螢幕＋鍵盤
  m.box(13, 9, 9, 23, 10, 16, GREY); m.box(14, 10, 10, 22, 11, 15, SCREEN)
  m.box(17, 11, 9, 19, 12, 10, GREY)
  m.box(14, 13, 9, 22, 15, 10, (214, 212, 226))
  m.box(25, 10, 9, 28, 13, 12, MINT)         # 杯子
  # 椅子（背對走道）
  m.box(15, 21, 4, 21, 27, 6, LAV)
  m.box(15, 26, 6, 21, 28, 13, LAV)
  m.box(17, 23, 1, 19, 25, 4, GREY)
  # 走道標線
  m.box(2, 32, 1, 34, 33, 2, YELLOW)
  return m.save("room-office")


def write_rust(counts):
  """層數常數（spec 2026-11-22 風水縮景 §7.11）：消除手動同步。"""
  rows = "\n".join(f"    (Room::{v}, {n})," for v, n in counts)
  src = f"""//! 選房間按鈕縮景的切片層數——由 `scripts/sprite-stack/rooms.py` 產生，**勿手改**。
//!
//! 切片在 `crates/web/assets/stack/room-<wire>/NN.webp`。

use ft_schema::fengshui::Room;

/// 切片的平面格數（寬＝高）。
pub const ROOM_STACK_GRID: usize = {N};

/// 各房型的層數。
pub const ROOM_STACK_LAYERS: [(Room, usize); 3] = [
{rows}
];
"""
  out = subprocess.run(["rustfmt", "--edition", "2021", "--emit", "stdout"], input=src.encode(),
                       capture_output=True, check=True).stdout.decode()
  with open(RUST_OUT, "w") as fh:
    fh.write(out)
  print("wrote", os.path.normpath(RUST_OUT))


if __name__ == "__main__":
  write_rust([("Bedroom", bedroom()), ("Study", study()), ("Office", office())])
