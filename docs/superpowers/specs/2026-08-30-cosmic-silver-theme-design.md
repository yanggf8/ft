# Cosmic Silver Theme — Design Spec

Date: 2026-08-30
Status: Reviewed — adversarial review (Kimi K3) findings verified and incorporated same day. Key deltas from first draft: explicit z-index scheme, band sampling/wrap/resize defined, particle floor 60→24, `--silver-faint` brightened for glass-ground AA, per-class reskin traps enumerated, mandatory `build-web.sh` copy, phased delivery.

## Summary

Reskin the ft-web frontend from the current light theme (white + indigo) to a
dark "cosmic silver" theme: a deep-space black borrowed from ~/a/hesocial's
`midnight-black` luxury palette, with a Milky-Way band of twinkling silver
stars across the whole site. hesocial's gold is explicitly rejected. The
galaxy feel (drifting particles, shooting stars) is new work, implemented as
a lightweight canvas particle layer on top of a pure-CSS star field.

## Goals

- Whole-site dark starfield background (fixed, behind all pages).
- Milky-Way band: particles concentrated on an inclined band, denser at the
  band core, sparse at the edges; band-aligned haze in CSS underneath.
- Silver as the dominant color family; a single blue-violet accent reserved
  for interaction (CTA, links, focus, selection).
- Serif Chinese headings (Noto Serif TC) with Playfair Display for Latin.
- Zero Rust/wasm changes: the reskin is pure CSS + one small vanilla JS file.

## Non-Goals

- No gold anywhere (including palace-grid main stars; those become silver).
- No mouse parallax, no gravity effects, no game-grade particle physics.
- No light/dark toggle — dark only.
- No changes to crates/api, crates/worker, crates/schema, or any Rust code.
- Leptos components stay untouched except for removing any light-theme
  assumptions discovered during the sweep (none expected: styling is
  class-based and lives in style.css).

## 1. Color System (CSS custom properties)

Borrowed from hesocial's tailwind.config.js palette: `midnight-black
#0C0C0C` and `platinum #E5E4E2`. Its `gold #D4AF37` and `deep-blue #1B2951`
are rejected (deep-blue is too bright; only a trace of blue-violet survives
in the nebula haze).

```css
:root {
  /* deep space base */
  --void:        #0C0C0C;   /* deepest layer (hesocial midnight-black) */
  --deep-space:  #10141F;   /* blue-black, far end of the body gradient */

  /* silver family (dominant) */
  --heading:     #E5E4E2;   /* titles (hesocial platinum) */
  --text:        #DDE2EC;   /* body text, near-white starlight */
  --silver-dim:  #A9B0BC;   /* secondary text */
  --silver-faint:#8A919C;   /* muted text (brightened: see contrast note) */
  --starlight:   #F5F7FA;   /* highlights: focus ring, hover glow */

  /* blue-violet accent (interaction only) */
  --nebula:        #8B93F8; /* links, CTA fill, focus, selected */
  --nebula-strong: #A5ACFA; /* hover */
  --nebula-deep:   #312E81; /* selected background */

  /* semantic — one shared warning token by design: palace-grid
     transformation stars and form errors are both "warning" semantics.
     A separate error hue would add a token without adding meaning.
     Transformation pills use a dark glass ground so the rose reads as a
     marker, not an alarm. */
  --rose:        #FB7185;   /* transformation stars, errors, destructive */

  /* glass surfaces */
  --glass-bg:        rgba(255,255,255,0.05);
  --glass-bg-strong: rgba(16,20,31,0.72);  /* nav, sticky bars */
  --glass-border:    rgba(255,255,255,0.10);
}
```

Accessibility floor — worst case is NOT bare `--deep-space`: text sits on
glass (`rgba(255,255,255,.05)` over the gradient, composite ≈ `#1C202A`).
Computed WCAG ratios on that composite ground (relative-luminance formula,
verified): `--text` 12.5:1, `--silver-dim` 7.5:1, `--silver-faint` 5.1:1,
`--nebula` 5.9:1, `--rose` 6.1:1. All pass AA for normal text (4.5:1).
(The first draft's `#7C828E` measured 4.22:1 on glass — that is why
`--silver-faint` is now `#8A919C`.) Any token change during implementation
must re-run this check against the glass composite, not bare deep-space.

