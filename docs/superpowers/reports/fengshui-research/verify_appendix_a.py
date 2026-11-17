#!/usr/bin/env python3
"""風水 v1 附錄 A 驗證(2026-11-17 重建;研究原檔遺失於第四台機器)。

執行:python3 -I docs/superpowers/reports/fengshui-research/verify_appendix_a.py
全部 assert 通過即印出附錄 A 表格。Batch 1 會把同樣斷言移植成 Rust 測試。
"""

# 卦數(洛書宮數)↔ 卦名 ↔ 方位(後天八卦;與 mingli-luoshu PALACE_NAME/PALACE_DIR 一致)
GUA = {1: "坎", 2: "坤", 3: "震", 4: "巽", 6: "乾", 7: "兌", 8: "艮", 9: "離"}
DIR = {"坎": "N", "坤": "SW", "震": "E", "巽": "SE", "乾": "NW", "兌": "W", "艮": "NE", "離": "S"}
# mingli-luoshu 1.2.0 src/lib.rs PALACE_NAME / PALACE_DIR(簡體),逐字抄錄用於交叉比對
LUOSHU_NAME = ["", "坎", "坤", "震", "巽", "中", "乾", "兑", "艮", "离"]
LUOSHU_DIR = ["", "北", "西南", "东", "东南", "中", "西北", "西", "东北", "南"]
S2T = {"兑": "兌", "离": "離"}
DIR_ZH = {"北": "N", "西南": "SW", "东": "E", "东南": "SE", "西北": "NW", "西": "W", "东北": "NE", "南": "S"}


def norm9(x):
    r = x % 9
    return 9 if r == 0 else r


def digital_root(y):
    return norm9(sum(int(c) for c in str(y)))


# ---- 命卦:三種公式 ----
def male_century(y):
    yy = y % 100
    if 1800 <= y <= 1899:
        return norm9(101 - yy)
    if 1900 <= y <= 1999:
        return norm9(100 - yy)
    if 2000 <= y <= 2099:
        return norm9(99 - yy)
    raise ValueError(y)


def female_century(y):
    yy = y % 100
    if 1800 <= y <= 1899:
        return norm9(yy - 5)
    if 1900 <= y <= 1999:
        return norm9(yy - 4)
    if 2000 <= y <= 2099:
        return norm9(yy + 6)
    raise ValueError(y)


def male_digitsum(y):
    return norm9(11 - digital_root(y))


def female_digitsum(y):
    return norm9(digital_root(y) + 4)


def male_yearstar(y):  # 年紫白:1984 七赤,逐年遞減
    return norm9(7 - (y - 1984))


def female_yearstar(y):  # 男女原數相加 ≡ 15(mod 9 → 6)
    return norm9(15 - male_yearstar(y))


def mingua(y, formula):
    raw = male_digitsum(y) if formula == "male" else female_digitsum(y)
    if raw == 5:
        raw = 2 if formula == "male" else 8  # 男寄坤、女寄艮
    return raw


for y in range(1899, 2100):
    assert male_century(y) == male_digitsum(y) == male_yearstar(y), y
    assert female_century(y) == female_digitsum(y) == female_yearstar(y), y
    for f in ("male", "female"):
        assert mingua(y, f) in GUA, (y, f)

# 公開年星錨點(男命原數 = 該年年紫白入中星)
for y, star in {1984: 7, 2024: 3, 2025: 2, 2026: 1}.items():
    assert male_yearstar(y) == star, (y, star)
# 寄卦專測:男命原數 5 的年份(例 2022:11−6=5)→ 坤;女命原數 5(例 1900)→ 艮
assert male_digitsum(2022) == 5 and mingua(2022, "male") == 2
assert female_digitsum(1900) == 5 and mingua(1900, "female") == 8

# ---- 卦 ↔ 方位:對 mingli-luoshu 交叉 ----
for n, name in GUA.items():
    assert S2T.get(LUOSHU_NAME[n], LUOSHU_NAME[n]) == name, n
    assert DIR_ZH[LUOSHU_DIR[n]] == DIR[name], n

