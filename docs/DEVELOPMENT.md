# Development and Operations

## Prerequisites

- Rust `1.95` or newer
- pnpm `11.22.0`
- the platform dependencies required by Tauri v2
- access to the configured Cloudflare R2 bucket for content-backed views
- Tailscale with MagicDNS for UGOS Dashboard telemetry

## Root commands

| Command                     | Purpose                                             |
| --------------------------- | --------------------------------------------------- |
| `pnpm dev`                  | Run the desktop application.                        |
| `pnpm dev:cli`              | Run the CLI in development.                         |
| `pnpm build:desktop`        | Build the desktop deliverable.                      |
| `pnpm build:cli`            | Build the CLI deliverable.                          |
| `pnpm content:build`        | Compile `content/` into a disposable build.         |
| `pnpm content:publish`      | Preview the R2 upload plan.                         |
| `pnpm content:publish:live` | Upload the planned artifacts.                       |
| `pnpm check`                | Run frontend checks and Cargo checks.               |
| `pnpm lint`                 | Run Vite Plus lint and Clippy with warnings denied. |
| `pnpm test`                 | Run frontend and Cargo tests.                       |
| `pnpm format:check`         | Verify frontend and Rust formatting.                |

Use root commands for workspace-wide verification. A focused change may use package-specific Cargo
or pnpm commands during iteration, but the owning package must pass before handoff.

Commit CI runs frontend and Rust verification as separate parallel jobs. The Rust job installs the
Tauri Linux build dependencies and explicitly runs rustfmt, Clippy with warnings denied, Cargo check,
and the complete workspace test suite. The `:frontend` and `:rust` root-script suffixes expose the
same individual checks for local diagnosis.

## Desktop releases

The `Release` GitHub Actions workflow is manual-only. It has only a `workflow_dispatch` trigger and
does not run for pushes, tags, pull requests, schedules, or GitHub Release events. Before starting it
from the Actions page, update the application version in `apps/desktop/src-tauri/tauri.conf.json` and
the desktop crate version in `apps/desktop/src-tauri/Cargo.toml`.

A successful run creates a draft `v<version>` GitHub Release and uploads Tauri bundles for macOS
Apple Silicon, macOS Intel, Linux, and Windows. After every matrix job succeeds, verify that all
expected assets are present and publish the draft manually. Releasing the same version again targets
the same tag, so advance the version before every new release.

## R2 configuration

Open Settings in Vesper and save the R2 Access Key ID and Secret Access Key. The token should be
restricted to the project bucket. Vesper stores the pair in the operating-system credential store
and passes it directly to the Rust S3 SDK. It does not read `rclone.conf` or persist secrets in the
repository.

R2 remains available for Moment image transfer, explicit Moment CLI operations, and publication
artifacts. Memo and Knowledge reads use their deployed APIs so the desktop cannot bypass D1 and KV
coordination.

## Consumer API configuration

Open Settings and save the separate my-memos, my-moment, and my-knowledge Bearer keys. Generate each
key in its application's settings. The services retain key digests rather than the original keys.
Vesper stores each value as a separate operating-system credential and does not expose stored values
to provider commands. The typed Settings read command does return them to the local Svelte webview
so the form can display and edit the current configuration; avoid retaining or forwarding them
outside that view.

## macOS development credentials

An unsigned or ad-hoc-signed `tauri dev` executable changes its macOS code identity whenever it is
rebuilt. Keychain may therefore request access again after an ordinary source edit. To avoid those
prompts, debug builds resolve UGOS, R2, and consumer API credentials only from process environment
variables:

```sh
export UGOS_USERNAME="..."
export UGOS_PASSWORD="..."
export R2_ACCESS_KEY_ID="..."
export R2_SECRET_ACCESS_KEY="..."
export MEMOS_API_KEY="..."
export MOMENT_API_KEY="..."
export KNOWLEDGE_API_KEY="..."
pnpm dev
```

Only define the values needed by the features under development. Missing values report the feature
as unconfigured; empty values and incomplete UGOS or R2 pairs fail explicitly. This environment path
is compiled only for debug builds. Release builds ignore it and use the operating-system credential
store. Settings writes still target Keychain and do not rewrite the shell environment.
The same names are shown as commented examples in the root `.env.example`; `.env` remains ignored by
Git. Debug desktop and CLI startup load the repository-root `.env` before credentials are resolved.
Values already exported by the parent process take precedence over entries in that file.

## Content workflow

Place Markdown and assets below `content/`. Use `pnpm content:build` to validate the build. Use
`pnpm content:publish` to inspect the planned keys before any remote mutation, then
`pnpm content:publish:live` for the explicit upload.

Publication is additive: destination-only objects are not deleted. Removing an obsolete remote
object requires a separate, explicit operation outside the current publisher.

## Credential boundaries

- R2, UGOS, and all three consumer API credentials belong to `crates/credentials` and the operating-system store.
- Debug builds may read App Lock from `APP_LOCK_PASSWORD` in the repository-root `.env` when the
  operating-system credential store has no App Lock value. Saving a password in Settings makes the
  credential-store value take precedence. Release builds use only the operating-system credential
  store. The typed Settings read response includes the resolved password solely to prefill the local
  form; verification remains in Rust.
- Codex reuses the authenticated local CLI session.
- Provider credentials reuse existing Codex, pi, and Cherry Studio sessions, as documented in
  [DASHBOARD.md](DASHBOARD.md). A successful CherryIN token refresh may conditionally update the
  matching Cherry Studio OAuth record.
- Packaged desktop and CLI code must not embed secrets.
- Only the typed Settings read command may return stored credentials to Svelte for form prefill.
- Logs may identify a provider or failed operation but must not include tokens, passwords, response
  bodies containing account data, or authorization headers.

`RUST_LOG` controls Rust logging. For example:

```sh
RUST_LOG=debug pnpm dev:cli
```

## Verification

Before finishing a non-trivial change, run the checks proportional to the affected boundaries:

```sh
pnpm format:check
pnpm lint
pnpm check
pnpm test
```

For desktop UI changes, also run the desktop production build. For provider changes, test response
parsing without credentials and keep live authenticated tests explicitly ignored. For publication
changes, inspect the dry-run plan before any live upload.

## Documentation synchronization

- Update [ARCHITECTURE.md](ARCHITECTURE.md) when boundaries, data flow, storage, or package ownership
  changes.
- Update [DASHBOARD.md](DASHBOARD.md) when a provider endpoint, credential source, polling rule,
  response unit or UGOS call changes.
- Update [DESIGN.md](DESIGN.md) when tokens, theme behavior, or reusable UI ownership changes.
- Update [STYLEGUIDE.md](STYLEGUIDE.md) when engineering conventions change.
- Keep the root README concise; it is an entry point, not the architecture specification.
