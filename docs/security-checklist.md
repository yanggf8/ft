# Security Checklist

> Updated 2026-08-29 to match the implemented Rust backend (verified against
> `crates/api/src` at the magic-link fix). Supersedes the Phase 4 claims written for the
> old TS backend — including three false "passwordless (email-only)" checkmarks called out
> by `docs/audit/2026-08-29-codebase-audit.md` (finding P0-01), now fixed.

## Authentication & Authorization
- [x] Magic-link email verification: `POST /api/auth/login` and `/api/auth/register` email a
      single-use token (10-minute expiry, only its SHA-256 hash is stored in D1
      `login_tokens`); the session is created only by `POST /api/auth/verify` after the
      token is atomically consumed. Register additionally defers the `users` row itself to
      `verify` (the requested name rides on the token row as `pending_full_name`), so no
      account exists before email ownership is proven. NOT email-only login — the old
      "passwordless (email-only)" checkmark was false and has been removed.
- [x] Token randomness fail-closed (A-01): 256-bit token from `globalThis.crypto`; if crypto
      is unavailable the login attempt is rejected, never fallen back to a weak source.
- [x] Session-based authentication with Durable Objects (SessionDO, 7-day TTL; refresh extends)
- [x] Session revocation epoch (A-02): every session carries a `createdAtIso` stamp; any
      session without it (i.e. minted before the magic-link deploy) is rejected and deleted
      on `get`/`refresh`, revoking all pre-fix sessions at once.
- [x] Auth middleware on protected routes (`common::auth_user`; users/charts/personality all
      scope D1 queries by the session `user_id` — audit found no IDOR)
- [x] Rate limiting on auth endpoints (RateLimitDO): login/register 10 req/min per IP plus
      5 req/min per email; verify 10 req/min per IP
- [x] Anti-enumeration (A-03): login and register return an identical 202 body whether or
      not the address exists; an unknown address on login silently receives no email. Both
      paths do the same D1 work before answering. KNOWN RESIDUAL: the Resend API call for
      an existing address still runs inline, so response latency can hint at account
      existence (worker 0.8.5 exposes no `wait_until` on `RouteContext`); closing it needs
      a vendored worker-crate patch or a queue-based sender.
- [ ] CSRF protection (N/A: the session rides in localStorage + Bearer header, never cookies)

## API Security
- [x] Rate limiting is Durable Object-backed (RateLimitDO, cross-isolate, sharded
      `rl:{fnv1a(key) % 8}`) — replaces the old isolate-local limiter whose counters reset
      on every cold start (audit P2-02)
- [x] Rate limiting on AI endpoints (10 req/min per IP on `/api/charts/story/generate` and
      `/api/charts/:type/interpret`), plus AIMutexDO provider rpm/rpd caps and
      1-concurrent request serialization
- [x] Rate limiting on personality endpoints (10 req/min per IP, own `personality:ip:`
      bucket — no longer shares the auth budget)
- [ ] Rate limiting on chart calculation endpoints — not implemented (calc is served from
      cache / engine service binding, no per-IP cap; revisit if abuse appears)
- [x] Input validation in Rust handlers (email shape, token length, finite-JD validation in
      the engine) — the old "Zod" claim died with the TS backend
- [x] SQL injection prevention (parameterized queries; audit found no concatenated SQL)
- [ ] Request size limits
- [x] CORS: exact-origin allowlist only — `https://fortunet.pages.dev`, localhost /
      127.0.0.1 / [::1] (dev, any port), plus exact origins from the `ALLOWED_ORIGINS` env
      var (preview deploys). Matched on scheme + hostname + port via URL parsing, never
      substring; the `*.workers.dev` / `*.pages.dev` wildcards are gone (P2-03). No
      `Access-Control-Allow-Credentials` (A-04 — unnecessary under the Bearer model);
      `Vary: Origin` is always set.

## Headers
(Set on every response, preflights included, in `crates/api/src/lib.rs::decorate`)
- [x] Content-Security-Policy: `default-src 'none'; frame-ancestors 'none'`
- [x] X-Frame-Options: DENY
- [x] X-Content-Type-Options: nosniff
- [x] Referrer-Policy: strict-origin-when-cross-origin (plus Permissions-Policy,
      X-XSS-Protection, x-request-id)
