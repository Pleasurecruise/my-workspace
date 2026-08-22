# My Workspace

A local-first CMS distribution foundation for managing content on the desktop and publishing it to
projects such as `my-memos` and `my-knowledge`. Svelte is limited to the view layer; application
commands, logging, and reusable runtime code live in the Cargo workspace. The current implementation
intentionally stops at Hello World.

## Structure

```text
apps/desktop          Tauri v2 desktop app with a Svelte 5 interface
apps/cli              Rust CLI binary
crates/logger         Shared tracing initialization
crates/cms-core       Shared Rust core, currently Hello World only
packages/ui           Svelte components and three-layer design tokens
packages/tsconfig     TypeScript configuration for the UI toolchain
```

There is no web application, standalone API, database, authentication service, or AI package in the
initial foundation. Add a package only when it has a stable responsibility or is genuinely shared.

## Commands

```sh
pnpm install
pnpm dev
pnpm build:desktop
pnpm build:cli
pnpm check
pnpm test
```

Install the CLI with `pnpm cli:install`, then run `my-workspace`. Use `RUST_LOG` to override the
default `info` log filter, for example `RUST_LOG=debug pnpm dev:cli`.

## Environment boundary

The desktop and CLI do not contain application secrets. Wrangler and `voidPlugin()` are intentionally
absent because there is no Cloudflare Worker in this repository. If a Worker is added later, its vars,
local `.dev.vars`, secrets, and bindings must be owned by Wrangler, and `voidPlugin()` belongs only in
that Worker application's Vite configuration.
