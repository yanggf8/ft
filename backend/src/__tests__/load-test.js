// k6 load test script
// Run: k6 run load-test.js
//
// NOTE: Chart endpoints require auth. This test registers a user,
// saves birth data, then repeatedly fetches charts.

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
    'http_req_duration{status:200}': ['p(95)<200'],
    'http_req_failed{status:!429}': ['rate<0.01'],
  },
};

const BASE_URL = 'https://fortunet-api.yanggf.workers.dev';

export function setup() {
  // Register a test user and save birth data
  const email = `loadtest-${Date.now()}@example.com`;
  const regRes = http.post(`${BASE_URL}/api/auth/register`, JSON.stringify({ email }), {
    headers: { 'Content-Type': 'application/json' },
  });
  const { sessionId } = JSON.parse(regRes.body);

  http.put(`${BASE_URL}/api/users/me/birth`, JSON.stringify({
    birth_year: 1990, birth_month: 5, birth_day: 15, birth_hour: 14, gender: 'male'
  }), {
    headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${sessionId}` },
  });

  return { sessionId };
}

export default function (data) {
  const headers = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${data.sessionId}`,
  };

  // Test ZiWei chart (GET — auto-calculates from stored birth data)
  const ziweiRes = http.get(`${BASE_URL}/api/charts/ziwei`, { headers, tags: { name: 'ziwei' } });

  check(ziweiRes, {
    'ziwei status is 200 or 429': (r) => r.status === 200 || r.status === 429,
    'ziwei has chart_data (if 200)': (r) => r.status !== 200 || JSON.parse(r.body).chart_data !== undefined,
  });

  sleep(2);

  // Test Western chart
  const westernRes = http.get(`${BASE_URL}/api/charts/western`, { headers, tags: { name: 'western' } });

  check(westernRes, {
    'western status is 200 or 429': (r) => r.status === 200 || r.status === 429,
    'western has chart_data (if 200)': (r) => r.status !== 200 || JSON.parse(r.body).chart_data !== undefined,
  });

  sleep(2);
}
