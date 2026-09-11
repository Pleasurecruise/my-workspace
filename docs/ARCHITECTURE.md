# Architecture

Vesper is a local-first content production and inspection tool. A trusted device owns authoring,
compilation, credentials, and application execution. Cloudflare R2 stores durable content and
publication artifacts; this repository does not run a cloud application backend.

Feature implementation details are maintained in [Music](MUSIC.md), [UGOS Pro](UGOS.md), and
[Games](GAMES.md), including their source boundaries and confirmed reference repositories where applicable.

## Repository layout

| Path                 | Responsibility                                                                              |
| -------------------- | ------------------------------------------------------------------------------------------- |
| `apps/desktop`       | Tauri v2 deliverable. Svelte renders views; Rust owns commands and application behavior.    |
| `apps/cli`           | `vesper` executable for provider status, builds, publication, Todo, and consumer workflows. |
| `crates/cms-core`    | Generic Markdown, content builds, static publication, and R2 access.                        |
| `crates/consumers`   | Memos, Moment, and Knowledge APIs, projections, and Moment media processing.                |
| `crates/database`    | Shared Diesel SQLite connection, schema initialization, and database location.              |
| `crates/credentials` | Typed credentials in debug SQLite or the operating-system credential store.                 |
| `crates/ledger`      | Local GBP expense records, exact-pence validation, and monthly category/day projections.    |
| `crates/logger`      | Shared `tracing` initialization.                                                            |
| `crates/md-dialect`  | Publication and Knowledge Markdown dialect compilation.                                     |
| `crates/music`       | Spotify and QQ Music authentication, collections, playback, album art, and lyrics.          |
| `crates/games`       | Game account authorization, daily notes, Steam activity, and local pull archives.           |
| `crates/quotes`      | Shared astronomy, exchange, GitHub, quotation, stock, weather, and status read providers.   |
| `crates/social`      | Outbound Telegram Channel and X publication.                                                |
| `crates/todo`        | Todo storage, ICS and Notion calendar projection.                                           |
| `crates/ugos`        | Read-only UGOS Pro authentication, certificate pinning, and Task Manager telemetry.         |
| `crates/useage`      | AI subscription and account-credit integrations. The spelling is intentional.               |
| `packages/ui`        | Reusable Svelte primitives and design tokens.                                               |
| `packages/tsconfig`  | Shared frontend TypeScript configuration.                                                   |

Create a crate or package only when it owns a stable independent boundary or is genuinely shared.
Except for the Svelte view layer and its build configuration, new application behavior belongs in
Rust.

## High-level system

```text
Trusted device
  ├─ Desktop / Svelte views
  │    └─ typed Tauri commands
  ├─ vesper CLI
  └─ Rust boundaries
       ├─ cms-core ─────── Rust S3 SDK ─────── Cloudflare R2
       │          └─────── md-dialect ──────── publication Markdown
       ├─ consumers ────── Worker APIs ─────── my-memos / my-moment / my-knowledge
       │          └─────── cms-core R2 / Markdown
       ├─ social ───────── MTProto / X API ─── outbound Memo publication
       ├─ todo ─────────── ICS files / ntn CLI ── SQLite task projections
       ├─ ledger ───────── local SQLite expenses and monthly projections
       ├─ credentials ──── debug SQLite / operating-system credential store
       ├─ quotes ───────── external read-only data used by Dashboard and Markdown compilation
       ├─ music ────────── Spotify Web API, QQ Music, and LRCLIB
       ├─ games ────────── miHoYo / Skland / Steam; shared vesper.sqlite3
       ├─ ugos ─────────── Tailscale ───────── UGOS Pro NAS
       └─ useage
            ├─ local Codex app-server and Grok runtime
            ├─ existing Claude Code OAuth and GitHub CLI sessions
            ├─ Cherry Studio OAuth session and conditional token renewal
            └─ provider HTTPS APIs

Remote consumer projects
  └─ their own Cloudflare Workers and R2 bindings
```

There is no cloud application login, hosted database, Worker, Wrangler configuration, or server-side
session in this repository. Game pull history uses a local SQLite database. The sidebar's editable local profile badge is presentation-only and does not
represent an authenticated session; its display name and cropped avatar remain in WebView local
storage. Each online consumer remains responsible for its public presentation and runtime.

## Credential storage

