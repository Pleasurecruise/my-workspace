# Architecture

Vesper is a local-first content production and inspection tool. A trusted device owns authoring,
compilation, credentials, and application execution. Cloudflare R2 stores durable content and
publication artifacts; this repository does not run a cloud application backend.

## Repository layout

| Path                 | Responsibility                                                                              |
| -------------------- | ------------------------------------------------------------------------------------------- |
| `apps/desktop`       | Tauri v2 deliverable. Svelte renders views; Rust owns commands and application behavior.    |
| `apps/cli`           | `vesper` executable for provider status, builds, publication, Todo, and consumer workflows. |
| `crates/cms-core`    | Todo storage, Worker APIs, Markdown, consumer projections, build planning, and R2 access.   |
| `crates/credentials` | Typed records in macOS Keychain, Windows Credential Manager, or Linux Secret Service.       |
| `crates/logger`      | Shared `tracing` initialization.                                                            |
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
       ├─ cms-core ─────── Consumer APIs ───── my-memos / my-moment / my-knowledge
       │          ├─────── Rust S3 SDK ─────── Cloudflare R2
       │          └─────── application data ── todos.json
       ├─ credentials ──── operating-system credential store
       ├─ ugos ─────────── Tailscale ───────── UGOS Pro NAS
       └─ useage
            ├─ local Codex app-server
            └─ provider HTTPS APIs and existing local sessions

Remote consumer projects
  └─ their own Cloudflare Workers and R2 bindings
```

There is no application login, database, Worker, Wrangler configuration, or server-side session in
this repository. The sidebar's editable local profile badge is presentation-only and does not
represent an authenticated session; its display name and cropped avatar remain in WebView local
storage. Each online consumer remains responsible for its public presentation and runtime.

## Desktop boundary

Svelte owns interaction state, presentation, accessibility, and invoking named Tauri commands. It
does not compile Markdown, access R2, open credential stores, authenticate to UGOS, spawn Codex, or
call provider APIs directly. The typed Settings read command is the one credential exception: it
returns stored values to prefill that trusted local form.

The Rust Dashboard runtime owns external-source concurrency, per-source request revisions, and
page-active polling. It sends a closed tagged event to Svelte as each source settles; the view layer
only updates the corresponding card.
Rust also owns the Dashboard widget layout stored as `dashboard-layout.json` below the
application data directory. Svelte adds, removes, and pointer-orders widgets through typed commands;
Rust rejects unsupported widget IDs, fields, stock symbols, and duplicate entries before saving. The
desktop canvas retains one fixed twelve-track arrangement at every window width, so narrowing the
window scrolls the canvas instead of deriving and displaying a different layout.

Below `apps/desktop/src-tauri`, `lib.rs` owns application setup and command registration. `cms.rs`
owns the consumer repository plus view and image caches, `consumer.rs` owns the Memo, Moment, and
Knowledge commands, and `todo.rs` owns the Todo command adapters. Provider and protocol behavior
continues to live in the owning crates rather than these Tauri modules.

The Tauri layer maps transport input and output. Domain and protocol behavior stays in its owning
crate. Commands return a tagged `ready` or `failed` response so expected provider and storage errors
remain data rather than uncaught frontend exceptions.

App Lock is a local privacy boundary, not content encryption. Rust owns password storage and
verification; Svelte only makes the application shell inert and renders the unlock surface. Debug
credential resolution and the Settings prefill exception are documented in
[DEVELOPMENT.md](DEVELOPMENT.md).

The main window is visible as soon as Tauri creates it. On macOS it retains the complete native title
bar, including the system title, traffic-light controls, and drag behavior. The frontend requests one
`InitialViews` snapshot asynchronously, so a slow or unavailable consumer API cannot block
application startup.

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
rewriting code or explicit Markdown links.

Moment cards decode their ThumbHash immediately and fetch the R2 thumbnail only when approaching the
viewport; the viewer requests the original. Rust retains up to 64 recently used image objects within
a 128 MiB process-memory limit. R2 credential changes clear that cache.

Newspaper is a Rust-owned projection of Knowledge. `cms-core` classifies established edition tags,
selects the latest Programmer Daily and Personal Daily documents, and marks editions for exclusion
from the regular Knowledge index. Svelte renders that typed projection without interpreting tags.
Entering Newspaper refreshes the first Knowledge page immediately while retaining its settled
content, and the active view refreshes near the top every 60 seconds. The desktop also refreshes
Knowledge daily at 09:00 local time.

Inbox is the consumer boundary for messages published through ntfy. Rust owns one authenticated SSE
subscription to the fixed `mail-summary` topic on `https://ntfy.you-find.me`, deduplicates ntfy
message IDs, retains the newest 200 notifications in `notifications.json` below the application data
directory, and reconnects with the last message ID so ntfy can replay cached messages. Svelte only
renders the typed local projection. A notification body may be a plain message or a normalized JSON
envelope with `source`, optional `title`, and `body`; the envelope separates the producer identity
from the transport topic. Newly received live messages also use the registered operating-system
notification adapter; replayed historical messages only populate Inbox. Marking a message as read
removes it from the Rust-owned notification list. No separate read-message archive is retained.

