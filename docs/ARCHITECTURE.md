# Architecture

Vesper is a local-first content production and inspection tool. A trusted device owns authoring,
compilation, credentials, and application execution. Cloudflare R2 stores durable content and
publication artifacts; this repository does not run a cloud application backend.

## Repository layout

| Path                 | Responsibility                                                                            |
| -------------------- | ----------------------------------------------------------------------------------------- |
| `apps/desktop`       | Tauri v2 deliverable. Svelte renders views; Rust owns commands and application behavior.  |
| `apps/cli`           | `vesper` executable for builds, publication, Todo, Memo, Knowledge, and Moment workflows. |
| `crates/cms-core`    | Todo storage, Worker APIs, Markdown, consumer projections, build planning, and R2 access. |
| `crates/credentials` | Typed records in macOS Keychain, Windows Credential Manager, or Linux Secret Service.     |
| `crates/logger`      | Shared `tracing` initialization.                                                          |
| `crates/ugos`        | Read-only UGOS Pro authentication, certificate pinning, and Task Manager telemetry.       |
| `crates/useage`      | Read-only AI subscription and account-credit integrations. The spelling is intentional.   |
| `packages/ui`        | Reusable Svelte primitives and design tokens.                                             |
| `packages/tsconfig`  | Shared frontend TypeScript configuration.                                                 |

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
            └─ read-only provider HTTPS APIs

Remote consumer projects
  └─ their own Cloudflare Workers and R2 bindings
```

There is no application login, database, Worker, Wrangler configuration, or server-side session in
this repository. Each online consumer remains responsible for its public presentation and runtime.

## Desktop boundary

Svelte owns interaction state, presentation, accessibility, and invoking named Tauri commands. It
does not compile Markdown, access R2, open credential stores, authenticate to UGOS, spawn Codex, or
call provider APIs directly. The typed Settings read command is the one credential exception: it
returns stored values to prefill that trusted local form.

The Tauri layer maps transport input and output. Domain and protocol behavior stays in its owning
crate. Commands return a tagged `ready` or `failed` response so expected provider and storage errors
remain data rather than uncaught frontend exceptions.

The main window is visible as soon as Tauri creates it. The frontend requests one `InitialViews`
snapshot asynchronously, so a slow or unavailable consumer API cannot block application startup.
Memo metadata and Knowledge load through authenticated APIs. Memo bodies and Moment images use the
shared R2 repository. The first Moment page remains cached for the desktop session. Memo pages use
the cached first page for immediate display, then refresh from the API whenever the view opens and
every 60 seconds while it remains active. Each photo immediately decodes its ThumbHash;
once the card approaches the viewport, the original R2 object loads and fades over that preview.

Dashboard architecture and external protocol details are documented separately in
[DASHBOARD.md](DASHBOARD.md).

The Dashboard's GitHub source is a desktop-local Rust process boundary. It invokes the authenticated
`gh` CLI for one typed GraphQL snapshot when Dashboard is entered or explicitly refreshed; Svelte
does not access GitHub or receive the CLI's credentials. GitHub query and projection details live in
`apps/desktop/src-tauri/github.rs` and [DASHBOARD.md](DASHBOARD.md).

## Content production

Three producer paths converge on the Rust content boundary:

- Tailscale or AirDrop supplies local images. The current compiler preserves files without image
  transformation or content-addressed renaming.
- The `Session to Blog` skill uses the CLI path. It is not a desktop command or editor action.
- The desktop memo editor calls the authenticated my-memos REST endpoint. The Worker coordinates R2
  bodies, D1 metadata, and KV invalidation.

Memo editing is intentionally separate from the temporary publication build. The desktop does not
write memo bodies around the Worker boundary and does not create a retained local mirror.

## Build pipeline

`vesper build` recursively compiles `content/` into an operating-system temporary directory:

1. Each Markdown file becomes HTML at the same relative path.
2. Other regular files are copied unchanged.
3. `.DS_Store` and `Thumbs.db` are ignored.
4. Symbolic links are rejected to prevent reads outside the source tree.
5. Colliding output paths fail the build.
6. `content.json` records rendered documents as `{ path, html }`.

A Rust guard owns the temporary directory and removes it after success or failure. Markdown is
trusted author input, so CommonMark raw HTML is preserved rather than sanitized.

## Publication

`vesper publish` compiles and prints an upload plan by default. `vesper publish --live` uploads the
planned objects concurrently below `blog/`. Publication does not delete objects that exist only at
the destination. R2 is the only durable artifact store.

The Rust S3 SDK is the Cloudflare storage boundary. Remote consumer projects own their own Worker
deployments and R2 bindings; Vesper does not contain Wrangler or Worker runtime code.

## Consumer projections

### Memos

Memos use `https://memos.you-find.me/api/v1`. List and search requests return D1 metadata, including
the R2 object key and cursor. Rust then reads each Markdown body directly from R2 with bounded
concurrency and compiles it for the desktop card presentation. Creation, body updates, and deletion
still pass through the REST Worker so its R2, D1, and KV changes remain one coordinated operation.

### Moment

Moment uses `https://moment.you-find.me/api/v1` for complete D1 photo metadata, including original
and thumbnail R2 keys. The desktop exposes 24-photo pages, renders the metadata ThumbHash immediately,
and reads original image bytes only for cards approaching the viewport. Uploads
place image objects in R2 before registering their metadata through the REST API; metadata updates
and deletion remain Worker operations. The current remote list endpoint
has no cursor and returns at most 100 records, so the desktop cursor pages only that returned set.

### Knowledge

Knowledge uses `https://knowledge.you-find.me/api/articles` with a generated Bearer key. A list read
returns D1 summaries and an optional cursor. Rust follows those summaries with bounded-concurrency
detail reads so the Worker can enforce its D1 authorization before resolving KV and R2 content. Rust
then compiles the Chinese Markdown into HTML, heading identifiers, a table of contents, and an
excerpt. Create, patch, visibility, and delete transports live with the rest of the Knowledge
feature, while the current desktop editor remains preview-only.

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

The CLI groups commands by feature in `todo.rs`, `memo.rs`, `knowledge.rs`, and `moment.rs`. The three
consumer features reuse the same typed REST modules as the desktop. Memo bodies are read from R2
after API discovery; Knowledge remains entirely behind its Worker API; Moment exposes explicit R2
binary transfer before REST metadata registration. Todo commands reuse `cms_core::todo`. Detailed
sequencing and rollback rules live in [WORKFLOW.md](WORKFLOW.md).

## Current limitations

- The desktop Knowledge editor does not yet submit mutations.
- Images are copied without transformations.
- Publication does not reconcile or delete destination-only objects.
- UGOS compatibility depends on the responses observed from the configured device.
- Provider usage APIs can change independently of this application.
