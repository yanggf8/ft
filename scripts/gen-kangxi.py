#!/usr/bin/env python3
"""gen-kangxi.py — 一次性離線產生 crates/schema/src/naming/strokes.rs 的康熙筆畫表。

不進 CI。規則與決策鎖定於 docs/superpowers/specs/2026-09-18-naming-v1-design.md §4：
  字集 = Big5 常用 5401 字（0xA440..0xC67E）＋ curated 補充（台灣姓氏差集＋複姓字）
  值   = kTotalStrokes，但 14 個「簡化部首」例外：
         氵→水4 忄→心4 扌→手4 犭→犬4 礻→示5 衤→衣6 王→玉5 艹→艸6
         辶→辵7 左阝→阜8 右阝→邑7 肉部之月→肉6 罒→网6 飠→食9
         啟用條件：radical_full + residual > kTotalStrokes（部首以簡化形出現）
         例外中的例外：residual==0（字即部首本身）一律 kTotalStrokes（王=4）
  kRSUnicode 解析：跳過帶 apostrophe 的簡體值、多值取第一個繁體值
  Unihan 缺漏/歧義 → skip（runtime UnknownChars，不猜）

Usage: python3 scripts/gen-kangxi.py [--unihan /path/to/Unihan.zip]
"""

import io
import sys
import urllib.request
import zipfile
from collections import Counter
from pathlib import Path

UNIHAN_URL = "https://www.unicode.org/Public/UCD/latest/ucd/Unihan.zip"
OUT = Path(__file__).resolve().parent.parent / "crates/schema/src/naming/strokes.rs"

# 14 個簡化部首：部首字 → 期滿畫數（與 Unihan kTotalStrokes(部首字) 對帳，錯即中止）
SPECIAL_RADICALS = {
    "水": 4, "心": 4, "手": 4, "犬": 4,
    "示": 5, "玉": 5, "衣": 6, "艸": 6, "网": 6, "肉": 6,
    "辵": 7, "邑": 7, "阜": 8, "食": 9,
}

# 台灣戶政次常見姓差集（Big5 二段或罕見）＋複姓用字＋名字 fixture 補充
SUPPLEMENT = (
    "鄺禤闞甯蒯蓋湛茹昝宦冼刁臧賁郗逯仇岑滕雍桂司徒"
    "司馬上官諸葛皇甫尉遲公孫長孫令狐慕容東方夏侯宇文端木"
    "西門南宮獨孤軒轅呼延赫連澹臺拓跋夾谷微生羊舌"
    "鍾鐘歐陽鵬娟婷雅淑惠玲珠芬芳"
)

GOLDEN = {
    "陳": 16, "林": 8, "黃": 12, "張": 11, "李": 7, "王": 4, "吳": 7, "劉": 15,
    "蔡": 17, "楊": 13, "許": 11, "鄭": 19, "洪": 10, "曾": 12, "邱": 12,
    "謝": 17, "高": 10, "周": 8, "葉": 15, "蘇": 22, "莊": 13, "呂": 7, "江": 7,
    "何": 7, "蕭": 18, "羅": 20, "簡": 18, "鐘": 20, "鍾": 17, "游": 13,
    "詹": 13, "方": 4, "顏": 18, "歐": 15, "陽": 17, "鵬": 19, "胡": 11,
    "明": 8, "娟": 10, "小": 3, "白": 5, "司": 5, "馬": 10, "光": 6, "芬": 10,
    "芳": 10, "婷": 12, "雅": 12, "淑": 12, "惠": 12, "玲": 10, "珠": 11,
    "四": 5, "五": 4, "七": 2, "十": 2, "玉": 5, "理": 12,
}


def load_unihan(path=None):
    """回傳 (total_strokes: dict, rs_unicode: dict)。"""
    if path is None:
        cache = Path("/tmp/Unihan.zip")
        if not cache.exists():
            print(f"downloading {UNIHAN_URL} ...")
            urllib.request.urlretrieve(UNIHAN_URL, cache)
        path = cache
    total, rs = {}, {}
    with zipfile.ZipFile(path) as zf:
        for name in zf.namelist():
            if not name.endswith(".txt"):
                continue
            with io.TextIOWrapper(zf.open(name), encoding="utf-8") as f:
                for line in f:
                    if line.startswith("#") or "\t" not in line:
                        continue
                    cp, field, value = line.rstrip("\n").split("\t", 2)
                    if field == "kTotalStrokes":
                        total[cp] = value.split()[0]  # 多值取第一
                    elif field == "kRSUnicode":
                        # 值如 "85.3 64.3'"；取第一個無 apostrophe（非簡體）的值
                        vals = [v for v in value.split() if not v.endswith("'")]
                        rs[cp] = (vals or value.split())[0]
    return total, rs