`crates/credentials` owns typed validation and build-specific storage. Credentials are grouped by
feature: `content/` owns consumer APIs and R2, `music/` owns Spotify and QQ Music, and `games/` owns
game sessions and account selections. Notifications, outbound publication, and UGOS have independent
modules. Shared backends live in `store/`; `environment.rs` loads `.env`, and `app_lock.rs` owns the
local application password.

Debug builds select the shared SQLite credential table at compile time; missing development values
never fall back to the system store. Music, games and other credentials retain their feature-owned validation
and environment-resolution rules. No legacy credential JSON reader or temporary-file recovery runs.

Release builds use operating-system storage. On macOS, values share one Keychain item: service
`me.you-find.vesper`, account `credentials`. A process-local Rust
cache loads it once. A mutex and the application-data `credentials.lock` file serialize desktop and
CLI access; its random revision invalidates another process's cache after a write. The file contains
no credentials, and a failed Keychain write never installs the attempted values in the cache.
Windows and Linux use per-provider system entries. Development credential resolution and macOS
setup are documented in [DEVELOPMENT.md](DEVELOPMENT.md).

## Desktop boundary

Svelte owns rendering, interaction state, accessibility, and named Tauri invocations. Rust owns
application behavior, provider protocols, external I/O, parsing, credentials, and durable storage.
Commands translate transport inputs and return tagged `ready` or `failed` responses. The trusted
Settings prefill commands are the only frontend reads that expose stored credential values.

### Shell and feature state

`apps/desktop/src/App.svelte` composes navigation, the shared page frame, sidebar/profile,
theme, App Lock, and the update overlay. It creates feature view sessions for the lifetime of the
WebView, distributes the initial Rust content snapshot, and connects route and configuration changes.
It does not implement content CRUD, photo byte submission, provider login, or Dashboard projections.

Under `src/lib/components`, `pages` owns complete navigation views and `layout` owns cross-page
controls and `page.css`. Supporting components and their view state live together by feature:

| Feature                       | View-state ownership                                                                                           |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `memos/session.svelte.ts`     | Feed cache, search/display filters, pagination, tag index, mutations, and publication command callbacks        |
| `moment/session.svelte.ts`    | Gallery cache, tag index, selected-file byte submission, and photo mutations                                   |
| `knowledge/session.svelte.ts` | Article and Newspaper overview, refresh scheduling, and editor save callbacks                                  |
| `dashboard/session.svelte.ts` | Independent source projections, route activation, event cleanup, and shared Planner date and Todo interactions |
| `ledger/session.svelte.ts`    | Dated expense reads, mutations, request ordering, and cross-window invalidation                                |
| `settings/session.svelte.ts`  | Configuration status, save/login callbacks, and explicit refresh effects supplied by the shell                 |
| `inbox/session.svelte.ts`     | Notification projection, mark-read interaction, and subscription activation                                    |

These sessions contain view state and typed command adaptation. Image processing, publication,
authorization, and content classification stay in Rust. Reusable primitives and semantic tokens
belong to `packages/ui`; visual composition is specified in [DESIGN.md](DESIGN.md).

The shell creates a Dashboard layout session alongside the data session. Each WebView rejects layout
responses superseded by a newer request and invalidates pending responses when its session is
destroyed. Dashboard and the macOS native Dynamic Island render the same WidgetContent component.
The layout uses Diesel models in the shared `vesper.sqlite3` database and stores a nullable
`islandWidgetId` referencing one placement; defaults select Daily Planner. Rust rejects dangling selections.
The bundled `dashboard-default.json` owns the initial and restored layout, including habit IDs and
island selection. Layout reads isolate configuration decoding and validation failures per widget as
an `Invalid` projection carrying the original configuration and diagnostic. Saving preserves that
original string. Missing metadata and invalid placement identities remain layout-level errors.
Opening the island requests only that widget's source through `refresh_island`, using the same
per-source request lock as Dashboard. It does not enable Dashboard route polling. Todo retains
its own read and mutation commands; game panels retain their existing source reads.

### Content lifecycle

