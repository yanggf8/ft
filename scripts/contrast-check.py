#!/usr/bin/env python3
"""contrast-check.py — offline WCAG 2.x contrast gate for style.css tokens.

Parses every `:root` block of crates/web/style.css (including ones nested in
@media / @supports, which are evaluated as overrides on top of the base
`:root`), resolves `var()` aliases, takes every colour stop of gradient
tokens, and checks the PAIRS table below.

  python3 scripts/contrast-check.py               # check, print failures
  python3 scripts/contrast-check.py -v            # also print passing pairs
  python3 scripts/contrast-check.py --css path    # other stylesheet
  python3 scripts/contrast-check.py --self-test   # formula + parser fixture

Exit 0 = all pass; 1 = a pair fails or a token is missing / unparsable;
2 = usage / file error.

Thresholds (WCAG 2.x AA): text >= 4.5, large text >= 3.0, non-text >= 3.0.
A missing token is a FAILURE, never a skip. A translucent background token
cannot be judged offline and is a failure too (make the surface opaque, or
change the pair to a literal composite).
"""

import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cssparse as cp  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_CSS = os.path.join(REPO, "crates", "web", "style.css")

THRESHOLD = {"text": 4.5, "large": 3.0, "nontext": 3.0}

# ============================================================== PAIR TABLE
# Maintain here. Each entry: (foregrounds, backgrounds, kind, note).
# A name starting with "--" is a token; anything else is a CSS colour literal.
# "@signs" expands to every colour stop of each of the twelve --sign-<name>
# tokens (or --sign-<name>-* tokens, e.g. -deep / -light).

PAGE_BASES = ["#fff", "--void", "--deep-space", "--surface-tint"]

TEXT_TOKENS = [
  "--heading", "--text", "--silver-dim", "--silver-faint", "--rose",
  "--success", "--starlight", "--metal-mid", "--metal-hi",
]

SIGNS = [
  "aries", "taurus", "gemini", "cancer", "leo", "virgo", "libra", "scorpio",
  "sagittarius", "capricorn", "aquarius", "pisces",
]

PAIRS = [
  (TEXT_TOKENS, PAGE_BASES, "text", "body text tokens on page/card surfaces"),
  (["--ink-on-primary"], ["--primary", "--primary-hover"], "text",
   "primary button / selected state"),
  (["--ink-on-accent"], ["--accent"], "text", "pink accent button / NEW badge"),
  (["--ink-on-sign"], ["@signs"], "text",
   "zodiac card + planet pill, every gradient stop"),
  (["--metal-lo"], PAGE_BASES, "nontext", "control borders (F03)"),
  (["--accent"], ["#fff"], "nontext", "pink button edge on white (F18)"),
  # B3 inline styles (crates/web/src/**/*.rs)
  (["--ink-on-primary"], ["--primary", "--rose"], "text",
   "nav avatar initial on --primary -> --rose gradient (layout.rs)"),
  (["--heading", "--text", "--silver-dim", "--primary-deep"],
   ["--primary-soft", "--rose-soft", "--surface-tint"], "text",
   "generation panel, tier badge, '?' avatar, tinted sub-panels (profile/layout)"),
  (["--rose"], ["--rose-soft"], "text", "admin badge in nav dropdown (layout.rs)"),
  (["--text", "--success"], ["--success-soft"], "text",
   "admin invite-created panel (admin.rs)"),
  (["--primary", "--silver-faint"], ["#fff"], "nontext",
   "personality radar data / prior lines on white card"),
  # 星座配對 /zodiac（zodiac.rs；宮格鈕／結果卡字 --ink-on-sign 已由 @signs 涵蓋）
  (["--primary-deep"], ["#fff", "--primary-soft"], "text",
   "zodiac match row theme / label badge / western-page CTA"),
  (["--text", "--primary-deep"], ["--surface-tint"], "text",
   "zodiac match aspect badge / line key chip"),
  (["--primary-deep"], ["--sign-pisces-light"], "nontext", "home fi-match icon stroke"),
  # 紫微流年（/ziwei/luck 宮格疊層、/ziwei/year 摘要；ziwei.rs／palace_grid.rs）
  (["--ink-on-primary"], ["--primary", "--primary-deep"], "text",
   "ziwei badges 流年命宮 / 大限命宮・童限"),
  (["--primary-deep"], ["--surface-tint", "--primary-soft", "#fff"], "text",
   "ziwei 小限 badge, 流年宮名, timeline seg on, summary chip value"),
  (["--rose"], ["--rose-soft"], "text", "ziwei 流祿／流權／流科／流忌 tag"),
  (["--primary", "--primary-deep", "--metal-lo"], ["#fff", "--surface-tint"], "nontext",
   "ziwei badge shapes on .palace.life (white) and normal palace (tint)"),
]
# ==========================================================================


