#!/usr/bin/env node
// web-smoke.mjs — 前端真瀏覽器驗證（2026-11-22）。
//
//   FT_API=https://fortunet-api-staging.yanggf.workers.dev FT_DB=fortunet-staging \
//     node scripts/web-smoke.mjs --base https://staging.fortunet.pages.dev [--login]
//   node scripts/web-smoke.mjs --base https://ahexagram.com --login
//
// 本專案沒有本機伺服器（規則：不起 server），所以「上線前驗證」跑在 Cloudflare Pages
// 的固定 staging 分支（staging.fortunet.pages.dev，已列入 API 的 ALLOWED_ORIGINS），
// 「上線後驗證」跑在正式站。deploy-web.sh 兩段都會跑；verify-deployment.sh 也會呼叫。
//
// 檢查（手機 390×844、無快取的新 context）：
//   - 每個公開頁 app 有 mount（<main> 有內容）、沒有未捕捉的 JS 例外、同源資產沒有 4xx/5xx；
//   - 快取防線：boot.js／ft_web.js／style.css 都帶 ?v= 內容 hash（CLAUDE.md 快取紅線）；
//   - 首頁功能卡、開運小物 6 張卡與盆栽 sprite stack 圖全部載入、匿名「我有興趣」登記→取消來回
//     （淨零：DELETE 依 anonId 刪掉剛寫的列）；
//   - 空間風水：選房間三個縮景載入；三房型各灌一組答案到 localStorage → 結果頁縮景圖全部載入、
//     有翻卡；
//   - 首頁吉祥物（圖集帶月相；新月夜叫醒有台詞）、塔羅吉祥物兩疊 sprite stack 載入；塔羅今日抽牌可翻牌；紫微選單三張子單元卡；上手小實驗 /try 未登入有登入連結；
//   - --login：DB 直插 login token 自造測試帳號（同 subunits-selftest.py，需本機 turso 已登入），
//     驗 /ziwei/luck 十二宮與流年標記、/ziwei/chart 命盤、/tarot/love 頁面（不抽牌、不打 AI）、
//     上手小實驗整鏈（抽三個→猜第 1 步→對答案第 2 步→結果卡「多走了 1 步」→UI 刪除→紀錄為空），
//     結束 SQL 硬清測試列。
// 結束碼：0 全過；1 有失敗；2 環境問題（找不到 playwright／chromium）。

import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const require = createRequire(import.meta.url);
const PW_DIR = process.env.PLAYWRIGHT_CORE
  || '/home/yanggf/.npm/_npx/9833c18b2d85bc59/node_modules/playwright-core';
// staging 頁打 staging API／DB（deploy-web.sh 會設 FT_API、FT_DB）
const API = process.env.FT_API || 'https://api.ahexagram.com';
const DB = process.env.FT_DB || 'fortunet';
const UA = 'Mozilla/5.0 (FortuneT web-smoke)';
const TEST_EMAIL = 'ft-web-smoke@example.invalid';

const args = { base: null, login: false, timeout: 25000 };
for (let i = 2; i < process.argv.length; i++) {
  const k = process.argv[i];
  if (k === '--base') args.base = process.argv[++i];
  else if (k === '--login') args.login = true;
  else if (k === '--timeout') args.timeout = Number(process.argv[++i]);
  else { console.error(`unknown arg ${k}`); process.exit(2); }
}
if (!args.base) { console.error('usage: web-smoke.mjs --base <url> [--login]'); process.exit(2); }
const BASE = args.base.replace(/\/$/, '');

const results = [];
function step(name, ok, detail = '') {
  results.push(ok);
  console.log(`${ok ? '✅' : '❌'} ${name}${detail ? ` — ${detail}` : ''}`);
}

function findChromium() {
  const root = path.join(os.homedir(), '.cache', 'ms-playwright');
  const dirs = fs.existsSync(root) ? fs.readdirSync(root).filter(d => d.startsWith('chromium_headless_shell-')).sort().reverse() : [];
  for (const d of dirs) {
    for (const sub of ['chrome-headless-shell-linux64/chrome-headless-shell', 'chrome-linux/headless_shell']) {
      const p = path.join(root, d, sub);
      if (fs.existsSync(p)) return p;
    }
  }
  return null;
}

