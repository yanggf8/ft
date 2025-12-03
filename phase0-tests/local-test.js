#!/usr/bin/env node
/**
 * Phase 0: Local D1 Compatibility Test
 * Tests SQLite compatibility without needing Cloudflare
 * Run: node local-test.js
 */

import Database from 'better-sqlite3';
import { randomUUID } from 'crypto';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Colors for output
const green = (s) => `\x1b[32m${s}\x1b[0m`;
const red = (s) => `\x1b[31m${s}\x1b[0m`;
const yellow = (s) => `\x1b[33m${s}\x1b[0m`;
const bold = (s) => `\x1b[1m${s}\x1b[0m`;

console.log(bold('\n🧪 Phase 0: D1 (SQLite) Compatibility Test\n'));
console.log('=' .repeat(50));

// Create in-memory database (simulates D1)
const db = new Database(':memory:');

const results = [];

function test(name, fn) {
  const start = Date.now();
  try {
    fn();
    const ms = Date.now() - start;
    results.push({ name, passed: true, ms });
    console.log(green(`✓ ${name}`) + ` (${ms}ms)`);
  } catch (e) {
    const ms = Date.now() - start;
    results.push({ name, passed: false, ms, error: e.message });
    console.log(red(`✗ ${name}`) + ` - ${e.message}`);
  }
}

// Test 1: Schema Creation
test('Schema Creation', () => {
  db.exec(`
    CREATE TABLE IF NOT EXISTS users (
      id TEXT PRIMARY KEY,
      email TEXT UNIQUE NOT NULL,
      full_name TEXT,
      birth_location TEXT,
      subscription_tier TEXT DEFAULT 'free',
      created_at TEXT DEFAULT (datetime('now')),
      updated_at TEXT DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS chart_records (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      chart_type TEXT NOT NULL,
      chart_name TEXT NOT NULL,
      birth_data TEXT NOT NULL,
      chart_data TEXT NOT NULL,
      tags TEXT DEFAULT '[]',
      is_favorite INTEGER DEFAULT 0,
      created_at TEXT DEFAULT (datetime('now')),
      updated_at TEXT DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS subscriptions (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      stripe_subscription_id TEXT UNIQUE,
      tier TEXT NOT NULL,
      status TEXT NOT NULL,
      metadata TEXT DEFAULT '{}',
      created_at TEXT DEFAULT (datetime('now'))
    );

    CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
    CREATE INDEX IF NOT EXISTS idx_charts_user ON chart_records(user_id);
    CREATE INDEX IF NOT EXISTS idx_charts_type ON chart_records(chart_type);
  `);
  
  // Enable foreign keys
  db.pragma('foreign_keys = ON');
});

// Test 2: Insert with UUID
const userId = randomUUID();
test('Insert User with UUID', () => {
  db.prepare('INSERT INTO users (id, email, full_name) VALUES (?, ?, ?)')
    .run(userId, 'test@example.com', 'Test User');
});

// Test 3: Select by Email
test('Select User by Email', () => {
  const user = db.prepare('SELECT * FROM users WHERE email = ?').get('test@example.com');
  if (!user) throw new Error('User not found');
  if (user.id !== userId) throw new Error('Wrong user returned');
});

// Test 4: JSON in TEXT Column
const chartId = randomUUID();
const birthData = JSON.stringify({ year: 1990, month: 5, day: 15, hour: 10, gender: 'male' });
const chartData = JSON.stringify({ lifePalace: '命宮', stars: ['紫微', '天機', '太陽'] });

test('Insert Chart with JSON Data', () => {
  db.prepare(`
    INSERT INTO chart_records (id, user_id, chart_type, chart_name, birth_data, chart_data)
    VALUES (?, ?, ?, ?, ?, ?)
  `).run(chartId, userId, 'ziwei', '測試命盤', birthData, chartData);
});

test('Select and Parse JSON', () => {
  const chart = db.prepare('SELECT * FROM chart_records WHERE id = ?').get(chartId);
  const parsed = JSON.parse(chart.birth_data);
  if (parsed.year !== 1990) throw new Error('JSON parsing failed');
  if (parsed.gender !== 'male') throw new Error('JSON data incorrect');
});

// Test 5: Update with datetime()
test('Update with datetime()', () => {
  db.prepare(`UPDATE users SET full_name = ?, updated_at = datetime('now') WHERE id = ?`)
    .run('Updated Name', userId);
  const user = db.prepare('SELECT * FROM users WHERE id = ?').get(userId);
  if (user.full_name !== 'Updated Name') throw new Error('Update failed');
});

// Test 6: Pagination
test('Pagination (LIMIT/OFFSET)', () => {
  // Insert more charts
  for (let i = 0; i < 5; i++) {
    db.prepare(`
      INSERT INTO chart_records (id, user_id, chart_type, chart_name, birth_data, chart_data)
      VALUES (?, ?, ?, ?, ?, ?)
    `).run(randomUUID(), userId, 'ziwei', `Chart ${i}`, '{}', '{}');
  }
  
  const page1 = db.prepare(`
    SELECT * FROM chart_records WHERE user_id = ? 
    ORDER BY created_at DESC LIMIT ? OFFSET ?
  `).all(userId, 3, 0);
  
  if (page1.length !== 3) throw new Error(`Expected 3 rows, got ${page1.length}`);
});