class TokenError(Exception):
  pass


VAR_RE = re.compile(r"var\(\s*(--[\w-]+)\s*(?:,([^()]*(?:\([^()]*\)[^()]*)*))?\)")


def resolve(value, tokens, deps, seen=()):
  """Substitute var() recursively. Records every token touched in deps."""
  def sub(m):
    name, fallback = m.group(1), m.group(2)
    deps.add(name)
    if name in seen:
      raise TokenError("var() 循環參照：%s" % " -> ".join(seen + (name,)))
    if name in tokens:
      return resolve(tokens[name][0], tokens, deps, seen + (name,))
    if fallback is not None:
      return resolve(fallback.strip(), tokens, deps, seen)
    raise TokenError("token %s 未定義（被 var() 引用）" % name)
  for _ in range(50):
    new = VAR_RE.sub(sub, value)
    if new == value:
      return new
    value = new
  return value


def colours_of(ref, tokens, deps):
  """ref -> list of (label, rgba). Gradients give every stop."""
  if ref.startswith("--"):
    deps.add(ref)
    if ref not in tokens:
      raise TokenError("token %s 不存在於 :root" % ref)
    raw = tokens[ref][0]
    text = resolve(raw, tokens, deps, (ref,))
  else:
    text = ref
  found = [m.group(1) for m in cp.COLOR_RE.finditer(text)]
  cols = []
  for lit in found:
    c = cp.parse_color(lit)
    if c is None:
      raise TokenError("%s 的色值 %r 無法解析" % (ref, lit))
    cols.append(c)
  if not cols:
    raise TokenError("%s 的值 %r 取不到任何色值" % (ref, text))
  multi = len(cols) > 1
  return [("%s[stop %d]" % (ref, i + 1) if multi else ref, c)
          for i, c in enumerate(cols)]


def expand_bg(ref, tokens):
  if ref != "@signs":
    return [ref]
  out = []
  for s in SIGNS:
    base = "--sign-" + s
    names = sorted(n for n in tokens
                   if n == base or n.startswith(base + "-"))
    if base not in tokens:
      # split form: both ends are required; keep missing names so they fail
      for end in ("-deep", "-light"):
        if base + end not in names:
          names.append(base + end)
    out.extend(names)
  return out


def evaluate(tokens, only_deps=None):
  """Return (results, errors). results: list of dicts. If only_deps is a set,
  pairs whose dependency set does not intersect it are skipped."""
  results, errors = [], []
  for fgs, bgs, kind, note in PAIRS:
    need = THRESHOLD[kind]
    bg_refs = [b for ref in bgs for b in expand_bg(ref, tokens)]
    for fg in fgs:
      for bg in bg_refs:
        deps = set()
        try:
          fcs = colours_of(fg, tokens, deps)
          bcs = colours_of(bg, tokens, deps)
        except TokenError as e:
          if only_deps is None or deps & only_deps:
            errors.append("%s × %s：%s" % (fg, bg, e))
          continue
        if only_deps is not None and not deps & only_deps:
          continue
        for fl, fc in fcs:
          for bl, bc in bcs:
            if bc[3] < 1.0:
              errors.append("%s × %s：底色半透明 (%s)，離線無法判定" %
                            (fl, bl, cp.fmt_color(bc)))
              continue
            ratio = cp.contrast(fc, bc)
            results.append({
              "fg": fl, "bg": bl, "fgc": cp.fmt_color(fc),
              "bgc": cp.fmt_color(bc), "ratio": ratio, "kind": kind,
              "need": need, "ok": ratio + 1e-9 >= need, "note": note,
            })
  return results, errors


