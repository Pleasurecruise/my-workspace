# Development and Operations

## Prerequisites

- Rust `1.95` or newer
- pnpm `12.3.4` (pinned in `package.json`)
- the platform dependencies required by Tauri v2
- macOS 12 or newer for the desktop app; native island geometry uses AppKit's safe-area APIs
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

The CLI binary is `vesper`; the desktop binary is `vesper-desktop`. Keep their Cargo target names
distinct so workspace builds and CLI integration tests cannot overwrite or launch the wrong app.

Use root commands for workspace-wide verification. A focused change may use package-specific Cargo
or pnpm commands during iteration, but the owning package must pass before handoff.

The macOS View menu exposes Reload and Toggle Developer Tools in debug and packaged builds through
Tauri's `devtools` feature. Reload recreates the WebView and restores the running process's App Lock
state before showing content. Developer tools are available only while unlocked and close when the
application is locked. Restarting the application begins a new session without a startup password.

Commit CI runs frontend and Rust verification as separate parallel jobs. The Rust job installs the
Tauri Linux build dependencies and explicitly runs rustfmt, Clippy with warnings denied, Cargo check,
and the complete workspace test suite. The `:frontend` and `:rust` root-script suffixes expose the
same individual checks for local diagnosis.

## Desktop releases

Run the `Release` workflow manually from GitHub Actions. Before each release, commit the same new
version in `apps/desktop/src-tauri/tauri.conf.json` and `apps/desktop/src-tauri/Cargo.toml`. The
workflow derives its `v<version>` tag and release name from that version and creates a draft with
macOS Apple Silicon, macOS Intel, Linux, and Windows bundles. Publish the draft manually after all
matrix jobs pass and the expected assets are present.