The window appears before content loading finishes. Rust supplies one asynchronous `InitialViews`
snapshot; each feature accepts only a snapshot that has not been superseded by its own read or
reset. Each feature retains settled content across navigation and owns its request generations.
Leaving a page invalidates its pending reads; writes still update their owning feature after
navigation. Successful writes invalidate pending list reads and startup snapshots before applying
their result, so older responses cannot restore deleted or edited content. If a refresh has already
observed a successful creation, the write response merges by ID; bounded gallery totals follow the
merged records rather than incrementing twice. Memo and Knowledge drafts
live in their feature's editor session and preserve edits made during a save. Failed background
reads expose an error while preserving settled content. These caches and drafts end with the WebView
session.

Memos and Moment own independent tag indexes. Startup, page entry, and active-page refresh request
tags without blocking content or each other. Failures preserve settled tags and offer retry;
successful writes supersede older tag reads. Credential changes clear the owning feature's cache
and tag session. Moment captures that session before reading a selected File and checks it before
submitting bytes and accepting the upload result.

Active content refreshes every sixty seconds near the top of the scroll surface. Memos retains
loaded pagination tails during a first-page refresh; its active, archived, and favorite feeds use
independent API filters. Writes during a filter change or within a filtered/sorted feed revalidate
the current first page before pagination resumes; cursors and retained tails never cross filter
changes. Moment receives the API's complete bounded gallery batch for local display filtering.
Knowledge and Newspaper share one Rust-classified overview, refreshed on Newspaper entry and daily
at 09:00 local time. Consumer projections and write boundaries are described below.

Rust's desktop `cms.rs` owns the consumer repository and view/image caches; `consumer.rs` adapts
content commands. Startup may reuse a first page for thirty seconds, while normal reads bypass that
cache. Consumer writes and credential changes invalidate it. Each channel has a cache revision:
invalidation and newer first-page reads prevent older in-flight reads from repopulating or replacing
the cache. Failed reads preserve settled entries until their normal expiry. Moment image bytes use
the main-WebView-only `vesper-asset` protocol, with shared in-flight reads and a cache bounded to 64
objects / 128 MiB. Clearing it invalidates pending entries. R2 reads have a twenty-second deadline;
consumer API requests have a thirty-second deadline.

### Runtime services

The Rust Dashboard runtime owns concurrent source reads, per-source locks, and active-route polling.
Its Svelte session projects typed source events without clearing other cards when one fails. UGOS
also requires a saved remote telemetry widget. Inbox independently activates the Rust ntfy stream
only while its route is active. See [DASHBOARD.md](DASHBOARD.md) for scheduling and failure behavior.

Music and game runtimes outlive page mounts. Music owns provider credentials, library refreshes,
rate-limit cooldowns, cancellable playback tasks, and audio workers. Each Spotify Web API refresh
grant retains its shared or personal application identity. Replacing a music login closes the old
runtime before saving new credentials; Spotify reconnection also invalidates the WebView collection.
QQ audio snapshots carry the loaded track identity, and Spotify events are matched to individual
load requests.

[MUSIC.md](MUSIC.md), [GAMES.md](GAMES.md), and [UGOS.md](UGOS.md) own their protocols, cache rules,
media and verification boundaries, and source maps. The desktop `lib.rs` owns setup and command
registration; `telemetry.rs`, `storage.rs`, `todo.rs`, and `gaming.rs` adapt their named capabilities.

App Lock is a Rust-owned in-memory privacy screen with a stored password. Svelte reads its state
before revealing the shell and keeps the shell inert while locked. WebView reload preserves the
lock; application restart starts unlocked. Locking closes developer tools and blocks reopening
until verification succeeds. It does not encrypt content.

The signed updater checks once per launch and on native-menu request. Svelte presents the version
and notes; installation requires an explicit action. Rust rechecks the version, downloads within a
bounded operation, verifies the signature, installs, and restarts while emitting progress events.
Update signing and operational setup belong to [DEVELOPMENT.md](DEVELOPMENT.md).

## Content production

Content changes converge on the Rust boundaries that own storage and remote protocols:

- Tailscale or AirDrop supplies local images. The current compiler preserves files without image
  transformation or content-addressed renaming.
- The `Session to Blog` skill uses the CLI path. It is not a desktop command or editor action.
- Desktop Memo and Knowledge editors call their authenticated APIs. Moment upload prepares image
  variants and camera metadata in Rust before coordinating R2 upload and API registration.

Consumer editing is separate from the temporary publication build. The desktop does not bypass
consumer APIs for Memo or Knowledge bodies and does not create a retained local mirror.

## Build pipeline

