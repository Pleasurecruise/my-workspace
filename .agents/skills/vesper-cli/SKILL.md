---
name: vesper-cli
description: Operate Vesper's typed CLI for provider status, local builds, Todo, R2 publication, and authenticated Memo, Knowledge, and Moment workflows.
---

# Vesper CLI

Use this skill when an AI agent needs to build or publish local content, operate Todo or Ledger, inspect
consumer data, or perform a Memo, Knowledge, or Moment mutation through `vesper`.

## Before running commands

- Run commands from the Vesper repository unless the task explicitly names another working tree.
- Use `vesper help` as the source of truth for the installed command surface.
- Never pass credentials as command arguments or print them. Debug builds read the selected variables
  documented in `.env.example`; release builds use the operating-system credential store.
- Treat `publish --live`, every `create`, `import-x`, `update`, `visibility`, and `delete` command,
  Moment uploads, and `moment remove-object` as mutations. Obtain clear user intent before running
  them.
- Prefer read commands and `vesper publish` preview while investigating.
- Read `docs/WORKFLOW.md` before changing delivery order or recovery behavior.

## Todo

Todo uses the same SQLite task records as Desktop. Local tasks and ICS need no credential;
Notion calendar reads use the view link saved in Settings or through the CLI and the existing `ntn login` session.
Commands default to today; use `todo --date YYYY-MM-DD` for another date. `list` and `sync` read
ICS and the configured Notion view and may update local projections. `sync-ics` reads only ICS.

```sh
vesper todo list
vesper todo get <id>
vesper todo create <text>
vesper todo update <id> <text>
vesper todo complete <id>
vesper todo reopen <id>
vesper todo delete <id>
vesper todo check-ins <habit-id>...
vesper todo check-in <habit-id>
vesper todo undo-check-in <habit-id>
vesper todo database-path
vesper todo schedule-path
vesper todo import-ics <path>...
vesper todo sync-ics
vesper todo sync
vesper todo notion status
vesper todo notion connect <calendar-view-url>
vesper todo notion disconnect
vesper todo --date 2026-08-26 list
```

`database-path` reports the shared SQLite file. `schedule-path` reports the original managed `ics/`
directory; existing ICS input files remain active and are not migrated into the database.
`import-ics` validates every source before replacing any file, then atomically installs each file.
It projects UTC and IANA TZID times into the local time zone and rejects unsupported recurrence
semantics. Occurrence keys prevent duplicates and preserve local deletion and completion.

Run `ntn login` before `notion connect`. The calendar view link must include its `v` parameter,
and the signed-in workspace must have access to the underlying database.
Completion and deletion stay local and do not modify Notion pages. `syncError` in a successful list
response reports a failed remote refresh while preserving the local projection. `status` reveals
configuration presence and the view URL; authentication remains owned by `ntn`.

`list` and `get` return a nullable top-level `description` for both manual and imported items.
Nullable `details` contains calendar, timing, and location for imported items; manual items return
`null`. CLI title updates preserve the existing description. Legacy Todo JSON is not read.

Habit commands accept stable habit IDs from the saved Planner configuration, not habit names or the
Planner placement ID. `check-ins` reads the selected date's state and history; `check-in` and
`undo-check-in` mutate that date. Historical writes are allowed and future writes fail. They never
synchronize calendars. Habit names and membership remain managed by Desktop.

## Ledger

Ledger uses Desktop's local GBP expense records without credentials. Commands default to today;
use `ledger --date YYYY-MM-DD` for another date. Quote categories containing spaces.

```sh
vesper ledger list
vesper ledger create 12.34 "Coffee shop"
vesper ledger update <id> 8.50 Dining
vesper ledger delete <id>
vesper ledger --date 2024-02-29 list
```

All commands return a snapshot containing the selected day's entries and total, calendar-month total,
category totals, daily totals, and category suggestions. Amounts are decimal GBP strings, limited to
two decimal places. Update/delete require the entry's own date. Writes are atomic; invalid input or
missing entries fail without changing data. Obtain user intent before create/update/delete or habit
check-in/undo. Desktop sees CLI writes on its next focus or periodic refresh.

## Provider status

`vesper status` concurrently reads UGOS, Claude, Codex, Copilot, Grok, OpenCode Go, DeepSeek, and
CherryIN through the same Rust boundaries as Desktop. Each source independently reports `ready`
or `failed` as JSON. `status <source>` queries only that source, prints its data, and exits with
failure if that read fails. Existing credential-renewal policies still apply.

```sh
vesper status
vesper status codex
```

Source names are `ugos`, `claude`, `codex`, `copilot`, `grok`, `opencode`, `deepseek`, and `cherryin`.

## Content input

Replace an inline Markdown or JSON payload with `--file <path>` or `--stdin` to preserve multiline
content. This applies to Memo create/update/page/patch, Knowledge page/create/update/visibility,
and Moment query/create/update/upload-photo. For image uploads, the image path follows the metadata
input: `vesper moment upload-photo --file metadata.json photo.heic`.

Read and parse errors stop before consumer requests. Do not pass credentials in files or stdin
intended for content payloads. These input forms do not change which commands are mutations.

