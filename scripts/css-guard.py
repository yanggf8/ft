#!/usr/bin/env python3
"""css-guard.py — colour-literal allowlist gate + var() existence gate.

(a) Literal gate (allowlist-based): any hex colour (#rgb/#rgba/#rrggbb/
    #rrggbbaa) or rgb( / rgba( / hsl( / hsla( found
      - in crates/web/style.css outside every `:root` block (declaration
        bodies only; selectors such as `#app` are not colours), or
      - anywhere in crates/web/src/**/*.rs
    is an error unless its line is listed in scripts/css-guard-allowlist.txt.
    Comments are ignored (CSS /* */, Rust // and /* */). `#` not followed by a
    3/4/6/8-digit hex run (SVG path data, Rust attributes, raw strings,
    HTML entities) never matches; `url(#id)` / `href="#id"` fragment refs are
    skipped.
(b) var() gate: every var(--name) in style.css and the .rs files must be
    defined in a `:root` block of style.css.

  python3 scripts/css-guard.py              # run both gates
  python3 scripts/css-guard.py --only literal|var
  python3 scripts/css-guard.py --allow-local-vars   # 元件區域變數視為已定義
  python3 scripts/css-guard.py --self-test  # fixture test in a temp dir

Exit 0 = clean; 1 = violations / bad allowlist entries; 2 = file error.
"""

import argparse
import glob
import os
import re
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cssparse as cp  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CSS_REL = "crates/web/style.css"
RS_GLOB = "crates/web/src/**/*.rs"
ALLOW_REL = "scripts/css-guard-allowlist.txt"

LITERAL_RE = re.compile(
  r"(?<![\w&#/-])#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})(?![\w-])"
  r"|\b(?:rgba?|hsla?)\(", re.I)
FRAGMENT_CTX = re.compile(r"""(?:url\(\s*['"]?|href\s*=\s*['"]?|getElementById\(\s*['"]?)$""")
VAR_RE = re.compile(r"var\(\s*(--[\w-]+)")


def literal_hits(clean, mask=None):
  """Yield (pos, literal). mask: optional bool list restricting positions."""
  for m in LITERAL_RE.finditer(clean):
    if mask is not None and not mask[m.start()]:
      continue
    if m.group(0).startswith("#") and FRAGMENT_CTX.search(clean[max(0, m.start() - 20):m.start()]):
      continue
    yield m.start(), m.group(0)


def load_allowlist(path, root):
  """-> (entries, errors). entry = (relpath, substring, reason, lineno)."""
  entries, errors = [], []
  if not os.path.exists(path):
    return entries, errors
  with open(path, encoding="utf-8") as f:
    for no, raw in enumerate(f, 1):
      line = raw.rstrip("\n")
      if not line.strip() or line.lstrip().startswith("#"):
        continue
      if " # " not in line:
        errors.append("%s:%d 缺理由（格式：路徑:子字串 # 理由）" % (ALLOW_REL, no))
        continue
      spec, reason = line.split(" # ", 1)
      if ":" not in spec or not reason.strip():
        errors.append("%s:%d 格式錯誤：%s" % (ALLOW_REL, no, line))
        continue
      rel, sub = spec.split(":", 1)
      rel, sub = rel.strip(), sub.strip()
      if not sub:
        errors.append("%s:%d 子字串為空" % (ALLOW_REL, no))
        continue
      if not os.path.exists(os.path.join(root, rel)):
        errors.append("%s:%d 檔案不存在：%s" % (ALLOW_REL, no, rel))
        continue
      entries.append((rel, sub, reason.strip(), no))
  return entries, errors


