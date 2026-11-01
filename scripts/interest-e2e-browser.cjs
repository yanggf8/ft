// interest-e2e-browser.cjs — 興趣探索 v1 正式站瀏覽器 E2E（由 scripts/interest-e2e.sh 呼叫）。
// 真登入（同 f8-acceptance.sh 自造 login token）、不攔截 API；finally 硬清測試帳號。
// 環境變數：FORTUNET_API / FORTUNET_WEB / FORTUNET_EMAIL / FORTUNET_DB（由 .sh 匯出）、
// PLAYWRIGHT_MODULE（playwright 套件路徑）、CHROME_PATH（chromium 執行檔）。
const { execFileSync } = require('child_process');
const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

const API = (process.env.FORTUNET_API || 'https://fortunet-api.yanggf.workers.dev').replace(/\/$/, '');
const WEB = (process.env.FORTUNET_WEB || 'https://ahexagram.com').replace(/\/$/, '');
const EMAIL = process.env.FORTUNET_EMAIL || 'interest-acceptance@ahexagram.com';
const DB = process.env.FORTUNET_DB || 'fortunet';
// Cloudflare 擋非瀏覽器 UA（1010）
const UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

function loadPlaywright() {
  if (process.env.PLAYWRIGHT_MODULE) return require(process.env.PLAYWRIGHT_MODULE);
  const root = execFileSync('npm', ['root', '-g'], { encoding: 'utf8' }).trim();
  for (const p of ['playwright', '@playwright/mcp/node_modules/playwright']) {
    if (fs.existsSync(path.join(root, p))) return require(path.join(root, p));
  }
  console.error('❌ 找不到 playwright：設 PLAYWRIGHT_MODULE 或 npm i -g playwright');
  process.exit(2);
}