`vesper build` recursively compiles `content/` into an operating-system temporary directory.
`md-dialect` owns this article-oriented compiler, while `cms-core::markdown` retains generic and
Memo rendering:

1. Each Markdown file becomes HTML at the same relative path. Fenced code blocks are highlighted
   with Syntect into inline-styled HTML, while `mermaid` blocks are rendered to self-contained SVG
   by the pure-Rust `mermaid-svg` renderer. Namespaced `embed:github` and `embed:stock` fences resolve
   their data locally through `quotes` and become semantic, self-styled content cards.
   `embed:architecture` and `embed:storyboard` produce transparent sanitized SVG with separate
   Claude-style architecture and Excalidraw-style profiles. Their retained structured syntax is
   upgraded to the same SVG output.
2. Other regular files are copied unchanged.
3. `.DS_Store` and `Thumbs.db` are ignored.
4. Symbolic links are rejected to prevent reads outside the source tree.
5. Colliding output paths fail the build.
6. `content.json` records rendered documents as `{ path, html }`.

A Rust guard owns the temporary directory and removes it after success or failure. Source raw HTML
is rendered as literal text; only compiler-generated code, diagram, embed, and embed-style markup
enters the artifact as raw HTML. Authored SVG is filtered through `svg-hush`; missing accessible
titles or descriptions, unsafe SVG, and invalid Mermaid or embed syntax fail the build before
publication.

## Publication

`vesper publish` compiles and prints an upload plan by default. `vesper publish --live` uploads the
planned objects concurrently below `blog/`. Publication does not delete objects that exist only at
the destination. R2 is the only durable artifact store.

The Rust S3 SDK is the Cloudflare storage boundary. Remote consumer projects own their own Worker
deployments and R2 bindings; Vesper does not contain Wrangler or Worker runtime code.

## Consumer projections

### Memos

Memos use `https://memos.you-find.me/api/v1`. List and search requests return complete D1 records,
including the mirrored Markdown body, R2 object key, and cursor. Rust compiles the returned body for
desktop presentation without repeating an R2 read. Creation, body updates, and deletion pass through
the REST Worker so its R2, D1, and KV changes remain one coordinated operation.
X/Twitter imports are prepared by the trusted Rust boundary: it validates a public status URL,
reads the post from the fixed FxTwitter endpoint, renders text and photo links as Markdown, and then
creates a favorite through the same authenticated my-memos API.

Outbound Memo publication belongs to `crates/social`. Public Memo cards expose compact Telegram and
X actions beside the visibility label; private cards expose neither action. Each desktop publication
command accepts only an ID, rereads that Memo through its authenticated API, and passes the returned
content and visibility to `crates/social`, which independently rejects non-public Memos. Both
providers receive a bounded plain-text projection followed by the Memo's canonical URL.

Telegram stores its serialized MTProto authorization key and peer cache in the `telegram_session`
table of the owner-only shared SQLite database. Each update commits before replacing the in-memory
session. X uses an OAuth 2.0 Authorization Code flow with PKCE and a loopback callback. Its access
token, rotating refresh token, Client ID, and expiration are stored as one operating-system
credential record; publishing refreshes an expiring access token before calling the user-context
posting endpoint. Provider failures expose operation and status only, never credentials or response
bodies.

### Moment

Moment uses `https://moment.you-find.me/api/v1` for photo metadata and authenticated listing, tag,
edit, and delete operations. Records contain the original and thumbnail R2 keys. The remote list
endpoint returns at most 100 records without a cursor; desktop pagination covers that returned set.
The CLI `moment list` and desktop gallery return one bounded batch; they expose no synthetic cursor.
Channel responses contain content only; tagged command failures represent unavailable connections,
and Memos/Moment tag indexes use independent reads.

The shared Rust upload path accepts PNG, JPEG, WebP, AVIF, and HEIC, normalizes orientation, and reads
available EXIF time and coordinates. It produces a PNG original, JPEG thumbnail, and ThumbHash,
assigns both `img/` keys, uploads the objects, and registers their metadata through the API.

Cleanup follows the registration boundary. If thumbnail upload fails, Rust removes the original
before any metadata request. After registration starts, an unsuccessful response may still follow a
committed record, so both objects are retained. The error reports their keys for checking the gallery
and reconciling the upload before retrying or removing objects.

### Knowledge

