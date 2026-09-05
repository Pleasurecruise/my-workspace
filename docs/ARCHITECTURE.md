# Architecture

Vesper is a local-first content production and inspection tool. A trusted device owns authoring,
compilation, credentials, and application execution. Cloudflare R2 stores durable content and
publication artifacts; this repository does not run a cloud application backend.

## Repository layout

| Path                 | Responsibility                                                                              |
| -------------------- | ------------------------------------------------------------------------------------------- |
| `apps/desktop`       | Tauri v2 deliverable. Svelte renders views; Rust owns commands and application behavior.    |
| `apps/cli`           | `vesper` executable for provider status, builds, publication, Todo, and consumer workflows. |
| `crates/cms-core`    | Generic Markdown, content builds, static publication, and R2 access.                        |
| `crates/consumers`   | Memos, Moment, and Knowledge APIs, projections, and Moment media processing.                |
| `crates/credentials` | Typed records in macOS Keychain, Windows Credential Manager, or Linux Secret Service.       |
| `crates/logger`      | Shared `tracing` initialization.                                                            |
| `crates/md-dialect`  | Publication and Knowledge Markdown dialect compilation.                                     |
| `crates/music`       | Spotify and QQ Music authentication, collections, playback, album art, and lyrics.          |
| `crates/quotes`      | Shared astronomy, exchange, GitHub, quotation, stock, weather, and status read providers.   |
| `crates/social`      | Outbound Telegram Channel and X publication.                                                |
| `crates/todo`        | Local Todo storage and ICS schedule projection.                                             |
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
       ├─ todo ─────────── application data ── todos.json / ICS
       ├─ credentials ──── operating-system credential store
       ├─ quotes ───────── external read-only data used by Dashboard and Markdown compilation
       ├─ music ────────── Spotify Web API, QQ Music, and LRCLIB
       ├─ ugos ─────────── Tailscale ───────── UGOS Pro NAS
       └─ useage
            ├─ local Codex app-server and Grok runtime
            ├─ existing Claude Code OAuth and GitHub CLI sessions
            └─ provider HTTPS APIs and existing local sessions

Remote consumer projects
  └─ their own Cloudflare Workers and R2 bindings