// Test 7: JOIN Query
test('LEFT JOIN Query', () => {
  const result = db.prepare(`
    SELECT u.*, s.tier, s.status as sub_status
    FROM users u
    LEFT JOIN subscriptions s ON u.id = s.user_id
    WHERE u.id = ?
  `).get(userId);
  
  if (!result) throw new Error('JOIN query failed');
  if (result.email !== 'test@example.com') throw new Error('Wrong data returned');
});

// Test 8: Aggregation
test('GROUP BY Aggregation', () => {
  const result = db.prepare(`
    SELECT chart_type, COUNT(*) as count 
    FROM chart_records 
    WHERE user_id = ? 
    GROUP BY chart_type
  `).all(userId);
  
  if (!result.length) throw new Error('Aggregation returned no results');
});

// Test 9: LIKE Search
test('LIKE Search', () => {
  const result = db.prepare(`
    SELECT * FROM chart_records WHERE chart_name LIKE ?
  `).all('%Chart%');
  
  if (result.length < 5) throw new Error('LIKE search failed');
});

// Test 10: Date Comparison
test('Date Comparison', () => {
  const result = db.prepare(`
    SELECT * FROM chart_records 
    WHERE created_at > datetime('now', '-1 day')
  `).all();
  
  if (result.length === 0) throw new Error('Date comparison failed');
});

// Test 11: Batch Insert (Transaction)
test('Batch Insert (Transaction)', () => {
  const insert = db.prepare(`
    INSERT INTO chart_records (id, user_id, chart_type, chart_name, birth_data, chart_data)
    VALUES (?, ?, ?, ?, ?, ?)
  `);
  
  const insertMany = db.transaction((charts) => {
    for (const c of charts) insert.run(...c);
  });
  
  insertMany([
    [randomUUID(), userId, 'western_natal', 'Batch 1', '{}', '{}'],
    [randomUUID(), userId, 'western_natal', 'Batch 2', '{}', '{}'],
    [randomUUID(), userId, 'western_natal', 'Batch 3', '{}', '{}'],
  ]);
});

// Test 12: Chinese Characters
test('Chinese Character Storage', () => {
  const cnChartId = randomUUID();
  const cnData = JSON.stringify({ 
    宮位: '命宮', 
    主星: ['紫微星', '天機星'],
    description: '這是一個測試'
  });
  
  db.prepare(`
    INSERT INTO chart_records (id, user_id, chart_type, chart_name, birth_data, chart_data)
    VALUES (?, ?, ?, ?, ?, ?)
  `).run(cnChartId, userId, 'ziwei', '中文命盤名稱', '{}', cnData);
  
  const chart = db.prepare('SELECT * FROM chart_records WHERE id = ?').get(cnChartId);
  const parsed = JSON.parse(chart.chart_data);
  if (parsed.宮位 !== '命宮') throw new Error('Chinese character storage failed');
});

// Test 13: Foreign Key Cascade Delete
test('CASCADE Delete', () => {
  const tempUserId = randomUUID();
  const tempChartId = randomUUID();
  
  db.prepare('INSERT INTO users (id, email) VALUES (?, ?)').run(tempUserId, 'temp@test.com');
  db.prepare(`
    INSERT INTO chart_records (id, user_id, chart_type, chart_name, birth_data, chart_data)
    VALUES (?, ?, ?, ?, ?, ?)
  `).run(tempChartId, tempUserId, 'ziwei', 'Temp', '{}', '{}');
  
  db.prepare('DELETE FROM users WHERE id = ?').run(tempUserId);
  
  const chart = db.prepare('SELECT * FROM chart_records WHERE id = ?').get(tempChartId);
  if (chart) throw new Error('CASCADE delete failed - chart still exists');
});

// Summary
console.log('\n' + '='.repeat(50));
const passed = results.filter(r => r.passed).length;
const failed = results.length - passed;
const score = Math.round((passed / results.length) * 100);
const avgMs = Math.round(results.reduce((sum, r) => sum + r.ms, 0) / results.length);

console.log(bold('\n📊 Results Summary\n'));
console.log(`Total Tests: ${results.length}`);
console.log(`Passed: ${green(passed)}`);
console.log(`Failed: ${failed > 0 ? red(failed) : '0'}`);
console.log(`Score: ${score >= 90 ? green(score + '%') : score >= 70 ? yellow(score + '%') : red(score + '%')}`);
console.log(`Avg Response: ${avgMs}ms`);

console.log(bold('\n🎯 Recommendation\n'));
if (score >= 90) {
  console.log(green('✅ GO') + ' - D1 compatibility is excellent. Proceed to Phase 1.');
} else if (score >= 70) {
  console.log(yellow('⚠️ GO WITH CAUTION') + ' - Some issues need workarounds.');
} else {
  console.log(red('❌ NO-GO') + ' - Critical compatibility issues. Review migration strategy.');
}

// Failed tests details
if (failed > 0) {
  console.log(bold('\n❌ Failed Tests:\n'));
  results.filter(r => !r.passed).forEach(r => {
    console.log(red(`  • ${r.name}: ${r.error}`));
  });
}

console.log('\n');
db.close();

// Exit with appropriate code
process.exit(failed > 0 ? 1 : 0);