def run(css_path, verbose):
  try:
    with open(css_path, encoding="utf-8") as f:
      css = f.read()
  except OSError as e:
    print("無法讀取 %s：%s" % (css_path, e), file=sys.stderr)
    return 2
  rel = os.path.relpath(css_path, REPO)
  ctxs = cp.root_tokens(css)
  if not ctxs:
    print("FAIL %s 找不到任何 :root 區塊" % rel)
    return 1

  base = {}
  for ctx in ctxs:
    if not ctx["chain"]:
      base.update(ctx["tokens"])
  runs = [("%s :root（基底，%d 個 token）" % (rel, len(base)), base, None)]
  for ctx in ctxs:
    if ctx["chain"]:
      merged = dict(base)
      merged.update(ctx["tokens"])
      label = "%s:%d %s :root 覆寫（%s）" % (
        rel, ctx["line"], " ".join(ctx["chain"]), ", ".join(sorted(ctx["tokens"])))
      runs.append((label, merged, set(ctx["tokens"])))

  total_fail = 0
  for label, toks, only in runs:
    results, errors = evaluate(toks, only)
    fails = [r for r in results if not r["ok"]]
    print("== %s" % label)
    if only is not None and not results and not errors:
      print("   （此覆寫不影響配對表內任何配對）")
    for e in errors:
      print("   FAIL  %s" % e)
    for r in results:
      if r["ok"] and not verbose:
        continue
      src = ""
      for name in (r["fg"].split("[")[0], r["bg"].split("[")[0]):
        if name in toks:
          src += " %s@L%d" % (name, toks[name][1])
      print("   %s  %-26s %-9s on %-28s %-9s %5.2f  (%s ≥%.1f)%s" % (
        "ok  " if r["ok"] else "FAIL", r["fg"], r["fgc"], r["bg"], r["bgc"],
        r["ratio"], r["kind"], r["need"], src))
    print("   -- %d 配對，%d 失敗，%d 錯誤" % (len(results), len(fails), len(errors)))
    total_fail += len(fails) + len(errors)
  print("結果：%s（共 %d 項失敗/錯誤）" % ("PASS" if not total_fail else "FAIL", total_fail))
  return 0 if not total_fail else 1


# ------------------------------------------------------------- self-test

FIXTURE_CSS = """
/* :root { --heading: #fff; } commented out, must be ignored */
:root {
  --heading: #2a1b3d;
  --text: var(--heading);
  --silver-dim: #5c4b70; --silver-faint: #6b5b7b;
  --rose: #be185d; --success: #047857; --starlight: #7c3aed;
  --metal-mid: #6d28d9; --metal-hi: #5b21b6; --metal-lo: #8b6fd6;
  --void: #fff5fa; --deep-space: #f4eeff; --surface-tint: #f6f1ff;
  --primary: #7c3aed; --primary-hover: #6d28d9; --ink-on-primary: #fff;
  --accent: #ec4899; --ink-on-accent: #2a0f1e; --ink-on-sign: var(--heading);
  --sign-aries: linear-gradient(135deg, #ff6f61, #ffb199);
  --sign-taurus: linear-gradient(135deg, #b5c334, #e0ffb3);
  --sign-gemini: linear-gradient(135deg, #ffd700, #fff7b2);
  --sign-cancer: linear-gradient(135deg, #00bcd4, #b2f7ff);
  --sign-leo: linear-gradient(135deg, #ff9800 0%, #ffe0b2 100%);
  --sign-virgo-deep: #8bc34a; --sign-virgo-light: #e6ffb2;
  --sign-libra: linear-gradient(135deg, #f472b6, #ffd1e8);
  --sign-scorpio: linear-gradient(135deg, #a98fe0, #e4d6ff);
  --sign-sagittarius: linear-gradient(135deg, #42a5f5, #b2d7ff);
  --sign-capricorn: linear-gradient(135deg, #a1887f, #e5d5cb);
  --sign-aquarius: linear-gradient(135deg, #26c6b0, #b2fff7);
  --sign-pisces: linear-gradient(135deg, rgb(127 143 230), #c9d4ff);
}
@media (pointer: coarse) {
  :root { --silver-faint: #9aa3b2; }
}
"""