```

There is no application login, database, Worker, Wrangler configuration, or server-side session in
this repository. The sidebar's editable local profile badge is presentation-only and does not
represent an authenticated session; its display name and cropped avatar remain in WebView local
storage. Each online consumer remains responsible for its public presentation and runtime.

## Credential storage

`crates/credentials` owns typed validation and operating-system storage. On macOS, all such values
share one Keychain item: service `me.you-find.vesper`, account `credentials`. A process-local Rust
cache loads it once. A mutex and the application-data `credentials.lock` file serialize desktop and
CLI access; its random revision invalidates another process's cache after a write. The file contains
no credentials, and a failed Keychain write never installs the attempted values in the cache.
Windows and Linux use per-provider system entries. Development credential resolution and macOS
setup are documented in [DEVELOPMENT.md](DEVELOPMENT.md).

## Desktop boundary

Svelte owns interaction state, presentation, accessibility, and invoking named Tauri commands. It
does not compile Markdown, access R2, open credential stores, authenticate to UGOS, spawn Codex, or
call provider APIs directly. The typed Settings read command is the one credential exception: it
returns stored values to prefill that trusted local form.

The Rust Dashboard runtime owns external-source concurrency, per-source request revisions, and
page-active polling. It sends a closed tagged event to Svelte as each source settles; the view layer
only updates the corresponding card.
Rust owns `layout.json` below application data. Typed commands validate widget IDs, configurations,
and duplicates before writing a same-directory temporary file, syncing it, and atomically replacing
the saved layout. Invalid stored data remains an explicit Dashboard error. Svelte owns add, remove,
and pointer-order interactions on one twelve-track canvas; narrowing the window scrolls the canvas.

Current-device CPU, memory, storage, and network telemetry is read in the desktop Rust boundary with
`sysinfo`. Its sampler runs on a blocking worker only while at least one Current Device widget is in
the saved layout. These widgets are separate from the remote UGREEN NAS telemetry owned by `ugos`.

Configured service-status widgets store only a Rust-validated catalog ID. The desktop Rust boundary
reads each provider's public status summary, narrows Codex to Codex-specific OpenAI components, and
projects component health and the names and states of affected services without exposing an
arbitrary frontend network or URL boundary.

Below `apps/desktop/src-tauri`, `lib.rs` owns application setup and command registration.
`telemetry.rs` owns desktop-device sampling and its short in-memory histories; `storage.rs` owns
startup-volume capacity and file-category estimates using `sysinfo` and `walkdir`. `cms.rs` owns
the consumer repository plus view and image caches, `consumer.rs` owns the Memo, Moment, and
Knowledge commands, and `todo.rs` owns the Todo command adapters. Provider and protocol behavior
continues to live in the owning crates rather than these Tauri modules.

The Tauri layer maps transport input and output. Domain and protocol behavior stays in its owning
crate. Commands return a tagged `ready` or `failed` response so expected provider and storage errors
remain data rather than uncaught frontend exceptions.

App Lock provides a privacy screen for the running application. Rust owns its password storage,
verification, and in-memory lock state. Svelte reads that state before revealing the shell, keeps the
shell inert while locked, and renders the unlock form. Reloading the WebView preserves the lock;
restarting the application starts a new unlocked session. Locking also closes developer tools and
prevents reopening them until the password is verified. App Lock does not encrypt content. Credential
resolution and Settings prefill behavior are documented in [DEVELOPMENT.md](DEVELOPMENT.md).

The main window is visible as soon as Tauri creates it. On macOS it retains the complete native title
bar, including the system title, traffic-light controls, and drag behavior. The frontend requests one
`InitialViews` snapshot asynchronously, so a slow or unavailable consumer API cannot block
application startup.

Music is owned by `crates/music`, with provider code grouped below `spotify/` and `qq/`. Spotify uses
separate PKCE grants for Web API reads and librespot playback; QQ Music uses a private QR exchange and
renews its session on demand. Refresh credentials are represented as one typed record per provider and
rotated under a provider lock. Release builds use the operating-system credential store, while debug
builds use owner-only files below application data to avoid repeated prompts from an ad-hoc app
identity. The WebView receives connection status, QR state, and typed music projections, never
tokens or cookies.

Rust reads Spotify Liked Songs and resolves QQ Music's Daily 30 from its authenticated recommendation
feed and dynamic playlist ID. Each collection is cached for five minutes, and concurrent cache misses
share one refresh. Rust also owns queue order, track advancement, lyrics, media downloads, and
decoding through librespot or Rodio; QQ's audio device remains on a dedicated thread. Remote artwork
and QQ audio URLs are restricted to their provider HTTPS domains, and artwork reaches only the main
webview through `vesper-music-cover`. The desktop persists neither a library mirror nor a lyric
cache. Spotify playback requires Premium, and QQ availability follows the signed-in account's
rights.

Memos and Knowledge load through authenticated APIs; Moment metadata uses its API while image bytes
use R2. At startup, Rust may reuse an unfiltered first page for up to 30 seconds. Normal reads bypass
that cache, and writes or credential changes invalidate it. Svelte retains settled pages while
refreshing active content near the top of its scroll container every 60 seconds. Tag indexes load
independently so they do not delay the first content page. R2 object reads have a 20-second deadline.
Consumer API requests have a 30-second deadline so normal network latency does not abort otherwise
valid paginated responses.
The Memos active, archived, and favorites views request independent API projections with the
`archivedOnly` and `favoritesOnly` query filters, so pagination never derives those views from the
default non-archived page.

Rust renders Memo and Knowledge Markdown. Memo rendering also links bare web addresses without
rewriting code or explicit Markdown links. Svelte retains composer and editor drafts in their
feature's session state across page unmounts. Pending saves update the owning view cache even after
navigation, and successful responses preserve text entered after submission. Drafts are not written
to disk and do not survive a WebView reload or application restart.

Moment loads the API's complete metadata batch in one request so local tag filters cover the whole
returned gallery without repeating requests for client-side pages. Cards decode their ThumbHash
immediately and fetch thumbnails near the viewport; the viewer retains its preview until the
original has decoded. A main-webview-only `vesper-asset` protocol serves image bytes from Rust
without JSON-array serialization or a separate browser cache.

Rust shares image buffers across cache hits and concurrent reads of the same object. The cache
retains at most 64 objects within 128 MiB; failed reads remain retryable. Clearing the cache also
invalidates pending entries so older reads cannot refill it. R2 credential changes reset both the
repository and image cache.

Knowledge and Newspaper share a Rust-owned overview. `consumers` traverses the default and
`tag=daily` summary pages, deduplicates articles, and rejects repeated cursors. Edition tags identify
Programmer Daily and Personal Daily; the overview retains all regular articles and the latest issue
from each stream, then fetches and compiles only those bodies. Historical issues remain accessible
through the CLI's tag-filtered cursor reads.

Svelte renders the resulting articles and edition IDs without interpreting tags. Regular articles
appear in Knowledge, while Newspaper displays the two current issues. Entering Newspaper refreshes
the overview while retaining settled content. The active view refreshes near the top every 60 seconds,
and the desktop also refreshes Knowledge daily at 09:00 local time.

Inbox receives ntfy messages through one Rust-owned authenticated SSE subscription to the fixed
`mail-summary` topic on `https://ntfy.you-find.me`. Rust deduplicates IDs, retains the newest 200
notifications, and reconnects from the last message ID. The current projection and cursor share
`notifications.json`; writes sync a same-directory temporary file before atomic replacement, and
in-memory state advances only after persistence succeeds. An unreadable or invalid file disables
notification consumption and produces an Inbox error without aborting application startup or
replacing the file with empty data.