Knowledge uses `https://knowledge.you-find.me/api/articles` with a generated Bearer key. A list read
returns D1 summaries and an optional cursor. Rust follows those summaries with bounded-concurrency
detail reads so the Worker can enforce its D1 authorization before resolving KV and R2 content. Rust
then uses `md-dialect` to compile the Chinese Markdown into HTML, heading identifiers, a table of
contents, and an excerpt. YAML front matter returned with an edition is excluded from both the
editable body and compiled output. The dialect compiler preserves math, portable wiki links, GFM
callouts, and supported content embeds. Structured fences without a renderer remain escaped code. If
optional embed enrichment fails, Knowledge preserves the embeds as code so a provider failure cannot
hide an article or turn a committed write into an apparent failure. The desktop editor creates and
updates drafts through the same API with content-hash conflict detection; visibility and delete
transports remain available to the CLI.
The editor uses a Tiptap rich-text surface with Markdown parsing and serialization, while an explicit
source mode preserves constructs that the configured rich-text schema cannot round-trip exactly.
The stored body, API payload, and content-hash conflict contract remain Markdown-based.

## Local persistence

`crates/database` owns the shared Diesel/SQLite schema and connection policy. Desktop and CLI use
`vesper.sqlite3` in local application data. Feature modules own typed records, validation and
transactions: layout, Todo, Ledger, Inbox, game archives and Telegram sessions. Debug credentials use the
same database; release credentials retain the operating-system store boundary. Legacy files are
neither read nor migrated. Todo reads use a read transaction; mutations retain their immediate
transactions. `schema.sql` is the only schema definition; there is no versioned migration layer.
Incompatible schema changes are handled by a one-time rebuild of the affected local tables.
[Persistence](PERSISTENCE.md) owns the schema inventory and failure rules.

### Daily Planner

`crates/todo` owns dated tasks, local habit records, ICS parsing, and Notion calendar projection.
Svelte renders Calendar, Todo, and daily habits in one Planner placement. The layout stores each
habit's stable ID and name. Planner is the only supported calendar/task/habit placement; old
standalone kinds are not converted.

Desktop and CLI construct the production store through `Store::shared()`. Tasks, imported-occurrence
keys, and `check_ins` records live in the shared local SQLite database. ICS files remain in
`dirs::data_dir()/me.you-find.vesper/ics`. Imported tasks retain source-owned content and local
completion/deletion state. Manual edits change title and description without changing date or
completion. Desktop request generations reject stale date responses and defer reads requested
during a mutation until the write finishes. Mutations notify the other trusted WebView.

