# my-monorepo

My monorepo used to start everything :-)

## Tech stack

The toolchain used here is sourced from the [VoidZero](https://voidzero.dev/about) ecosystem.

- [mise](https://mise.jdx.dev/) - project-level runtime and toolchain version management
- [pnpm](https://pnpm.io/) - package manager (workspace-native, fast monorepo installs)
- [Bun](https://bun.sh/) - API runtime and bundler
- [vite-plus](https://github.com/voidzero-dev/vite-plus) - unified toolchain (`vp` CLI) bundling vite, vitest, rolldown, oxlint, and oxfmt
- [TypeScript](https://www.typescriptlang.org/docs/handbook/tsconfig-json.html) - web and API language
- [Rust](https://www.rust-lang.org/) - CLI and Tauri core language, with Cargo for builds and tests

## CLI

Run the Rust CLI from the workspace:

```sh
pnpm dev:cli
```

Install it into Cargo's binary directory and activate the `my-monorepo` command:

```sh
pnpm cli:install
my-monorepo
```

Remove the installed command with `pnpm cli:uninstall`.

## TODO List

- [x] Add ai-sdk package
- [x] Set up CI/CD pipeline
- [x] Add authentication package
- [x] Add database package
- etc.

## References

- [midday](https://github.com/midday-ai/midday)