## Local artifacts and publication

```sh
vesper build
vesper publish
vesper publish --live
```

`build` validates local Markdown and assets in a temporary directory. `publish` is a dry-run plan.
Only `publish --live` uploads the staged artifacts through the R2 SDK. Publication is additive and
does not remove destination-only objects.

## Markdown authoring

Read `docs/MARKDOWN.md` for the compiler contract. Supported kinds are `github`, `stock`, `link`,
`article`, `media`, `architecture`, `storyboard`, `annotation`, `quote`, and `diff`. Keep semantic
fences in Knowledge payloads; do not submit compiled HTML or Vesper-generated SVG.

Use registered namespaced fences:

````markdown
```embed:github
repo: owner/repository
align: left
```

```embed:stock
code: AAPL
align: wide
```

```embed:architecture
align: wide
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 680 230" role="img">
  <title>Request path</title>
  <desc>The Svelte view calls a typed Rust command and receives a projection.</desc>
  <g class="node c-teal">
    <rect x="24" y="70" width="170" height="88" rx="12" />
    <text class="th" x="44" y="104">Svelte view</text>
    <text class="ts" x="44" y="128">interaction only</text>
  </g>
  <path class="arr" d="M194 114 C250 114 260 114 316 114" />
  <g class="node c-purple">
    <rect x="316" y="70" width="170" height="88" rx="12" />
    <text class="th" x="336" y="104">Rust command</text>
    <text class="ts" x="336" y="128">typed boundary</text>
  </g>
</svg>
```

```embed:storyboard
align: wide
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 680 250" role="img">
  <title>From rough idea to article</title>
  <desc>Three hand-drawn notes connected by curved arrows.</desc>
  <g class="fill-blue" transform="rotate(-1 120 120)">
    <path class="note" d="M28 48 Q116 42 204 49 L207 181 Q117 188 25 180 Z" />
    <path class="sketch-shadow" d="M30 46 Q116 45 202 51 L205 179 Q116 185 27 182 Z" />
    <text class="hand title" x="52" y="92">Idea</text>
    <path class="scribble muted" d="M52 112 C82 108 116 116 168 110 M52 132 C92 128 130 138 176 130" />
  </g>
  <path class="arrow-shadow" d="M218 118 C250 90 276 147 308 115 M296 104 L310 115 L296 125" />
  <path class="arrow" d="M218 116 C250 88 276 145 308 113 M296 102 L310 113 L296 123" />
</svg>
```
````

Use a GitHub card when a named repository is part of the explanation, and a stock card when a ticker
is discussed as an entity. Do not add them as decoration or repeat a nearby ordinary link. GitHub
and stock data are resolved locally during compilation through the shared `quotes` providers. Every
embed accepts `align: left`, `right`, `narrow` (centered), or `wide`; omit it for `wide`.
Do not invent embed kinds or fields.

Architecture accepts `flowchart LR` or `graph LR` with one `node --> node` edge per line and
optional `[labels]`. Storyboard accepts one `title` and two to six repeated
`step: heading | description` fields. Engineering diagrams group parallel nodes by dependency
and select a vertical layout when their container, after alignment, is at most 640px wide.
Both kinds also accept authored SVG. Each authored SVG must include a meaningful `<title>`, `<desc>`
and `viewBox`; the compiler sanitizes it before emitting HTML. Architecture canvases remain
transparent and follow the Claude-style vocabulary used by `canmi21/press`: rounded
`.node` groups, restrained curved `.arr` or dashed `.leader` paths, `.th`/`.t`/`.ts` text, and the
semantic color groups `.c-purple`, `.c-teal`, `.c-coral`, `.c-blue`, `.c-green`, `.c-amber`,
`.c-red`, or `.c-gray`.

Storyboard canvases follow an Excalidraw-style visual language and remain transparent, without an
outer frame or white/dark canvas fill. Use irregular quadratic or cubic paths, round-ended
`.scribble`, `.arrow`, and `.arrow-shadow` strokes, an offset `.sketch-shadow`, `.hand` text, and
the restrained `.fill-blue`, `.fill-violet`, `.fill-green`, or `.fill-orange` groups. Run
`vesper build` before publication so invalid dialect input and unsafe SVG fail locally.

````markdown
```embed:article
align: narrow
https://knowledge.you-find.me/articles/11111111-1111-4111-8111-111111111111
```

```embed:annotation
mark: 内容优先
note: 让文字成为主角
color: red
---
我的博客坚持内容优先。
```

```embed:quote
author: Project notes
---
Keep the knowledge and its context.
```

```embed:diff
title: Default visibility
---
--- a/config.rs
+++ b/config.rs
@@ -1 +1 @@
-private
+public
```
````

Article lists accept 1–50 URLs and preserve their order. Use UUIDs returned by authorized
`knowledge page`/`get`, never derive an address from a title. Existing legacy slug URLs remain
aliases. Missing/external targets render disabled cards. Vesper's single-card `id`, `title`, and
`description` extensions are local-only; do not send them to my-knowledge. Use URL lines or a single
`url` field for shared content.

