// k6 load test script
// Run: k6 run load-test.js

import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 20 },  // Ramp up to 20 users
    { duration: '1m', target: 50 },   // Stay at 50 users
    { duration: '30s', target: 100 }, // Peak at 100 users
    { duration: '1m', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<200'], // 95% of requests under 200ms
    http_req_failed: ['rate<0.01'],   // Less than 1% errors
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
  });

  check(ziweiRes, {
    'ziwei status is 200': (r) => r.status === 200,
    'ziwei has palaces': (r) => JSON.parse(r.body).palaces !== undefined,
  });

  sleep(1);

  // Test Western calculation
  const westernRes = http.post(`${BASE_URL}/api/charts/calculate/western`, JSON.stringify({
    year: 1990,
    month: 3,
    day: 25,
    hour: 12
  }), {
    headers: { 'Content-Type': 'application/json' },
  });

  check(westernRes, {
    'western status is 200': (r) => r.status === 200,
    'western has sunSign': (r) => JSON.parse(r.body).sunSign !== undefined,
  });

  sleep(1);
}