## 2. Typography

- Headings (h1-h6): `'Playfair Display', 'Noto Serif TC', serif` — Latin and
  digits render in Playfair, CJK falls through to Noto Serif TC (weight
  **700 only** — a single CJK weight halves the unicode-range slice set;
  differentiate heading levels with size instead). This mirrors hesocial's
  serif-heading luxury language.
- Body: unchanged system sans stack.
- Numeric UI (OCEAN scores, chart meta) keeps `tabular-nums`.
- Loaded via Google Fonts `<link>` with `display=swap`; CJK arrives as
  unicode-range slices, so only used glyphs are fetched.
- Fallback if fonts fail: system serif for headings — acceptable.

## 3. Starfield System (5 fixed layers)

All decorative layers are `position: fixed; inset: 0; pointer-events: none`.
Stacking is by explicit z-index — this is mandatory, not incidental: CSS
paint order puts positioned elements above static content regardless of DOM
order, so an unlayered fixed canvas would paint over the entire UI.

| Layer | Element | z-index |
|---|---|---|
| Body gradient | `body` background | (root background) |
| Nebula haze | `.sky-nebula` | -4 |
| Static stars | `.sky-stars-a`, `.sky-stars-b` | -3 |
| Galaxy particles | `#galaxy-canvas` | -2 |
| Page content | `.app` | static (paints above all negative-z layers) |

Negative-z fixed elements paint above the root background and below static
flow content, so `.app` needs no positioning of its own. (Leptos CSR
appends its mount div to `<body>`; the static `<canvas>` declared in
index.html survives the mount, and the page's own `.quiz-submit` sticky bar
is positioned and therefore still sits above the canvas.)

1. **Body gradient** — `linear-gradient(135deg, var(--void), var(--deep-space))`
   painted on `body`, plus two ultra-faint radial glows to prevent gradient
   banding.
2. **CSS nebula haze** (`.sky-nebula`, z -4) — a large rectangle rotated
   -18deg (the Milky-Way axis), filled with an elongated radial gradient:
   white at 2-5% alpha at the band core fading out, plus a faint
   blue-violet tint (`rgba(76,81,140,0.06)`).
3. **CSS static star field, 2 layers** — multi-`box-shadow` star painting on
   two dot layers, 1px and 2px, 30 stars each = 60 total. Coordinates are
   generated, not hand-written: a throwaway generator script
   (`scripts/gen-stars.js`, committed) emits the two `box-shadow` lists;
   output is pasted into style.css under a `/* generated — do not edit */`
   banner. Each layer twinkles via opacity animation (3.5s / 7s cycles,
   ease-in-out, alternate, offset delays). These stay visible even if
   JS/canvas never runs. 60 stars is deliberate: anything denser is
   indistinguishable behind the particle canvas anyway.
4. **Canvas galaxy particle layer** (`#galaxy-canvas`, z -2) — see §4.
5. **Page content** — glass cards float on the starfield.

## 4. Galaxy Particles (galaxy.js)

