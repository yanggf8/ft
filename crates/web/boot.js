// boot.js — wasm bootstrap. Kept as an external module file because the
// Pages CSP (crates/web/_headers, P3) sets `script-src 'self'
// 'wasm-unsafe-eval'`, which blocks inline module scripts — the previous
// inline bootstrap in index.html was silently refused in production
// (2026-09-11) and the app never mounted.
import init from '/wasm/ft_web.js';
await init();