const sleep = ms => new Promise(r => setTimeout(r, ms));

// ── 頁面載入：等 <main> 有內容；收集 JS 例外與同源壞資產 ──
async function openPage(ctx, route, problems) {
  const page = await ctx.newPage();
  page.on('pageerror', e => problems.push(`${route}: JS 例外 ${String(e.message).slice(0, 160)}`));
  page.on('response', r => {
    const u = r.url();
    if (u.startsWith(BASE) && r.status() >= 400) problems.push(`${route}: ${r.status()} ${u.slice(BASE.length)}`);
  });
  await page.goto(BASE + route, { waitUntil: 'commit', timeout: args.timeout });
  await page.waitForFunction(() => {
    const m = document.querySelector('main');
    return m && m.innerText.trim().length > 20;
  }, null, { timeout: args.timeout });
  await sleep(800);
  return page;
}

async function imagesLoaded(page, selector) {
  return page.evaluate(async sel => {
    const root = document.querySelector(sel);
    if (!root) return { found: false, n: 0, ok: 0 };
    root.scrollIntoView({ block: 'center' });
    const imgs = [...root.querySelectorAll('img')];
    for (const i of imgs) i.loading = 'eager';
    const t0 = Date.now();
    while (Date.now() - t0 < 15000) {
      if (imgs.every(i => i.complete)) break;
      await new Promise(r => setTimeout(r, 200));
    }
    return { found: true, n: imgs.length, ok: imgs.filter(i => i.complete && i.naturalWidth > 0).length };
  }, selector);
}

// ── 測試帳號（--login）──
function turso(sql) {
  return execFileSync('turso', ['db', 'shell', DB, sql], { encoding: 'utf8', timeout: 30000 });
}
async function apiCall(method, p, body, token) {
  const headers = { 'Content-Type': 'application/json', 'User-Agent': UA, Origin: 'https://ahexagram.com' };
  if (token) headers.Authorization = `Bearer ${token}`;
  const r = await fetch(API + p, { method, headers, body: body ? JSON.stringify(body) : undefined });
  let j = null; try { j = await r.json(); } catch { /* empty */ }
  return [r.status, j];
}
async function makeTestUser() {
  const plain = crypto.randomBytes(32).toString('hex');
  const hash = crypto.createHash('sha256').update(plain).digest('hex');
  const exp = new Date(Date.now() + 10 * 60000).toISOString();
  turso(`INSERT INTO login_tokens (token_hash, email, expires_at, used_at, pending_full_name, pending_invite_code) VALUES ('${hash}', '${TEST_EMAIL}', '${exp}', NULL, 'Web smoke', NULL)`);
  const [s, b] = await apiCall('POST', '/api/auth/verify', { token: plain });
  if (s !== 200) throw new Error(`verify ${s} ${JSON.stringify(b)}`);
  const sid = b.sessionId;
  const [, me] = await apiCall('GET', '/api/users/me', null, sid);
  const uid = me.id || me.userId;
  const [sb] = await apiCall('PUT', '/api/users/me/birth', { birth_year: 1990, birth_month: 5, birth_day: 15, birth_hour: 12, gender: 'male' }, sid);
  if (sb !== 200) throw new Error(`birth ${sb}`);
  return { sid, uid };
}
function cleanupTestUser(uid) {
  for (const t of ['tarot_readings', 'ziwei_year_readings', 'interpretations', 'shop_interest', 'zodiac_match_readings', 'talent_trials']) {
    try { turso(`DELETE FROM ${t} WHERE user_id='${uid}'`); } catch { /* table may not exist */ }
  }
  turso(`DELETE FROM users WHERE id='${uid}'`);
  turso(`DELETE FROM login_tokens WHERE email='${TEST_EMAIL}'`);
}