def run(root, only, allow_path, allow_local_vars=False):
  css_path = os.path.join(root, CSS_REL)
  try:
    with open(css_path, encoding="utf-8") as f:
      css = f.read()
  except OSError as e:
    print("無法讀取 %s：%s" % (css_path, e), file=sys.stderr)
    return 2
  rs_files = sorted(glob.glob(os.path.join(root, RS_GLOB), recursive=True))

  css_clean = cp.strip_css_comments(css)
  roots = cp.root_blocks(css_clean)
  in_root = [False] * len(css_clean)
  for _, s, e in roots:
    for k in range(s, e):
      in_root[k] = True
  decl = cp.decl_mask(css_clean)
  css_mask = [d and not r for d, r in zip(decl, in_root)]

  sources = [(CSS_REL, css, css_clean, css_mask)]
  for p in rs_files:
    with open(p, encoding="utf-8") as f:
      txt = f.read()
    sources.append((os.path.relpath(p, root), txt, cp.strip_rust_comments(txt), None))

  # defined tokens
  defined = {}
  for ctx in cp.root_tokens(css):
    for name, (_, line) in ctx["tokens"].items():
      defined.setdefault(name, line)
  local_defs = {}
  for prelude, s, e, _ in cp.css_blocks(css_clean):
    if cp.is_root_prelude(prelude):
      continue
    for name, _, pos in cp.split_decls(css_clean[s:e], s):
      if name.startswith("--"):
        local_defs.setdefault(name, (prelude, cp.line_of(css_clean, pos)))

  allow, allow_errors = load_allowlist(allow_path, root)
  allow_used = {}
  by_file = {}

  def add(rel, line, kind, msg, text):
    by_file.setdefault(rel, []).append((line, kind, msg, text.strip()))

  for rel, raw, clean, mask in sources:
    raw_lines = raw.split("\n")
    if only in (None, "literal"):
      file_allow = [a for a in allow if a[0] == rel]
      for a in file_allow:
        hits = [i + 1 for i, l in enumerate(raw_lines) if a[1] in l]
        allow_used[a] = hits
      for pos, lit in literal_hits(clean, mask):
        ln = cp.line_of(clean, pos)
        if any(ln in allow_used[a] and len(allow_used[a]) == 1 for a in file_allow):
          continue
        add(rel, ln, "literal", "寫死色值 %s" % lit, raw_lines[ln - 1])
    if only in (None, "var"):
      for m in VAR_RE.finditer(clean):
        name = m.group(1)
        if name in defined or (allow_local_vars and name in local_defs):
          continue
        ln = cp.line_of(clean, m.start())
        why = "var(%s) 未定義於 :root" % name
        if name in local_defs:
          why += "（僅定義於非 :root 區塊 `%s` L%d）" % local_defs[name]
        add(rel, ln, "var", why, raw_lines[ln - 1])

  errors = list(allow_errors)
  warnings = []
  for a, hits in allow_used.items():
    if not hits:
      warnings.append("%s:%d 過期條目（%s 內找不到 %r）" % (ALLOW_REL, a[3], a[0], a[1]))
    elif len(hits) > 1:
      errors.append("%s:%d 子字串不唯一（%s 第 %s 行皆命中）" % (
        ALLOW_REL, a[3], a[0], ",".join(map(str, hits))))

  n_lit = n_var = 0
  for rel in sorted(by_file, key=lambda r: (r != CSS_REL, r)):
    items = sorted(by_file[rel])
    print("== %s（%d）" % (rel, len(items)))
    for line, kind, msg, text in items:
      if kind == "literal":
        n_lit += 1
      else:
        n_var += 1
      print("   %s:%d  [%s] %s\n      | %s" % (rel, line, kind, msg, text[:160]))
  for w in warnings:
    print("WARN  %s" % w)
  for e in errors:
    print("ERROR %s" % e)
  bad = n_lit + n_var + len(errors)
  print("結果：%s — 寫死色值 %d、未定義 var() %d、allowlist 錯誤 %d、警告 %d"
        "（掃描 1 個 css + %d 個 rs；:root 區塊 %d 個；已定義 token %d 個）" % (
          "PASS" if not bad else "FAIL", n_lit, n_var, len(errors), len(warnings),
          len(rs_files), len(roots), len(defined)))
  return 0 if not bad else 1


# ------------------------------------------------------------- self-test

FIX_CSS = """/* header #ffffff rgba(0,0,0,.5) — comment, ignored */
:root {
  --a: #123456;
  --b: rgba(1, 2, 3, 0.5);
}
#app, #add { color: var(--a); }
.x { background: #fff; border: 1px solid rgb(1 2 3); }
.y { filter: url(#beaded); color: var(--missing); }
.ok { stroke: rgba(255, 255, 255, 0.9); }
@media (pointer: coarse) {
  :root { --c: #000; }
  .z { color: hsl(0 0% 0%); background: var(--c); }
}
"""

FIX_RS = r'''// comment #abcdef rgba(1,1,1,1)
/* block #abcdef /* nested #fedcba */ still comment */
#[derive(Clone)]
fn f() -> &'static str {
  let url = "https://x.test/#abc"; // url ok? (#abc after / is an anchor)
  let raw = r#"<path d="M10 10 L20 20" fill="url(#face)"/>"#;
  let c = '#';
  let ent = "&#123;";
  view! { <p style="color:#9aa3b2">"x"</p> }
  view! { <a href="#bad">"y"</a> }
  view! { <p style="color:rgba(0,0,0,.5);border:var(--b)">"z"</p> }
  view! { <p style="color:var(--nope)">"w"</p> }
  "keep #ff00ff allowed line"
}
'''

