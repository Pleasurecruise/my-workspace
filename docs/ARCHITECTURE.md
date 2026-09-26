# Architecture

Vesper is a local-first desktop and CLI workspace. A trusted device owns authoring, credentials,
compilation and application execution. Remote consumer APIs own published content; Cloudflare R2
stores artifacts. This repository does not host a cloud application backend.

## Repository layout

| Path                    | Responsibility                                                    |
| ----------------------- | ----------------------------------------------------------------- |
| `apps/desktop`          | Tauri v2 shell, Svelte 5 views and Rust command adapters          |
| `apps/cli`              | The `vesper` command-line interface                               |
| `crates/cms-core`       | Markdown compilation, content builds, publication and R2          |
| `crates/consumers`      | Memo, Moment and Knowledge APIs and projections                   |
| `crates/database`       | Shared Diesel/SQLite connection and schema                        |
| `crates/credentials`    | Typed validation and build-specific credential storage            |
| `crates/social`         | Telegram Channel and X publication                                |
| `crates/todo`           | Tasks, habits, ICS, Notion and Codex Resets calendar sources      |
| `crates/ledger`         | Local GBP expenses and monthly statistics                         |
| `crates/md-dialect`     | Custom publication and Knowledge Markdown fences                  |
| `crates/music`          | Spotify and QQ Music authentication, library and playback         |
| `crates/oauth`          | Shared OAuth PKCE, loopback callbacks and token transport         |
| `crates/games`          | Game accounts, daily notes, Steam and pull archives               |
| `crates/github`         | GitHub CLI dashboard and repository reads                         |
| `crates/link-preview`   | SSRF-safe link metadata and fixed-endpoint X previews             |
| `crates/market-data`    | ECB exchange and Yahoo stock reads                                |
| `crates/quotes`         | Random quotation reads                                            |
| `crates/service-status` | Statuspage service catalog and health reads                       |
| `crates/weather`        | Open-Meteo weather, astronomy, and geocoding reads                |
| `crates/ugos`           | Read-only UGOS Pro authentication and telemetry                   |
| `crates/useage`         | AI subscriptions and account credits; the spelling is intentional |
| `packages/ui`           | Self-owned Svelte primitives and semantic design tokens           |
| `packages/tsconfig`     | Shared UI TypeScript configuration                                |

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

`App.svelte` composes navigation, page layout, profile, theme and App Lock. Feature directories own
their views and `session.svelte.ts` state; `pages` composes complete routes and `layout` holds shell
controls. Reusable primitives belong to `packages/ui`. [Design](DESIGN.md) defines presentation.

Sessions survive page mounts within a WebView. Rust supplies an initial content snapshot; a feature
accepts it only if a later read or write has not superseded it. Request generations reject stale
responses, successful writes invalidate older reads, and failed refreshes retain settled data with
an error. Drafts survive navigation and preserve edits made during saves. Credential changes reset
only the affected feature.

Dashboard and Dynamic Island share `WidgetContent` and feature panels. The Rust runtime owns source
polling and per-source request locks; the WebView holds typed projections. Opening the island reads
only the sources required by its pinned widget, concurrently for composite widgets. Layout records
preserve invalid widget configurations for repair while rejecting dangling references. [Dashboard](DASHBOARD.md) owns scheduling and source contracts.

`apps/desktop/src-tauri/src/terminal` owns Tailscale discovery, local shells, and system OpenSSH sessions through
`portable-pty`; xterm.js renders typed byte channels. The renderer selects discovered node IDs.
Rust bounds session resources and expires idle SSH connections independently of the WebView.
The local terminal launches the current account’s default shell without a Tailscale dependency,
remains open while idle, and survives tailnet identity changes. Navigation
preserves hidden terminals; selecting a sidebar device replaces its terminal with a fresh connection.
Rust replaces same-device sessions atomically and rejects superseded launch requests. App Lock, window reload/destruction and shutdown close them and cancel pending
launches. [Development](DEVELOPMENT.md#service-setup) describes authentication and idle limits.

`crates/oauth` adapts `oauth2` to the workspace reqwest transport and uses `httparse` for loopback
request parsing. Spotify and X share that boundary; their feature crates retain endpoints, scopes,
client selection, credential storage and refresh-token rotation policy. Token requests use system
proxies, bounded timeouts and no redirects; callback and token failures omit response bodies.

Music and game runtimes outlive route mounts. Their authentication, cancellation, cache and playback
rules belong in [Music](MUSIC.md) and [Games](GAMES.md); NAS protocols belong in [UGOS](UGOS.md).
Inbox independently activates its ntfy stream while its route is active.
Device storage reads OS capacity only; category inspection is delegated to system storage settings.
Knowledge and Newspaper load a summary-only index without fetching or compiling bodies. The desktop
Rust `KnowledgeReader` owns lazy detail compilation, a 30-second/16-document cache, and same-ID
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

`cms-core::markdown` owns document and Memo compilation. `md-dialect` validates and renders custom
`embed:*` fences, lays out structured diagrams, and normalizes source examples consistently for
provider discovery and compilation. Annotation, quote and diff rendering require no provider reads. Inline image shortcodes use a bundled
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

`crates/consumers` owns authenticated API access and content projection. Desktop `cms.rs` owns
repository and image caches; `consumer.rs` adapts commands. Cache revisions prevent a late read from
restoring invalidated content. Writes retain each consumer's server-side coordination.

| Consumer  | Boundary                                                                                                                                                                                                                                         |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Memos     | API records include Markdown; Rust compiles it without another R2 read. CRUD and X imports use the Memo API.                                                                                                                                     |
| Moment    | API owns metadata; Rust prepares image variants, uploads them to R2, then registers them. The list is a bounded batch without a synthetic cursor.                                                                                                |
| Knowledge | API summaries form the metadata-only index and classify Newspaper editions. Opening a document performs an authorized detail read and Rust compilation. Writes require both the content hash and exact updated timestamp for conflict detection. |

Knowledge stores Markdown. Milkdown owns browser editing and selection, while `cms-core::markdown`
owns dialect classification and semantic compatibility. The editor uses Rust source spans to retain
special syntax as source blocks with inline compiled rendering. The `preview_knowledge` transport
calls `consumers::api::knowledge::preview`, which resolves authorized article references and uses
the existing Rust dialect compiler. `cms-core::markdown::fragment` includes document reference
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

`crates/database` owns `vesper.sqlite3` in local application data. Feature crates own typed records,
validation and transactions. `schema.sql` is the sole schema definition; startup creates missing
tables without an upgrade or reset layer. [Persistence](PERSISTENCE.md) defines table ownership,
explicit schema rebuilds, locking and backups.

Credentials use SQLite in debug builds and operating-system storage in release builds. Missing
debug configuration never falls through to the OS store. Public calendar source preferences belong
to Todo, not credentials. [Development](DEVELOPMENT.md#credential-resolution) defines resolution.

### Daily Planner

`crates/todo` owns dated tasks, habit history and calendar projections. ICS parsing uses `icalendar`;
`rrule` validates and evaluates supported recurrence rules. Notion CLI reads and Codex Resets HTTP
reads stay in this crate. The Notion CLI owns authentication; Codex Resets needs only a local enable
preference. Each source updates its own records and preserves saved data on failure. Configuration
changes and reconciliation share locks through commit.

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
