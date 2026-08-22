# Repository Guidelines

## Project Structure & Module Organization
- `apps/` hosts the runnable products with their frameworks:
  - `api/`: Bun runtime, Hono HTTP server (`@hono/node-server`), tRPC API layer (uses `@my-monorepo/ai`, `@my-monorepo/auth`, `@my-monorepo/env`, `@my-monorepo/logger`, and `@my-monorepo/utils`). Dev server via `bun --watch`.
  - `cli/`: Rust command-line application managed through Cargo and exposed through pnpm/Vite+ workspace scripts.
  - `web/`: React + TanStack Router/React Query/React Start, built with Vite (uses `ui`, `i18n`).
  - `tauri/`: Tauri v2 desktop app (Rust core in `apps/tauri/src-tauri`) with a React + Vite frontend (uses `ui`, `i18n`).
- `packages/` contains shared libraries and their roles:
  - `tsconfig/`: shared TS configs (`base`, `hono`, `react-app`, `react-library`).
  - `utils/`: cross-app helpers for crypto/formatting/validation, plus shared libs (zod, validator, date-fns, superjson, etc.).
  - `env/`: type-safe environment variable validation using Zod.
  - `i18n/`: i18next setup, locale exports, and React hooks.
  - `ui/`: shared web UI components and styles (shadcn/ui, Radix, Tailwind, CVA utilities).
  - `logger/`: pino-based logger with context helpers.
  - `db/`: Prisma client wrapper and database schema management.
  - `auth/`: authentication layer using better-auth with Prisma adapter.
  - `ai/`: AI SDK wrapper (ai-sdk + OpenAI-compatible provider) for streaming chat.
- `docs/README.md` documents the dependency graph and how apps consume shared packages.

## Build, Test, and Development Commands
- `pnpm install` installs dependencies (pnpm is the package manager).
- `pnpm run dev:api|dev:cli|dev:web|dev:tauri` runs a single app via Vite+ task orchestration.
- `pnpm run build:cli`, `pnpm run build:web`, and `pnpm run build:tauri` build key targets.
- `pnpm run test` runs all Vitest suites in the workspace; `pnpm run test:coverage` enables coverage.
- `pnpm run check` runs Vite+ type checks; `pnpm run lint` runs `oxlint` plus `manypkg check`.
- `pnpm run format` / `pnpm run format:check` apply or verify formatting.
- `pnpm run precommit` is the full gate (format check, lint, types, tests).

## Coding Style & Naming Conventions
- Indentation is tabs in TypeScript/TSX (see existing source files); keep double quotes and let `oxfmt` enforce details.
- Use `oxlint` for linting; avoid manual reformatting—run `pnpm run format` instead.
- Workspace packages are imported as `@my-monorepo/<name>` (e.g., `@my-monorepo/ui`).
- Internal workspace dependencies use the `workspace:*` protocol in `package.json`.

## Testing Guidelines
- Tests use Vitest and typically live alongside code as `*.test.ts` (for example, `apps/web/src/app.test.ts`).
- Prefer focused unit tests for shared packages; app tests can cover integration of `api`, `i18n`, and `theme`.
- Use `pnpm run test` for full runs or `vp test --run` inside a package for targeted runs.

## Commit & Pull Request Guidelines
- Commit messages follow Conventional Commits observed in history (`feat: ...`, `fix: ...`).
- PRs should include a short summary, testing notes, and screenshots for UI-facing changes.
- Link relevant issues when applicable and call out any follow-up work.

## Security & Configuration Tips
- `.env` files are part of Turborepo global dependencies; keep secrets out of git and document required variables in PRs.
- Required env variables are documented in `.env.example` at the repo root.
