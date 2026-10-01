# Architecture

Vesper is a local-first desktop and CLI workspace. A trusted device owns authoring, credentials,
compilation and application execution. Remote consumer APIs own published content; Cloudflare R2
stores artifacts. This repository does not host a cloud application backend.

## Repository layout

| Path                    | Responsibility                                                        |
| ----------------------- | --------------------------------------------------------------------- |
| `apps/desktop`          | Tauri v2 shell, Svelte 5 views and Rust command adapters              |
| `apps/cli`              | The `vesper` command-line interface                                   |
| `crates/cms`            | Content builds, publication and R2 object storage                     |
| `crates/markdown`       | Markdown compilation for publications, Knowledge, Memos and chat      |
| `crates/md-dialect`     | Custom fence and shortcode validation and rendering, without I/O      |
| `crates/consumers`      | Memo, Moment and Knowledge APIs and projections                       |
| `crates/database`       | Application identifier, shared Diesel/SQLite path, connection, schema |
| `crates/vault`          | Generic credential storage and development overrides                  |
| `crates/dashboard`      | Dashboard widget layout records, validation and source selection      |
| `crates/inbox`          | ntfy mail-summary subscription and notification records               |
| `crates/social`         | Telegram Channel and X publication                                    |
| `crates/todo`           | Tasks, habits, ICS, Notion and Codex Resets calendar sources          |
| `crates/ledger`         | Local GBP expenses and monthly statistics                             |
| `crates/music`          | Spotify and QQ Music authentication, library and playback             |
| `crates/oauth`          | Shared OAuth PKCE, loopback callbacks and token transport             |
| `crates/games`          | Game accounts, daily notes, Steam and pull archives                   |
| `crates/github`         | GitHub CLI dashboard and repository reads                             |
| `crates/link-preview`   | SSRF-safe link metadata and fixed-endpoint X previews                 |
| `crates/market-data`    | ECB exchange and Yahoo stock reads                                    |
| `crates/quotes`         | Random quotation reads                                                |
| `crates/service-status` | Statuspage service catalog and health reads                           |
| `crates/weather`        | Open-Meteo weather, astronomy, and geocoding reads                    |
| `crates/ugos`           | Read-only UGOS Pro authentication and telemetry                       |
| `crates/useage`         | AI subscriptions and account credits; the spelling is intentional     |
| `packages/ui`           | Self-owned Svelte primitives and semantic design tokens               |
| `packages/tsconfig`     | Shared UI TypeScript configuration                                    |

Create a package only for a stable independent or genuinely shared responsibility. Application
behavior belongs in Rust; Svelte owns presentation and interaction state.
Shared TypeScript settings and path aliases belong to `packages/tsconfig`: `@/` resolves each
consumer's `src`, and `@workspace/` resolves repository files such as cross-language test fixtures.
Workspace packages are imported through their declared exports. Vite reads the inherited tsconfig
paths; consumer configs only select their files.

## Data flow

```text
Desktop views ── typed Tauri commands ─┐
CLI commands ────────────────────────┤
                                    └─ Rust feature crates
                                       ├─ shared SQLite / OS credentials
                                       ├─ consumer REST APIs / R2
                                       ├─ calendar sources / provider APIs
                                       └─ native playback and device services
```

Each Rust feature owns its wire types, validation, external I/O and transactions. Tauri commands
translate inputs and return tagged `ready` or `failed` results. CLI commands reuse these feature
boundaries rather than desktop commands. Provider output excludes credentials; Settings prefill is
the narrow exception for values the user edits locally.

## Desktop boundary

Desktop Rust modules are named after the crate they adapt; `content` adapts `consumers`,
`settings` serves the Settings form, `app_lock` owns App Lock and `protocol` serves the asset and
music-cover URI schemes.

`App.svelte` composes navigation, page layout, profile, theme and App Lock. Feature directories own
their views and `session.svelte.ts` state; `pages` composes complete routes and `layout` holds shell
controls. Reusable primitives belong to `packages/ui`. [Design](DESIGN.md) defines presentation.

