#!/usr/bin/env node
// contrast-audit.mjs — real-render WCAG contrast audit (plan §2.4 layer 2).
//
//   node scripts/contrast-audit.mjs --base http://localhost:8788 \
//     --pages /,/fengshui,/tarot,/palm,/glossary,/naming,/login \
//     --out /tmp/audit [--states] [--only mobile|desktop] [--max-states 60]
//     [--dump-all]   (also write every measured item, passing ones too, to audit.json .items)
//
// For every page × context (390×844 touch/mobile, 1280×800 desktop):
//   - abort fonts.googleapis / fonts.gstatic requests (offline-safe);
//   - goto(waitUntil:'commit'), wait until <main> has children, let finite
//     animations finish; full-page screenshot to <out>/<ctx>-<slug>.png;
//   - every visible text node: actual text colour (color, -webkit-text-fill-
//     color, background-clip:text gradient → every stop, worst wins; SVG
//     fill/stroke + fill-opacity) against the actual backdrop composited from
//     the paint stack at that point (document.elementsFromPoint, so non-
//     ancestor overlays count), with translucent layers alpha-composited and
//     `opacity` applied as true group opacity along the ancestor chain;
//   - also: placeholder text, input/select values, ::before/::after text,
//     control boundaries (border or fill vs. outside, ≥3.0, non-text),
//     SVG shapes (stroke/fill, ≥3.0, non-text);
//   - background image (url) / canvas / img / video / filter / blend under the
//     text → "manual" (needs human check), never a pass. A gradient under the
//     text is evaluated at every stop: all stops fail → violation; otherwise
//     → manual (with min/max ratio).
// --states: for button, .fs-opt, .fs-guess, input — force :focus/:focus-visible
// and :hover (CDP CSS.forcePseudoState) and re-measure that element; .fs-opt,
// .fs-guess and checkbox/radio are additionally clicked via element.click()
// (selected state). Generic buttons are never clicked (could submit/navigate).
//
// Output: <out>/audit.json  { violations[], manual[], summary, ... } and PNGs.
// Exit 0 = no violations, 1 = violations, 2 = setup / navigation error.
//
// Env overrides: PLAYWRIGHT_CORE (module dir), CHROME_PATH (executable).

import { createRequire } from 'node:module';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const require = createRequire(import.meta.url);
const PW_DIR = process.env.PLAYWRIGHT_CORE
  || '/home/yanggf/.npm/_npx/9833c18b2d85bc59/node_modules/playwright-core';

// ------------------------------------------------------------------ args
function parseArgs(argv) {
  const a = {
    base: 'http://localhost:8788',
    pages: '/,/fengshui,/tarot,/palm,/glossary,/naming,/login',
    out: null, states: false, dumpAll: false, only: null, maxStates: 60, timeout: 20000,
  };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    const next = () => { if (i + 1 >= argv.length) usage(`missing value for ${k}`); return argv[++i]; };
    if (k === '--base') a.base = next();
    else if (k === '--pages') a.pages = next();
    else if (k === '--out') a.out = next();
    else if (k === '--states') a.states = true;
    else if (k === '--dump-all') a.dumpAll = true;
    else if (k === '--only') a.only = next();
    else if (k === '--max-states') a.maxStates = parseInt(next(), 10);
    else if (k === '--timeout') a.timeout = parseInt(next(), 10);
    else if (k === '-h' || k === '--help') usage();
    else usage(`unknown argument ${k}`);
  }
  if (!a.out) usage('--out <dir> is required');
  a.pages = a.pages.split(',').map(s => s.trim()).filter(Boolean);
  return a;
}
function usage(msg) {
  if (msg) console.error(`error: ${msg}`);
  console.error('usage: node scripts/contrast-audit.mjs --base URL --pages /,/a,/b --out DIR [--states] [--only mobile|desktop] [--max-states N] [--timeout MS] [--dump-all]');
  process.exit(2);
}

function findChrome() {
  if (process.env.CHROME_PATH) return process.env.CHROME_PATH;
  const root = path.join(os.homedir(), '.cache', 'ms-playwright');
  const dirs = fs.existsSync(root) ? fs.readdirSync(root).filter(d => d.startsWith('chromium_headless_shell-')) : [];
  dirs.sort((x, y) => (x === 'chromium_headless_shell-1243' ? -1 : y === 'chromium_headless_shell-1243' ? 1 : y.localeCompare(x)));
  for (const d of dirs) {
    for (const sub of fs.readdirSync(path.join(root, d))) {
      const p = path.join(root, d, sub, 'chrome-headless-shell');
      if (fs.existsSync(p)) return p;
    }
  }
  return null;
}

const CONTEXTS = [
  { name: 'mobile', viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true },
  { name: 'desktop', viewport: { width: 1280, height: 800 }, hasTouch: false, isMobile: false },
];