const FS_ANSWERS = {
  bedroom: { bed_back: 'window', bed_door_view: 'facing', bed_overhead: 'beam', bed_mirror: 'visible', bed_corner: 'present', bed_door_door: 'facing', bed_light: 'leak', bed_clutter: 'lots' },
  study: { seat_back: 'passage', seat_door_view: 'hidden', seat_overhead: 'beam', seat_front: 'wall', seat_corner: 'present', seat_light: 'enough', seat_clutter: 'some' },
  office: { office_back: 'aisle', office_door: 'entrance', office_front: 'open', office_overhead: 'lamp', office_corner: 'none', office_light: 'enough', office_clutter: 'little' },
};

async function main() {
  let chromium;
  try { ({ chromium } = require(PW_DIR)); } catch (e) { console.error(`cannot load playwright-core from ${PW_DIR}: ${e.message}`); process.exit(2); }
  const exe = findChromium();
  if (!exe) { console.error('no chromium_headless_shell under ~/.cache/ms-playwright'); process.exit(2); }
  const browser = await chromium.launch({ executablePath: exe, args: ['--disable-gpu'], headless: true });
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, userAgent: UA });
  ctx.setDefaultTimeout(args.timeout);
  const problems = [];
  console.log(`web-smoke → ${BASE}\n`);

  try {
    // 1. 公開頁 mount
    for (const route of ['/', '/tarot', '/fengshui', '/fengshui/office', '/shop', '/try', '/ziwei', '/zodiac', '/naming', '/glossary', '/palm', '/login']) {
      try {
        const p = await openPage(ctx, route, problems);
        step(`mount ${route}`, true);
        await p.close();
      } catch (e) { step(`mount ${route}`, false, String(e.message).split('\n')[0]); }
    }

    // 2. 快取防線
    {
      const p = await openPage(ctx, '/', problems);
      const assets = await p.evaluate(() => [...document.querySelectorAll('script[src],link[rel=stylesheet]')].map(e => e.src || e.href));
      const html = await p.evaluate(async () => (await fetch('/boot.js', { cache: 'no-store' })).text());
      const need = [['boot.js', assets], ['style.css', assets]];
      for (const [name, list] of need) {
        const hit = list.find(u => u.includes(name));
        step(`快取 hash ${name}`, !!hit && /\?v=/.test(hit), hit || '找不到');
      }
      step('快取 hash ft_web.js（boot.js 內）', /ft_web\.js\?v=/.test(html));
      // 首頁功能卡
      const cards = await p.evaluate(() => [...document.querySelectorAll('a.feature')].map(a => a.getAttribute('href')));
      step('首頁功能卡', cards.length >= 8 && cards.includes('/shop') && cards.includes('/fengshui') && cards.includes('/try'), `${cards.length} 張`);
      const ms = await imagesLoaded(p, '.mascot-hero');
      step('首頁吉祥物 sprite stack 圖集載入', ms.found && ms.n > 0 && ms.ok === ms.n, `${ms.ok}/${ms.n}`);
      // 引路貓：圖集檔名帶月相（<pose>-<phase>-<hash>.webp）；新月夜睡著時按一下叫醒要出現台詞
      const sheet = await p.evaluate(() => document.querySelector('.mascot-hero')?.getAttribute('style') || '');
      step('首頁吉祥物圖集帶月相', /\/(wave|sleep)-(new|crescent|quarter|gibbous|full)-[0-9a-f]{12}\.webp/.test(sheet), sheet.match(/stack\/[^']+/)?.[0] || sheet);
      if (await p.locator('.mascot-wake').count()) {
        await p.locator('.mascot-wake').click();
        await sleep(400);
        const said = await p.locator('p.mascot-say').count();
        step('新月夜叫醒月亮貓有台詞', said > 0);
      }
      await p.close();
    }

    // 3. 開運小物
    {
      const p = await openPage(ctx, '/shop', problems);
      const n = await p.locator('.shop-card').count();
      step('開運小物 6 張卡', n === 6, `${n}`);
      const st = await imagesLoaded(p, '.stack-stage');
      step('盆栽 sprite stack 圖全部載入', st.found && st.n > 0 && st.ok === st.n, `${st.ok}/${st.n}`);
      const ill = await imagesLoaded(p, '.shop-grid, main');
      step('開運小物圖片全部載入', ill.ok === ill.n, `${ill.ok}/${ill.n}`);
      // 匿名登記→取消（淨零）
      const btn = p.locator('.shop-card button').first();
      const before = (await btn.innerText()).trim();
      await btn.click();
      await p.waitForFunction(() => !document.querySelector('.shop-card button')?.disabled, null, { timeout: 15000 });
      await sleep(400);
      const mid = (await btn.innerText()).trim();
      await btn.click();
      await p.waitForFunction(() => !document.querySelector('.shop-card button')?.disabled, null, { timeout: 15000 });
      await sleep(400);
      const after = (await btn.innerText()).trim();
      step('匿名「我有興趣」登記→取消', before !== mid && after === before, `${before} → ${mid} → ${after}`);
      await p.close();
    }

    // 4. 空間風水：選房間縮景＋三房型結果頁縮景
    {
      const p = await openPage(ctx, '/fengshui', problems);
      const thumbs = await p.evaluate(() => [...document.querySelectorAll('.fs-room-stack')].map(s => {
        const im = [...s.querySelectorAll('img')]; return [im.length, im.filter(i => i.complete && i.naturalWidth > 0).length];
      }));
      await sleep(1500);
      const thumbs2 = await p.evaluate(() => [...document.querySelectorAll('.fs-room-stack')].map(s => {
        const im = [...s.querySelectorAll('img')]; return [im.length, im.filter(i => i.complete && i.naturalWidth > 0).length];
      }));
      step('選房間三個縮景載入', thumbs2.length === 3 && thumbs2.every(([n, ok]) => n > 0 && n === ok), JSON.stringify(thumbs2 || thumbs));
      await p.evaluate(a => { for (const [room, ans] of Object.entries(a)) localStorage.setItem(`fengshui_room_v1_${room}`, JSON.stringify(ans)); }, FS_ANSWERS);
      await p.close();
      for (const room of Object.keys(FS_ANSWERS)) {
        const q = await openPage(ctx, `/fengshui/${room}`, problems);
        const st = await imagesLoaded(q, '.fs-diorama');
        const cards = await q.locator('.fs-card').count();
        step(`風水結果縮景 ${room}`, st.found && st.n > 0 && st.ok === st.n && cards > 0, `${st.ok}/${st.n} 圖、${cards} 張卡`);
        await q.close();
      }
      // 清掉灌的答案（同一個 context 的 localStorage）
      const c = await openPage(ctx, '/fengshui', problems);
      await c.evaluate(() => { for (const r of ['bedroom', 'study', 'office']) localStorage.removeItem(`fengshui_room_v1_${r}`); });
      await c.close();
    }

    // 5. 塔羅今日抽牌、紫微選單
    {
      const p = await openPage(ctx, '/tarot', problems);
      // 吉祥物兩張圖集（期待／開心）翻牌前就要載完（隱藏預載 img），翻牌瞬間才不會閃
      const tm = await imagesLoaded(p, '.tarot-mascot');
      step('塔羅吉祥物兩張圖集預載', tm.found && tm.n === 2 && tm.ok === tm.n, `${tm.ok}/${tm.n}`);
      const card = p.locator('.tarot-card').first();
      await card.click();
      await sleep(1200);
      const flipped = await card.evaluate(e => e.classList.contains('flipped'));
      step('塔羅今日抽牌可翻', flipped);
      await p.close();
      const z = await openPage(ctx, '/ziwei', problems);
      const links = await z.evaluate(() => [...document.querySelectorAll('a')].map(a => a.getAttribute('href')));
      // 未登入時子單元連結走 /login?next=…（設計如此），登入後才是直接路徑
      step('紫微選單三張子單元', ['/ziwei/chart', '/ziwei/luck', '/ziwei/year'].every(h => links.some(l => l === h || l === `/login?next=${h}`)));
      await z.close();
      // 上手小實驗：未登入顯示登入連結（回流 /try）
      const t = await openPage(ctx, '/try', problems);
      const tryLogin = await t.locator('a[href="/login?next=/try"]').count();
      step('上手小實驗未登入有登入連結', tryLogin > 0, `${tryLogin}`);
      await t.close();
    }

    // 6. 登入後（--login）
    if (args.login) {
      let user = null;
      try {
        user = await makeTestUser();
        step('自造測試帳號登入', true);
        const p = await openPage(ctx, '/', problems);
        await p.evaluate(sid => localStorage.setItem('sessionId', sid), user.sid);
        await p.close();
        const luck = await openPage(ctx, '/ziwei/luck', problems);
        await luck.waitForFunction(() => document.querySelectorAll('.palace').length === 12, null, { timeout: args.timeout });
        const badges = await luck.evaluate(() => document.querySelectorAll('.zw-badge').length);
        step('流年大運十二宮＋標記', badges >= 2, `${badges} 個標記`);
        await luck.close();
        const chart = await openPage(ctx, '/ziwei/chart', problems);
        await chart.waitForFunction(() => document.querySelectorAll('.palace').length === 12, null, { timeout: args.timeout });
        step('紫微命盤十二宮', true);
        await chart.close();
        const love = await openPage(ctx, '/tarot/love', problems);
        const h1 = await love.evaluate(() => document.querySelector('h1')?.innerText || '');
        step('塔羅感情頁（不抽牌）', h1.includes('感情'), h1);
        await love.close();
        // 上手小實驗整鏈（淨零：最後由 UI 刪除；cleanup 另以 SQL 兜底）
        const tr = await openPage(ctx, '/try', problems);
        try {
          await tr.waitForSelector('[data-testid=try-history]', { timeout: args.timeout });
          await tr.click('[data-testid=try-draw]');
          await tr.waitForFunction(() => document.querySelectorAll('.try-card').length === 3, null, { timeout: 5000 });
          step('上手小實驗抽三個', true);
          await tr.locator('.try-card').first().click();
          await tr.click('[data-testid=try-predict] .try-step[data-step="1"]');
          await tr.click('[data-testid=try-predict-submit]');
          await tr.waitForSelector('[data-testid=try-open] [data-testid=try-report]', { timeout: args.timeout });
          await tr.click('[data-testid=try-report] .try-step[data-step="2"]');
          await tr.click('[data-testid=try-report-submit]');
          await tr.waitForSelector('[data-testid=try-result]', { timeout: args.timeout });
          const res = await tr.locator('[data-testid=try-result]').innerText();
          step('上手小實驗猜 1 → 實際 2 結果卡', res.includes('多走了 1 步'), res.replace(/\s+/g, ' ').slice(0, 80));
          const del = tr.locator('.try-history-row .try-del').first();
          await del.click();
          await tr.waitForFunction(() => (document.querySelector('.try-history-row .try-del')?.innerText || '').includes('再點一次'), null, { timeout: 5000 });
          await del.click();
          await tr.waitForSelector('[data-testid=try-history-empty]', { timeout: args.timeout });
          step('上手小實驗 UI 刪除後紀錄為空', true);
        } catch (e) {
          step('上手小實驗整鏈', false, String(e.message).split('\n')[0]);
        }
        await tr.close();
      } catch (e) {
        step('登入後檢查', false, String(e.message).split('\n')[0]);
      } finally {
        if (user) { cleanupTestUser(user.uid); console.log('ℹ️  測試帳號已 SQL 硬清'); }
        else { try { turso(`DELETE FROM login_tokens WHERE email='${TEST_EMAIL}'`); } catch { /* ignore */ } }
      }
    }

    // 7. 彙總資產／例外問題
    const uniq = [...new Set(problems)];
    step('無 JS 例外、無同源壞資產', uniq.length === 0, uniq.slice(0, 8).join('；'));
  } finally {
    await browser.close();
  }
  const ok = results.length > 0 && results.every(Boolean);
  console.log(`\n${ok ? '✅ web-smoke 全部通過' : `❌ web-smoke 有 ${results.filter(r => !r).length} 項失敗`}`);
  process.exit(ok ? 0 : 1);
}

main().catch(e => { console.error(e); process.exit(1); });