One vanilla JS file (~250 lines including the shooting-star state machine —
the first draft's "~120" was unrealistic), loaded with `defer` from
index.html. Never enters the wasm bundle. Renders on a single
full-viewport canvas.

- **Band distribution**: the band axis is the -18deg line through the
  viewport center. The sampling domain is the band-aligned bounding box of
  the viewport **inflated by 3σ on every side** (σ = 12% of the viewport
  diagonal), so off-screen reservoir particles exist and entering/exiting
  looks continuous. Along-axis coordinate: uniform over that domain.
  Perpendicular offset: Gaussian via Box-Muller, values beyond 3σ rejected
  and resampled. 30 additional uniform "scatter" stars anywhere in the
  viewport rect.
- **Wrap-around**: toroidal within the inflated band domain (not the
  viewport rect — wrapping against the viewport would pop stars out
  mid-band, since a large perpendicular offset exits a side edge long
  before the along-axis coordinate wraps).
- **Count**: `clamp(area / 14400px², 24, 150)` band particles (desktop
  ~120; the old floor of 60 made a 375×667 phone 5× denser than a 1080p
  desktop — phones now get ~24+30=54). Recomputed on debounced resize
  (200ms) **by resampling all positions** — one visible reshuffle frame,
  accepted as the cheap option.
- **Per-particle**: radius 0.4-1.6px (device pixels), base alpha 0.3-0.9,
  individual sine twinkle with random phase and 2-6s period, slow drift
  along the band at 2-6px/s (either direction). Positions accumulate in
  **float** — at 0.03-0.1px/frame, integer truncation would freeze the
  drift.
- **Shooting stars**: a random trigger every 8-20s; spawns on the band,
  direction along the band within ±25deg, speed ~900px/s, lifetime ~0.9s,
  ~120px gradient tail, alpha eased in/out. At most one at a time.
- **Loop**: single `requestAnimationFrame`; pauses on
  `visibilitychange` hidden; caps DPR at `min(devicePixelRatio, 2)`.
- **Reduced motion**: `matchMedia('(prefers-reduced-motion: reduce)')` →
  the script never starts, and a CSS media query applies
  `animation: none` with an explicit static `opacity: 0.7` on the star
  layers (freezing via `animation-play-state` would hold an arbitrary
  frame per layer). Result: a static star field (body gradient + haze +
  fixed-opacity stars).
- **Canvas failure** (exception, no 2d context): swallowed; CSS layers 1-3
  remain. The site must look complete without the canvas.

## 5. Dark-Reskin Sweep of style.css

The current style.css (266 lines) is rewritten against the new tokens.
The implementation plan must walk the file **class by class** — there are
no admin-specific classes in the file (the admin page reuses `.card`,
`.page`, `.btn-*`, `.field`; it is covered by the generic sweep).

Review against the actual file surfaced the traps a category-based sweep
would miss:

- Hardcoded *dark* text that vanishes on a dark ground:
  `.hero-head h1` / `.feature h3` / `.quiz-item legend` (`#1f2937`),
  `.ocean-copy` / `.ocean-foot strong` (`#4B5563`).
- Light-theme artifacts that will look broken rather than merely dim:
  `.quiz-no` indigo-on-`#EEF2FF` pill; `.quiz-key span::before` hardcoded
  indigo counters; `.quiz-submit` white sticky bar
  (`rgba(255,255,255,.92)`) and its `#E5E7EB`/`#9CA3AF` disabled state;
  `.ocean-marker` white border/ring.
- The explicit "no gold anywhere" violation: `.star.main` background
  `#FEF3C7` (amber) — becomes a silver pill.

Token mapping, section by section:

- Reset/body: dark gradient ground, `--text` color.
- `.nav`: replaces solid indigo with `--glass-bg-strong` + blur +
  `--glass-border`; brand text `--heading`, links `--silver-dim`.
- `.footer`: transparent dark, `--silver-faint`.
- `.card`, `.feature`: glass surface (`--glass-bg`, blur 20px,
  `--glass-border`), silver headings.
- `.btn-primary`, `.cta`: `--nebula` fill with `--void` text; hover lifts to
  `--nebula-strong` + subtle `--starlight` glow shadow.
- `.cta-alt`, `.btn-link`, `.back-link`, `a`: `--nebula`.
- Forms (`.field`, `.form-input`, `input/select`): dark glass fields,
  `--glass-border`, focus ring 2px `--starlight` (replaces indigo outline).
- `.palace-grid`: palace cells become dark glass; `.palace.life` gets a
  `--nebula` border + `--nebula-deep` tint; `.star.main` silver pill;
  `.star.transformation` `--rose` text on dark glass (see §1 note).
- `.ocean-*` scores/tracks: track `rgba(255,255,255,0.08)`, band
  `--nebula` at reduced alpha, marker `--starlight` with `--nebula` ring,
  score digits `--nebula`.
- `.quiz-*`: items dark glass; selected choice `--nebula` fill + `--void`
  text; focus-visible 2px `--starlight`; sticky submit bar
  `--glass-bg-strong`.
- `.error` → `--rose`; `.muted` → `--silver-faint`.
- Optional polish: dark `::-webkit-scrollbar` styling.

## 6. Files

| File | Action |
|---|---|
| `crates/web/index.html` | add `preconnect` to `fonts.googleapis.com` and `fonts.gstatic.com`, Google Fonts link, `<canvas id="galaxy-canvas">`, `galaxy.js` defer script |
| `crates/web/style.css` | full dark reskin + token refactor (bulk of the work), incl. generated star-field block |
| `crates/web/galaxy.js` | new, ~250 lines vanilla JS |
| `crates/web/scripts/build-web.sh` | **mandatory edit**: the script copies a fixed set (`index.html`, `style.css` only — verified, lines 15-17); `cp galaxy.js dist/` must be added or the defer script silently 404s |
| `crates/web/scripts/gen-stars.js` | new throwaway generator for the box-shadow star coordinates (§3) |
| Leptos `.rs` files | untouched (verification sweep only) |
| Rust crates | untouched |

## 6a. Phasing

Implementation is planned in two shippable phases:

- **Phase 1 — theme**: typography (§2), tokens + full reskin (§1/§5),
  star-field layers 1-3 incl. `gen-stars.js` (§3). The site must look
  complete at the end of phase 1 — the spec already requires the CSS star
  field to stand alone, so this phase ships value on its own.
- **Phase 2 — galaxy**: `galaxy.js` particles + shooting stars (§4) and the
  mandatory `build-web.sh` copy line (§6).

The shooting-star machine is the highest-code piece; it stays in scope
(a core requirement) but lives behind the phase-1 boundary.

## 7. Verification

- `./scripts/build-web.sh` produces a working dist (ask before running —
  heavy builds hang this machine; one build at a time).
- After phase 2, verify `test -f dist/galaxy.js` — a missing copy fails
  silently (defer script 404s while the CSS layers render fine), so
  "canvas absent" and "canvas failed" are visually identical; only the
  file check catches it.
- Manual visual checklist on the built dist (user opens it; deployment stays
  manual per project rules):
  - five layers visible; band axis consistent between haze and particles;
    canvas does **not** cover content (check nav links, buttons, inputs
    are clickable — the z-index trap of §3);
  - contrast spot-checks (body, muted, nebula buttons) on glass surfaces;
  - palace grid: silver main stars, rose transformation stars, nebula life
    palace; OCEAN tracks; quiz selection state; nav glass over stars;
  - mobile width (375px): band still reads; total particles ~54
    (24 band + 30 scatter), not denser than desktop;
  - emulate `prefers-reduced-motion`: canvas absent, static stars remain;
  - with JS disabled: static star field still renders (CSS only).
- `cargo fmt --check` unaffected but CI runs it; no Rust touched.

## 8. Risks

- **CJK webfont weight**: Noto Serif TC arrives as ~100 unicode-range
  slices per weight (halved by the single 700 weight), so cost is bounded,
  but headings pop in glyph-by-glyph on cold load and Playfair's metrics
  differ from system serif, so some heading CLS is expected — and accepted,
  since the CSR app's first paint is already gated on the wasm fetch.
  Mitigations: both `preconnect`s, `display=swap`, system serif fallback.
- **Backdrop-blur over an animating canvas**: glass surfaces with
  `backdrop-filter: blur(20px)` force the browser to recompute the blurred
  backdrop every rAF frame wherever they overlap `#galaxy-canvas`, and
  again on every scroll. Ship `-webkit-backdrop-filter` alongside the
  unprefixed property (still required on Safari releases common on iOS in
  Taiwan). Pre-decide the low-end fallback instead of "tune if janky":
  under a coarse-pointer media query, drop blur to 0 and raise
  `--glass-bg-strong` alpha so nav/sticky bars stay legible.
- **Gradient banding** on the near-black body gradient: mitigated by the
  two faint radial glows (layer 1).
- **Star/haze over-visibility**: alpha values are ceilings; implementation
  should start at the low end and tune visually.