// ------------------------------------------------------------ in-page audit
// Serialized by Playwright: must be self-contained.
function pageAudit(opts) {
  const { scopeSel, state, svgLimit } = opts;
  const scope = scopeSel ? document.querySelector(scopeSel) : document.body;
  if (!scope) return { items: [], stats: { noScope: 1 } };

  // pointer-events:none elements are skipped by elementsFromPoint; force them
  // hit-testable while measuring (CSSOM sheet — not blocked by CSP).
  const sheet = new CSSStyleSheet();
  sheet.replaceSync('*, *::before, *::after { pointer-events: auto !important; }');
  document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
  const savedScroll = [scrollX, scrollY];
  try {
    return run();
  } finally {
    document.adoptedStyleSheets = document.adoptedStyleSheets.filter(s => s !== sheet);
    if (!scopeSel) scrollTo(savedScroll[0], savedScroll[1]);
  }

  function run() {
    const WHITE = [255, 255, 255, 1];
    const csCache = new Map();
    const cs = (e, pseudo) => {
      if (pseudo) return getComputedStyle(e, pseudo);
      let s = csCache.get(e);
      if (!s) { s = getComputedStyle(e); csCache.set(e, s); }
      return s;
    };

    // ---- colour parsing (canvas fallback handles oklch/color()/named)
    const pcache = new Map();
    const cv = document.createElement('canvas'); cv.width = cv.height = 1;
    const c2 = cv.getContext('2d', { willReadFrequently: true });
    function parse(str) {
      if (str == null) return null;
      str = String(str).trim();
      if (pcache.has(str)) return pcache.get(str);
      let c = null;
      const m = str.match(/^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:\s*[,/]\s*([\d.]+%?))?\s*\)$/);
      if (m) {
        const a = m[4] === undefined ? 1 : (m[4].endsWith('%') ? parseFloat(m[4]) / 100 : parseFloat(m[4]));
        c = [+m[1], +m[2], +m[3], a];
      } else if (str === 'transparent' || str === 'none') {
        c = [0, 0, 0, 0];
      } else {
        c2.fillStyle = '#010203';
        c2.fillStyle = str;
        if (c2.fillStyle !== '#010203' || /^#010203$/i.test(str)) {
          c2.clearRect(0, 0, 1, 1); c2.fillRect(0, 0, 1, 1);
          const d = c2.getImageData(0, 0, 1, 1).data;
          c = d[3] ? [d[0], d[1], d[2], d[3] / 255] : [0, 0, 0, 0];
        }
      }
      pcache.set(str, c);
      return c;
    }
    const COLOR_TOKEN = /(rgba?\([^()]*\)|hsla?\([^()]*\)|oklch\([^()]*\)|oklab\([^()]*\)|lab\([^()]*\)|lch\([^()]*\)|color\([^()]*\)|#[0-9a-fA-F]{3,8}\b|\btransparent\b)/g;
    function gradientStops(bgImage) {
      return (bgImage.match(COLOR_TOKEN) || []).map(parse).filter(Boolean);
    }
    const hex = c => '#' + c.slice(0, 3).map(v => Math.round(v).toString(16).padStart(2, '0')).join('') + (c[3] < 1 ? `@${c[3].toFixed(2)}` : '');
    const over = (t, b) => { const a = t[3]; return [t[0] * a + b[0] * (1 - a), t[1] * a + b[1] * (1 - a), t[2] * a + b[2] * (1 - a), 1]; };
    const lerp = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, 1];
    const lum = c => { const f = v => { v /= 255; return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }; return 0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2]); };
    const ratio = (a, b) => { const x = lum(a), y = lum(b); return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05); };

    // ---- opacity groups
    const chainCache = new Map();
    function chainOf(e) {
      if (chainCache.has(e)) return chainCache.get(e);
      const out = [];
      for (let n = e; n && n.nodeType === 1; n = n.parentElement) {
        if (parseFloat(cs(n).opacity) < 1) out.unshift(n);
      }
      chainCache.set(e, out);
      return out;
    }
    const effOpacity = e => chainOf(e).reduce((p, n) => p * parseFloat(cs(n).opacity), 1);
    // Exact group-opacity compositing: a group with opacity o over backdrop B
    // equals lerp(B, content-composited-over-B, o).
    function render(layers, pick, backdrop, depth) {
      let X = backdrop;
      for (let i = 0; i < layers.length;) {
        const g = layers[i].chain[depth];
        if (g) {
          let j = i; while (j < layers.length && layers[j].chain[depth] === g) j++;
          const inner = render(layers.slice(i, j), pick.slice(i, j), X, depth + 1);
          X = lerp(X, inner, parseFloat(cs(g).opacity));
          i = j;
        } else { X = over(pick[i], X); i++; }
      }
      return X;
    }

    function sel(e) {
      const parts = [];
      for (let n = e; n && n.nodeType === 1 && parts.length < 5; n = n.parentElement) {
        let s = n.tagName.toLowerCase();
        if (n.id) { parts.unshift(`${s}#${n.id}`); break; }
        const cls = [...n.classList].filter(c => !c.startsWith('data-')).slice(0, 2);
        if (cls.length) s += '.' + cls.join('.');
        const p = n.parentElement;
        if (p) { const same = [...p.children].filter(x => x.tagName === n.tagName); if (same.length > 1) s += `:nth-of-type(${same.indexOf(n) + 1})`; }
        parts.unshift(s);
      }
      return parts.join(' > ');
    }

    const REPLACED = new Set(['IMG', 'CANVAS', 'VIDEO', 'IFRAME', 'OBJECT', 'EMBED', 'PICTURE']);
    const SHAPES = new Set(['path', 'circle', 'rect', 'line', 'polyline', 'polygon', 'ellipse']);
    const isSvgShape = e => e instanceof SVGElement && SHAPES.has(e.tagName.toLowerCase());
    const clipsText = s => (s.webkitBackgroundClip || s.backgroundClip || '').includes('text');

    // every paint of an SVG element: fill then stroke (a <line> has no fill area)
    function svgPaints(e) {
      const s = cs(e);
      const out = [];
      for (const [prop, opProp] of [['fill', 'fillOpacity'], ['stroke', 'strokeOpacity']]) {
        if (prop === 'fill' && e.tagName.toLowerCase() === 'line') continue;
        const v = s[prop];
        if (!v || v === 'none') continue;
        if (prop === 'stroke' && parseFloat(s.strokeWidth) <= 0) continue;
        if (v.startsWith('url(')) { out.push({ manual: `SVG ${prop} 為 ${v}`, prop }); continue; }
        const c = parse(v); if (!c || c[3] === 0) continue;
        out.push({ color: [c[0], c[1], c[2], c[3] * parseFloat(s[opProp] || 1)], prop });
      }
      return out;
    }
    const svgPaint = e => svgPaints(e)[0] || null;

    // one layer per painted element in the stack (bottom → top)
    function layerOf(e, textEl) {
      const s = cs(e);
      const L = { chain: chainOf(e), variants: [[0, 0, 0, 0]], manual: null, el: e };
      const reasons = [];
      if (e instanceof SVGElement && e.tagName.toLowerCase() !== 'svg') {
        if (isSvgShape(e)) {
          const p = svgPaint(e);
          if (p && p.manual) reasons.push(p.manual);
          else if (p && p.prop === 'fill') L.variants = [p.color];
        }
      } else {
        const own = e === textEl && clipsText(s);
        if (!own) {
          const bg = parse(s.backgroundColor) || [0, 0, 0, 0];
          L.variants = [bg];
          const img = s.backgroundImage;
          if (img && img !== 'none') {
            if (/url\(/.test(img)) reasons.push(`背景圖片 ${sel(e)}`);
            else {
              const stops = gradientStops(img);
              if (stops.length) L.variants = stops.map(st => (bg[3] === 0 || st[3] >= 1 ? st : (bg[3] >= 1 ? over(st, bg) : st)));
              L.gradient = true;
            }
          }
        }
        if (REPLACED.has(e.tagName)) reasons.push(`<${e.tagName.toLowerCase()}> 圖像在下方 ${sel(e)}`);
      }
      if (s.filter && s.filter !== 'none') reasons.push(`filter: ${s.filter} @ ${sel(e)}`);
      if (s.backdropFilter && s.backdropFilter !== 'none') reasons.push(`backdrop-filter @ ${sel(e)}`);
      if (s.mixBlendMode && s.mixBlendMode !== 'normal') reasons.push(`mix-blend-mode @ ${sel(e)}`);
      if (reasons.length) L.manual = reasons.join('; ');
      return L;
    }
    const paints = o => {
      const s = cs(o);
      return (parse(s.backgroundColor) || [0, 0, 0, 0])[3] > 0 || (s.backgroundImage && s.backgroundImage !== 'none') || REPLACED.has(o.tagName);
    };

    // backdrop layers at (x,y) for `el`. includeSelf: el's own bg layer too.
    function stackAt(x, y, el, includeSelf) {
      const stack = document.elementsFromPoint(x, y);
      let idx = stack.indexOf(el);
      let anchor = el;
      if (idx < 0) {
        for (let a = el.parentElement; a; a = a.parentElement) { idx = stack.indexOf(a); if (idx >= 0) { anchor = a; break; } }
        if (idx < 0) return { occluded: 'not-hit' };
      }
      for (let k = 0; k < idx; k++) {
        const o = stack[k];
        if (el.contains(o) || o.contains(el)) continue;
        if (paints(o)) return { occluded: sel(o) };
      }
      const svgRoot = el instanceof SVGElement ? (el.ownerSVGElement || el) : null;
      let below = stack.slice(idx + (anchor === el && !includeSelf ? 1 : 0)).reverse();
      if (svgRoot) {
        // inside an SVG keep only shapes painted before el (document order)
        below = below.filter(e => !(e instanceof SVGElement) || e === svgRoot || e === el ||
          (e.compareDocumentPosition(el) & Node.DOCUMENT_POSITION_FOLLOWING));
      }
      const layers = below.map(e => layerOf(e, el));
      if (anchor !== el && includeSelf) layers.push(layerOf(el, el));
      // root canvas: html bg, else body bg propagates
      const rootS = cs(document.documentElement), bodyS = document.body ? cs(document.body) : null;
      let canvasLayer = null;
      if ((parse(rootS.backgroundColor) || [0, 0, 0, 0])[3] === 0 && (!rootS.backgroundImage || rootS.backgroundImage === 'none') &&
          bodyS && !stack.includes(document.body)) canvasLayer = layerOf(document.body, null);
      if (canvasLayer) { canvasLayer.chain = []; layers.unshift(canvasLayer); }
      // cut everything beneath the topmost opaque solid layer
      let cut = 0;
      layers.forEach((L, k) => { if (!L.chain.length && L.variants.length === 1 && L.variants[0][3] >= 1 && !L.manual && !L.gradient) cut = k; });
      return { layers: layers.slice(cut) };
    }

    // evaluate fg variants against bg layers (cartesian product, capped)
    function measure(layers, fgLayer) {
      const all = fgLayer ? [...layers, fgLayer] : layers;
      let combos = [[]];
      for (const L of all) {
        const next = [];
        for (const c of combos) for (const v of L.variants) { if (next.length < 512) next.push([...c, v]); }
        combos = next;
      }
      let min = Infinity, max = 0, worst = null;
      for (const pick of combos) {
        const pBg = render(layers, pick.slice(0, layers.length), WHITE, 0);
        const pFg = fgLayer ? render(all, pick, WHITE, 0) : null;
        const r = ratio(pFg, pBg);
        if (r < min) { min = r; worst = { fg: pFg, bg: pBg }; }
        if (r > max) max = r;
      }
      return { min, max, worst };
    }

    const items = [];
    const stats = { text: 0, occluded: 0, hidden: 0, offscreen: 0, controls: 0, svg: 0, pseudo: 0 };

    function inViewport(r) { const cy = r.top + r.height / 2, cx = r.left + r.width / 2; return cy >= 1 && cy <= innerHeight - 1 && cx >= 0 && cx <= innerWidth; }
    function bringIntoView(getRect) {
      let r = getRect();
      if (!r) return null;
      if (!inViewport(r)) {
        const cx = r.left + r.width / 2;
        if (cx < 0 || cx > innerWidth) return null; // horizontally off-canvas
        scrollBy(0, r.top + r.height / 2 - innerHeight / 2);
        r = getRect();
        if (!r || !inViewport(r)) return null;
      }
      return r;
    }
    function hiddenEl(e) {
      const s = cs(e);
      if (s.visibility !== 'visible') return true;
      if (effOpacity(e) === 0) return true;
      return false;
    }

    function record(kind, el, text, fgLayer, layers, extra) {
      const m = measure(layers, fgLayer);
      const manual = layers.map(L => L.manual).filter(Boolean).concat(fgLayer && fgLayer.manual ? [fgLayer.manual] : []);
      const gradientBg = layers.some(L => L.gradient);
      items.push(Object.assign({
        state, kind, selector: sel(el), text: (text || '').replace(/\s+/g, ' ').trim().slice(0, 80),
        ratio: +m.min.toFixed(2), ratioMax: +m.max.toFixed(2),
        fg: m.worst && m.worst.fg ? hex(m.worst.fg) : null, bg: m.worst ? hex(m.worst.bg) : null,
        manual: manual.length ? manual.join('; ') : (gradientBg ? '背景為漸層（已逐色標計算）' : null),
        gradientBg,
      }, extra));
    }

    function fgLayerFor(el) {
      const s = cs(el);
      const chain = chainOf(el);
      if (el instanceof SVGElement) {
        const p = svgPaint(el);
        if (!p) return null;
        if (p.manual) return { chain, variants: [[0, 0, 0, 1]], manual: p.manual };
        return { chain, variants: [p.color], note: `svg-${p.prop}` };
      }
      const fill = parse(s.webkitTextFillColor) || parse(s.color);
      if (clipsText(s) && fill[3] === 0 && s.backgroundImage !== 'none') {
        if (/url\(/.test(s.backgroundImage)) return { chain, variants: [[0, 0, 0, 1]], manual: `文字填色為圖片 ${sel(el)}` };
        const stops = gradientStops(s.backgroundImage).filter(c => c[3] > 0);
        if (stops.length) return { chain, variants: stops, note: 'background-clip:text 漸層（取最差色標）' };
      }
      if (fill[3] === 0) return null; // nothing painted
      const noteFill = s.webkitTextFillColor && parse(s.webkitTextFillColor) && s.webkitTextFillColor !== s.color ? '-webkit-text-fill-color' : undefined;
      return { chain, variants: [fill], note: noteFill };
    }
    const sizeInfo = s => {
      const px = parseFloat(s.fontSize), w = parseInt(s.fontWeight, 10) || 400;
      const large = px >= 24 || (px >= 18.66 && w >= 700);
      return { fontSize: px, fontWeight: w, need: large ? 3 : 4.5, large };
    };

    // ---------------- text nodes
    const SKIP = new Set(['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEMPLATE', 'OPTION', 'TITLE', 'TEXTAREA']);
    const walker = document.createTreeWalker(scope, NodeFilter.SHOW_TEXT, {
      acceptNode: n => (n.nodeValue.trim() && n.parentElement && !SKIP.has(n.parentElement.tagName) && !n.parentElement.closest('svg defs, select'))
        ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT,
    });
    const textNodes = [];
    for (let n = walker.nextNode(); n; n = walker.nextNode()) textNodes.push(n);
    const firstRect = node => {
      const rg = document.createRange(); rg.selectNodeContents(node);
      for (const r of rg.getClientRects()) if (r.width >= 2 && r.height >= 2) return r;
      return null;
    };
    const ordered = textNodes.map(n => ({ n, r: firstRect(n) })).filter(o => {
      if (!o.r) { stats.hidden++; return false; }
      return true;
    }).sort((a, b) => (a.r.top - b.r.top));
    const seen = new Map();
    for (const { n } of ordered) {
      const el = n.parentElement;
      if (hiddenEl(el)) { stats.hidden++; continue; }
      const r = bringIntoView(() => firstRect(n));
      if (!r) { stats.offscreen++; continue; }
      const fgL = fgLayerFor(el);
      if (!fgL) { stats.hidden++; continue; }
      const st = stackAt(r.left + Math.min(r.width / 2, 8), r.top + r.height / 2, el, true);
      if (st.occluded) { stats.occluded++; continue; }
      stats.text++;
      const s = cs(el);
      const si = sizeInfo(s);
      const before = items.length;
      record('text', el, n.nodeValue, fgL, st.layers, { ...si, note: fgL.note });
      const it = items[before];
      const key = it.selector + '|' + it.kind;
      const prev = seen.get(key);
      if (prev !== undefined) { // keep the worst per element
        if (items[prev].ratio <= it.ratio) items.pop();
        else { items[prev] = it; items.pop(); }
      } else seen.set(key, before);
    }

    // ---------------- pseudo-element text
    for (const el of [scope, ...scope.querySelectorAll('*')]) {
      for (const ps of ['::before', '::after']) {
        const s = getComputedStyle(el, ps);
        const content = s.content;
        if (!content || content === 'none' || content === 'normal' || !/^["']/.test(content) || /^["']\s*["']$/.test(content)) continue;
        if (s.visibility !== 'visible' || s.display === 'none' || hiddenEl(el)) continue;
        const r = bringIntoView(() => { const b = el.getBoundingClientRect(); return b.width >= 2 && b.height >= 2 ? b : null; });
        if (!r) continue;
        const st = stackAt(r.left + r.width / 2, r.top + r.height / 2, el, true);
        if (st.occluded) continue;
        const bg = parse(s.backgroundColor) || [0, 0, 0, 0];
        const layers = bg[3] > 0 ? [...st.layers, { chain: chainOf(el), variants: [bg] }] : st.layers;
        const fill = parse(s.webkitTextFillColor) || parse(s.color);
        if (!fill || fill[3] === 0) continue;
        stats.pseudo++;
        record('pseudo-text', el, `${ps} ${content}`, { chain: chainOf(el), variants: [fill] }, layers, { ...sizeInfo(s), note: '偽元素位置以宿主元素中心近似' });
      }
    }

    // ---------------- form controls: value / placeholder text + boundary
    const CONTROL_SEL = 'input:not([type=hidden]), select, textarea, button, .fs-opt, .fs-guess, [role=button]';
    const controls = scope.matches && scope.matches(CONTROL_SEL) ? [scope, ...scope.querySelectorAll(CONTROL_SEL)] : [...scope.querySelectorAll(CONTROL_SEL)];
    for (const el of controls) {
      if (hiddenEl(el)) continue;
      const r = bringIntoView(() => { const b = el.getBoundingClientRect(); return b.width >= 2 && b.height >= 2 ? b : null; });
      if (!r) continue;
      const cx = r.left + r.width / 2, cy = r.top + r.height / 2;
      const outside = stackAt(cx, cy, el, false);
      if (outside.occluded) continue;
      const s = cs(el);
      const chain = chainOf(el);
      const tag = el.tagName;
      const textual = (tag === 'INPUT' && !/^(checkbox|radio|range|color|file|submit|button|reset|image)$/i.test(el.type)) || tag === 'TEXTAREA' || tag === 'SELECT';
      if (textual) {
        const inner = stackAt(cx, cy, el, true);
        const val = tag === 'SELECT' ? (el.selectedOptions[0] ? el.selectedOptions[0].text : '') : el.value;
        if (val) {
          const fill = parse(s.webkitTextFillColor) || parse(s.color);
          record('control-value', el, val, { chain, variants: [fill] }, inner.layers, sizeInfo(s));
        } else if (el.placeholder) {
          const ph = getComputedStyle(el, '::placeholder');
          const c = parse(ph.webkitTextFillColor) || parse(ph.color);
          if (c && c[3] > 0) record('placeholder', el, el.placeholder, { chain, variants: [c] }, inner.layers, sizeInfo(ph));
        }
      }
      // boundary (non-text ≥3.0): border OR own fill vs. what is outside
      const bw = Math.max(...['Top', 'Right', 'Bottom', 'Left'].map(sd => s[`border${sd}Style`] !== 'none' ? parseFloat(s[`border${sd}Width`]) : 0));
      const border = bw > 0 ? parse(s.borderTopColor) : null;
      const fill = parse(s.backgroundColor) || [0, 0, 0, 0];
      const hasImg = s.backgroundImage && s.backgroundImage !== 'none';
      const isButtonish = tag === 'BUTTON' || el.matches('.fs-opt, .fs-guess, [role=button]') || /^(submit|button|reset)$/i.test(el.type || '');
      if (isButtonish && (!border || border[3] === 0) && fill[3] === 0 && !hasImg) continue; // text-only button: no boundary required
      if (/^(checkbox|radio)$/i.test(el.type || '') && s.appearance !== 'none') continue; // native widget, UA-drawn
      stats.controls++;
      const self = layerOf(el, null);
      const mFill = measure(outside.layers, { chain, variants: self.variants, manual: self.manual });
      let best = mFill, via = 'fill';
      if (border && border[3] > 0) {
        const mB = measure(outside.layers, { chain, variants: [border] });
        if (mB.min > best.min) { best = mB; via = 'border'; }
      }
      const manual = outside.layers.map(L => L.manual).concat(self.manual ? [self.manual] : []).filter(Boolean);
      items.push({
        state, kind: 'nontext-control', selector: sel(el), text: (el.innerText || el.value || el.placeholder || el.type || '').replace(/\s+/g, ' ').trim().slice(0, 60),
        ratio: +best.min.toFixed(2), ratioMax: +best.max.toFixed(2), fg: hex(best.worst.fg), bg: hex(best.worst.bg),
        need: 3, via, manual: manual.length ? manual.join('; ') : (outside.layers.some(L => L.gradient) ? '背景為漸層（已逐色標計算）' : null),
        gradientBg: outside.layers.some(L => L.gradient),
      });
    }

    // ---------------- SVG shapes (non-text ≥3.0)
    let nsvg = 0;
    for (const el of scope.querySelectorAll('svg path, svg circle, svg rect, svg line, svg polyline, svg polygon, svg ellipse')) {
      if (nsvg >= svgLimit) break;
      if (el.closest('defs, clipPath, mask, pattern, marker, symbol')) continue;
      if (hiddenEl(el)) continue;
      const paints = svgPaints(el);
      if (!paints.length) continue;
      const r = bringIntoView(() => { const b = el.getBoundingClientRect(); return (b.width >= 1 || b.height >= 1) && (b.width + b.height) >= 3 ? b : null; });
      if (!r) continue;
      const svgRoot = el.ownerSVGElement;
      const st = stackAt(r.left + r.width / 2, r.top + r.height / 2, svgRoot, true);
      if (st.occluded) continue;
      nsvg++; stats.svg++;
      const ariaHidden = !!el.closest('[aria-hidden="true"]');
      const solid = paints.filter(p => !p.manual);
      if (!solid.length) {
        items.push({ state, kind: 'nontext-svg', selector: sel(el), text: '', ratio: null, need: 3, manual: paints.map(p => p.manual).join('; '), ariaHidden });
        continue;
      }
      // shapes painted earlier in the same SVG at this point count as backdrop
      const under = document.elementsFromPoint(r.left + r.width / 2, r.top + r.height / 2)
        .filter(e => isSvgShape(e) && e !== el && svgRoot.contains(e) && (e.compareDocumentPosition(el) & Node.DOCUMENT_POSITION_FOLLOWING))
        .reverse().map(e => layerOf(e, null));
      // fill OR stroke may carry the shape: keep the better-contrasting paint
      const before = items.length;
      let bestIdx = -1;
      for (const p of solid) {
        record('nontext-svg', el, el.getAttribute('class') || el.tagName, { chain: chainOf(el), variants: [p.color] }, [...st.layers, ...under], { need: 3, via: p.prop, ariaHidden });
        const k = items.length - 1;
        if (bestIdx < 0 || items[k].ratio > items[bestIdx].ratio) bestIdx = k;
      }
      const best = items[bestIdx];
      items.length = before;
      items.push(best);
      if (items[before]) items[before].note = 'SVG 取外框中心點的背景';
    }

    return { items, stats, url: location.href };
  }
}

