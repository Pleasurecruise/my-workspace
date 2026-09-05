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
restricted to the project bucket. Vesper stores the pair in the operating-system credential store
and passes it directly to the Rust S3 SDK. It does not read `rclone.conf` or persist secrets in the
repository.

R2 remains available for Moment image transfer, explicit Moment CLI operations, and publication
artifacts. Memo and Knowledge reads use their deployed APIs so the desktop cannot bypass D1 and KV
coordination.

## Consumer API configuration

Open Settings and save the separate my-memos, my-moment, and my-knowledge Bearer keys. Generate each
key in its application's settings. The services retain key digests rather than the original keys.
Vesper stores each value in the operating-system credential store and does not expose stored values
to provider commands. On macOS these values share the unified Keychain item described below. The typed Settings read command does return them to the local Svelte webview
so the form can display and edit the current configuration; avoid retaining or forwarding them
outside that view.

## ntfy notification configuration

Vesper is only an ntfy consumer. It subscribes to the fixed
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

- configuration: `read_publication`, `save_telegram`, and `connect_x`;
- Telegram session: `read_auth`, `begin_auth`, `submit_code`, `submit_password`, and `cancel_auth`;
- publication: `publish_telegram` and `publish_x`, each accepting only the Memo `id` and returning
  the provider, external post ID, and public URL when available. Rust rereads the authoritative Memo
  and rejects it unless the API still reports `public` before contacting either provider.

For Telegram, create an application at `my.telegram.org` and configure its numeric API ID,
32-character hexadecimal API hash, and the public username of a broadcast channel where the signed-in
account can post. Call the authorization commands in order: begin with the account phone number,
complete the verification code, then complete the 2FA password only when requested. The API ID, API
hash, and channel username form one typed record in the operating-system credential store. The resulting
MTProto session is stored separately as `telegram.session` below the application-data directory with
owner-only file permissions on Unix. Login codes and 2FA passwords are not persisted.

For X, create the Vesper project application in the X Developer Console, enable OAuth 2.0, and
register `http://127.0.0.1:8792/callback` exactly as a callback URL. Set its public Client ID as the
`VESPER_X_CLIENT_ID` environment variable while compiling the desktop application. Settings exposes
only Connect/Reconnect: Vesper opens the browser and completes Authorization Code with PKCE,
requesting `tweet.read`, `tweet.write`, `users.read`, and `offline.access`. It stores the returned
access and refresh grants in the operating-system credential store and rotates them automatically;
no Client ID, Client Secret, or manually copied token is accepted by the desktop UI.

## macOS Keychain access

Vesper reads one Keychain item, service `me.you-find.vesper` and account `credentials`, then serves
provider reads from a Rust cache. Separate items from previous installations are not read or
migrated: save configuration in Settings and reconnect music and X accounts to populate this item.

macOS controls authorization. An ad-hoc application update can request access again; Allow grants
one access, while Always Allow records permission for the app. Startup reads one item rather than
one per provider. Writes and refreshes after another process changes credentials can require further
authorization. See [Apple's Keychain guidance](https://support.apple.com/guide/keychain-access/if-youre-asked-for-access-to-your-keychain-kyca1243/mac).

## macOS development credentials

An unsigned or ad-hoc-signed `tauri dev` executable changes its macOS code identity whenever it is
rebuilt. Keychain may therefore request access again after an ordinary source edit. To avoid those
prompts, debug builds resolve UGOS, R2, consumer API, and ntfy notification credentials only from
process environment variables. Telegram checks its development variables first and otherwise reads
the saved credential record so the Settings authorization flow remains usable; X OAuth grants are
always read from the operating-system credential store.

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

Only define values needed by the features under development. Debug desktop and CLI startup load the
ignored repository-root `.env`; variables inherited from the parent process take precedence. Missing
values leave a feature unconfigured, while empty values and incomplete credential pairs fail
explicitly. This resolution path is compiled only for debug builds. Settings writes target Keychain
and never rewrite the process environment or `.env`; release builds ignore these variables and use
only the operating-system credential store.

Music providers are exceptions to environment-backed debug credentials. Spotify connects through the
browser PKCE flow and requires no `.env` entry. Debug builds store both refresh grants in a private
`development-spotify.json` file below the local application-data directory and never access
Keychain for Spotify. Release builds store the same typed record in the operating-system credential
store. QQ Music connects through its QR flow and uses `development-qq-music.json` in debug builds;
release builds store its renewable session in the operating-system credential store.

## Spotify Music configuration

Choose Connect in Settings to run two PKCE grants in sequence: the shared Web API identity reads
Liked Songs and Spotify's desktop identity authorizes librespot playback. No user-created Spotify
application, Client ID, or Client Secret is required. Closing or denying either browser grant fails
the connection immediately; a successful connection stores both refresh grants together, and later
token rotations are serialized before the credential record is replaced.

Local playback requires Spotify Premium and uses librespot's Rodio backend. Spotify Web requests,
token exchange, album artwork, and playback have bounded operations and honor the operating-system
HTTP(S) proxy. Browser-only proxy extensions are not visible to the desktop process.

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

- R2, UGOS, all three consumer API credentials, Telegram publication configuration, the X OAuth
  grants, the Spotify refresh grants, the QQ Music session, and the ntfy read token belong
  to `crates/credentials` and the operating-system store. The Telegram MTProto authorization key is
  the narrow exception: it lives in the private application-data session file required by the
  client. Upstream producer secrets remain outside Vesper.
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

## Dependency updates

Keep dependency manifests and lockfiles synchronized. Two upstream constraints currently require
locked versions: librespot's `vergen-gitcl` 1.x needs `vergen` 9.0.6, and `grammers-crypto` 0.10 needs
`glass_pumpkin` 2.0.0-rc0. Later versions change shared traits or types and fail to compile. Reassess
these constraints when upgrading their owning dependencies.

## Verification

Before finishing a non-trivial change, run the checks proportional to the affected boundaries:

```sh
pnpm format:check
pnpm lint
pnpm check
pnpm test
```

Frontend tests run through Vite Plus projects: shared UI tests use Node, and desktop component
regressions use Happy DOM. Component tests mock Tauri commands and exercise view interactions without
accessing live services. For desktop UI changes, also run the desktop production build. For provider changes, test response
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