def self_test():
  ok = True

  def check(name, cond, detail=""):
    nonlocal ok
    print("  %s  %s %s" % ("ok  " if cond else "FAIL", name, detail))
    ok = ok and cond

  print("公式：")
  for fg, bg, want in [("#ffffff", "#7c3aed", 5.70), ("#2a1b3d", "#a1887f", 4.80),
                       ("#9aa3b2", "#ffffff", 2.54)]:
    got = cp.contrast(cp.parse_color(fg), cp.parse_color(bg))
    check("%s on %s = %.2f" % (fg, bg, want), abs(got - want) <= 0.01, "(got %.3f)" % got)

  print("色值解析：")
  for lit, want in [("#fff", (255, 255, 255, 1.0)), ("#ffffff80", (255, 255, 255, 128 / 255)),
                    ("rgba(10, 23, 51, 0.72)", (10, 23, 51, 0.72)),
                    ("rgb(10 23 51 / 50%)", (10, 23, 51, 0.5)),
                    ("hsl(0, 100%, 50%)", (255, 0, 0, 1.0))]:
    got = cp.parse_color(lit)
    check(lit, got is not None and all(abs(a - b) < 0.6 for a, b in zip(got, want)), str(got))
  half = cp.composite((0, 0, 0, 0.5), (255, 255, 255, 1))
  check("alpha 合成 black@.5 over white = 127.5", abs(half[0] - 127.5) < 1e-6)

  print("fixture 解析：")
  ctxs = cp.root_tokens(FIXTURE_CSS)
  check("找到 2 個 :root（註解內的不算）", len(ctxs) == 2, str(len(ctxs)))
  check("第 2 個帶 @media 出處", ctxs[1]["chain"] == ("@media (pointer: coarse)",),
        str(ctxs[1]["chain"]))
  base = ctxs[0]["tokens"]
  results, errors = evaluate(base)
  check("基底無錯誤", not errors, "; ".join(errors))
  fails = [r for r in results if not r["ok"]]
  check("基底全數通過", not fails, ", ".join("%s/%s" % (r["fg"], r["bg"]) for r in fails))
  sign_pairs = [r for r in results if r["fg"] == "--ink-on-sign"]
  check("星座 24 個色標全部被量到", len(sign_pairs) == 24, str(len(sign_pairs)))
  cap = [r for r in sign_pairs if r["bg"].startswith("--sign-capricorn")][0]
  check("var() 別名解析 + capricorn 深端 4.80", abs(cap["ratio"] - 4.80) <= 0.01,
        "%.3f" % cap["ratio"])

  merged = dict(base)
  merged.update(ctxs[1]["tokens"])
  r2, e2 = evaluate(merged, set(ctxs[1]["tokens"]))
  check("覆寫只評估受影響配對（silver-faint × 4 底）", len(r2) == 4, str(len(r2)))
  check("覆寫 #9aa3b2 被抓到失敗", r2 and all(not r["ok"] for r in r2))

  broken = dict(base)
  del broken["--success"]
  del broken["--sign-libra"]
  _, e3 = evaluate(broken)
  check("缺 token 明確失敗（--success）", any("--success" in e for e in e3))
  check("缺星座 token 明確失敗（--sign-libra）", any("--sign-libra" in e for e in e3))
  broken3 = dict(base)
  del broken3["--sign-virgo-light"]
  _, e5 = evaluate(broken3)
  check("拆分式星座缺淺端明確失敗（--sign-virgo-light）",
        any("--sign-virgo-light" in e for e in e5))
  broken2 = dict(base)
  broken2["--void"] = ("rgba(255,255,255,0.5)", 0)
  _, e4 = evaluate(broken2)
  check("半透明底色明確失敗", any("半透明" in e for e in e4))

  print("self-test：%s" % ("PASS" if ok else "FAIL"))
  return 0 if ok else 1


def main():
  ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
  ap.add_argument("--css", default=DEFAULT_CSS)
  ap.add_argument("-v", "--verbose", action="store_true", help="也列出通過的配對")
  ap.add_argument("--self-test", action="store_true")
  a = ap.parse_args()
  if a.self_test:
    return self_test()
  return run(a.css, a.verbose)


if __name__ == "__main__":
  sys.exit(main())