FIX_ALLOW = """# comment
crates/web/style.css:stroke: rgba(255, 255, 255, 0.9) # fixture reason
crates/web/src/a.rs:keep #ff00ff allowed # fixture reason
crates/web/src/a.rs:does-not-exist # stale -> warning
"""


def self_test():
  import contextlib
  import io
  ok = True

  def check(name, cond, detail=""):
    nonlocal ok
    print("  %s  %s %s" % ("ok  " if cond else "FAIL", name, detail))
    ok = ok and cond

  with tempfile.TemporaryDirectory() as d:
    os.makedirs(os.path.join(d, "crates/web/src"))
    os.makedirs(os.path.join(d, "scripts"))
    for rel, txt in [(CSS_REL, FIX_CSS), ("crates/web/src/a.rs", FIX_RS),
                     (ALLOW_REL, FIX_ALLOW)]:
      with open(os.path.join(d, rel), "w", encoding="utf-8") as f:
        f.write(txt)
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
      rc = run(d, None, os.path.join(d, ALLOW_REL))
    out = buf.getvalue()
    lines = [l.strip() for l in out.splitlines()]

    def hit(rel_line, kind):
      return any(l.startswith(rel_line + "  [" + kind + "]") for l in lines)

    css = CSS_REL
    check("exit code 1", rc == 1, str(rc))
    check("CSS 註解內色值不報", not hit(css + ":1", "literal"))
    check(":root 內色值不報", not hit(css + ":3", "literal") and not hit(css + ":4", "literal"))
    check("選擇器 #app/#add 不報", not hit(css + ":6", "literal"))
    check(".x 的 #fff 與 rgb( 報出", sum(1 for l in lines if l.startswith(css + ":7  [literal]")) == 2)
    check("url(#beaded) 不報", not hit(css + ":8", "literal"))
    check("var(--missing) 報出", hit(css + ":8", "var"))
    check("allowlist 放行 :9", not hit(css + ":9", "literal"))
    check("@media 內 :root 不報", not hit(css + ":11", "literal"))
    check("@media 內 .z 的 hsl( 報出", hit(css + ":12", "literal"))
    check("@media :root 定義的 --c 視為已定義", not hit(css + ":12", "var"))
    rs = "crates/web/src/a.rs"
    for ln, what in [(1, "// 註解"), (2, "巢狀 /* */ 註解"), (3, "#[derive]"),
                     (5, "URL 錨點 /#abc"), (6, "raw string 內 url(#face)/SVG path"),
                     (7, "char '#'"), (8, "HTML entity &#123;"), (10, 'href="#bad"'),
                     (13, "allowlist 放行")]:
      check("rs:%d %s 不報" % (ln, what), not hit("%s:%d" % (rs, ln), "literal"))
    check("rs:9 #9aa3b2 報出", hit(rs + ":9", "literal"))
    check("rs:11 rgba( 報出", hit(rs + ":11", "literal"))
    check("rs:11 var(--b) 已定義不報", not hit(rs + ":11", "var"))
    check("rs:12 var(--nope) 報出", hit(rs + ":12", "var"))
    check("過期 allowlist 條目給警告", any(l.startswith("WARN") and "does-not-exist" in l for l in lines))
    if not ok:
      print(out)
  print("self-test：%s" % ("PASS" if ok else "FAIL"))
  return 0 if ok else 1


def main():
  ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
  ap.add_argument("--only", choices=["literal", "var"])
  ap.add_argument("--root", default=REPO, help="repo 根目錄（預設：本腳本上一層）")
  ap.add_argument("--allowlist", default=None)
  ap.add_argument("--allow-local-vars", action="store_true",
                  help="把 style.css 非 :root 區塊定義的元件區域變數（如 .zc-sign-* 的 --zc-deep）視為已定義")
  ap.add_argument("--self-test", action="store_true")
  a = ap.parse_args()
  if a.self_test:
    return self_test()
  return run(a.root, a.only, a.allowlist or os.path.join(a.root, ALLOW_REL), a.allow_local_vars)


if __name__ == "__main__":
  sys.exit(main())
