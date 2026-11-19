"""Shared helpers for scripts/contrast-check.py and scripts/css-guard.py.

Standard library only. Not a full CSS parser: enough structure to find
`:root` blocks (also inside @media / @supports), tell selector preludes from
declaration bodies, strip comments while keeping line numbers, and parse
CSS color values (hex / rgb / rgba / hsl / hsla / a few keywords).
"""

import re

# ---------------------------------------------------------------- comments


def strip_css_comments(src):
  """Blank out /* ... */ (newlines kept so line numbers stay valid)."""
  out = []
  i, n = 0, len(src)
  in_str = None
  while i < n:
    c = src[i]
    if in_str:
      out.append(c)
      if c == "\\" and i + 1 < n:
        out.append(src[i + 1])
        i += 2
        continue
      if c == in_str:
        in_str = None
      i += 1
      continue
    if c in "\"'":
      in_str = c
      out.append(c)
      i += 1
      continue
    if src.startswith("/*", i):
      j = src.find("*/", i + 2)
      j = n if j < 0 else j + 2
      out.append(re.sub(r"[^\n]", " ", src[i:j]))
      i = j
      continue
    out.append(c)
    i += 1
  return "".join(out)


def strip_rust_comments(src):
  """Blank out Rust // and (nested) /* */ comments; string, raw-string and
  char literals are kept intact (so `https://` inside a string survives)."""
  out = []
  i, n = 0, len(src)

  def blank(s):
    return re.sub(r"[^\n]", " ", s)

  while i < n:
    c = src[i]
    # line comment
    if src.startswith("//", i):
      j = src.find("\n", i)
      j = n if j < 0 else j
      out.append(blank(src[i:j]))
      i = j
      continue
    # block comment (nested)
    if src.startswith("/*", i):
      depth, j = 1, i + 2
      while j < n and depth:
        if src.startswith("/*", j):
          depth += 1
          j += 2
        elif src.startswith("*/", j):
          depth -= 1
          j += 2
        else:
          j += 1
      out.append(blank(src[i:j]))
      i = j
      continue
    # raw string r"..." / r#"..."# / br#"..."#
    m = re.match(r'b?r(#*)"', src[i:i + 300])
    if m and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
      hashes = m.group(1)
      end = '"' + hashes
      j = src.find(end, i + m.end())
      j = n if j < 0 else j + len(end)
      out.append(src[i:j])
      i = j
      continue
    # normal string
    if c == '"':
      j = i + 1
      while j < n:
        if src[j] == "\\":
          j += 2
          continue
        if src[j] == '"':
          j += 1
          break
        j += 1
      out.append(src[i:j])
      i = j
      continue
    # char literal vs lifetime
    if c == "'":
      m = re.match(r"'(\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'\n])'",
                   src[i:i + 16])
      if m:
        out.append(m.group(0))
        i += m.end()
        continue
    out.append(c)
    i += 1
  return "".join(out)


# ---------------------------------------------------------------- structure


GROUP_AT = ("@media", "@supports", "@layer", "@container", "@document",
            "@keyframes", "@-webkit-keyframes", "@scope")


def css_blocks(clean):
  """Yield (prelude, body_start, body_end, at_chain) for every block whose
  body holds declarations. `clean` must be comment-stripped. at_chain is the
  tuple of enclosing @-rule preludes (e.g. ('@media (pointer: coarse)',))."""
  stack = []  # entries: (prelude, kind, start)
  prelude_start = 0
  i, n = 0, len(clean)
  in_str = None
  while i < n:
    c = clean[i]
    if in_str:
      if c == "\\":
        i += 2
        continue
      if c == in_str:
        in_str = None
      i += 1
      continue
    if c in "\"'":
      in_str = c
    elif c == "{":
      prelude = " ".join(clean[prelude_start:i].split())
      parent_kind = stack[-1][1] if stack else "group"
      if parent_kind == "decl":
        kind = "decl"  # nested rule inside a declaration block (CSS nesting)
      elif prelude.startswith(GROUP_AT):
        kind = "group"
      else:
        kind = "decl"
      stack.append((prelude, kind, i + 1))
      prelude_start = i + 1
    elif c == "}":
      if stack:
        prelude, kind, start = stack.pop()
        if kind == "decl":
          chain = tuple(p for p, k, _ in stack if k == "group")
          yield prelude, start, i, chain
      prelude_start = i + 1
    elif c == ";" and (not stack or stack[-1][1] == "group"):
      prelude_start = i + 1  # @import / @charset statements
    i += 1


def decl_mask(clean):
  """Boolean list: True where the char lies inside a declaration body."""
  mask = [False] * len(clean)
  for _, s, e, _ in css_blocks(clean):
    for k in range(s, e):
      mask[k] = True
  # nested selector preludes inside declaration bodies stay masked as True;
  # acceptable — this site does not use CSS nesting.
  return mask


def is_root_prelude(prelude):
  return any(sel.strip() == ":root" for sel in prelude.split(","))


def root_blocks(clean):
  """List of (at_chain, body_start, body_end) for every `:root` block."""
  return [(chain, s, e) for p, s, e, chain in css_blocks(clean)
          if is_root_prelude(p)]