Bodies may be plain text or a JSON envelope containing `source`, optional `title`, and `body`.
Live messages can trigger operating-system notifications; historical replay only populates Inbox.
Marking a message as read removes it from local storage. Settings owns only the ntfy read token;
producer routes and credentials remain outside Vesper.

The desktop checks the latest published GitHub Release once per application launch through Tauri's
signed updater manifest. The native application menu can request another check without restarting;
overlapping checks and installations are rejected while later retries remain available.
When a newer version exists, Svelte presents its version and notes; Rust rechecks the selected
version with a bounded request, downloads and installs it within a bounded operation while emitting
progress events, verifies its signature, and restarts the application. Update signing uses a public
key embedded in the application and a private key available only to the release workflow.

Dashboard architecture and external protocol details are documented separately in
[DASHBOARD.md](DASHBOARD.md).

The Dashboard's GitHub source is a desktop-local Rust process boundary. It invokes the authenticated
`gh` CLI for GraphQL contributions and REST unread notifications when Dashboard is entered or
explicitly refreshed. Notification failures remain separate from the contribution projection;
Svelte does not access GitHub or receive the CLI's credentials. GitHub query and projection details live in
`crates/quotes/src/github.rs` and [DASHBOARD.md](DASHBOARD.md).

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

Telegram uses a serialized MTProto user session whose authorization key and peer cache use crash-safe
replacement in an owner-only application-data file. X uses an OAuth 2.0 Authorization Code flow with
PKCE and a loopback callback. Its access token, rotating refresh token, Client ID, and expiration are
stored as one operating-system credential record; publishing refreshes an expiring access token
before calling the user-context posting endpoint. Provider failures expose operation and status only,
never credentials or response bodies.

### Moment

Moment uses `https://moment.you-find.me/api/v1` for photo metadata and authenticated listing, tag,
edit, and delete operations. Records contain the original and thumbnail R2 keys. The remote list
endpoint returns at most 100 records without a cursor; desktop pagination covers that returned set.

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