Content sessions survive page mounts within a WebView. Chat instead discards its conversation and
draft on navigation. Rust supplies an initial content snapshot; a feature
accepts it only if a later read or write has not superseded it. Request generations reject stale
responses, successful writes invalidate older reads, and failed refreshes retain settled data with
an error. Content drafts survive navigation and preserve edits made during saves. Credential changes reset
only the affected feature.

Dashboard and Dynamic Island share `WidgetContent` and feature panels. The Rust runtime owns source
polling and per-source request locks; the WebView holds typed projections. The island starts hidden
on every application launch and opens only after an explicit visibility action with a pinned widget.
Opening it reads only that widget's sources, concurrently for composite widgets. Layout records
preserve invalid widget configurations for repair while rejecting dangling references.
[Dashboard](DASHBOARD.md) owns scheduling and source contracts.

`apps/desktop/src-tauri/src/chat` owns one system Pi RPC subprocess in the current account's home
directory. Tokio and `tokio-util::codec::LinesCodec` own process and JSONL pipe I/O; Serde decodes
records. The runtime correlates responses by ID, projects text, thinking and tool activity, and uses
the `markdown` crate for completed assistant messages. Typed Tauri commands and revisioned events
connect Rust to the main WebView; Svelte owns drafts and presentation. Pi owns authentication and
configuration. Vesper neither embeds its npm SDK nor exposes its credentials to the WebView.
Unsupported extension dialogs are cancelled through Pi's UI subprotocol.

`apps/desktop/src-tauri/src/terminal` owns Tailscale discovery, local shells and system OpenSSH
sessions through `portable-pty`; xterm.js renders typed byte channels. Selecting a sidebar device
automatically connects it, replacing any earlier session for that device. Navigation preserves
hidden terminals. Rust bounds resources, expires idle SSH sessions and rejects superseded launches;
local shells remain open while idle and survive tailnet identity changes.