def split_decls(body, offset):
  """Split a declaration body on top-level ';'. Yields (name, value, pos)."""
  depth = 0
  start = 0
  in_str = None
  for i, c in enumerate(body + ";"):
    if in_str:
      if c == in_str:
        in_str = None
      continue
    if c in "\"'":
      in_str = c
    elif c == "(":
      depth += 1
    elif c == ")":
      depth -= 1
    elif c == ";" and depth <= 0:
      part = body[start:i]
      if ":" in part:
        name, value = part.split(":", 1)
        lead = len(part) - len(part.lstrip())
        yield name.strip(), " ".join(value.split()), offset + start + lead
      start = i + 1


def line_of(text, pos):
  return text.count("\n", 0, pos) + 1


def root_tokens(css_text):
  """Return list of contexts: [{'chain': tuple, 'line': int, 'tokens':
  {name: (value, line)}}] — one per `:root` block, in file order."""
  clean = strip_css_comments(css_text)
  out = []
  for chain, s, e in root_blocks(clean):
    toks = {}
    for name, value, pos in split_decls(clean[s:e], s):
      if name.startswith("--"):
        toks[name] = (value, line_of(clean, pos))
    out.append({"chain": chain, "line": line_of(clean, s), "tokens": toks})
  return out


# ---------------------------------------------------------------- colors

NAMED = {
  "white": (255, 255, 255, 1.0),
  "black": (0, 0, 0, 1.0),
  "transparent": (0, 0, 0, 0.0),
}

HEX_RE = r"#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})(?![0-9a-zA-Z_-])"
FUNC_RE = r"(?:rgba?|hsla?)\([^()]*\)"
COLOR_RE = re.compile(
  r"(?<![\w&#-])(" + HEX_RE + r"|" + FUNC_RE +
  r"|\b(?:white|black|transparent)\b)", re.I)


def _num(tok, scale):
  tok = tok.strip()
  if tok.endswith("%"):
    return float(tok[:-1]) / 100.0 * scale
  return float(tok)


def _hsl_to_rgb(h, s, l):
  h = (h % 360) / 360.0
  def f(p, q, t):
    t %= 1.0
    if t < 1 / 6:
      return p + (q - p) * 6 * t
    if t < 1 / 2:
      return q
    if t < 2 / 3:
      return p + (q - p) * (2 / 3 - t) * 6
    return p
  if s == 0:
    v = l
    return v * 255, v * 255, v * 255
  q = l * (1 + s) if l < 0.5 else l + s - l * s
  p = 2 * l - q
  return (f(p, q, h + 1 / 3) * 255, f(p, q, h) * 255, f(p, q, h - 1 / 3) * 255)


def parse_color(s):
  """Parse one CSS color literal -> (r, g, b, a) floats, or None."""
  s = s.strip().lower()
  if s in NAMED:
    return NAMED[s]
  if s.startswith("#"):
    h = s[1:]
    if len(h) in (3, 4):
      h = "".join(ch * 2 for ch in h)
    if len(h) not in (6, 8) or not re.fullmatch(r"[0-9a-f]+", h):
      return None
    r, g, b = int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)
    a = int(h[6:8], 16) / 255.0 if len(h) == 8 else 1.0
    return (r, g, b, a)
  m = re.fullmatch(r"(rgba?|hsla?)\((.*)\)", s)
  if not m:
    return None
  fn, args = m.group(1), m.group(2)
  if "/" in args:
    main, alpha = args.split("/", 1)
    parts = re.split(r"[\s,]+", main.strip())
    parts.append(alpha.strip())
  else:
    parts = re.split(r"[\s,]+", args.strip())
  parts = [p for p in parts if p]
  try:
    a = _num(parts[3], 1.0) if len(parts) > 3 else 1.0
    if fn.startswith("rgb"):
      r, g, b = (_num(p, 255.0) for p in parts[:3])
    else:
      hue = float(parts[0].replace("deg", ""))
      r, g, b = _hsl_to_rgb(hue, _num(parts[1], 1.0), _num(parts[2], 1.0))
  except (ValueError, IndexError):
    return None
  return (r, g, b, max(0.0, min(1.0, a)))


def composite(fg, bg):
  """Alpha-composite fg over an opaque bg."""
  a = fg[3]
  return (fg[0] * a + bg[0] * (1 - a), fg[1] * a + bg[1] * (1 - a),
          fg[2] * a + bg[2] * (1 - a), 1.0)


def rel_lum(c):
  def ch(v):
    v /= 255.0
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
  return 0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])


def contrast(fg, bg):
  """WCAG 2.x contrast; fg may be translucent (composited on bg)."""
  if bg[3] < 1.0:
    raise ValueError("background must be opaque")
  if fg[3] < 1.0:
    fg = composite(fg, bg)
  l1, l2 = rel_lum(fg), rel_lum(bg)
  hi, lo = max(l1, l2), min(l1, l2)
  return (hi + 0.05) / (lo + 0.05)


def fmt_color(c):
  r, g, b, a = c
  hx = "#%02x%02x%02x" % (round(r), round(g), round(b))
  return hx if a >= 1.0 else "%s@%.2f" % (hx, a)
