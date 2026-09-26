# Repository Guidelines

## Read first

- [Architecture](docs/ARCHITECTURE.md)
- [Dashboard integrations](docs/DASHBOARD.md)
- [Development and operations](docs/DEVELOPMENT.md)
- [Design system](docs/DESIGN.md)
- [Code style](docs/STYLEGUIDE.md)

## Architecture

`apps/desktop` is the Tauri v2 application, `apps/cli` provides the Rust `vesper` binary,
`crates/` owns feature and provider behavior, and `packages/` contains shared UI and TypeScript
configuration. [Architecture](docs/ARCHITECTURE.md#repository-layout) owns the full directory map
and each package's responsibility.

Except for the Svelte view layer and its build configuration, new application code should be Rust.
Create a package only when it owns a stable independent responsibility or is genuinely shared.

## Working expectations

- Prefer root commands from `package.json`.
- Use Vite Plus for frontend formatting, tests, and orchestration. ESLint owns frontend linting;
  use `vp run lint` and `vp run check`, not the built-in Oxlint `vp lint` or `vp check`.
- Use Cargo fmt, Clippy, check, and test for Rust.
- JavaScript, TypeScript, and Svelte use tabs and double quotes. Rust follows `cargo fmt`.
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