Settings stores only the ntfy read token; the server address and topic are fixed application policy.
Producer routes, credentials, signing secrets, and processing remain outside Vesper.

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
`gh` CLI for one typed GraphQL snapshot when Dashboard is entered or explicitly refreshed; Svelte
does not access GitHub or receive the CLI's credentials. GitHub query and projection details live in
`apps/desktop/src-tauri/github.rs` and [DASHBOARD.md](DASHBOARD.md).

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

`vesper build` recursively compiles `content/` into an operating-system temporary directory:

1. Each Markdown file becomes HTML at the same relative path. Fenced code blocks are highlighted
   with Syntect into inline-styled HTML, while `mermaid` blocks are rendered to self-contained SVG
   by the pure-Rust `mermaid-svg` renderer.
2. Other regular files are copied unchanged.
3. `.DS_Store` and `Thumbs.db` are ignored.
4. Symbolic links are rejected to prevent reads outside the source tree.
5. Colliding output paths fail the build.
6. `content.json` records rendered documents as `{ path, html }`.

A Rust guard owns the temporary directory and removes it after success or failure. Source raw HTML
is rendered as literal text; only compiler-generated code and diagram markup enters the artifact as
raw HTML. Unsupported or invalid Mermaid syntax fails the build before publication.

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

### Moment

Moment uses `https://moment.you-find.me/api/v1` for complete D1 photo metadata, including original
and thumbnail R2 keys. The shared Rust upload path accepts PNG, JPEG, WebP, AVIF, and HEIC sources,
normalizes orientation, reads available EXIF time and coordinates, and produces a PNG original, JPEG
thumbnail, and ThumbHash. It then assigns both `img/` keys, uploads the objects, and registers the
metadata, removing objects already written if a later step fails. Listing, tags, metadata edits, and
deletion remain authenticated Worker operations. The current remote list endpoint has no cursor and
returns at most 100 records, so the desktop cursor pages only that returned set.

### Knowledge

Knowledge uses `https://knowledge.you-find.me/api/articles` with a generated Bearer key. A list read
returns D1 summaries and an optional cursor. Rust follows those summaries with bounded-concurrency
detail reads so the Worker can enforce its D1 authorization before resolving KV and R2 content. Rust
then compiles the Chinese Markdown into HTML, heading identifiers, a table of contents, and an
excerpt. YAML front matter returned with an edition is excluded from both the editable body and the
compiled reader output. The Knowledge compiler also preserves math, maps portable wiki links to the
consumer's article route, and recognizes the consumer's GFM callouts. Structured fences without a
desktop renderer remain visible as escaped code. The desktop editor creates and updates drafts
through the same API with content-hash conflict detection; visibility and delete transports remain
available to the CLI.
The editor uses a Tiptap rich-text surface with Markdown parsing and serialization, while an explicit
source mode preserves constructs that the configured rich-text schema cannot round-trip exactly.
The stored body, API payload, and content-hash conflict contract remain Markdown-based.

## Local Todo persistence

The `cms_core::todo` module owns a date-keyed calendar of Todo lists in the single `todos.json` file
below the operating system's application data directory for `me.you-find.vesper`. The previous
`today-todos.json` file is not read or migrated. Desktop and CLI use the new file. A sidecar lock
serializes their operations, and every operation reloads the complete calendar before applying a
change.
While the desktop is running, a Rust timer emits the new date at local midnight without deleting any
prior list. This clears the visible daily list by advancing to the new, initially empty date while
preserving history. A deliberately selected historical or future date remains selected.

## CLI consumer surface

The CLI groups commands by feature in `status.rs`, `todo.rs`, `memo.rs`, `knowledge.rs`, and
`moment.rs`. The three consumer features reuse the same typed REST modules as the desktop, including
Memo's X import.
Memo and Knowledge content stays behind their Worker APIs; Moment exposes explicit R2 binary transfer
before REST metadata registration. Todo commands reuse `cms_core::todo` and accept an explicit date
for the same calendar-day operations available in the desktop. Detailed sequencing and rollback
rules live in [WORKFLOW.md](WORKFLOW.md).

## Current limitations

- Static publication copies non-Markdown assets without transformations.
- Publication does not reconcile or delete destination-only objects.
- UGOS compatibility depends on the responses observed from the configured device.
- Provider usage APIs can change independently of this application.