def big5_common_chars():
    chars = []
    for hi in range(0xA4, 0xC7):
        for lo in range(0x40, 0xFF):
            if lo in (0x7F,):
                continue
            if hi == 0xC6 and lo > 0x7E:
                continue
            cp = (hi << 8) | lo
            try:
                ch = bytes([hi, lo]).decode("big5")
            except UnicodeDecodeError:
                continue
            chars.append(ch)
    return chars


def main():
    unihan_arg = None
    if len(sys.argv) == 3 and sys.argv[1] == "--unihan":
        unihan_arg = sys.argv[2]
    total, rs = load_unihan(unihan_arg)

    # 部首號 → 滿畫數：由部首字自身的 kRSUnicode（N.0）反查，並與期滿畫數對帳
    radical_number = {}
    for ch, expected in SPECIAL_RADICALS.items():
        cp = f"U+{ord(ch):04X}"
        entry = rs.get(cp, "")
        if not entry.endswith(".0"):
            sys.exit(f"FATAL: 部首字 {ch} 的 kRSUnicode 非自指：{entry!r}")
        number = int(entry.split(".")[0])
        strokes = total.get(cp)
        if strokes is None or int(strokes) != expected:
            sys.exit(
                f"FATAL: 部首字 {ch} kTotalStrokes={strokes} 與期望 {expected} 不符"
                f"（Unihan 版本漂移？）"
            )
        radical_number[number] = expected
    print("部首對帳 OK:", {c: n for c, n in sorted(radical_number.items())})

    def resolve(ch):
        cp = f"U+{ord(ch):04X}"
        ts = total.get(cp)
        rsval = rs.get(cp)
        if ts is None or rsval is None:
            return None
        ts = int(ts)
        if "." not in rsval:
            return ts
        r, s = rsval.split(".")
        r, s = int(r), int(s)
        full = radical_number.get(r)
        if full is not None and s > 0 and full + s > ts:
            return full + s
        return ts  # 含 residual==0（王=4）與非簡化形

    table = {}
    unresolved = []
    for ch in dict.fromkeys(big5_common_chars() + list(SUPPLEMENT)):
        v = resolve(ch)
        if v is None or not (1 <= v <= 48):
            unresolved.append(ch)
        else:
            table[ch] = v

    # golden 對帳：產品鎖定值與機械推導不符即中止（不靜默放行）
    mismatches = {c: (v, resolve(c)) for c, v in GOLDEN.items() if resolve(c) != v}
    if mismatches:
        sys.exit(f"FATAL: golden 不符（期望 vs 推導）：{mismatches}")

    entries = sorted(table.items(), key=lambda kv: ord(kv[0]))
    lines = [f"('{ch}', {v})" for ch, v in entries]
    body = ",\n".join(
        "    " + ", ".join(lines[i : i + 8]) for i in range(0, len(lines), 8)
    )
    content = f"""//! 康熙筆畫表 — 由 scripts/gen-kangxi.py 產生（一次性離線，不進 CI；勿手改）。
//!
//! 產製規則、來源與決策（14 簡化部首、residual==0 例外、肉部=6、數目字形）：
//! docs/superpowers/specs/2026-09-18-naming-v1-design.md §4。
//! golden 值經 Grok 跨源核對（康熙原文/漢典 × Name104 × LocalPapa）。
//! 查無此字 = `None`：呼叫端收集為 UnknownChars，**絕不猜測**。

/// 表資料版本（字集或規則變更時 bump；kangxi-b5c-1 = Big5 常用＋姓氏補充、規則 v1）。
pub const STROKE_TABLE_VERSION: &str = "kangxi-b5c-1";

/// (char, 康熙筆畫)，以 codepoint 排序；generator 產出（{len(entries)} 字）。
pub static KANGXI: [(char, u8); {len(entries)}] = [
{body}
];

/// 查單字康熙筆畫。表以 codepoint 排序，binary search。
pub fn kangxi_strokes(c: char) -> Option<u8> {{
    KANGXI
        .binary_search_by_key(&c, |&(ch, _)| ch)
        .ok()
        .map(|i| KANGXI[i].1)
}}
"""
    # 保留既有 tests 區塊：只重寫非 tests 部分
    old = OUT.read_text(encoding="utf-8")
    if "#[cfg(test)]" in old:
        tests = old[old.index("#[cfg(test)]") :]
        content = content + "\n" + tests
    OUT.write_text(content, encoding="utf-8")

    hist = Counter(table.values())
    print(f"寫出 {len(entries)} 字 → {OUT}")
    print(f"未解析（runtime Unknown）：{''.join(unresolved) or '（無）'}")
    print("筆畫分布 top10:", hist.most_common(10))


if __name__ == "__main__":
    main()
