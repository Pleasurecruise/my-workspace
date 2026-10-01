# Development and Operations

## Prerequisites

Use Rust `1.95` or newer, Node.js `^22.18.0 || ^24.11.0 || >=26.0.0`,
pnpm `12.6.0` (pinned in `package.json`), and the platform
build dependencies required by Tauri v2. The desktop app requires macOS 12 or newer on Mac.
R2 access is needed for publication and Moment image transfer; UGOS requires Tailscale with MagicDNS.

## Root commands

| Command                     | Purpose                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `pnpm dev`                  | Run the desktop application.                                          |
| `pnpm dev:cli`              | Run the CLI in development.                                           |
| `pnpm build:desktop`        | Build the desktop deliverable.                                        |
| `pnpm build:cli`            | Build the CLI deliverable.                                            |
| `pnpm content:build`        | Compile local content into a disposable build.                        |
| `pnpm content:publish`      | Preview the R2 upload plan.                                           |
| `pnpm content:publish:live` | Upload the planned artifacts.                                         |
| `pnpm format:check`         | Check frontend and Rust formatting.                                   |
| `pnpm lint`                 | Run ESLint with Svelte/shadcn checks and Clippy with warnings denied. |
| `pnpm check`                | Run formatting, lint, and type checks for both toolchains.            |
| `pnpm test`                 | Run frontend and Cargo tests.                                         |

Root pnpm scripts and `vp run <task>` are equivalent workspace entries. Root `check` combines
`check:frontend` (formatting, whole-workspace ESLint, and each UI package's `typecheck`) and
`check:rust` (Cargo fmt, Clippy, and check). The pre-commit hook runs `check` followed by `test`.
Package `check` runs local formatting, lint, and types. `lint:fix:frontend` applies ESLint fixes;
`precommit:fix` follows those fixes with frontend and Rust formatting. Scripts use Vite Plus's uncached default;
Cargo retains its incremental build cache. Use `vp run lint` and `vp run check`, since built-in
`vp lint` and `vp check` still invoke Oxlint. The binaries are `vesper` (CLI) and `vesper-desktop`.

The macOS View menu offers Reload and Developer Tools in debug and packaged builds. Reload preserves
App Lock state. Tools require an unlocked session and close when locked; a restart starts unlocked.
Set `RUST_LOG=debug` when investigating Rust behavior, without logging secrets or account responses.

## CLI installation

The CLI binary is `vesper`, distributed separately from the desktop installer and updater.
`pnpm cli:install` builds and installs the release CLI into Cargo's binary directory (normally
`~/.cargo/bin`, which must be on PATH). Rerun it to update; `pnpm cli:uninstall` removes the
`vesper-cli` package.