# ---- 大遊年:由歌訣推表 ----
SONG = {
    "乾": "六天五禍絕延生",
    "坎": "五天生延絕禍六",
    "艮": "六絕禍生延天五",
    "震": "延生禍絕五天六",
    "巽": "天五六禍生絕延",
    "離": "六五絕延禍生天",
    "坤": "天延絕生禍五六",
    "兌": "生禍延絕六五天",
}
CYCLE = ["乾", "坎", "艮", "震", "巽", "離", "坤", "兌"]  # 歌訣自身之後依此序循環
STAR = {"生": "生氣", "天": "天醫", "延": "延年", "伏": "伏位", "禍": "禍害", "六": "六煞", "五": "五鬼", "絕": "絕命"}


def song_table():
    t = {}
    for me in CYCLE:
        row = {DIR[me]: "伏位"}
        i = CYCLE.index(me)
        for k, ch in enumerate(SONG[me]):
            other = CYCLE[(i + 1 + k) % 8]
            row[DIR[other]] = STAR[ch]
        t[me] = row
    return t


# 手寫轉錄表(獨立於歌訣,依八宅通行表逐格抄錄)
HAND = {
    "坎": {"SE": "生氣", "E": "天醫", "S": "延年", "N": "伏位", "W": "禍害", "NE": "五鬼", "NW": "六煞", "SW": "絕命"},
    "離": {"E": "生氣", "SE": "天醫", "N": "延年", "S": "伏位", "NE": "禍害", "W": "五鬼", "SW": "六煞", "NW": "絕命"},
    "震": {"S": "生氣", "N": "天醫", "SE": "延年", "E": "伏位", "SW": "禍害", "NW": "五鬼", "NE": "六煞", "W": "絕命"},
    "巽": {"N": "生氣", "S": "天醫", "E": "延年", "SE": "伏位", "NW": "禍害", "SW": "五鬼", "W": "六煞", "NE": "絕命"},
    "乾": {"W": "生氣", "NE": "天醫", "SW": "延年", "NW": "伏位", "SE": "禍害", "E": "五鬼", "N": "六煞", "S": "絕命"},
    "坤": {"NE": "生氣", "W": "天醫", "NW": "延年", "SW": "伏位", "E": "禍害", "SE": "五鬼", "S": "六煞", "N": "絕命"},
    "艮": {"SW": "生氣", "NW": "天醫", "W": "延年", "NE": "伏位", "S": "禍害", "N": "五鬼", "E": "六煞", "SE": "絕命"},
    "兌": {"NW": "生氣", "SW": "天醫", "NE": "延年", "W": "伏位", "N": "禍害", "S": "五鬼", "SE": "六煞", "E": "絕命"},
}

T = song_table()
assert T == HAND, "歌訣推表與手寫表不一致"

STARS = set(STAR.values())
INV_DIR = {v: k for k, v in DIR.items()}
GOOD = {"生氣", "天醫", "延年", "伏位"}
EAST = {"坎", "離", "震", "巽"}
for me, row in T.items():
    assert set(row.values()) == STARS and len(row) == 8, me  # 每列為八星排列
    for d, s in row.items():
        other = INV_DIR[d]
        assert T[other][DIR[me]] == s, (me, other)  # 對稱:A 看 B = B 看 A
        same_group = (me in EAST) == (other in EAST)
        assert (s in GOOD) == same_group, (me, other, s)  # 東西四命:同組四吉
# 三審錨點:乾命八方(合大遊年歌)
assert T["乾"] == {"NW": "伏位", "N": "六煞", "NE": "天醫", "E": "五鬼", "SE": "禍害", "S": "絕命", "SW": "延年", "W": "生氣"}

print("ALL CHECKS PASSED (1899–2099 × 2 formulas; 8×8 song == hand; invariants)")
ORDER = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"]
print("| 命卦 | " + " | ".join(ORDER) + " |")
print("|" + "---|" * 9)
for me in ["坎", "坤", "震", "巽", "乾", "兌", "艮", "離"]:
    print(f"| {me} | " + " | ".join(T[me][d] for d in ORDER) + " |")