const results = [];
function check(name, ok, note = '') {
  results.push(ok);
  console.log(`${ok ? '✅' : '❌'} ${name}${note ? ' — ' + note : ''}`);
}
function q(s) { return s.replace(/'/g, "''"); }
function turso(sql) {
  return execFileSync('turso', ['db', 'shell', DB, sql], { encoding: 'utf8', timeout: 30000 });
}
function lastCell(out) { return out.trim().split('\n').pop().trim(); }
// headless Chrome 對 Cloudflare 偶發 hang，重試即過
async function go(page, url, tries = 4) {
  for (let i = 0; i < tries; i++) {
    try { await page.goto(url, { waitUntil: 'commit', timeout: 15000 }); return; }
    catch (e) {
      console.log(`   (goto retry ${i + 1} — ${url})`);
      await page.goto('about:blank', { waitUntil: 'commit', timeout: 5000 }).catch(() => {});
      if (i === tries - 1) throw e;
    }
  }
}
// 作答 48 題；perType[t] 為第 t 型（R I A S E C）的作答值。題號 k 為顯示序，交錯後 k % 6 即型別。
async function answerAll(page, perType) {
  await page.click('.interest-card button:has-text("開始")', { force: true });
  for (let p = 0; p < 6; p++) {
    await page.waitForSelector(`.interest-quiz .quiz-progress:has-text("第 ${p + 1} / 6 頁")`, { timeout: 8000 });
    for (const f of await page.$$('.interest-quiz fieldset.quiz-item')) {
      const k = parseInt((await f.$eval('.quiz-no', e => e.textContent)).trim(), 10) - 1;
      await (await f.$(`input[value="${perType[k % 6]}"]`)).check({ force: true });
    }
    if (p < 5) await page.click('.interest-quiz button:has-text("下一頁")', { force: true });
  }
  await page.click('.interest-quiz button:has-text("查看結果")', { force: true });
  await page.waitForSelector('.interest-result', { timeout: 15000 });
}

(async () => {
  const { chromium } = loadPlaywright();
  let uid = null, browser = null;
  try {
    // ── 0. 登入 ──
    const plain = crypto.randomBytes(32).toString('hex');
    const hash = crypto.createHash('sha256').update(plain).digest('hex');
    const exp = new Date(Date.now() + 10 * 60 * 1000).toISOString();
    turso(`INSERT INTO login_tokens (token_hash, email, expires_at, used_at, pending_full_name, pending_invite_code) VALUES ('${hash}', '${q(EMAIL)}', '${exp}', NULL, 'Interest acceptance test', NULL)`);
    const vr = await fetch(API + '/api/auth/verify', { method: 'POST', headers: { 'content-type': 'application/json', 'user-agent': UA }, body: JSON.stringify({ token: plain }) });
    const vb = await vr.json();
    if (vr.status !== 200 || !vb.sessionId) throw new Error(`verify -> ${vr.status} ${JSON.stringify(vb)}`);
    const sid = vb.sessionId;
    const me = await (await fetch(API + '/api/users/me', { headers: { authorization: `Bearer ${sid}`, 'user-agent': UA } })).json();
    uid = me.id;
    check('登入測試帳號', !!uid, EMAIL);
    if (!uid) throw new Error('no user id');

    // ── 1. 瀏覽器（galaxy.js 在 headless 會凍結頁面，擋掉）──
    browser = await chromium.launch({
      executablePath: process.env.CHROME_PATH || undefined,
      headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu'],
    });
    const ctx = await browser.newContext({ viewport: { width: 390, height: 900 } });
    await ctx.route('**/galaxy.js', r => r.abort());
    const page = await ctx.newPage();
    const errors = [];
    const calls = [];
    page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
    page.on('pageerror', e => errors.push(String(e)));
    page.on('response', r => { if (r.url().includes('/api/interest')) calls.push(`${r.request().method()} ${new URL(r.url()).pathname} ${r.status()}`); });

    await go(page, WEB + '/');
    await page.evaluate(s => localStorage.setItem('sessionId', s), sid);
    await go(page, WEB + '/profile#interest-title');
    await page.waitForSelector('text=開始（約 5 分鐘，48 題）', { timeout: 25000 });
    check('我的命格顯示興趣卡開始鈕', true);

    // ── 2. 作答（R5 I4 A3 S3 E2 C1）──
    await answerAll(page, [5, 4, 3, 3, 2, 1]);
    const res = await page.textContent('.interest-result');
    const picked = await page.$$eval('.interest-directions li strong', s => s.map(x => x.textContent.trim()));
    check('結果頁六條長條', (await page.$$('.interest-bar-row')).length === 6);
    check('較明顯的方向＝只有實作型（R5 與 I4 差 1.0 > 0.5）', res.includes('較明顯的方向') && JSON.stringify(picked) === '["實作型"]', JSON.stringify(picked));
    check('共 1 筆紀錄', (await page.textContent('.interest-card')).includes('共 1 筆紀錄'));
    const db = turso(`SELECT answers FROM interest_results WHERE user_id = '${q(uid)}'`);
    check('DB 寫入一列、作答依題庫索引', db.includes('[5,5,5,5,5,5,5,5,4,4,4,4,4,4,4,4,3,'), lastCell(db).slice(0, 60));

    // ── 3. 重新整理後讀回 ──
    await page.reload({ waitUntil: 'commit' });
    await page.waitForSelector('.interest-result', { timeout: 25000 });
    check('重新整理後由 API 讀回結果', (await page.textContent('.interest-result')).includes('實作型'));

    // ── 4. 歷史 ──
    await page.click('.interest-card button:has-text("查看過去結果")', { force: true });
    await page.waitForSelector('.interest-card .practice-history-row', { timeout: 10000 });
    check('歷史列含較明顯：實作型', (await page.textContent('.interest-card .practice-history')).includes('較明顯：實作型'));

    // ── 5. 刪除 ──
    let msg = '';
    page.once('dialog', d => { msg = d.message(); d.accept(); });
    await page.click('.interest-card button:has-text("刪除全部興趣探索紀錄")', { force: true });
    await page.waitForSelector('text=興趣探索紀錄已刪除。', { timeout: 10000 });
    check('刪除確認含筆數', msg.includes('全部 1 筆'), msg.slice(0, 40));
    check('DB 已刪', lastCell(turso(`SELECT COUNT(*) FROM interest_results WHERE user_id = '${q(uid)}'`)) === '0');
    check('刪除後回到開始', (await page.$('.interest-card button:has-text("開始")')) !== null);

    // ── 6. 刪除後立即再作答（epoch 已更新，不應 409）──
    await answerAll(page, [1, 1, 1, 1, 1, 1]);
    check('刪除後再作答直接成功＋無突出偏好文案', (await page.textContent('.interest-result')).includes('目前沒有特別突出的偏好'));

    console.log('   interest API calls:', calls.join(' | '));
    check('interest API 無 4xx/5xx', calls.length > 0 && calls.every(c => /\s(200|201)$/.test(c)));
    const csp = errors.filter(e => /Content Security Policy|Refused to/i.test(e));
    check('無 CSP 違規', csp.length === 0, csp.slice(0, 2).join(' | '));
    // galaxy.js 被擋會留下每次載入一則 net::ERR_FAILED，屬預期
    const other = errors.filter(e => !/Content Security Policy|Refused to|net::ERR_FAILED/i.test(e));
    if (other.length) console.log('   其他 console errors:', other.slice(0, 6));
  } catch (e) {
    check('執行未中斷', false, String(e).slice(0, 200));
  } finally {
    if (browser) await browser.close();
    if (uid) {
      turso(`DELETE FROM interest_results WHERE user_id = '${q(uid)}'`);
      turso(`DELETE FROM users WHERE id = '${q(uid)}'`);
    }
    turso(`DELETE FROM login_tokens WHERE email = '${q(EMAIL)}'`);
    console.log(`ℹ️  cleanup：測試帳號殘留 ${lastCell(turso(`SELECT COUNT(*) FROM users WHERE email = '${q(EMAIL)}'`))} 列`);
    const fail = results.filter(r => !r).length;
    console.log(`\n${fail ? '❌' : '✅'} 瀏覽器段 ${results.length - fail}/${results.length}`);
    process.exit(fail ? 1 : 0);
  }
})();