Installed CLI and packaged desktop share the release credential store. `pnpm dev:cli` and
`content:*` run debug builds with the separate [debug credential boundary](#credential-resolution).
The CLI owns terminal input and output; shared Rust crates own business rules, storage and provider
access. Commands are documented by `vesper --help` and the [workflow guide](WORKFLOW.md).

## Desktop releases

Run the `Release` workflow manually after committing the same new version in
`apps/desktop/src-tauri/tauri.conf.json` and `apps/desktop/src-tauri/Cargo.toml`.
It creates a draft `v<version>` release for macOS Apple Silicon, macOS Intel, Linux, and Windows.
Publish only after all matrix jobs pass and the expected assets are present.

The [release script](../.github/scripts/tauri-release.sh) ad-hoc signs macOS apps and runs
`codesign --verify --deep --strict` before uploading. This provides neither Developer ID trust nor
notarization; downloaded apps may require **Privacy & Security → Open Anyway** on first launch.
See [Apple's guidance](https://support.apple.com/en-us/102445).

Actions secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` sign updater
archives against the public key in `tauri.conf.json`. Preserve this pair for existing installations.
Local builds need no private key. Releases include archives, `.sig` files, and `latest.json`.
Startup and Check for Updates read the published stable release's `latest.json` from this repository.
Update requests honor the operating-system HTTP(S) proxy.

The workspace disables stripping for release build dependencies so proc-macro libraries remain
loadable on macOS 27 with affected Rust toolchains ([Rust issue 157750](https://github.com/rust-lang/rust/issues/157750)).
Use the standard `pnpm build:desktop` command; no local environment override is needed.

## Credential resolution

Debug builds exclude the OS credential backend and use the shared database for saved credentials.
Release builds use the OS store and ignore debug credentials. Invalid debug data never falls through
to Keychain. Storage, locking, and backup rules belong to [Persistence](PERSISTENCE.md).

| Configuration                                                | Debug source                                                |
| ------------------------------------------------------------ | ----------------------------------------------------------- |
| UGOS, R2, consumer API keys, ntfy                            | Process environment / root `.env`                           |
| App Lock, Telegram configuration                             | Environment, then database                                  |
| X, music, miHoYo, Skland, Notion view link, certificate pins | Database                                                    |
| Steam                                                        | Complete `STEAM_API_KEY` and `STEAM_ID` pair, then database |
| CherryIN                                                     | Existing Cherry Studio OAuth session                        |

Copy needed entries from [`.env.example`](../.env.example) into the ignored root `.env`.
Desktop and CLI load it in debug builds; inherited variables take precedence. Feature
`credentials` modules read overrides through `vault::variables`, so empty values and incomplete
groups fail validation. Settings writes the database, never `.env` or the environment;
restart after changing an overriding environment value. Define only features under development.

On macOS, release credentials share Keychain service `me.you-find.vesper`, account `credentials`.
Each process caches one credential snapshot; unchanged saves do not write Keychain or invalidate
other processes. A new CLI process still reads Keychain once. “Always Allow” authorizes the current
program identity; ad-hoc updates change that identity and may require renewed authorization.
Keychain also checks the item's partition list: trusting the CLI in the application list does not
resolve a partition list that permits only the desktop's code hash. Repeated prompts for an unchanged
binary require checking both controls. Repair only this service/account with macOS `security`
using the installed applications' code hashes; never authorize all tools or pass a Keychain password
on the command line. Stable signed identities are needed for authorization to survive binary updates.
Browser and QR login do not require copied session tokens. Codex, pi, and Claude integrations retain
their existing CLI authentication. Claude reads its existing macOS Keychain item through
`/usr/bin/security` in debug and release builds; this is separate from Vesper's credential store.
Claude Code owns token renewal. If the usage card reports an expired session, renew it in Claude Code
or run `claude auth login`, then refresh the card.

Only typed Settings reads may return editable credentials to the trusted local form. Provider
responses, logs, packaged code, commits, and bug reports must exclude secrets. Telegram's MTProto
session is the private-database exception to release credential storage. Upstream producer secrets
remain outside Vesper; App Lock verification stays in Rust.

## Service setup

Settings accepts bucket-scoped R2 credentials and separate Bearer keys generated in my-memos,
my-moment, and my-knowledge. Memo and Knowledge use their APIs; direct R2 access cannot bypass
server metadata and cache coordination. [Workflow](WORKFLOW.md) covers publication and recovery.

Chat requires a system installation of `@earendil-works/pi-coding-agent` with RPC support. Install
and authenticate Pi outside this workspace; Vesper packages no Pi npm dependency. The current
account's login-shell PATH resolves Pi and Node, including mise installations. The connection icon
starts Pi in `~/` with its existing configuration. Enter sends, Shift+Enter inserts a newline, Stop
cancels the response and New Chat resets the conversation. Startup uses `--no-session` to avoid
conversation files and `--no-approve` to ignore project-local resources. Unsupported extension
dialogs are cancelled with a visible explanation.

The sidebar's This device entry automatically starts the account's default local shell in a native
PTY, without Tailscale or an idle timeout. Remote entries require a signed-in Tailscale CLI and system
OpenSSH. Enable Tailscale SSH on `tag:server` devices and authorize the remote account in the tailnet
policy. Vesper remembers a login username per device, initially using the local OS user; edit it while
offline. Host-key and authentication prompts appear in the terminal. Host-key checking stays enabled;
SSH configuration files, agent forwarding and port forwarding are disabled. Remote sessions expire
after five minutes without user interaction; use tmux for unattended commands.

Chat and Terminal show Online while connected and Offline after disconnecting; their status icon
connects or disconnects. Chat initially opens offline and discards messages, drafts and its process
when leaving the page. Terminal connects automatically when selected and survives navigation;
selecting its sidebar device again replaces it with a fresh connection. App Lock, main-window
reload/close and shutdown end both process runtimes.

Dynamic Island starts hidden on each launch. Dashboard Edit displays it only after a widget is pinned
and Show Dynamic Island is selected. Hide retains the saved layout and pin.

For ntfy, save a token with read access to `mail-summary`. Vesper consumes the fixed
`https://ntfy.you-find.me/mail-summary/sse` endpoint only while Inbox is active. It does not configure
producers; retention and availability remain the self-hosted server's responsibility.

For CherryIN, sign in through Cherry Studio. Vesper reuses and refreshes that session; reconnect
there if its refresh grant is rejected. UGOS setup belongs to [UGOS](UGOS.md#connection-and-login),
and game authorization to [Games](GAMES.md#authorization-and-account-selection).

## Memo social publication configuration

Public Memo cards expose Telegram and X publication. Rust rereads the authoritative Memo and
requires public visibility before sending; private cards expose neither action.

For Telegram, create an app at `my.telegram.org` and save its numeric API ID, 32-character hexadecimal
API hash, and public broadcast-channel username in Settings. Sign in with your phone number,
verification code, and 2FA password when requested. The account must be able to post to the channel.
Login codes and passwords are not persisted.

For X, enable OAuth 2.0 in the Developer Console and register `http://127.0.0.1:8792/callback` exactly.
Compile with the public Client ID in `VESPER_X_CLIENT_ID`, then use Settings → Connect/Reconnect.
The browser PKCE flow requests `tweet.read`, `tweet.write`, `users.read`, and `offline.access`;
Rust stores and rotates the grants. The UI accepts no Client Secret or manually copied token.

## Notion calendar

Install the official `ntn` CLI and run `ntn login`. Save the view link, including `?v=...`, in Settings
or run `vesper todo notion connect <view-url>`. Table views need exactly one Date property.
Clear the link to disconnect. Desktop finds `ntn` on PATH or in `~/.local/bin`, `/opt/homebrew/bin`,
and `/usr/local/bin`. Vesper stores the view link; `ntn` owns login, and task completion stays local.

Codex Resets is a separate public calendar source: enable it in Settings without credentials.
Its source preference belongs to Todo. Neither calendar source configures AI credit reads.

## Spotify Music configuration

Settings → Music connects Web API library access and librespot playback through separate browser
PKCE grants. An empty Personal Spotify Client ID selects shared Web API access. Playback requires
a successful Premium eligibility check; library access can remain available if playback fails.

To use your own Web API quota:

1. Create an app in Spotify's developer dashboard.
2. Register `http://127.0.0.1:8989/login` as its redirect URI.
3. Save its Client ID in Settings → Music and choose Connect/Reconnect.
4. Complete both browser grants; no Client Secret is required.

Clear the Client ID and reconnect to restore shared access. Denying either grant fails the connection.
Requests and playback honor the operating-system HTTP(S) proxy, including token and artwork reads;
browser-only proxy extensions do not apply. See [Music](MUSIC.md) for limits and session lifecycle.

## QQ Music configuration

Choose Connect in Settings, scan the QR code in QQ, and confirm. Closing the dialog cancels login.
Rust retains and renews the private session; the WebView receives only the QR image and login state.
A revoked refresh grant requires reconnecting. Region, copyright, purchase, and membership rules
can make tracks unavailable. These personal web endpoints are not a public OpenAPI; their protocol
and playback behavior are documented in [Music](MUSIC.md#qq-music-lifecycle).

## Dependency updates

Keep manifests and lockfiles synchronized. librespot's `vergen-gitcl` 1.x requires `vergen` 9.0.6,
and `grammers-crypto` 0.10 requires `glass_pumpkin` 2.0.0-rc0 because later shared types fail to compile.
Keep `@vitest/coverage-v8` aligned with Vite Plus's bundled Vitest (`5.0.1` for Vite Plus
`1.0.0`). Reassess these constraints when upgrading the owning dependencies.

## Verification

Reproduce the reported behavior, define an observable success criterion, and repeat that scenario
after the change. Use isolated data or mocks for experiments. If evidence is missing, identify the
untested assumption instead of claiming a fix; compare baselines when a performance claim needs it.

Run formatting, lint, checks, tests, and the relevant build. Frontend tests use Happy DOM with mocked
Tauri commands; controlled promises and clocks exercise failure and response ordering. UI changes
also require a desktop production build. Provider parsing should be testable without credentials;
live authenticated tests stay explicitly ignored. Inspect publication plans before live uploads.

`cargo test -p vesper terminal::` covers discovery and native PTY lifecycle with a synthetic local shell.
Real Tailscale login still needs an interactive check against a permitted device.

Coverage reports are separate and ignored by Git:

- `pnpm test:coverage:frontend`: HTML and `coverage-summary.json` under `coverage/frontend/`.
- `pnpm test:coverage:rust`: `coverage/rust/html/index.html`; default-feature, all-target workspace tests.

Rust coverage requires `cargo install cargo-llvm-cov --locked` and
`rustup component add llvm-tools-preview`. Its first run builds separate instrumented artifacts.
Stable Rust coverage excludes branch coverage, doctests, and ignored live tests. Neither command
sets a coverage threshold; passing coverage tests does not establish live-provider or native UI correctness.

On macOS, `cargo run -p vesper --example cookie_probe` checks native WebView domain-cookie behavior;
`cargo run -p vesper --example captcha_probe` checks the production page-to-native proof callback.
Both require a desktop session and use synthetic inputs without credentials or game API requests.

Apply the independent [review process](STYLEGUIDE.md#review) to non-trivial behavior and boundaries.
Handoff records commands actually run, before/after evidence, unresolved cases, and untested environments.

The Knowledge consumer fixture is captured from the generated local Worker using
`KNOWLEDGE_CONTRACT_OUTPUT` during its REST/MCP contract journey. Store only synthetic responses in
`crates/consumers/tests/fixtures/knowledge-contract.json`; `cargo test -p consumers
decodes_api_contracts` checks deserialization and desktop projection without credentials.

## Documentation synchronization

Keep each fact in its owning document: [Architecture](ARCHITECTURE.md) for boundaries,
[Dashboard](DASHBOARD.md) for integrations, [Persistence](PERSISTENCE.md) for durable records,
[Design](DESIGN.md) for UI conventions, and [Styleguide](STYLEGUIDE.md) for engineering rules.
Rewrite affected sections coherently instead of appending implementation history. Keep the root README
an entry point and link to the relevant owner for detail.
