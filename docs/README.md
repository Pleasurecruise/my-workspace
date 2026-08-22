# Architecture

The repository is the starting point for a local-first CMS distribution platform. It currently has
two deliverables and two shared Rust crates:

- `apps/desktop` packages the Tauri application. Svelte owns rendering and calls Rust through Tauri
  commands; business logic must not move into the view layer.
- `apps/cli` builds the `my-workspace` executable.
- `crates/cms-core` is the shared business boundary. It contains Hello World only until concrete CMS
  behavior is specified.
- `crates/logger` configures `tracing` for both Rust binaries.

Frontend-only packages are limited to `packages/ui` and `packages/tsconfig`. The UI design system has
three layers:

```text
palette.css  -> physical color values
tokens.css   -> light/dark semantic roles
theme.css    -> Tailwind utility mapping
```

Application and component code consumes semantic tokens only. Palette variables never leave
`tokens.css`, and raw colors do not belong in components.

Future distribution adapters may target `my-memos` and `my-knowledge`. They are not implemented in
the Hello World foundation. `voidPlugin()` is reserved for a future Cloudflare Worker boundary; it is
not a general environment loader for packaged desktop or CLI binaries.