macOS uses Tauri's ad-hoc signing identity (`-`) to sign the complete application before creating
DMG and updater archives. Each macOS job uses the [release script](../.github/scripts/tauri-release.sh)
to build and run `codesign --verify --deep --strict` before uploading its assets. A build or signature
failure stops that job. Ad-hoc signing needs no Apple certificate but provides neither Developer ID
trust nor notarization; downloaded apps may still need first-launch approval in
**Privacy & Security > Open Anyway**. See
[Apple's instructions](https://support.apple.com/en-us/102445) and
[Tauri's signing guide](https://v2.tauri.app/distribute/sign/macos/#ad-hoc-signing).

Updater signatures use the Actions secrets `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, paired with the public key in `tauri.conf.json`. The release
configuration enables updater archives, `.sig` files, and `latest.json`; local builds do not require
the private key. Preserve the signing key pair while existing installations depend on it.

Vesper reads `https://github.com/Pleasurecruise/my-workspace/releases/latest/download/latest.json`
at startup or through the native Check for Updates menu. Only published stable releases are offered.
Requests honor the operating-system HTTP/HTTPS proxy; proxy applications must expose their settings
to the operating system.

## R2 configuration

Open Settings in Vesper and save the R2 Access Key ID and Secret Access Key. The token should be
restricted to the project bucket. Vesper resolves the pair through the credential policy below
and passes it directly to the Rust S3 SDK. It does not read `rclone.conf` or persist secrets in the
repository.

R2 remains available for Moment image transfer, explicit Moment CLI operations, and publication
artifacts. Memo and Knowledge reads use their deployed APIs so the desktop cannot bypass D1 and KV
coordination.

## Consumer API configuration

Open Settings and save the separate my-memos, my-moment, and my-knowledge Bearer keys. Generate each
key in its application's settings. The services retain key digests rather than the original keys.
Vesper resolves each value through the credential policy below and does not expose stored values
to provider commands. The typed Settings read command does return them to the local Svelte webview
so the form can display and edit the current configuration; avoid retaining or forwarding them
outside that view.

## ntfy notification configuration

Vesper is only an ntfy consumer. With a configured token, it subscribes while Inbox is active and stops on leaving that route. It uses the fixed
`https://ntfy.you-find.me/mail-summary/sse` endpoint and does not connect to or configure upstream
producers. Settings only configures the ntfy token; the server and topic are application policy and
are not displayed as editable fields. The token must have read permission for `mail-summary`.
Notification contents stay within the self-hosted ntfy deployment and are subject to its caching and
availability policy.

## Memo social publication configuration

Memo publication supports Telegram Channels through a Telegram user account and X through OAuth 2.0
user authorization. Settings owns provider setup. A public Memo card shows both publication actions
beside its visibility label; private Memo cards do not show them.

The registered Tauri surface is:

- configuration: `read_configuration`, `save_telegram`, and `connect_x`;
- Telegram session: `read_auth`, `begin_auth`, `submit_code`, `submit_password`, and `cancel_auth`;
- publication: `publish_telegram` and `publish_x`, each accepting only the Memo `id` and returning
  the provider, external post ID, and public URL when available. Rust rereads the authoritative Memo
  and rejects it unless the API still reports `public` before contacting either provider.

For Telegram, create an application at `my.telegram.org` and configure its numeric API ID,
32-character hexadecimal API hash, and the public username of a broadcast channel where the signed-in
account can post. Call the authorization commands in order: begin with the account phone number,
complete the verification code, then complete the 2FA password only when requested. The API ID, API
hash, and channel username form one typed credential record. The resulting
MTProto session is stored in the shared local database with owner-only file permissions on Unix. Login codes and 2FA passwords are not persisted.

For X, create the Vesper project application in the X Developer Console, enable OAuth 2.0, and
register `http://127.0.0.1:8792/callback` exactly as a callback URL. Set its public Client ID as the
`VESPER_X_CLIENT_ID` environment variable while compiling the desktop application. Settings exposes
only Connect/Reconnect: Vesper opens the browser and completes Authorization Code with PKCE,
requesting `tweet.read`, `tweet.write`, `users.read`, and `offline.access`. It stores the returned
access and refresh grants through the build-specific credential store and rotates them automatically;
no Client ID, Client Secret, or manually copied token is accepted by the desktop UI.

## CherryIN balance

Sign in using Cherry Studio. Vesper reuses and refreshes that OAuth session, including one retry
after a 401 response. Sign in again there if its refresh token is missing or rejected. No Vesper
callback URL or Settings OAuth configuration is required.

## Notion calendar

Install the official `ntn` CLI and run `ntn login`, then save the calendar
view link (including `?v=...`; table views need exactly one Date property) in Settings or `vesper todo notion connect <view-url>`. Todo invokes
`ntn api`; marking a task complete remains local. Clear the link to disconnect. Desktop looks for
`ntn` on PATH, in `~/.local/bin`, `/opt/homebrew/bin`, and `/usr/local/bin`. Notion login credentials
belong to the CLI; Vesper retains only the view link.

## Credential resolution

Debug builds do not compile the system credential-store backend. Saved credentials and renewable
sessions use the `credentials` table in the shared local database. Invalid development data does
not fall back to Keychain. Release builds use the OS store and ignore the debug credential table.

| Credential                                                                    | Debug source                                           |
| ----------------------------------------------------------------------------- | ------------------------------------------------------ |
| UGOS login, R2, consumer APIs, ntfy                                           | Process environment / root `.env`                      |
| App Lock, Telegram configuration                                              | Environment first, then database                       |
| X, music, miHoYo, Skland, Notion view link, certificate pins, Settings writes | Database                                               |
| Steam                                                                         | `STEAM_API_KEY` and `STEAM_ID` together, then database |
| CherryIN                                                                      | Existing Cherry Studio OAuth session                   |

Steam environment values must be complete and valid. Settings writes the database and does not
rewrite `.env`; change environment values and restart when they override a saved account. Secrets
must not be committed or attached to bug reports. [Persistence](PERSISTENCE.md) owns the storage and
transaction details.

For environment-backed configuration:

```sh
export UGOS_USERNAME="..."
export UGOS_PASSWORD="..."
export R2_ACCESS_KEY_ID="..."
export R2_SECRET_ACCESS_KEY="..."
export MEMOS_API_KEY="..."
export MOMENT_API_KEY="..."
export KNOWLEDGE_API_KEY="..."
export NTFY_TOKEN="..."
export TELEGRAM_API_ID="..."
export TELEGRAM_API_HASH="..."
export TELEGRAM_CHANNEL_USERNAME="channel_username"
pnpm dev
```

Only define values needed by the features under development. Debug desktop and CLI startup load
the ignored repository-root `.env`; inherited variables take precedence. Missing values leave
environment-only features unconfigured; empty values and incomplete pairs are errors. Settings
writes never change the process environment or `.env`. `APP_LOCK_PASSWORD` may also supply a debug
App Lock password. Browser and QR authorization need no manually copied session tokens.

Claude usage reads the existing Claude Code credential file in debug builds and does not invoke
macOS `security`. Third-party CLI integrations retain their own authentication policies.

On macOS, release credentials share one Keychain item: service `me.you-find.vesper`, account
`credentials`. Rust caches the collection and coordinates changes across desktop and CLI. An
ad-hoc application update can require renewed Keychain authorization. Older per-provider items are
not migrated automatically; save configuration or reconnect in the release application. See
[Apple's Keychain guidance](https://support.apple.com/guide/keychain-access/if-youre-asked-for-access-to-your-keychain-kyca1243/mac).

## Spotify Music configuration

Settings → Music connects the library and local playback through two browser PKCE grants. The
library uses shared Web API access when Personal Spotify Client ID is empty. Spotify's desktop
identity authorizes librespot playback independently. Before starting librespot, Vesper verifies
Premium eligibility with that playback grant; a failed or inconclusive check returns an error while
leaving library access available.

To use a personal Web API application:

1. Create a Web API app in the Spotify developer dashboard.
2. Register `http://127.0.0.1:8989/login` as its redirect URI.
3. Enter the app's Client ID in Settings → Music and choose Connect or Reconnect.
4. Complete both browser authorizations.

A Client Secret is not required. The personal app uses its own quota, which can reduce contention
on the shared app but remains subject to Spotify limits. Clearing the Client ID and reconnecting
restores shared access.

Closing or denying either browser grant fails the connection. A successful connection stores both
refresh grants and the selected Web API Client ID together. Subsequent token rotations preserve
that identity and serialize credential replacement. Existing records without a Client ID continue
to use shared access.

Spotify Web requests, token exchange, album artwork, and playback have bounded operations and honor
the operating-system HTTP(S) proxy. Browser-only proxy extensions are not visible to the desktop
process. For 429 cooldowns, retry behavior, and library state, see [MUSIC.md](MUSIC.md#library-reads-and-rate-limits).

## QQ Music configuration

Choose Connect in Settings. Rust requests a QQ login QR code, retains its `qrsig` only in the active
in-memory login session, and sends the image to the centered Settings dialog. After the user scans
and confirms in the QQ mobile app, Rust follows the trusted QQ redirect, exchanges its authorization
code for QQ Music credentials, and stores the complete renewable session. The WebView receives only
the QR image and waiting, scanned, complete, or expired states; it never receives the resulting
Cookie or refresh token. Closing the dialog cancels the in-memory login session.

Rust renews the private session on demand after twenty hours and persists all rotated fields
together. Failed renewals back off for one hour before another attempt. A server-revoked refresh
credential requires reconnecting through Settings.

Rust reads the authenticated recommendation feed, locates its `每日30首` card, and resolves that
card's dynamic playlist ID. Playback accepts only HTTPS media URLs below QQ Music's domain, chooses
the best available FLAC, MP3, or M4A response, enforces a 100 MiB download limit, and decodes on the
default system output. Session expiry, region, copyright, purchase, and membership rules can still
make an individual track unavailable. These personal web endpoints are not a public QQ Music OpenAPI
and may require maintenance when its web protocol changes.

## Content workflow

Place Markdown and assets below `content/`. Use `pnpm content:build` to validate the build. Use
`pnpm content:publish` to inspect the planned keys before any remote mutation, then
`pnpm content:publish:live` for the explicit upload.

Publication is additive: destination-only objects are not deleted. Removing an obsolete remote
object requires a separate, explicit operation outside the current publisher.

## Credential boundaries

- Vesper-owned credentials belong to `crates/credentials`, using the build-specific resolution
  policy above. The Telegram MTProto authorization key is
  the narrow exception: it lives in the private local database. Upstream producer secrets remain outside Vesper.
- App Lock verification remains in Rust; its resolved password is returned only to the trusted
  Settings form for editing.
- Codex reuses the authenticated local CLI session.
- Provider credentials reuse existing Codex and pi sessions, as documented in
  [DASHBOARD.md](DASHBOARD.md). CherryIN refreshes Cherry Studio's session with conditional token writes.
- Packaged desktop and CLI code must not embed secrets.
- Only the typed Settings read command may return stored credentials to Svelte for form prefill.
- Logs may identify a provider or failed operation but must not include tokens, passwords, response
  bodies containing account data, or authorization headers.

`RUST_LOG` controls Rust logging. For example:

```sh
RUST_LOG=debug pnpm dev:cli
```

## Dependency updates

Keep dependency manifests and lockfiles synchronized. Two upstream constraints currently require
locked versions: librespot's `vergen-gitcl` 1.x needs `vergen` 9.0.6, and `grammers-crypto` 0.10 needs
`glass_pumpkin` 2.0.0-rc0. Later versions change shared traits or types and fail to compile. Reassess
these constraints when upgrading their owning dependencies.

## Verification

Begin with the behavior under investigation. Record the input or action, relevant environment,
expected result, and observed result. Reproduce a bug before changing it when possible; otherwise
label the diagnosis provisional and identify the missing trace, response, or scenario. Choose a
success criterion that distinguishes the proposed cause from a plausible alternative.

Implement the smallest change that meets that criterion, then repeat the same scenario. When the claimed
benefit depends on an added mechanism and its contribution is unclear, compare a baseline with one
element removed or disabled at a time under equivalent conditions. A reproduced failure and a
passing regression may already establish a focused correctness fix without a separate ablation. Record the metric and variation across runs when applicable;
request count, error recovery, and preserved state can matter more than elapsed time. Do these
experiments with mocks or isolated local configurations, without weakening live credential or data
protection. Unmeasured benefit remains a hypothesis.

Run checks proportional to the affected boundaries before finishing a non-trivial change:

```sh
pnpm format:check
pnpm lint
pnpm check
pnpm test
```

`pnpm test:coverage:frontend` runs the same frontend suite with V8 instrumentation and writes
`coverage/index.html` plus `coverage/coverage-summary.json`. The report includes all Desktop
TypeScript/Svelte source files and excludes test code. Keep `@vitest/coverage-v8` aligned with the
Vitest version bundled by Vite Plus. Use per-feature branch gaps to guide tests; an overall frontend
percentage is not Rust coverage and does not establish live provider or native audio compatibility.

Frontend component tests run through the desktop Vite Plus project in Happy DOM with mocked Tauri
commands. Use controlled promises and clocks to check
partial failure and response ordering. For desktop UI changes, also run the desktop production
build. For provider changes, verify parsing without credentials and keep live authenticated tests
explicitly ignored. For publication changes, inspect the dry-run plan before any live upload.
Passing these checks does not substitute for the scenario that motivated the change.

On macOS, `cargo run -p vesper --example cookie_probe` reproduces the WebView parent-domain cookie
regression with a hidden isolated blank window and one synthetic cookie. It reads no credentials
and makes no game API requests. It reports that the cookie exists in the native store while Wry’s
exact-domain URL getter returns no cookies. Run it in a desktop session with native WebView access.
`cargo run -p vesper --example captcha_probe` additionally loads the production captcha HTML in a
native WebView with a synthetic local SDK. It checks the real page-to-native proof callback without
contacting miHoYo or Geetest, reading credentials, or solving a real challenge.

Apply the independent review process in [STYLEGUIDE.md](STYLEGUIDE.md#review) to non-trivial behavior
and boundary changes. The handoff records the before/after result, commands actually run, unresolved
counterexamples, and an explicit uncertainty list covering missing evidence, untested environments,
and assumptions. If no implementation changed, report a diagnosis or experiment rather than a fix.

## Documentation synchronization

- Update [ARCHITECTURE.md](ARCHITECTURE.md) when boundaries, data flow, storage, or package ownership
  changes.
- Update [DASHBOARD.md](DASHBOARD.md) when a provider endpoint, credential source, polling rule,
  response unit or UGOS call changes.
- Update [DESIGN.md](DESIGN.md) when tokens, theme behavior, or reusable UI ownership changes.
- Update [STYLEGUIDE.md](STYLEGUIDE.md) when engineering conventions change.
- Keep the root README concise; it is an entry point, not the architecture specification.