The Notion view link belongs to `crates/credentials`; the official `ntn` CLI owns authentication.
Rust runs bounded CLI processes and retains one complete view snapshot in memory for five minutes.
Date selection projects that snapshot into the dated SQLite task list. Explicit synchronization
bypasses the cache, while configuration saves invalidate it. Calendar reads and configuration
writes share an in-process gate and cross-process lock, retained through the database commit.
Failed or incomplete reads preserve the last saved task projection. Provider protocols and refresh
rules are detailed in [DASHBOARD.md](DASHBOARD.md#calendar-and-todo).

Habit history is keyed by stable habit ID and date. Calendar, tasks, and habits share one selected
Planner date; their existing SQLite tables remain separate and require no schema rebuild or
migration. Reads explicitly target that date and return habit IDs, editability, completion, total
days through the selection, ongoing streak, and the 28 days ending on the selection. Historical
check-ins can be added or undone; Rust rejects future writes inside the transaction. Pending reads
and writes cannot install another date's projection. The previous day's streak remains active when
the selected day is unfinished. Reordering and restarting preserve records. Removing and re-adding a habit
creates a new ID without deleting or reassigning the old history. The panel reads habits together
on mount, date changes, focus, minute ticks, and cross-window events, deferring reads while a write
is pending. Rust emits local-date changes every thirty seconds independently of task synchronization;
Planner refresh and focus also read the date. Following today advances all three sections even if
ICS, Notion, or SQLite fails; manually selected dates remain selected.

### Spending

`crates/ledger` owns local GBP expense records and their calendar-month projections, independently
of Todo and habit state. `ledger_entries` in the shared `vesper.sqlite3` stores an ID, local date,
positive integer pence, category, and creation timestamp. The table and date index are initialized
from the current schema; no migration or existing-table rebuild is required. Amounts are parsed
from decimal strings in Rust, accept at most two decimal places, and never use floating-point
arithmetic in storage or aggregation. Rust also normalizes category whitespace and case, rejects
invalid dates/amounts/categories, and computes day totals, category totals, and every day of the
selected month including zero-spend days. Writes and their resulting projections share one
immediate transaction; reads use a consistent read transaction.

The standalone Spending widget shares the Dashboard session's `selectedDate` with Planner.
Calendar and Spending navigate through `selectDate`; mounted Planner views react by loading their
Todo projection, while Spending reads only its ledger projection. Reads never change the selection. The feature's `ledger/session.svelte.ts`
owns its read/write lifecycle and receives the selected date from the widget composition. A date
change clears the prior projection, superseded reads are discarded, and navigation during a write
rereads the latest selection after the submitted date commits. `expenses-updated` invalidates the
other WebView when any date in its displayed month changes. Hiding or unpinning the widget leaves
all entries intact; no provider, login, credential, or network service is involved.

## CLI surface

The CLI groups commands by feature in `status.rs`, `game.rs`, `todo.rs`, `ledger.rs`, `memo.rs`,
`knowledge.rs`, and `moment.rs`. Consumer commands reuse the desktop's typed Rust REST boundaries. Compact Knowledge
summary pages and filtered Moment queries expose the corresponding consumer MCP business
capabilities without introducing an MCP proxy or duplicating server-side filtering.

Markdown and JSON payloads can come from arguments, UTF-8 files or standard input; read and parse
failures precede remote operations. Memo and Knowledge writes retain Worker coordination and
Knowledge content-hash checks. Moment coordinates image preparation and R2 transfer before metadata
registration. Todo shares the date-keyed local calendar and habit history; Ledger reuses its own
crate for expense CRUD and month projections. Neither duplicates Desktop business logic. Provider
status can query all sources
or one explicitly selected source. Desktop layout, player state, consumer chat memory and interactive
visuals remain outside the CLI. Command contracts and recovery behavior live in
[WORKFLOW.md](WORKFLOW.md).

## Current limitations

- Static publication copies non-Markdown assets without transformations.
- Publication does not reconcile or delete destination-only objects.
- UGOS compatibility depends on the responses observed from the configured device.
- Provider usage APIs can change independently of this application.

Game-note task projections may include typed numeric progress for visual completion indicators.
The miHoYo record session retains a per-game verification trace from the last restricted daily
response; each pending challenge captures that trace for registration and proof submission.
Verification diagnostics retain only game, stage, numeric return code and trace presence.

The most recent miHoYo verification response replaces the single `game_diagnostic` database row. This diagnostic contains only game, register/submit stage, numeric return code,
trace presence and timestamp. It excludes provider messages, account IDs and all credentials.
Saving diagnostics never initiates a provider request.

Manual miHoYo pull synchronization shares the account's existing `RecordSession` and role cache
with daily notes. Genshin pull AuthKeys use a separate SToken request profile and are scoped to the manual
sync operation. Sync does not invalidate or refresh the daily-note cache.

miHoYo verification return code `30001` means this verification request needs no captcha;
it does not confirm daily access and supplies no challenge token. Registration/submission surface
an explanatory failure and preserve the cached restriction. Only a successful response with a valid
challenge can transition the card to `refreshRequired`; this performs no daily request.
Both games bind `x-rpc-challenge_path` to the full daily endpoint URL. The official RPG client's
Axios dispatcher combines its base URL and relative path before exposing the response config.

Completing verification replaces only that game/login's cached verification error with a typed
`refreshRequired` state. The native window emits the same state to the card immediately. This
local transition performs no provider I/O, persists across view remounts and cache replays, and
preserves successful cached notes. Only an explicit refresh reads fresh daily data.

Star Rail manual pull sync uses `mihoyo/rail_gacha.rs` to exchange the existing record cookies for
an activity-only badge cookie and read official pool totals and five-star records. The shared
`game_reports` table is keyed by game and UID with an account foreign key. An atomic transaction
merges older five-star entries and replaces statistics without modifying real rows in `game_pulls`.
`archive::Summary.official` is explicitly null for ordinary archives and otherwise carries this
report. The view labels its partial coverage; it does not infer missing pulls or timestamps. The
badge cookie is memory-only and scoped to one manual sync. No daily-note cache is changed.
