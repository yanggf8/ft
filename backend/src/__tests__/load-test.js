// k6 load test script
// Run: k6 run load-test.js
//
// NOTE: Calc endpoints have 30 req/min/IP rate limit.
// This test uses distributed IPs (k6 cloud) or expects 429s in local runs.
// For local testing, reduce VUs or increase sleep to stay under limits.

import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 10 },  // Ramp up to 10 users
    { duration: '1m', target: 20 },   // Stay at 20 users
    { duration: '30s', target: 30 },  // Peak at 30 users
    { duration: '30s', target: 0 },   // Ramp down
  ],
  thresholds: {
    // Measure only successful requests (exclude 429s)
    'http_req_duration{status:200}': ['p(95)<200'],
    'http_req_failed{status:!429}': ['rate<0.01'], // Non-rate-limit errors < 1%
  },
};

const BASE_URL = 'https://fortunet-api.yanggf.workers.dev';

export default function () {
  // Test ZiWei calculation
  const ziweiRes = http.post(`${BASE_URL}/api/charts/calculate/ziwei`, JSON.stringify({
    year: 1990,
    month: 5,
    day: 15,
    hour: 14,
    gender: 'male'
  }), {
    headers: { 'Content-Type': 'application/json' },
    tags: { name: 'ziwei' },
  });

  check(ziweiRes, {
    'ziwei status is 200 or 429': (r) => r.status === 200 || r.status === 429,
    'ziwei has palaces (if 200)': (r) => r.status !== 200 || JSON.parse(r.body).palaces !== undefined,
  });

  // Sleep 2s to respect rate limits (30 req/min = 1 req per 2s)
  sleep(2);

  // Test Western calculation
  const westernRes = http.post(`${BASE_URL}/api/charts/calculate/western`, JSON.stringify({
    year: 1990,
    month: 3,
    day: 25,
    hour: 12
  }), {
    headers: { 'Content-Type': 'application/json' },
    tags: { name: 'western' },
  });

  check(westernRes, {
    'western status is 200 or 429': (r) => r.status === 200 || r.status === 429,
    'western has sunSign (if 200)': (r) => r.status !== 200 || JSON.parse(r.body).sunSign !== undefined,
  });

  sleep(2);
}