Local files live below the operating-system application data directory for `me.you-find.vesper`
(`~/Library/Application Support/me.you-find.vesper` on macOS). Each feature owns its format and
validation; Vesper does not maintain a local content database or a disk-backed provider cache.

| Data                                      | Owner and storage                                |
| ----------------------------------------- | ------------------------------------------------ |
| Widget order and configuration            | Desktop Rust, `layout.json`                      |
| Date-keyed tasks and imported occurrences | `todo`, `todos.json` with `todos.lock`           |
| Calendar sources                          | `todo`, sibling `ics/` directory                 |
| Pending Inbox messages and replay cursor  | Desktop Rust, `notifications.json`               |
| Telegram authorization session            | `social`, owner-only `telegram.session`          |
| Credentials and renewable grants          | `credentials`, operating-system credential store |
| Theme, local profile and sidebar width    | Svelte, WebView local storage                    |

The layout reader accepts only the current `layout.json` format. A missing file uses the default
layout; malformed data remains an explicit error. Development credential exceptions are described
in [DEVELOPMENT.md](DEVELOPMENT.md).

### Todo and calendar

The `todo` crate owns one date-keyed `todos.json` calendar below the application data directory for
`me.you-find.vesper`. Desktop and CLI serialize writes with a sidecar lock, reload the file before a
mutation, and replace a synced temporary file so an interrupted write does not truncate the last
calendar. They never read or migrate the former `today-todos.json` format. A Rust midnight timer
advances only a view that is still showing today; deliberately selected historical or future dates
remain unchanged, and prior lists are retained.

An optional sibling `ics` directory contains editable schedule sources as application data. Rust
validates every source and materializes matching VEVENT occurrences when a date is read or when the
desktop advances at midnight. The supported recurrence subset is DAILY, WEEKLY, MONTHLY, and YEARLY
with INTERVAL, COUNT, UNTIL, simple BYDAY or BYMONTHDAY values, and EXDATE. Date-only and floating
values retain their calendar date; UTC and IANA TZID-qualified times are converted into the device's
current time zone before selecting a Todo date. Unknown time zones, malformed calendar structure,
unsupported RRULE fields, and unsupported recurrence overrides fail explicitly instead of producing
an approximate schedule.

An occurrence identity combines the source file, UID, and date and is retained independently from
the visible item. Consequently, completing or deleting an imported Todo is stable across later
syncs, calendars may reuse UIDs, and a later hand-authored Todo with the same text is never converted
into an imported item. Adding or replacing an ICS file is additive and does not delete existing
Todos. Imported items carry calendar, start, end, location, and description details; hand-authored
items store an explicit null detail projection. Svelte renders this typed data without parsing ICS.

## CLI consumer surface

The CLI groups commands by feature in `status.rs`, `todo.rs`, `memo.rs`, `knowledge.rs`, and
`moment.rs`. Consumer commands reuse the desktop's typed Rust REST boundaries. Compact Knowledge
summary pages and filtered Moment queries expose the corresponding consumer MCP business
capabilities without introducing an MCP proxy or duplicating server-side filtering.

Markdown and JSON payloads can come from arguments, UTF-8 files or standard input; read and parse
failures precede remote operations. Memo and Knowledge writes retain Worker coordination and
Knowledge content-hash checks. Moment coordinates image preparation and R2 transfer before metadata
registration. Todo shares the date-keyed local calendar, and provider status can query all sources
or one explicitly selected source. Desktop layout, player state, consumer chat memory and interactive
visuals remain outside the CLI. Command contracts and recovery behavior live in
[WORKFLOW.md](WORKFLOW.md).

## Current limitations

- Static publication copies non-Markdown assets without transformations.
- Publication does not reconcile or delete destination-only objects.
- UGOS compatibility depends on the responses observed from the configured device.
- Provider usage APIs can change independently of this application.
