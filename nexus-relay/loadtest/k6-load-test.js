import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const errorRate = new Rate('errors');
const responseDuration = new Trend('response_duration');

export const options = {
  stages: [
    { duration: '2m', target: 100 },
    { duration: '5m', target: 100 },
    { duration: '2m', target: 200 },
    { duration: '5m', target: 200 },
    { duration: '2m', target: 0 },
  ],
  thresholds: {
    'http_req_duration': ['p(95)<500', 'p(99)<1000'],
    'http_req_failed': ['rate<0.1'],
    'errors': ['rate<0.05'],
  },
};

const BASE_URL = `${__ENV.BASE_URL || 'https://localhost:8443'}`;

export default function () {
  group('Health Check', () => {
    const res = http.get(`${BASE_URL}/health`);
    check(res, {
      'status is 200': (r) => r.status === 200,
      'response time < 100ms': (r) => r.timings.duration < 100,
    });
    responseDuration.add(res.timings.duration);
    errorRate.add(res.status !== 200);
  });

  group('User Registration', () => {
    const payload = JSON.stringify({
      public_key: `user_${Date.now()}@example.com`,
      identity_key: `identity_${Date.now()}`,
    });

    const params = {
      headers: { 'Content-Type': 'application/json' },
      timeout: '10s',
    };

    const res = http.post(`${BASE_URL}/register`, payload, params);
    check(res, {
      'registration succeeds': (r) => r.status === 200 || r.status === 201,
      'response time < 200ms': (r) => r.timings.duration < 200,
    });
    responseDuration.add(res.timings.duration);
    errorRate.add(res.status >= 400);
  });

  group('Prekey Bundle Fetch', () => {
    const res = http.post(`${BASE_URL}/prekeys/user123`);
    check(res, {
      'prekey fetch succeeds': (r) => r.status === 200,
      'response time < 150ms': (r) => r.timings.duration < 150,
    });
    responseDuration.add(res.timings.duration);
    errorRate.add(res.status >= 400);
  });

  group('Message Send', () => {
    const payload = JSON.stringify({
      recipient: 'user456',
      sealed_content: 'encrypted_message_data',
      ttl: 3600,
    });

    const res = http.post(`${BASE_URL}/send`, payload, {
      headers: { 'Content-Type': 'application/json' },
    });
    responseDuration.add(res.timings.duration);
    errorRate.add(res.status >= 400);
  });

  sleep(Math.random() * 3 + 1);
}