Annotation requires a `mark` occurring exactly once in its plain-text body and a `note`; optional
colors are blue (default), red, green, amber, and purple. Annotation and quote accept an optional
credential-free HTTP(S) `url`; quote also accepts `title`. Diff requires a complete unified text
patch with matching hunk counts. These three blocks separate metadata and body with an exact `---`.
Use four backticks or tildes around source examples containing triple-backtick embeds. The compiler
also recognizes the upstream bare triple-backtick wrapper around one embed with adjacent closers.

## Memo

Memo reads and writes go through the my-memos REST API so the consumer can coordinate R2 bodies, D1
metadata, and KV invalidation. List and search responses already include the mirrored Markdown body.

```sh
vesper memo get <id>
vesper memo tags
vesper memo list [limit]
vesper memo page '<json>'
vesper memo search <query>
vesper memo create <markdown>
vesper memo import-x <url> [public|private]
vesper memo update <id> <markdown>
vesper memo patch <id> '<json>'
vesper memo visibility <id> <public|private>
vesper memo pin <id>
vesper memo unpin <id>
vesper memo favorite <id>
vesper memo unfavorite <id>
vesper memo archive <id>
vesper memo restore <id>
vesper memo delete <id>
```

The list limit must be between 1 and 25. `page` accepts `cursor`, `limit`, `search`, `tags`,
`sortByUpdated`, `archivedOnly`, and `favoritesOnly`; the last two are mutually exclusive. `patch`
accepts the optional fields in `consumers::api::memos::Update` and rejects an empty object. Quote
Markdown and JSON that contain shell metacharacters.
`import-x` creates a favorite through the same Rust workflow as the desktop and defaults to private
visibility.

## Knowledge

Knowledge operations use the my-knowledge REST API. Create and update payloads are typed JSON passed
as one quoted argument. Preserve the server-provided hash and send it as `expectedHash`; a stale hash
must fail rather than overwrite a newer article.

```sh
vesper knowledge list [cursor]
vesper knowledge page '<json>'
vesper knowledge get <id-or-url>
vesper knowledge create '<json>'
vesper knowledge update-draft <id> '<json-with-expectedHash>'
vesper knowledge update-documents <id> '<json-with-expectedHash>'
vesper knowledge visibility <id> '<json-with-expectedHash>'
vesper knowledge delete <id> <expected-hash>
```

`knowledge get` accepts a UUID or a complete `https://knowledge.you-find.me/articles/{uuid}`
URL, including existing legacy slug aliases. Updates and deletion require the returned UUID, not a
URL or title. Canonical URLs survive title edits; `slug` remains legacy metadata.
New articles start public on my-knowledge; a subsequent visibility change requires user intent.

`knowledge page` accepts `cursor`, `limit` (1–100), `tags` (up to five), and `visibility`. It returns
compact `{ articles, cursor }` summaries through REST, corresponding to MCP `listArticles`.
`knowledge list` returns `{ documents, cursor }` summary projections without bodies; use `get` for
source and `contentHash`.

Inspect an existing article with `knowledge get` before constructing an update. Do not invent fields;
use the Rust input types in `crates/consumers/src/api/knowledge.rs` as the local contract.

## Moment

Moment metadata goes through the my-moment REST API. Image bytes intentionally use the R2 SDK.
Prefer `upload-photo` for a coordinated create; use the separate object upload and metadata commands
for explicit recovery workflows.

```sh
vesper moment get <id>
vesper moment query '<json>'
vesper moment tags
vesper moment list
vesper moment search <query>
vesper moment upload-photo '<json>' <source-image>
vesper moment upload <r2-key> <local-path>
vesper moment create '<json-with-r2Key-and-thumbnailR2Key>'
vesper moment update <id> '<json>'
vesper moment download <r2-key> <local-path>
vesper moment delete <id>
vesper moment remove-object <r2-key>
```

`moment query` accepts `fromDate`/`toDate` in `YYYY-MM-DD`, `tags`, and `limit` (1–100, service
default 20). Alternatively, use `search` and `limit`; search cannot be combined with dates or tags.
It returns `{ photos }` using the same REST operations as MCP browsing and search. `moment get`
reads one photo directly by ID.

`upload-photo` uses the desktop's coordinated Rust workflow. Its JSON follows
`consumers::api::moment::Upload`. Rust accepts PNG, JPEG, WebP, AVIF, or HEIC up to 20 MB, applies
camera orientation and available EXIF defaults, and derives the normalized PNG, JPEG thumbnail, and
ThumbHash. Failures before metadata registration clean up objects written by the operation; once
registration starts, retain them for reconciliation because the server may already have committed.

If metadata creation fails after an upload, retry metadata creation before removing anything.
`remove-object` is only for a verified orphan and can break an existing photo if its key is still
referenced. Use the Rust input types in `crates/consumers/src/api/moment.rs` as the local contract.

## Output and failures

Consumer commands print JSON on success and write an error to stderr with a failing exit status.
Do not interpret a successful R2 upload as successful metadata registration. Report partial success
and the affected object keys without exposing credentials.
