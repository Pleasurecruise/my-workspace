# Repository Guidelines

## Read first

- [Architecture](docs/ARCHITECTURE.md)
- [Dashboard integrations](docs/DASHBOARD.md)
- [Development and operations](docs/DEVELOPMENT.md)
- [Design system](docs/DESIGN.md)
- [Code style](docs/STYLEGUIDE.md)

## Architecture

- `apps/desktop`: Tauri v2 application. Svelte 5 owns the view layer; Rust owns commands and runtime
  behavior below `src-tauri` and the shared crates.
- `apps/cli`: Rust `vesper` command-line binary.
- `crates/cms-core`: Markdown, content builds, static publication, and R2 access.
- `crates/consumers`: Memos, Moment, and Knowledge APIs, projections, and Moment media processing.
- `crates/social`: outbound Telegram Channel and X publication.
- `crates/todo`: local Todo storage and ICS schedule projection.
- `crates/credentials`: operating-system credential-store boundary.
- `crates/ugos`: read-only UGOS Pro boundary.
- `crates/useage`: read-only AI subscription and credit providers. The spelling is intentional.
- `crates/logger`: shared `tracing` initialization.
- `packages/ui`: reusable Svelte components and design tokens.
- `packages/tsconfig`: UI-only TypeScript configuration.

Except for the Svelte view layer and its build configuration, new application code should be Rust.
Create a package only when it owns a stable independent responsibility or is genuinely shared.

## Working expectations

- Prefer root commands from `package.json`.
- Use Vite Plus for frontend formatting, linting, tests, and orchestration.
- Use Cargo fmt, Clippy, check, and test for Rust.
- TypeScript and Svelte use tabs and double quotes. Rust follows `cargo fmt`.
- Workspace-owned dependencies use `workspace:*` in pnpm and workspace dependencies in Cargo.
- Preserve unrelated user changes. Commit messages follow Conventional Commits.
- Do not embed secrets in packaged code, logs, or source files. Provider responses must not expose
  credentials; the typed Settings read command may return stored values solely to prefill its form.
- Do not add broad `try/catch` blocks, one-line helper wrappers, or generic utility modules without a
  real boundary or repeated policy.

## UI tokens

The token layers are `palette.css`, `tokens.css`, and `theme.css`. Application and component code
consumes semantic `--color-*`, `--font-*`, `--radius-*`, `--shadow-*`, and `--duration-*` tokens only.
Never reference `--palette-*` outside `tokens.css`.

## Change checklist

1. Run formatting, Clippy, checks, tests, and the relevant build.
2. Update `docs/ARCHITECTURE.md` for boundary, storage, or data-flow changes.
3. Update `docs/DASHBOARD.md` for provider, credential, UGOS, or polling changes.
4. Update `docs/DESIGN.md` for token or reusable UI changes.
5. Update `docs/STYLEGUIDE.md` for engineering-rule changes.
6. Keep the root README concise.
