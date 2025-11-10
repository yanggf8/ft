# Repository Guidelines

## Project Structure & Module Organization
The root directory houses the migration playbooks (`README.md`, `IMPLEMENTATION_GUIDE.md`, `TESTING_MONITORING_PROCEDURES.md`, etc.); treat them as the canonical contract for agents. Code lives in `backend/` (Workers, Durable Objects, Wrangler configs), `frontend/` (Vite + React UI), `shared/` (cross-service types), and `scripts/` (D1 + automation helpers). Place new diagrams or decision logs inside `docs/` and cross-link them from `README.md` for traceability.

## Build, Test, and Development Commands
- `npm run dev --prefix backend` — Spins up Wrangler’s local edge runtime with mock bindings.
- `npm run dev --prefix frontend` — Launches the Vite dev server against `http://localhost:8787`.
- `npm run build --prefix frontend` — Runs `tsc` + Vite to emit the production bundle.
- `npm run deploy --prefix backend` (use `deploy:prod` for production) — Publishes the Worker.
- `npm run db:migrate --prefix backend` — Executes the latest D1 SQL in `backend/scripts/`.

## Coding Style & Naming Conventions
Use TypeScript everywhere with `strict` enabled; keep data contracts in `shared/types` so both runtimes stay in sync. Prefer functional React components, hooks, and PascalCase component names; stick to camelCase for variables and kebab-case filenames (`chart-cache-durable-object.ts`). Maintain two-space indentation. Run `npm run lint --prefix frontend` before opening a PR; Wrangler’s types cover backend checks.

## Testing Guidelines
Vitest drives both stacks: `npm run test --prefix backend`, `npm run test --prefix frontend`, and `npm run test:watch --prefix backend` for rapid loops. Keep unit tests close to their sources with `*.test.ts`, store heavier integration flows in `backend/tests/`, and stage Playwright/Cypress E2E specs under `tests/e2e/`. Adhere to the documented targets (≥90% unit, ≥85% integration, 100% critical E2E) from `TESTING_MONITORING_PROCEDURES.md` and summarize coverage plus any exclusions in each PR.

## Commit & Pull Request Guidelines
History shows concise, imperative commits (`docs: refine migration narrative`); follow the `<scope>: <action>` pattern and reserve the body for “why” context or follow-ups. PRs should include a short narrative, linked issue or doc anchor, screenshots for UI shifts, evidence of the relevant test commands, and a rollback note when touching Workers, D1 schema, or auth. Treat reviewers as downstream operators—flag migrations, feature toggles, and config prerequisites explicitly.

## Security & Configuration Tips
Keep secrets in Wrangler (`wrangler secret put ...`) and leave `.env*` files local. Scrub tenant IDs, API keys, and user data from examples before committing, and document any required environment variables beside the command that consumes them.
