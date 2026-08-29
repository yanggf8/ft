# Cosmic Silver Theme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reskin ft-web from light indigo to a dark cosmic-silver theme: deep-space black, silver star palette, blue-violet interaction accent, a CSS static star field plus a canvas Milky-Way particle layer with shooting stars.

**Architecture:** Pure frontend reskin — zero Rust/wasm changes. All styling lives in `crates/web/style.css` (semantic classes from Leptos components are reused as-is). Decorative starfield elements are static divs in `crates/web/index.html` (they survive Leptos's mount-to-body append). One new vanilla JS file (`galaxy.js`) renders band-distributed particles on a full-viewport canvas; it never enters the wasm bundle. Shipped in two phases per spec §6a: theme first, galaxy second.

**Tech Stack:** Plain CSS (custom properties, box-shadow star painting, keyframes), vanilla ES5-style JS + Canvas 2D API, Google Fonts (`Playfair Display` 700 + `Noto Serif TC` 700), bash build script.

**Spec:** `docs/superpowers/specs/2026-08-30-cosmic-silver-theme-design.md` — the plan argues from the spec; executors read both. Section references below (§1-§8) point into the spec.

## Global Constraints

- **No Rust changes.** Do not touch any `crates/**/*.rs`, `Cargo.toml`, or `crates/schema`. CI gates on `cargo fmt --check`; nothing here may affect it.
- **No gold anywhere.** `#D4AF37` and `#FEF3C7` must not survive the reskin (`grep -n "D4AF37\|FEF3C7" crates/web/style.css` must return nothing at the end).
- **WCAG AA on the glass composite ground** (`#1C202A` ≈ white@5% over `#10141F`), not on bare deep-space. Spec §1's token set is already verified to pass; any token you change must be re-verified against the composite.
- **`prefers-reduced-motion` must fully degrade**: no canvas, frozen CSS animations at explicit static opacity. The site must look complete with JS disabled or canvas failed.
- **z-index scheme is mandatory** (spec §3): `.sky-nebula` -4, star layers -3, `#galaxy-canvas` -2, content static. Never omit these.
- **Build discipline:** `./scripts/build-web.sh` is a cargo release wasm build — **ask the user before running it** (heavy builds can hang this machine), never run two builds concurrently. No `wrangler` commands, ever (deployment is manual/OAuth, user-owned).
- **Commits:** conventional-commit style, one commit per task, ending each message with `Co-Authored-By: Claude Code <noreply@anthropic.com>`. The human gates every commit (review diff before proposing).

## File Structure

| File | Responsibility | Phase |
|---|---|---|
| `crates/web/scripts/gen-stars.js` (new) | Throwaway generator emitting two `box-shadow` star-coordinate lists to stdout; output is pasted into style.css with a generated banner | 1 |
| `crates/web/index.html` | Font preconnects + Google Fonts link (head); three fixed sky divs (body) — later the canvas + galaxy.js script tag | 1, then 2 |
| `crates/web/style.css` | Tokens, sky layers, full dark reskin of every existing class | 1 |
| `crates/web/galaxy.js` (new) | Canvas particle system: band sampling, toroidal wrap, twinkle/drift, shooting stars, reduced-motion + failure bail-outs | 2 |
| `crates/web/scripts/build-web.sh` | Add `cp galaxy.js dist/` to the fixed copy list (currently lines 15-17 copy only index.html + style.css) | 2 |

Phases: Tasks 1-8 = Phase 1 (theme; shippable alone), Tasks 9-12 = Phase 2 (galaxy).

---

### Task 1: Star-coordinate generator (`gen-stars.js`)

**Files:**
- Create: `crates/web/scripts/gen-stars.js`

**Interfaces:**
- Consumes: nothing (standalone node script).
- Produces: stdout — two CSS `box-shadow` lists (`STARS_A` = 30 dots at 1px, `STARS_B` = 30 dots at 2px), one `Xvw Yvh 0 0 <color>` entry per star. Task 3 pastes this output verbatim into style.css.

- [ ] **Step 1: Write the script**

```js
#!/usr/bin/env node
// gen-stars.js — throwaway generator for the cosmic-silver star field
// (spec §3). Run once from crates/web: node scripts/gen-stars.js
// Output is pasted into style.css under a "generated — do not edit" banner.
'use strict';

function mulberry32(a) {
  return function () {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    var t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function layer(count, dim) {
  var rnd = mulberry32(dim ? 0x5EEDB : 0x5EEDA);
  var parts = [];
  for (var i = 0; i < count; i++) {
    var x = (rnd() * 100).toFixed(2);
    var y = (rnd() * 100).toFixed(2);
    var a = (dim ? 0.35 + rnd() * 0.4 : 0.5 + rnd() * 0.5).toFixed(2);
    parts.push(x + 'vw ' + y + 'vh 0 0 rgba(229,228,226,' + a + ')');
  }
  if (parts.length !== count) throw new Error('bad count'); // sanity
  return parts.join(',\n  ');
}

var a = layer(30, false); // .sky-stars-a — 1px, brighter
var b = layer(30, true);  // .sky-stars-b — 2px, dimmer
if (!/^[\d.]+vw/.test(a) || !/^[\d.]+vw/.test(b)) throw new Error('bad format');
console.log('-- STARS-A --\n' + a + '\n-- STARS-B --\n' + b);
```

- [ ] **Step 2: Run and verify the output format**

Run: `cd crates/web && node scripts/gen-stars.js`
Expected: two labelled blocks; every line matches `N.NNvw N.NNvh 0 0 rgba(229,228,226,0.NN)`; 30 entries per block (count the `,` separators + 1). No exceptions thrown.

- [ ] **Step 3: Commit**

```bash
git add crates/web/scripts/gen-stars.js
git commit -m "feat(web): star-coordinate generator for cosmic-silver sky"
```

### Task 2: `index.html` — fonts + static sky elements

**Files:**
- Modify: `crates/web/index.html`

**Interfaces:**
- Consumes: nothing yet (Task 3 styles the sky divs).
- Produces: `<div class="sky-nebula">`, `<div class="sky-stars-a">`, `<div class="sky-stars-b">` fixed elements present in the DOM before Leptos mounts; font families `Playfair Display` + `Noto Serif TC` (700) available for Task 3's heading rule. Task 10 later appends the canvas + script tag to this file.

- [ ] **Step 1: Edit index.html to its full Phase-1 form**

Replace the whole file content with:

```html
<!doctype html>
<html lang="zh-TW">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>FortuneT - 命理分析</title>
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link
      rel="stylesheet"
      href="https://fonts.googleapis.com/css2?family=Playfair+Display:wght@700&family=Noto+Serif+TC:wght@700&display=swap"
    />
    <link rel="stylesheet" href="/style.css" />
  </head>
  <body>
    <div class="sky-nebula" aria-hidden="true"></div>
    <div class="sky-stars-a" aria-hidden="true"></div>
    <div class="sky-stars-b" aria-hidden="true"></div>
    <script type="module">
      import init from '/wasm/ft_web.js';
      await init();
    </script>
  </body>
</html>
```

Notes: the wasm module script must stay exactly as-is (it is the app entry). `aria-hidden="true"` keeps the decorative divs out of the accessibility tree. Both preconnects precede the fonts stylesheet.

- [ ] **Step 2: Verify structure**

Run: `grep -c 'preconnect' crates/web/index.html && grep -c 'sky-' crates/web/index.html`
Expected: `2` and `3`.

- [ ] **Step 3: Commit**

```bash
git add crates/web/index.html
git commit -m "feat(web): load serif fonts + fixed sky layers in index.html"
```

### Task 3: `style.css` — tokens, sky layers, reduced-motion

**Files:**
- Modify: `crates/web/style.css` (prepend new sections; leave existing rules for Tasks 4-7)

**Interfaces:**
- Consumes: Task 1's generator output (pasted verbatim), Task 2's `.sky-*` divs.
- Produces: the `:root` token set used by every later task (exact names below); `.sky-nebula`, `.sky-stars-a`, `.sky-stars-b`, `@keyframes twinkleA/twinkleB`; the reduced-motion block. Later tasks reference tokens `--void --deep-space --heading --text --silver-dim --silver-faint --starlight --nebula --nebula-strong --nebula-deep --rose --glass-bg --glass-bg-strong --glass-border`.

- [ ] **Step 1: Prepend the token + sky sections to style.css**

Insert at the very top of the file, above the existing `* { ... }` reset:

```css
/* ═══ cosmic-silver tokens (spec §1) ═══ */
:root {
  --void: #0c0c0c;
  --deep-space: #10141f;
  --heading: #e5e4e2;
  --text: #dde2ec;
  --silver-dim: #a9b0bc;
  --silver-faint: #8a919c;
  --starlight: #f5f7fa;
  --nebula: #8b93f8;
  --nebula-strong: #a5acfa;
  --nebula-deep: #312e81;
  --rose: #fb7185;
  --glass-bg: rgba(255, 255, 255, 0.05);
  --glass-bg-strong: rgba(16, 20, 31, 0.72);
  --glass-border: rgba(255, 255, 255, 0.1);
}

/* ═══ deep-space ground (spec §3 layer 1) ═══ */
body {
  background:
    radial-gradient(120vw 90vh at 15% 8%, rgba(49, 46, 129, 0.10), transparent 60%),
    radial-gradient(90vw 70vh at 85% 92%, rgba(139, 147, 248, 0.05), transparent 55%),
    linear-gradient(135deg, var(--void), var(--deep-space));
  background-attachment: fixed;
  color: var(--text);
}

/* ═══ starfield: fixed decorative layers (spec §3) ═══ */
.sky-nebula,
.sky-stars-a,
.sky-stars-b {
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: -3;
}
.sky-nebula { z-index: -4; }

/* band haze — same -18deg axis as the canvas particles (spec §4) */
.sky-nebula::before {
  content: '';
  position: absolute;
  left: -40%;
  right: -40%;
  top: 50%;
  height: 130vmax;
  transform: translateY(-50%) rotate(-18deg);
  background:
    radial-gradient(45% 12% at 50% 50%, rgba(245, 247, 250, 0.05), transparent 70%),
    radial-gradient(70% 20% at 50% 50%, rgba(245, 247, 250, 0.03), transparent 75%),
    radial-gradient(55% 16% at 50% 50%, rgba(76, 81, 140, 0.06), transparent 70%);
}

/* static star field — generated by scripts/gen-stars.js, do not edit */
.sky-stars-a,
.sky-stars-b {
  border-radius: 50%;
  animation: twinkleA 3.5s ease-in-out infinite alternate;
}
.sky-stars-a { width: 1px; height: 1px; box-shadow: <PASTE STARS-A BLOCK HERE>; }
.sky-stars-b {
  width: 2px; height: 2px;
  animation-name: twinkleB;
  animation-duration: 7s;
  animation-delay: -2.2s;
  box-shadow: <PASTE STARS-B BLOCK HERE>;
}
@keyframes twinkleA { from { opacity: 1; } to { opacity: 0.35; } }
@keyframes twinkleB { from { opacity: 0.8; } to { opacity: 0.3; } }

/* serif headings (spec §2) — Latin/digits via Playfair, CJK via Noto Serif TC */
h1, h2, h3, h4, h5, h6 {
  font-family: 'Playfair Display', 'Noto Serif TC', serif;
}

/* ═══ reduced motion: freeze to a static sky (spec §4) ═══ */
@media (prefers-reduced-motion: reduce) {
  .sky-stars-a,
  .sky-stars-b {
    animation: none;
    opacity: 0.7;
  }
}
```

Run the generator from Task 1 and replace `<PASTE STARS-A BLOCK HERE>` / `<PASTE STARS-B BLOCK HERE>` with its two blocks verbatim.

- [ ] **Step 2: Static checks**

Run: `grep -c 'z-index: -[34]' crates/web/style.css`
Expected: `2` (shared rule `-3` once, `.sky-nebula` override `-4` once; canvas `-2` arrives in Task 10).
Run: `grep -c 'vw ' crates/web/style.css`
Expected: ≥ 60 (generator output landed).

- [ ] **Step 3: Visual check (build-gated)**

Ask the user, then run `./scripts/build-web.sh` once; user opens the page. Expected: near-black ground with a faint inclined haze band; 60 tiny stars; headings in serif Chinese. Content still readable on the old light-theme card colors (they are restyled in Tasks 4-7 — that mismatch is expected mid-phase).

- [ ] **Step 4: Commit**

```bash
git add crates/web/style.css
git commit -m "feat(web): cosmic-silver tokens + CSS starfield layers"
```

### Task 4: `style.css` — global, layout, shared darkening

**Files:**
- Modify: `crates/web/style.css` (sections: reset/body/`a`/`button`/`input`, "layout", "shared")

**Interfaces:**
- Consumes: Task 3's tokens.
- Produces: dark global ground and shared surfaces (`.card`, buttons, links) that page-level tasks build on.

- [ ] **Step 1: Update the base element rules**

Keep the existing reset (`* { box-sizing ... }`). Replace the original `body` rule (font-family/line-height/color/background) with — the background gradient already lives in the Task 3 block, so this rule must not redeclare one:

```css
body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell', sans-serif;
  line-height: 1.6;
}
```

Replace `a` / `input, select` / focus rules:

```css
a { color: var(--nebula); text-decoration: none; }
a:hover { text-decoration: underline; }
button { font-family: inherit; cursor: pointer; }
button:disabled { opacity: 0.6; cursor: not-allowed; }
input, select {
  font-family: inherit;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: 4px;
  color: var(--text);
}
input::placeholder { color: var(--silver-faint); }
input:focus, select:focus {
  outline: 2px solid var(--starlight);
  outline-offset: 1px;
  border-color: var(--nebula);
}
```

- [ ] **Step 2: Replace the "layout" section**

```css
.app { min-height: 100vh; display: flex; flex-direction: column; }
.nav {
  background: var(--glass-bg-strong);
  -webkit-backdrop-filter: blur(20px);
  backdrop-filter: blur(20px);
  border-bottom: 1px solid var(--glass-border);
  color: var(--text);
  padding: 1rem 2rem;
  display: flex; justify-content: space-between; align-items: center;
  flex-wrap: wrap; gap: 1rem;
}
.nav a { color: var(--silver-dim); }
.nav a:hover { color: var(--starlight); }
.nav-brand { color: var(--heading); font-size: 1.25rem; font-weight: bold; text-decoration: none; }
.nav-links { display: flex; gap: 1rem; align-items: center; }
.nav-logout {
  background: transparent; border: 1px solid var(--glass-border); color: var(--silver-dim);
  padding: 0.25rem 0.75rem; border-radius: 4px;
}
.nav-logout:hover { border-color: var(--silver-dim); color: var(--starlight); }
main { flex: 1; }
.footer { padding: 1rem; text-align: center; font-size: 0.875rem; color: var(--silver-faint); }
```

- [ ] **Step 3: Replace the "shared" section**

```css
.card {
  background: var(--glass-bg);
  -webkit-backdrop-filter: blur(20px);
  backdrop-filter: blur(20px);
  border: 1px solid var(--glass-border);
  padding: 1.5rem; border-radius: 8px;
  margin-bottom: 1.5rem;
}
.page { padding: 1.5rem; max-width: 800px; margin: 0 auto; }
.page-narrow { padding: 1.5rem; max-width: 600px; margin: 0 auto; }
.center-note { padding: 2rem; text-align: center; }
.error { color: var(--rose); margin-bottom: 1rem; font-size: 0.875rem; }
.muted { color: var(--silver-faint); }
.btn-primary {
  background: var(--nebula); color: var(--void); padding: 0.75rem 1.5rem;
  border-radius: 6px; border: none; cursor: pointer; font-size: 1rem;
  font-weight: 500;
}
.btn-primary:hover { background: var(--nebula-strong); box-shadow: 0 0 12px rgba(165, 172, 250, 0.35); }
.btn-block { width: 100%; padding: 1rem 1.5rem; border-radius: 8px; border: none; font-size: 1rem; }
.btn-link, .back-link { background: none; border: none; color: var(--nebula); cursor: pointer; }
.back-link { margin-bottom: 1rem; }
.prose { white-space: pre-wrap; line-height: 1.8; }
```

- [ ] **Step 4: Verify no light-theme leftovers in the touched sections**

Run: `grep -n '#f9fafb\|#333\|#4F46E5\|#6b7280\|#f3f4f6\|white' crates/web/style.css`
Expected: no hits in the base/layout/shared regions (hits in "home"/"forms"/later sections are fine until their task).

- [ ] **Step 5: Commit**

```bash
git add crates/web/style.css
git commit -m "feat(web): darken global, layout and shared surfaces"
```

### Task 5: `style.css` — home + forms darkening

**Files:**
- Modify: `crates/web/style.css` (sections "home", "forms")

**Interfaces:**
- Consumes: Task 3 tokens, Task 4 buttons.
- Produces: restyled `.hero*`, `.cta*`, `.feature*`, `.auth-page`, `.field*`, `.form-*`, `.hour-*`.

- [ ] **Step 1: Replace the "home" section**

Traps here: `.hero-head h1` and `.feature h3` use hardcoded `#1f2937` — dark text that vanishes on the dark ground (spec §5).

```css
.hero { padding: 3rem 2rem; max-width: 1200px; margin: 0 auto; }
.hero-head { text-align: center; margin-bottom: 3rem; }
.hero-head h1 { font-size: 2.5rem; margin-bottom: 1rem; color: var(--heading); }
.hero-sub { font-size: 1.25rem; color: var(--silver-dim); margin-bottom: 2rem; }
.hero-actions { display: flex; gap: 1rem; justify-content: center; flex-wrap: wrap; }
.cta {
  background: var(--nebula); color: var(--void); padding: 0.75rem 2rem;
  border-radius: 8px; text-decoration: none; font-weight: 500;
}
.cta:hover { background: var(--nebula-strong); box-shadow: 0 0 12px rgba(165, 172, 250, 0.35); }
.cta-alt {
  background: var(--glass-bg); color: var(--nebula); padding: 0.75rem 2rem;
  border-radius: 8px; text-decoration: none; font-weight: 500;
  border: 2px solid var(--nebula);
}
.cta-alt:hover { background: var(--nebula-deep); }
.feature-grid {
  display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 2rem; margin-top: 3rem;
}
.feature {
  background: var(--glass-bg); border: 1px solid var(--glass-border);
  -webkit-backdrop-filter: blur(20px); backdrop-filter: blur(20px);
  padding: 2rem; border-radius: 8px;
}
.feature h3 { font-size: 1.25rem; margin-bottom: 0.5rem; color: var(--heading); }
.feature p { color: var(--silver-dim); }
```

- [ ] **Step 2: Replace the "forms" section**

```css
.auth-page { padding: 2rem; max-width: 400px; margin: 3rem auto; }
.field { margin-bottom: 1rem; }
.field label { display: block; margin-bottom: 0.5rem; font-size: 0.875rem; font-weight: 500; color: var(--silver-dim); }
.field input { width: 100%; padding: 0.75rem; font-size: 1rem; }
.form-grid3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.5rem; }
.form-col { display: flex; flex-direction: column; gap: 1rem; }
.form-label { font-size: 0.875rem; color: var(--silver-faint); }
.form-input {
  padding: 0.5rem; border: 1px solid var(--glass-border); border-radius: 4px;
  width: 100%; background: var(--glass-bg); color: var(--text);
}
.hour-row { display: flex; gap: 0.5rem; align-items: center; }
.hour-unknown { display: flex; align-items: center; gap: 0.25rem; font-size: 0.875rem; color: var(--silver-dim); }
```

- [ ] **Step 3: Verify**

Run: `grep -n '#1f2937\|#6b7280\|white' crates/web/style.css | sed -n '1,10p'`
Expected: zero hits in "home"/"forms" sections.

- [ ] **Step 4: Commit**

```bash
git add crates/web/style.css
git commit -m "feat(web): darken home hero and form styles"
```

### Task 6: `style.css` — palace grid darkening

**Files:**
- Modify: `crates/web/style.css` (section "ziwei palace grid")

**Interfaces:**
- Consumes: Task 3 tokens (incl. `--rose`).
- Produces: restyled `.chart-meta`, `.palace*`, `.star*` classes.

- [ ] **Step 1: Replace the "ziwei palace grid" section**

The headline fix: `.star.main`'s amber `#FEF3C7` is the spec's explicit "no gold anywhere" violation — it becomes a silver pill. Transformation stars are `--rose` text on dark glass, not filled alarms (spec §1 note).

```css
.chart-meta { display: flex; flex-wrap: wrap; gap: 1rem; font-size: 0.9rem; color: var(--silver-dim); }
.palace-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.5rem; }
.palace {
  border: 1px solid var(--glass-border); border-radius: 8px; padding: 0.5rem;
  background: rgba(16, 20, 31, 0.55);
}
.palace.life { border: 2px solid var(--nebula); background: rgba(49, 46, 129, 0.35); }
.palace-head { font-weight: 600; font-size: 0.85rem; color: var(--heading); }
.palace-stars { margin-top: 0.25rem; display: flex; flex-wrap: wrap; gap: 0.25rem; }
.star { font-size: 0.8rem; padding: 0.15rem 0.35rem; border-radius: 4px; background: rgba(255, 255, 255, 0.07); color: var(--silver-dim); }
.star.main { background: rgba(229, 228, 226, 0.14); color: var(--starlight); }
.star.transformation { background: rgba(251, 113, 133, 0.12); color: var(--rose); }
```

- [ ] **Step 2: Verify the gold violation is gone**

Run: `grep -n 'FEF3C7\|D4AF37' crates/web/style.css`
Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add crates/web/style.css
git commit -m "feat(web): dark glass palace grid, silver main stars"
```

### Task 7: `style.css` — OCEAN + quiz darkening, scrollbar polish

**Files:**
- Modify: `crates/web/style.css` (sections "personality result" ocean-\*, "personality quiz" quiz-\*; append scrollbar rules)

**Interfaces:**
- Consumes: Task 3 tokens.
- Produces: restyled `.ocean-*`, `.quiz-*`; dark scrollbar.

- [ ] **Step 1: Replace the OCEAN block**

Traps: `.ocean-copy`/`.ocean-foot strong` use `#4B5563`; `.ocean-marker` has a white border/ring.

```css
.ocean-dim {
  background: rgba(255, 255, 255, 0.04);
  border-radius: 8px;
  padding: 1rem 1.125rem 1.125rem;
  margin-bottom: 0.75rem;
}
.ocean-dim:last-of-type { margin-bottom: 0; }
.ocean-dim-head { display: flex; justify-content: space-between; align-items: baseline; gap: 1rem; margin-bottom: 0.375rem; }
.ocean-dim-head strong { font-size: 0.9375rem; color: var(--heading); font-weight: 600; }
.ocean-score {
  font-variant-numeric: tabular-nums;
  font-size: 1.375rem; font-weight: 700; line-height: 1; letter-spacing: -0.02em;
  color: var(--nebula);
}
.ocean-track-wrap { padding: 10px 12px 8px; }
.ocean-track { position: relative; height: 8px; background: rgba(255, 255, 255, 0.08); border-radius: 999px; }
.ocean-band { position: absolute; top: 0; height: 100%; background: rgba(139, 147, 248, 0.45); border-radius: 999px; }
.ocean-mean {
  position: absolute; top: -5px; width: 2px; height: 18px;
  background: var(--silver-faint); border-radius: 1px; transform: translateX(-50%);
}
.ocean-marker {
  position: absolute; top: 50%;
  width: 14px; height: 14px;
  background: var(--starlight);
  border: 2px solid var(--deep-space);
  box-shadow: 0 0 0 1px var(--nebula);
  border-radius: 50%;
  transform: translate(-50%, -50%);
}
.ocean-caption { font-size: 0.75rem; color: var(--silver-faint); margin: 0.375rem 0 0; }
.ocean-copy { margin-top: 0.75rem; font-size: 0.9375rem; line-height: 1.65; color: var(--silver-dim); }
.ocean-foot { margin-top: 1.5rem; padding-top: 1.25rem; border-top: 1px solid var(--glass-border); }
.ocean-foot strong { display: block; font-size: 0.875rem; font-weight: 600; color: var(--silver-dim); }
.ocean-foot > .muted { font-size: 0.8rem; margin-top: 0.375rem; }
.ocean-foot details { margin-top: 1rem; }
.ocean-foot summary { color: var(--silver-faint); font-size: 0.875rem; cursor: pointer; }
.ocean-foot .actions { display: flex; flex-wrap: wrap; gap: 0.75rem; margin-top: 1.25rem; }
```

- [ ] **Step 2: Replace the quiz block**

Traps: `.quiz-no` indigo-on-`#EEF2FF` pill; `.quiz-key span::before` hardcoded indigo counters; `.quiz-submit` white sticky bar and `#E5E7EB`/`#9CA3AF` disabled state; `.quiz-item legend` `#1F2937`.

```css
.quiz-head { display: flex; justify-content: space-between; gap: 1rem; align-items: start; margin-bottom: 1rem; }
.quiz-head .btn-link { flex-shrink: 0; font-size: 0.875rem; white-space: nowrap; }
.quiz-consent { font-size: 0.8125rem; margin-bottom: 1.25rem; color: var(--silver-dim); }
.quiz-progress { font-size: 0.8125rem; color: var(--silver-faint); margin: 0; font-variant-numeric: tabular-nums; }
.quiz-progress strong { color: var(--nebula); font-weight: 700; }
.quiz-progress-bar { height: 3px; background: rgba(255, 255, 255, 0.08); border-radius: 999px; margin: 0.5rem 0 1.25rem; overflow: hidden; }
.quiz-progress-bar > span { display: block; height: 100%; background: var(--nebula); border-radius: inherit; }
.quiz-key {
  display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 0.375rem;
  padding: 0 1rem 0.75rem; font-size: 0.6875rem; line-height: 1.3;
  color: var(--silver-faint); text-align: center;
}
.quiz-item {
  border: 0; min-width: 0; background: rgba(255, 255, 255, 0.04); border-radius: 8px;
  margin: 0 0 0.5rem; padding: 0.75rem 1rem 0.875rem;
}
.quiz-item:last-of-type { margin-bottom: 0; }
.quiz-item legend {
  float: none; width: 100%; padding: 0; margin: 0 0 0.625rem;
  font-weight: 600; font-size: 0.9375rem; color: var(--heading);
}
.quiz-no {
  display: inline-block; min-width: 1.5rem; text-align: center; margin-right: 0.5rem;
  font-size: 0.75rem; font-weight: 700; font-variant-numeric: tabular-nums;
  color: var(--nebula); background: rgba(49, 46, 129, 0.45); border-radius: 4px; padding: 0.1rem 0.35rem;
}
.quiz-choices { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 0.375rem; }
.quiz-choice {
  position: relative; display: flex; align-items: center; justify-content: center;
  min-height: 44px; border: 1px solid var(--glass-border); border-radius: 8px;
  background: rgba(16, 20, 31, 0.55);
  cursor: pointer; font-variant-numeric: tabular-nums; font-weight: 600; color: var(--silver-dim);
}
.quiz-choice:hover { border-color: var(--nebula); background: rgba(49, 46, 129, 0.25); }
.quiz-choice:has(input:checked) { background: var(--nebula); border-color: var(--nebula); color: var(--void); }
.quiz-choice:has(input:focus-visible) { outline: 2px solid var(--starlight); outline-offset: 2px; }
.quiz-choice input {
  appearance: none; position: absolute; inset: 0; opacity: 0; margin: 0; border: 0; cursor: pointer;
}
.quiz-submit {
  position: sticky; bottom: 0;
  margin: 1.25rem -1.5rem -1.5rem;
  padding: 0.75rem 1.5rem calc(0.75rem + env(safe-area-inset-bottom));
  background: var(--glass-bg-strong);
  -webkit-backdrop-filter: blur(20px); backdrop-filter: blur(20px);
  border-top: 1px solid var(--glass-border);
}
.quiz-submit .btn-primary { width: 100%; }
.quiz-submit .btn-primary:disabled { opacity: 1; background: rgba(255, 255, 255, 0.08); color: var(--silver-faint); cursor: not-allowed; }
@media (max-width: 639px) {
  .quiz-key { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 0.25rem 0.75rem; padding: 0 0.25rem 0.75rem; }
  .quiz-key { counter-reset: k; }
  .quiz-key span { counter-increment: k; }
  .quiz-key span::before { content: counter(k) " "; font-weight: 600; color: var(--nebula); }
}
```

- [ ] **Step 3: Append scrollbar polish**

```css
::-webkit-scrollbar { width: 10px; }
::-webkit-scrollbar-track { background: var(--void); }
::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.12); border-radius: 999px; }
::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.2); }
```

- [ ] **Step 3b: Append the coarse-pointer blur fallback (spec §8)**

Pre-decided low-end fallback — drop blur entirely and raise the strong-glass alpha so nav/sticky bars stay legible over the animating canvas:

```css
@media (pointer: coarse) {
  .nav,
  .card,
  .feature,
  .quiz-submit {
    -webkit-backdrop-filter: none;
    backdrop-filter: none;
  }
  :root { --glass-bg-strong: rgba(12, 14, 22, 0.9); }
}
```

- [ ] **Step 4: Whole-file verification (all traps cleared)**

Run: `grep -n '#1f2937\|#4B5563\|#4F46E5\|#EEF2FF\|#FEF3C7\|#E5E7EB\|#9CA3AF\|#d1d5db\|#e5e7eb\|rgba(255, 255, 255, 0.92)\|#D4AF37\|#f9fafb\|#333\|#6b7280\|#f3f4f6' crates/web/style.css`
Expected: no output — every light-theme hex is gone.

- [ ] **Step 5: Commit**

```bash
git add crates/web/style.css
git commit -m "feat(web): darken ocean/quiz blocks, dark scrollbar"
```

### Task 8: Phase-1 verification gate

**Files:** none created — verification only.

- [ ] **Step 1: Build (ask the user first)**

Run: `./scripts/build-web.sh` — one build, nothing else running. Expected: dist/ ready.

- [ ] **Step 2: Phase-1 visual checklist (user opens the page)**

- ground: near-black with faint blue-violet glows, no banding;
- haze band visible at ≈-18deg; 60 stars in two sizes, slow twinkle;
- nav/footers/cards are glass over the sky; all text readable;
- hero, forms, palace grid (silver main stars, rose transformation, nebula life palace), OCEAN tracks, quiz selection all legible;
- OS "reduce motion" on: stars freeze at one steady opacity, no canvas present yet (expected in phase 1);
- mobile 375px: no horizontal scroll; layout intact.

Fix anything that fails before moving on (small Edit + re-check + amend commit).

---

### Task 9: `galaxy.js` — core band particles (no shooting stars yet)

**Files:**
- Create: `crates/web/galaxy.js`

**Interfaces:**
- Consumes: nothing (standalone; expects an element `#galaxy-canvas` to exist — added in Task 10, so this task is verified by syntax check only).
- Produces: the full particle system. Task 11 inserts the shooting-star functions into named anchor points in this file (variable block, `frame()`, `draw()`, `resize()`).

- [ ] **Step 1: Write the file**

```js
// galaxy.js — cosmic-silver Milky-Way particle layer (spec §4).
// Standalone, dependency-free, defer-loaded. Every failure path bails
// silently: the CSS star field must stand alone (spec §3).
(function () {
  'use strict';

  var mql = window.matchMedia('(prefers-reduced-motion: reduce)');
  if (mql.matches) return;

  var canvas = document.getElementById('galaxy-canvas');
  if (!canvas) return;
  var ctx;
  try { ctx = canvas.getContext('2d'); } catch (err) { return; }
  if (!ctx) return;

  var BAND_RAD = (-18 * Math.PI) / 180;
  var COS = Math.cos(BAND_RAD), SIN = Math.sin(BAND_RAD);
  var ABS_COS = Math.abs(COS), ABS_SIN = Math.abs(SIN);
  var SIGMA_FRAC = 0.12;      // band width, fraction of viewport diagonal
  var AREA_PER_STAR = 14400;  // px^2 per band particle
  var COUNT_MIN = 24, COUNT_MAX = 150;
  var SCATTER = 30;           // uniform stars outside the band
  var DPR_CAP = 2;

  var W = 0, H = 0, sigma = 0, axMax = 0, ayMax = 0;
  var parts = [];
  var raf = 0, last = 0, running = false;
  var resizeTimer = 0;

  function rand(a, b) { return a + Math.random() * (b - a); }

  // Box-Muller with 3-sigma rejection (spec §4).
  function gauss() {
    var u, v, s, g;
    do {
      u = Math.random() * 2 - 1;
      v = Math.random() * 2 - 1;
      s = u * u + v * v;
      if (s === 0 || s >= 1) continue;
      g = u * Math.sqrt((-2 * Math.log(s)) / s);
    } while (g > 3 || g < -3);
    return g;
  }

  function resize() {
    W = window.innerWidth;
    H = window.innerHeight;
    var dpr = Math.min(window.devicePixelRatio || 1, DPR_CAP);
    canvas.width = Math.round(W * dpr);
    canvas.height = Math.round(H * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    sigma = Math.sqrt(W * W + H * H) * SIGMA_FRAC;
    var pad = 3 * sigma;
    // viewport extents along/acrocss the band axes, inflated by 3 sigma:
    axMax = (W * ABS_COS + H * ABS_SIN) / 2 + pad;
    ayMax = (W * ABS_SIN + H * ABS_COS) / 2 + pad;
    seed();
    draw(performance.now());
  }

  // Full resample on resize — one visible reshuffle frame, accepted (spec §4).
  function seed() {
    parts.length = 0;
    var n = Math.round(Math.min(COUNT_MAX, Math.max(COUNT_MIN, (W * H) / AREA_PER_STAR)));
    var i;
    for (i = 0; i < n; i++) parts.push(bandParticle());
    for (i = 0; i < SCATTER; i++) parts.push(scatterParticle());
  }

  function bandParticle() {
    return {
      ax: rand(-axMax, axMax),   // along-band coordinate (float, always)
      ay: gauss() * sigma,       // perpendicular Gaussian offset
      vx: (Math.random() < 0.5 ? -1 : 1) * rand(2, 6), // px/s along band
      r: rand(0.4, 1.6),
      a0: rand(0.3, 0.9),
      phase: rand(0, Math.PI * 2),
      tw: rand(2, 6)             // twinkle period, seconds
    };
  }

  function scatterParticle() { // uniform inside the viewport rect
    var ax, ay, x, y;
    do {
      ax = rand(-axMax, axMax);
      ay = rand(-ayMax, ayMax);
      x = ax * COS - ay * SIN;
      y = ax * SIN + ay * COS;
    } while (x < -W / 2 || x > W / 2 || y < -H / 2 || y > H / 2);
    var p = bandParticle();
    p.ax = ax;
    p.ay = ay;
    return p;
  }

  // Toroidal wrap in the inflated band domain — never the viewport rect
  // (spec §4: viewport wrap would pop stars out mid-band).
  function wrap(p) {
    if (p.ax > axMax) p.ax -= axMax * 2;
    else if (p.ax < -axMax) p.ax += axMax * 2;
    if (p.ay > ayMax) p.ay -= ayMax * 2;
    else if (p.ay < -ayMax) p.ay += ayMax * 2;
  }

  function step(dt) {
    var i, p;
    for (i = 0; i < parts.length; i++) {
      p = parts[i];
      p.ax += p.vx * dt; // float accumulation — sub-pixel speeds
      wrap(p);
    }
  }

  function draw(now) {
    ctx.clearRect(0, 0, W, H);
    ctx.fillStyle = '#f5f7fa';
    var i, p, t = now / 1000, x, y;
    for (i = 0; i < parts.length; i++) {
      p = parts[i];
      x = W / 2 + p.ax * COS - p.ay * SIN;
      y = H / 2 + p.ax * SIN + p.ay * COS;
      ctx.globalAlpha = p.a0 * (0.55 + 0.45 * Math.sin(p.phase + (t * Math.PI * 2) / p.tw));
      ctx.fillRect(x - p.r / 2, y - p.r / 2, p.r, p.r);
    }
    ctx.globalAlpha = 1;
  }

  function frame(now) {
    if (!running) return;
    var dt = Math.min((now - last) / 1000, 0.05);
    last = now;
    step(dt);
    draw(now);
    raf = requestAnimationFrame(frame);
  }

  function start() {
    if (running) return;
    running = true;
    last = performance.now();
    raf = requestAnimationFrame(frame);
  }

  function stop() {
    running = false;
    cancelAnimationFrame(raf);
  }

  window.addEventListener('resize', function () {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(resize, 200);
  });

  document.addEventListener('visibilitychange', function () {
    if (document.hidden) stop();
    else start();
  });

  function onReducedChange(ev) {
    if (ev.matches) {
      stop();
      ctx.clearRect(0, 0, W, H);
    } else {
      start();
    }
  }
  if (mql.addEventListener) mql.addEventListener('change', onReducedChange);

  try {
    resize();
    start();
  } catch (err) {
    stop();
  }
})();
```

- [ ] **Step 2: Syntax check**

Run: `node --check crates/web/galaxy.js`
Expected: exit 0, no output.

- [ ] **Step 3: Commit**

```bash
git add crates/web/galaxy.js
git commit -m "feat(web): galaxy canvas particle layer (band particles)"
```

### Task 10: Wire canvas into page + build

**Files:**
- Modify: `crates/web/index.html`
- Modify: `crates/web/style.css` (append one rule)
- Modify: `crates/web/scripts/build-web.sh:17` (after the style.css copy line)

**Interfaces:**
- Consumes: Task 9's `galaxy.js` (expects `#galaxy-canvas`), Task 3's z-index scheme (canvas = -2).
- Produces: canvas live on the page; `dist/galaxy.js` shipped by the build.

- [ ] **Step 1: index.html — add canvas + script**

Inside `<body>`, between the three sky divs and the wasm module script, insert:

```html
    <canvas id="galaxy-canvas" aria-hidden="true"></canvas>
    <script src="/galaxy.js" defer></script>
```

- [ ] **Step 2: style.css — canvas layer rule**

Append (keeps the spec §3 z-index scheme: above CSS stars (-3), below content):

```css
#galaxy-canvas {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: -2;
}
```

- [ ] **Step 3: build-web.sh — mandatory copy line**

After line 17 (`cp style.css dist/style.css`), add:

```bash
cp galaxy.js dist/galaxy.js
```

- [ ] **Step 4: Build + ship check (ask the user first)**

Run: `./scripts/build-web.sh && test -f crates/web/dist/galaxy.js && echo SHIPPED`
Expected: `SHIPPED`. (The `test -f` is the only reliable catch for a silent 404 — visually "canvas absent" and "canvas failed" look identical, spec §7.)

- [ ] **Step 5: Visual check (user opens page)**

- particles form an inclined band matching the CSS haze axis; denser at the core;
- **content check (the z-index trap):** nav links, buttons, form inputs, quiz choices all still receive clicks;
- resize the window: one reshuffle, band reforms; switch tabs away/back: animation resumes.

- [ ] **Step 6: Commit**

```bash
git add crates/web/index.html crates/web/style.css crates/web/scripts/build-web.sh
git commit -m "feat(web): wire galaxy canvas into page and build"
```

### Task 11: Shooting stars

**Files:**
- Modify: `crates/web/galaxy.js`

**Interfaces:**
- Consumes: Task 9's anchors — the variable block (`var resizeTimer = 0;`), `frame()`, `draw()`, `resize()`.

- [ ] **Step 1: Insert state variables**

After the line `var resizeTimer = 0;` insert:

```js
  var meteor = null, nextMeteorAt = 0;
```

- [ ] **Step 2: Insert meteor functions**

After the `wrap(p)` function, insert:

```js
  // Shooting stars (spec §4): one at a time, random 8-20 s gap, along the
  // band within +/-25deg, ~900 px/s, ~0.9 s life, ~120 px gradient tail.
  function spawnMeteor(now) {
    var ang = (rand(-25, 25) * Math.PI) / 180;
    var dir = Math.random() < 0.5 ? -1 : 1;
    meteor = {
      ax: rand(-axMax * 0.9, axMax * 0.9),
      ay: gauss() * sigma * 0.5,
      vx: dir * Math.cos(ang) * 900,
      vy: Math.sin(ang) * 900,
      born: now,
      ttl: 0.9
    };
  }

  function updateMeteor(now, dt) {
    if (!meteor) {
      if (now >= nextMeteorAt) {
        spawnMeteor(now);
        nextMeteorAt = now + rand(8000, 20000);
      }
      return;
    }
    meteor.ax += meteor.vx * dt;
    meteor.ay += meteor.vy * dt;
    if ((now - meteor.born) / 1000 > meteor.ttl) meteor = null;
  }

  function drawMeteor(now) {
    if (!meteor) return;
    var age = (now - meteor.born) / (meteor.ttl * 1000);
    if (age < 0 || age > 1) { meteor = null; return; }
    var ease = age < 0.15 ? age / 0.15 : (1 - age) / 0.85;
    var x = W / 2 + meteor.ax * COS - meteor.ay * SIN;
    var y = H / 2 + meteor.ax * SIN + meteor.ay * COS;
    var sp = Math.sqrt(meteor.vx * meteor.vx + meteor.vy * meteor.vy);
    var tx = x - (meteor.vx / sp) * 120;
    var ty = y - (meteor.vy / sp) * 120;
    var grad = ctx.createLinearGradient(x, y, tx, ty);
    grad.addColorStop(0, 'rgba(245,247,250,' + (0.9 * Math.max(ease, 0)).toFixed(3) + ')');
    grad.addColorStop(1, 'rgba(245,247,250,0)');
    ctx.strokeStyle = grad;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.lineTo(tx, ty);
    ctx.stroke();
  }
```

- [ ] **Step 3: Hook into frame / draw / resize**

In `frame()`, change `step(dt);` to:

```js
    step(dt);
    updateMeteor(now, dt);
```

In `draw()`, immediately before `ctx.globalAlpha = 1;`, insert:

```js
    drawMeteor(now);
```

At the end of `resize()`, after `seed();` and before `draw(...)`, insert:

```js
    nextMeteorAt = performance.now() + rand(8000, 20000);
```

- [ ] **Step 4: Syntax check + build (ask the user first)**

Run: `node --check crates/web/galaxy.js`
Expected: exit 0.
Run: `./scripts/build-web.sh && test -f crates/web/dist/galaxy.js && echo SHIPPED`
Expected: `SHIPPED`.

- [ ] **Step 5: Commit**

```bash
git add crates/web/galaxy.js
git commit -m "feat(web): occasional band-aligned shooting stars"
```

### Task 12: Phase-2 verification gate

**Files:** none created — verification only.

- [ ] **Step 1: Full checklist from spec §7 (user opens page)**

- five layers visible, band axis consistent between haze and particles; canvas does not cover content (click nav/buttons/inputs);
- contrast spot-checks on glass surfaces (body, muted, nebula buttons);
- palace grid silver main stars / rose transformation / nebula life palace; OCEAN tracks; quiz selection state; nav glass over stars;
- mobile 375px: band reads, total particles ≈ 54 (24 band + 30 scatter) — not denser than desktop;
- OS reduce-motion on: canvas gone entirely, stars frozen at steady opacity;
- JS disabled (devtools): static CSS star field still renders;
- `test -f crates/web/dist/galaxy.js` passes.

- [ ] **Step 2: Final no-gold + no-light-hex sweep**

Run: `grep -n 'D4AF37\|FEF3C7' crates/web/style.css; grep -cn '#f9fafb\|#333;\|#4F46E5' crates/web/style.css`
Expected: no output from either.

- [ ] **Step 3: Wrap-up**

Report completion to the user with the checklist results. Deployment stays manual (`./scripts/deploy-web.sh`) — user-owned; remind, do not run.


