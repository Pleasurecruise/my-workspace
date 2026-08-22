# Repository Guidelines

## Architecture

- `apps/desktop`: Tauri v2 application. The frontend uses Svelte 5 and Vite; Rust owns commands and
  application behavior under `src-tauri`.
- `apps/cli`: Rust command-line binary.
- `crates/cms-core`: shared Rust CMS boundary; keep it at Hello World until concrete behavior is
  requested.
- `crates/logger`: shared Rust `tracing` initialization.
- `packages/ui`: Svelte components and design tokens.
- `packages/tsconfig`: UI-only TypeScript configuration.

Except for the Svelte view layer and its build configuration, new application code should be Rust.
Do not create a package until it is genuinely shared or owns a stable independent responsibility.

## Tooling

- `pnpm dev` runs the desktop application.
- `pnpm build:desktop` and `pnpm build:cli` build the two deliverables.
- `pnpm check`, `pnpm lint`, `pnpm test`, and `pnpm format:check` cover both pnpm and Cargo workspaces.
- Use Vite Plus for frontend formatting, linting, tests, and task orchestration.
- Use Cargo fmt, Clippy, check, and test for Rust.

TypeScript/Svelte uses tabs and double quotes. Rust follows `cargo fmt`. Dependencies owned by this
workspace use `workspace:*` in pnpm and workspace dependencies in Cargo.

## UI tokens

The three layers are `palette.css`, `tokens.css`, and `theme.css`. Components and application styles
consume semantic `--color-*`, `--font-*`, `--radius-*`, `--shadow-*`, and `--duration-*` tokens only.
Never reference `--palette-*` outside `tokens.css`; add new physical values to `palette.css` first.

## Environment

Packaged desktop and CLI code must not embed secrets. `RUST_LOG` controls local Rust logging.
Wrangler owns environment variables and secrets only when a Cloudflare Worker exists. Use
`voidPlugin()` only in that Worker app, not in the Tauri frontend.

Preserve unrelated user changes. Commit messages follow Conventional Commits.

<!-- context7 -->

Use the `ctx7` CLI to fetch current documentation whenever the user asks about a library, framework,
SDK, API, CLI tool, or cloud service. Resolve the library before fetching docs and do not run more
than three Context7 commands per question.
<!-- context7 -->
