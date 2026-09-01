---
name: vesper-cli
description: Operate Vesper's typed CLI for provider status, local builds, Todo, R2 publication, and authenticated Memo, Knowledge, and Moment workflows.
---

# Vesper CLI

Use this skill when an AI agent needs to build or publish local content, operate Todo, inspect
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

Todo commands operate on the same local calendar-day JSON file as the desktop and need no
credential. `list` and `get` are reads; the remaining commands mutate today's list. Prefix any
action with `todo --date YYYY-MM-DD` to use another desktop-visible calendar day. The desktop
advances to a new empty daily list at local midnight while retaining earlier dates.

```sh
vesper todo list
vesper todo get <id>
vesper todo create <text>
vesper todo update <id> <text>
vesper todo complete <id>
vesper todo reopen <id>
vesper todo delete <id>
vesper todo --date 2026-08-26 list
vesper todo --date 2026-08-26 create <text>
```

## Provider status

`vesper status` concurrently reads the same shared Rust UGOS, Codex, OpenCode Go, DeepSeek, and
CherryIN boundaries used by Desktop. It is read-only apart from CherryIN's existing token-renewal
policy. Each source reports `ready` or `failed` independently as JSON, so one unavailable provider
does not hide the others. The command never prints credentials.

```sh
vesper status
```

## Local artifacts and publication

```sh
vesper build
vesper publish
vesper publish --live
```

`build` validates local Markdown and assets in a temporary directory. `publish` is a dry-run plan.
Only `publish --live` uploads the staged artifacts through the R2 SDK. Publication is additive and
does not remove destination-only objects.

For locally compiled article cards, use only the registered namespaced fences:

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
embed accepts `align: left`, `right`, or `wide`; omit it for `wide`. Do not invent embed kinds or
fields.

Architecture and storyboard canvases are authored SVG, not Mermaid. Every canvas must include a
meaningful `<title>` and `<desc>` and use a `viewBox`; the compiler sanitizes the SVG before emitting
HTML. Architecture canvases remain transparent and follow the Claude-style vocabulary used by
`canmi21/press`: rounded
`.node` groups, restrained curved `.arr` or dashed `.leader` paths, `.th`/`.t`/`.ts` text, and the
semantic color groups `.c-purple`, `.c-teal`, `.c-coral`, `.c-blue`, `.c-green`, `.c-amber`,
`.c-red`, or `.c-gray`.

Storyboard canvases follow an Excalidraw-style visual language and remain transparent, without an
outer frame or white/dark canvas fill. Use irregular quadratic or cubic paths, round-ended
`.scribble`, `.arrow`, and `.arrow-shadow` strokes, an offset `.sketch-shadow`, `.hand` text, and
the restrained `.fill-blue`, `.fill-violet`, `.fill-green`, or `.fill-orange` groups. Run
`vesper build` before publication so invalid dialect input and unsafe SVG fail locally.

## Memo

Memo reads and writes go through the my-memos REST API so the consumer can coordinate R2 bodies, D1
metadata, and KV invalidation. List and search responses already include the mirrored Markdown body.

```sh
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
accepts the optional fields in `cms_core::api::memos::Update` and rejects an empty object. Quote
Markdown and JSON that contain shell metacharacters.
`import-x` creates a favorite through the same Rust workflow as the desktop and defaults to private
visibility.

## Knowledge

Knowledge operations use the my-knowledge REST API. Create and update payloads are typed JSON passed
as one quoted argument. Preserve the server-provided hash and send it as `expectedHash`; a stale hash
must fail rather than overwrite a newer article.

```sh
vesper knowledge list [cursor]
vesper knowledge get <id>
vesper knowledge create '<json>'
vesper knowledge update-draft <id> '<json-with-expectedHash>'
vesper knowledge update-documents <id> '<json-with-expectedHash>'
vesper knowledge visibility <id> '<json-with-expectedHash>'
vesper knowledge delete <id> <expected-hash>
```

Inspect an existing article with `knowledge get` before constructing an update. Do not invent fields;
use the Rust input types in `crates/cms-core/src/api/knowledge.rs` as the local contract.

## Moment

Moment metadata goes through the my-moment REST API. Image bytes intentionally use the R2 SDK.
Prefer `upload-photo` for a coordinated create; use the separate object upload and metadata commands
for explicit recovery workflows.

```sh
vesper moment tags
vesper moment list [cursor]
vesper moment search <query>
vesper moment upload-photo '<json>' <source-image>
vesper moment upload <r2-key> <local-path>
vesper moment create '<json-with-r2Key-and-thumbnailR2Key>'
vesper moment update <id> '<json>'
vesper moment download <r2-key> <local-path>
vesper moment delete <id>
vesper moment remove-object <r2-key>
```

`upload-photo` uses the desktop's coordinated Rust workflow. Its JSON follows
`cms_core::api::moment::Upload`. Rust accepts PNG, JPEG, WebP, AVIF, or HEIC up to 20 MB, applies
camera orientation and available EXIF defaults, derives the normalized PNG, JPEG thumbnail, and
ThumbHash, then rolls back objects written by the operation when a later step fails.

If metadata creation fails after an upload, retry metadata creation before removing anything.
`remove-object` is only for a verified orphan and can break an existing photo if its key is still
referenced. Use the Rust input types in `crates/cms-core/src/api/moment.rs` as the local contract.

## Output and failures

Consumer commands print JSON on success and write an error to stderr with a failing exit status.
Do not interpret a successful R2 upload as successful metadata registration. Report partial success
and the affected object keys without exposing credentials.