- [ ] Strict-Transport-Security — not emitted by the Worker; HTTPS is enforced at the
      Cloudflare edge. Add an explicit header if HSTS preloading matters.

## Data Protection
- [x] No password credentials exist or are stored — email ownership is proven by the
      magic link (this corrects the removed false "passwordless auth" claim)
- [x] User data isolated by user_id
- [x] Login tokens stored hashed (SHA-256); the plain token exists only inside the emailed link
- [x] API keys in secrets (`RESEND_API_KEY` secret, `MAIL_FROM` var, AI provider keys)
- [ ] PII encryption at rest (considered, not required for astrology app; deferred)

## CI (`.github/workflows/deploy.yml`)
- [x] `cargo fmt --all --check`
- [x] `cargo clippy` on wasm32, `--all-targets`
- [x] Native `cargo test --locked` for ft-schema / ft-ziwei / ft-western / ft-big5
      (audit P1-01: the 26 `#[test]`s previously never ran in CI)
- [x] Wasm builds for ft-worker / ft-api / ft-web with `--locked`
- Deployment stays manual (wrangler via OAuth); no cloud credentials in CI

## OWASP Top 10 (2021)
1. **Broken Access Control** - ✅ Auth middleware enforced, queries scoped by session user_id
2. **Cryptographic Failures** - ✅ HTTPS via Cloudflare; login tokens 256-bit random, stored hashed
3. **Injection** - ✅ Parameterized queries
4. **Insecure Design** - ✅ DO-backed rate limiting, magic-link verification, anti-enumeration
5. **Security Misconfiguration** - ✅ Security headers + exact-origin CORS allowlist (P2-03/A-04 fixed)
6. **Vulnerable Components** - ✅ CI builds and tests with `--locked` (lockfile-pinned resolution)
7. **Auth Failures** - ✅ Magic-link email verification (single-use hashed 10-min token),
   session revocation epoch, DO-backed rate limits. Previously this row falsely certified
   email-only login with no ownership proof (audit P0-01); fixed 2026-08-29.
8. **Data Integrity Failures** - ✅ Input validation in Rust handlers
9. **Logging Failures** - ⚠️ Basic console logging; structured logging deferred (nice-to-have)
10. **SSRF** - ✅ No user-controlled URLs

## Cloudflare-Specific
- [x] Workers deployed with secrets
- [x] D1 database with proper schema (`login_tokens` added for magic links)
- [x] Durable Objects for session isolation (SessionDO) and rate limiting (RateLimitDO)
- [ ] WAF rules (Cloudflare default protection active; custom rules deferred)
- [x] DDoS protection (automatic via Cloudflare)

## Action Items (Historical — Phase 4)
> **Status**: the original Phase 4 list is kept for the record; the 2026-08-29 audit
> (`docs/audit/2026-08-29-codebase-audit.md`) was fixed on top of it.

1. ~~Add security headers middleware~~ ✅ Done (Phase 5; carried into the Rust `lib.rs::decorate`)
2. ~~Implement session TTL~~ ✅ Done (7-day via SessionDO, plus the A-02 revocation epoch)
3. Add structured logging — Deferred (nice-to-have)
4. Configure WAF rules — Deferred (Cloudflare defaults sufficient)
5. Add request size limits — Still open (see API Security; Workers/D1 tolerate oversized
   bodies, but the cap was never actually added)

### 2026-08-29 audit follow-ups (fixed)
- ~~P0-01 — login created sessions from an email alone, no verification; the checklist
  marked it secure~~ ✅ Magic-link email verification shipped
- ~~P1-01 — CI never ran tests~~ ✅ `cargo test --locked` step added
- ~~P2-02 — rate limiter was isolate-local, auth/personality shared a bucket, the AI limiter
  was per-request~~ ✅ RateLimitDO with per-endpoint namespaced buckets
- ~~P2-03 — CORS allowed any `*.workers.dev` / `*.pages.dev` origin with credentials~~ ✅
  Exact allowlist, no credentials header
- A-01..A-04 (fail-closed token randomness, revocation epoch, uniform 202, no credentials
  header) ✅