// mark state targets; returns count
function markTargets(max) {
  const els = [...document.querySelectorAll('button, .fs-opt, .fs-guess, input:not([type=hidden])')]
    .filter(e => { const r = e.getBoundingClientRect(); const s = getComputedStyle(e); return r.width > 1 && r.height > 1 && s.visibility === 'visible'; })
    .slice(0, max);
  els.forEach((e, i) => e.setAttribute('data-ca-idx', String(i)));
  return els.map((e, i) => ({ i, click: e.matches('.fs-opt, .fs-guess, input[type=checkbox], input[type=radio]') }));
}

// ------------------------------------------------------------------ driver
async function settle(page, timeout) {
  await page.waitForFunction(() => { const m = document.querySelector('main'); return m && m.children.length > 0; }, null, { timeout, polling: 250 });
  await page.evaluate(() => Promise.race([
    Promise.all(document.getAnimations().filter(a => { const t = a.effect && a.effect.getComputedTiming(); return t && Number.isFinite(t.endTime); }).map(a => a.finished.catch(() => {}))),
    new Promise(r => setTimeout(r, 3000)),
  ]));
  await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
}

async function forcePseudo(cdp, idx, classes) {
  const { result } = await cdp.send('Runtime.evaluate', { expression: `document.querySelector('[data-ca-idx="${idx}"]')` });
  if (!result.objectId) return false;
  await cdp.send('DOM.getDocument', { depth: 0 });
  const { nodeId } = await cdp.send('DOM.requestNode', { objectId: result.objectId });
  if (!nodeId) return false;
  await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses: classes });
  return nodeId;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  let chromium;
  try { ({ chromium } = require(PW_DIR)); } catch (e) { console.error(`cannot load playwright-core from ${PW_DIR}: ${e.message}`); process.exit(2); }
  const exe = findChrome();
  if (!exe) { console.error('chrome-headless-shell not found (set CHROME_PATH)'); process.exit(2); }
  fs.mkdirSync(args.out, { recursive: true });

  const browser = await chromium.launch({ executablePath: exe, args: ['--disable-gpu'], headless: true });
  const report = { base: args.base, pages: args.pages, generatedAt: new Date().toISOString(), runs: [], violations: [], manual: [], errors: [], items: args.dumpAll ? [] : undefined };
  const baseUrl = args.base.endsWith('/') ? args.base : args.base + '/';
  try {
    for (const ctxSpec of CONTEXTS) {
      if (args.only && args.only !== ctxSpec.name) continue;
      const context = await browser.newContext({ viewport: ctxSpec.viewport, hasTouch: ctxSpec.hasTouch, isMobile: ctxSpec.isMobile, deviceScaleFactor: 1 });
      await context.route(u => /fonts\.(googleapis|gstatic)\.com/.test(u.href || String(u)), r => r.abort());
      for (const p of args.pages) {
        const url = new URL(p.replace(/^\//, ''), /^https?:/.test(baseUrl) ? new URL('/', baseUrl) : baseUrl).href;
        const slug = (p.replace(/^\/+|\/+$/g, '').replace(/[^\w.-]+/g, '_') || 'home').replace(/\.html?$/, '');
        const page = await context.newPage();
        const consoleErrors = [];
        page.on('console', m => { if (m.type() === 'error') consoleErrors.push(m.text().slice(0, 300)); });
        const run = { context: ctxSpec.name, page: p, url, screenshot: null, stats: null, items: 0 };
        try {
          await page.goto(url, { waitUntil: 'commit', timeout: args.timeout });
          await settle(page, args.timeout);
          const shot = path.join(args.out, `${ctxSpec.name}-${slug}.png`);
          try { await page.screenshot({ path: shot, fullPage: true, animations: 'disabled', timeout: 15000 }); run.screenshot = shot; }
          catch (e) { run.screenshotError = e.message.split('\n')[0]; }
          const res = await page.evaluate(pageAudit, { scopeSel: null, state: 'default', svgLimit: 400 });
          const all = res.items.map(it => ({ context: ctxSpec.name, page: p, ...it }));
          if (args.states) {
            const cdp = await context.newCDPSession(page);
            await cdp.send('DOM.enable'); await cdp.send('CSS.enable');
            const targets = await page.evaluate(markTargets, args.maxStates);
            for (const t of targets) {
              const s = `[data-ca-idx="${t.i}"]`;
              const exists = await page.evaluate(q => { const e = document.querySelector(q); if (e) e.scrollIntoView({ block: 'center' }); return !!e; }, s);
              if (!exists) continue;
              for (const [state, classes] of [['focus', ['focus', 'focus-visible', 'focus-within']], ['hover', ['hover']]]) {
                let nodeId = false;
                try {
                  if (state === 'focus') await page.evaluate(q => { const e = document.querySelector(q); if (e) e.focus({ preventScroll: true }); }, s);
                  nodeId = await forcePseudo(cdp, t.i, classes);
                  await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
                  await page.waitForTimeout(250); // let transitions finish
                  const r2 = await page.evaluate(pageAudit, { scopeSel: s, state, svgLimit: 50 });
                  all.push(...r2.items.map(it => ({ context: ctxSpec.name, page: p, ...it })));
                } catch (e) { report.errors.push({ context: ctxSpec.name, page: p, state, target: s, error: e.message.split('\n')[0] }); }
                if (nodeId) await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses: [] }).catch(() => {});
                if (state === 'focus') await page.evaluate(q => { const e = document.querySelector(q); if (e) e.blur(); }, s).catch(() => {});
              }
              if (t.click) {
                try {
                  await page.evaluate(q => { const e = document.querySelector(q); if (e) e.click(); }, s);
                  await page.waitForTimeout(400);
                  if (await page.evaluate(q => !!document.querySelector(q), s)) {
                    const r3 = await page.evaluate(pageAudit, { scopeSel: s, state: 'selected', svgLimit: 50 });
                    all.push(...r3.items.map(it => ({ context: ctxSpec.name, page: p, ...it })));
                  }
                } catch (e) { report.errors.push({ context: ctxSpec.name, page: p, state: 'selected', target: s, error: e.message.split('\n')[0] }); }
              }
            }
            await cdp.detach().catch(() => {});
          }
          run.stats = res.stats; run.items = all.length;
          if (args.dumpAll) report.items.push(...all);
          for (const it of all) {
            if (it.ratio == null || it.manual) {
              // gradient backdrop that fails at every stop is a definite violation
              if (it.ratio != null && it.gradientBg && it.ratioMax < it.need && !/背景圖片|圖像|filter|blend|url\(/.test(it.manual || '')) report.violations.push(it);
              else report.manual.push(it);
            } else if (it.ratio < it.need) report.violations.push(it);
          }
          if (consoleErrors.length) run.consoleErrors = consoleErrors;
        } catch (e) {
          run.error = e.message.split('\n')[0];
          report.errors.push({ context: ctxSpec.name, page: p, error: run.error });
        }
        report.runs.push(run);
        await page.close();
      }
      await context.close();
    }
  } finally {
    await browser.close();
  }
  report.summary = { violations: report.violations.length, manual: report.manual.length, errors: report.errors.length };
  const outFile = path.join(args.out, 'audit.json');
  fs.writeFileSync(outFile, JSON.stringify(report, null, 2));

  // human-readable digest
  const byPage = new Map();
  for (const v of report.violations) { const k = `${v.context} ${v.page}`; if (!byPage.has(k)) byPage.set(k, []); byPage.get(k).push(v); }
  for (const [k, vs] of byPage) {
    console.log(`== ${k}（${vs.length}）`);
    for (const v of vs.slice(0, 40)) console.log(`   [${v.state}/${v.kind}] ${v.ratio} < ${v.need}  ${v.fg} on ${v.bg}  ${v.selector}  “${v.text}”`);
    if (vs.length > 40) console.log(`   …另 ${vs.length - 40} 項見 audit.json`);
  }
  for (const e of report.errors) console.log(`ERROR ${e.context} ${e.page}${e.state ? ' ' + e.state : ''}: ${e.error}`);
  console.log(`結果：${report.violations.length || report.errors.length ? 'FAIL' : 'PASS'} — 違規 ${report.violations.length}、需人工 ${report.manual.length}、錯誤 ${report.errors.length}；報告 ${outFile}`);
  process.exit(report.errors.length && !report.runs.some(r => r.stats) ? 2 : report.violations.length ? 1 : (report.errors.length ? 2 : 0));
}

main().catch(e => { console.error(e); process.exit(2); });