Both process runtimes cancel pending launches and close on App Lock, main-window reload/destruction
and shutdown. Chat also closes when leaving its page and runs with `--no-session`, so no conversation
is persisted. Terminal state survives navigation until its device is selected again or disconnected.
[Development](DEVELOPMENT.md#service-setup) owns installation, authentication and user-facing controls.

`crates/oauth` adapts `oauth2` to the workspace reqwest transport and uses `httparse` for loopback
request parsing. Spotify and X share that boundary; their feature crates retain endpoints, scopes,
client selection, credential storage and refresh-token rotation policy. Token requests use system
proxies, bounded timeouts and no redirects; callback and token failures omit response bodies.

QQ Music keeps session renewal and Cookie authentication in `qq/auth.rs`, shared with QR login;
the provider root owns library and playback coordination, and `qq/audio.rs` owns audio execution.
Music and game runtimes outlive route mounts. Their authentication, cancellation, cache and playback
rules belong in [Music](MUSIC.md) and [Games](GAMES.md); NAS protocols belong in [UGOS](UGOS.md).
Inbox independently activates its ntfy stream while its route is active.
Device storage reads OS capacity only; category inspection is delegated to system storage settings.
Knowledge and Newspaper load a summary-only index without fetching or compiling bodies.
`content::knowledge::Reader` owns lazy detail compilation, a 30-second/16-document cache, and same-ID
in-flight request sharing. A changed index content hash bypasses cached content. Visible index entries, pointer intent,
keyboard focus and touch request prefetch through Tauri, with at most two speculative reads and six
reads overall. Writes and credential resets clear cached documents and invalidate pending results.
Svelte owns loading/error presentation and discards detail responses after switching or leaving.
The Rust compiler supplies prose word counts and estimated reading minutes with each document detail.

Compiled external links open in the system browser; same-article fragments remain in the reader.
Knowledge uses article IDs throughout its API and desktop contracts.
Internal article cards resolve UUID URLs and metadata through the authorized paginated summary
index. Rendering never reads target bodies or external previews; unresolved
references become disabled cards. Clicking reads the ID-based detail endpoint and opens Knowledge
while preserving unsaved drafts. Chapter navigation is consumed by the destination after it mounts,
so replacing the source reader cannot discard it. Copied web links use canonical UUID addresses.

App Lock is an in-memory privacy screen backed by a stored password. Reload preserves the lock;
restart starts unlocked. It blocks developer tools while locked and does not encrypt content.
The updater verifies signed artifacts before installation; setup belongs in
[Development](DEVELOPMENT.md#desktop-releases).

## Content production

The `markdown` crate owns document and Memo compilation (`publication::render`,
`knowledge::compile`, `knowledge::fallback`, `render_memo`) and reads embed provider data.
`md-dialect` collects embed references without I/O, validates and renders custom `embed:*` fences,
lays out structured diagrams, and normalizes source examples consistently for provider discovery
and compilation. Annotation, quote and diff rendering require no provider reads. Inline image shortcodes use a bundled
`md-dialect` catalog and transform prose events before HTML assembly; compilation performs no image reads.
Article cards use host-provided index metadata; `github`, `market-data`, and `link-preview` supply
repository, market, website and X post metadata for provider cards. [Markdown](MARKDOWN.md) owns
syntax and rendering safety;
[Workflow](WORKFLOW.md) owns operations and recovery.

`vesper build` compiles Markdown under `content/` to HTML in a temporary directory, copies other
regular assets, and emits `content.json`. It rejects symlinks and output collisions. A Rust guard
removes temporary artifacts after success or failure. Source HTML stays escaped; generated markup
and sanitized diagrams enter the output through the compiler boundary.
Media fences compile to native players; desktop readers add playback controls and stop media on exit.
GitHub file links resolve to raw resources. The builder validates document-relative media
inside `content/`; static publication streams copied assets to R2 with extension-based MIME types.
Remote media stays a URL and is loaded by the reader rather than the compilation pipeline.

`vesper publish` shows an upload plan by default; `--live` uploads beneath `blog/` through the Rust
S3 SDK. It does not delete destination-only objects. Remote consumer projects own their Worker
runtimes, R2 bindings and deployment configuration.

## Consumer projections

`crates/consumers` owns authenticated API access and content projection. Desktop `content.rs` owns
repository, view and image caches; its submodules adapt commands. Cache revisions prevent a late
read from
restoring invalidated content. Writes retain each consumer's server-side coordination.

| Consumer  | Boundary                                                                                                                                                                                                                                         |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Memos     | API records include Markdown; Rust compiles it without another R2 read. CRUD and X imports use the Memo API.                                                                                                                                     |
| Moment    | API owns metadata; Rust prepares image variants, uploads them to R2, then registers them. The list is a bounded batch without a synthetic cursor.                                                                                                |
| Knowledge | API summaries form the metadata-only index and classify Newspaper editions. Opening a document performs an authorized detail read and Rust compilation. Writes require both the content hash and exact updated timestamp for conflict detection. |

Knowledge keeps API contracts and index policy in `api/knowledge.rs`; its `render` submodule owns
document compilation, editor previews and authorized reference enrichment.
Knowledge stores Markdown. Milkdown owns browser editing and selection, while the `markdown` crate
owns dialect classification and semantic compatibility. The editor uses Rust source spans to retain
special syntax as source blocks with inline compiled rendering. The `preview_knowledge` transport
calls `consumers::api::knowledge::preview`, which resolves authorized article references and uses
the existing Rust dialect compiler. `markdown::fragment` includes document reference
and footnote definitions when compiling a block. Preview returns HTML or an explicit failure and
never persists the draft. [Markdown](MARKDOWN.md#editing) defines the round-trip contract;
[Design](DESIGN.md#knowledge-interaction) defines the editing surface.
Content and staged visibility share one Save request. Article-list URLs resolve authorized metadata
and desktop destinations in Rust; individual enrichment failures preserve neighboring cards and
leave source code visible when an article cannot be enriched.

Moment shares a Rust EXIF reader between desktop preview and CLI upload without decoding pixels.
Desktop publication uses reviewed form values, including cleared metadata; CLI upload uses EXIF for
unspecified fields. Failed partial uploads can be removed before metadata registration starts.
After registration starts, objects remain available for reconciliation because the server may have
committed them.

Outbound Memo publication belongs to `crates/social`. Commands reread a Memo by ID; the social
boundary independently rejects non-public content before sending bounded text and its canonical URL.
Telegram session persistence and X OAuth credentials use their respective storage boundaries.

## Local persistence

`crates/database` owns the application identifier and `vesper.sqlite3` in local application data;
Desktop and CLI both resolve it with `database::path()`. Feature crates own typed records,
validation and transactions. `schema.sql` is the sole schema definition; startup creates missing
tables without an upgrade or reset layer. [Persistence](PERSISTENCE.md) defines table ownership,
explicit schema rebuilds, locking and backups.

Credentials use SQLite in debug builds and operating-system storage in release builds. Missing
debug configuration never falls through to the OS store. `crates/vault` stores values; each feature
owns its credential types and validation. Public calendar source preferences belong
to Todo, not credentials. [Development](DEVELOPMENT.md#credential-resolution) defines resolution.

### Daily Planner

`crates/todo` owns dated tasks, habit history and calendar projections. ICS parsing uses `icalendar`;
`rrule` validates and evaluates supported recurrence rules. Notion CLI reads and Codex Resets HTTP
reads stay in this crate. The Notion CLI owns authentication; Codex Resets needs only a local enable
preference. `store/calendar.rs` owns calendar snapshots, source synchronization and reconciliation into saved
tasks; the store root owns task mutations and database transactions. Source adapters mark live
remote content as source-owned; SQLite reads reconstruct that ownership from existing source-prefixed
IDs. ICS imports and carried follow-ups own local content, independently of attached calendar
metadata. Completed tasks require reopening before edits, deletion or carry-forward changes.
Each source preserves saved data on failure. Configuration changes and reconciliation share locks
through commit.

Rust moves opted-in unfinished tasks to today and computes monthly completion from stored tasks and
current habit IDs. Completion is derived, never saved as a second state. The layout owns habit IDs
and names; the Dashboard session owns the selected date. Following today advances after midnight or
resume, while historical selection remains fixed. [Dashboard](DASHBOARD.md#calendar-and-todo) owns
behavior and [Persistence](PERSISTENCE.md#planner) owns record invariants.

### Spending

`crates/ledger` stores GBP expenses as integer pence and derives monthly category/day totals.
Validation and aggregation stay in Rust. Spending shares Planner's selected date, but has independent
records and request state. Navigation during a write cannot change the submitted expense date;
mutations invalidate other views of that month. Removing a widget preserves its records.

## CLI surface

Clap defines the command tree, options, generated help, version output, and usage errors before
credential or runtime initialization. Parsed arguments are normalized for the existing feature
adapters; the feature crates retain business validation and external I/O.

The CLI groups commands by feature and reuses the same Rust providers, consumer APIs and stores.
File/stdin parsing finishes before remote writes. Status reads can target one provider or all;
content publication remains an explicit operation. Desktop layout and player state stay outside the
CLI. Use `vesper --help` and [Workflow](WORKFLOW.md) for command usage and recovery.

Command hierarchy follows business ownership, independently of desktop page and widget placement:

| Boundary                | CLI surface                                                        | Desktop relationship                                                                                                       |
| ----------------------- | ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| Independent domains     | `memo`, `knowledge`, `photo`, `todo`, `ledger`, `game`             | Own records and operations through shared Rust crates. Spending is a widget, but its Ledger domain is independent of Todo. |
| Domain capabilities     | `todo notion`, Todo habit check-ins, `photo object`, game archives | Stay under the owning domain instead of becoming top-level commands for each panel or control.                             |
| Provider projections    | `status <source>`                                                  | Expose useful reads such as weather, usage and balances without reproducing Dashboard composition or polling.              |
| Publication workflow    | `build`, `publish`                                                 | Explicit local-content operations, separate from individual consumer records.                                              |
| Presentation components | No CLI commands                                                    | Date pickers, chart modes, widget placement, Dynamic Island and window state belong to Desktop.                            |

Extend the CLI when a desktop capability offers a useful terminal read or operation with explicit
inputs and output. Reuse its Rust feature boundary; do not call a Svelte component or Tauri adapter.
A new widget alone does not justify a new top-level command. Ledger exposes dated expense CRUD and
monthly projections; its category and daily charts consume those projections without separate commands.
