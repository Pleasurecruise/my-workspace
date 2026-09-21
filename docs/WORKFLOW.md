# Local-to-Consumer Workflow

Vesper either publishes a complete local build to R2 or operates on individual records through a
consumer's authenticated API. Direct R2 access is reserved for static artifacts and explicit binary
transfer. [Architecture](ARCHITECTURE.md) defines package ownership; [Development](DEVELOPMENT.md)
covers credentials and build commands. Use `vesper --help` for the complete command surface.

## Static publication

Place Markdown and assets in `content/`, then preview the build and upload plan:

```sh
vesper build
vesper publish
vesper publish --live
```

The builder renders Markdown, highlighted code, Mermaid, and custom `md-dialect` embeds, copies
assets, and writes `content.json` in a disposable temporary directory. It rejects symbolic links,
output collisions, and invalid dialect input before publication. Authored SVG is sanitized;
GitHub and stock embeds resolve through `quotes`. Syntax and examples belong to [Markdown](MARKDOWN.md).

Only `--live` uploads, under R2's `blog/` prefix. Publication is additive: destination-only objects
are not deleted. Removing obsolete objects is a separate explicit maintenance operation. A failed
upload can leave a partial remote build; correct the error and republish the intended content.

## CLI input

`vesper --version` (or `-V`) prints the CLI package version. `vesper --help`, `-h`,
`help`, and running without arguments print usage. Each command has its own help, for example
`vesper memo --help`, `vesper memo list --help`, or `vesper help todo notion`.
These informational commands exit successfully without loading development credentials,
initializing logging, or accessing Keychain.
Invalid command names, missing arguments, and conflicting options report usage on stderr and exit
with status 2; operation failures exit with status 1. Todo and Ledger accept `--date` before or after
their action. Use `--` to separate options from literal content that begins with a dash.

Content payloads accept inline Markdown or JSON, `--file <path>`, or `--stdin` in the payload position.
File and stdin reads preserve newlines, require UTF-8, and fail before requests if parsing fails.
This applies to Memo create/update/page/patch, Knowledge page/create/update-draft/update-documents/
visibility, and Moment query/create/update/upload-photo.

```sh
vesper memo create --file note.md
vesper knowledge update-documents <id> --file article.json
cat filters.json | vesper moment query --stdin
vesper moment upload-photo --file metadata.json photo.heic
```

Successful operations return JSON; errors use stderr and a failing exit code. Desktop and CLI use
the same Rust business operations and validation.

## Memos

Memo writes pass through the my-memos Worker, coordinating R2 bodies, D1 metadata, and KV invalidation.
Lists and searches return the D1 body mirror; Vesper renders it without a second R2 read.

`page` accepts `cursor`, `limit`, `search`, `tags`, `sortByUpdated`, `archivedOnly`, and `favoritesOnly`;
the final two filters are mutually exclusive. `patch` accepts optional `content`, `visibility`,
`tags`, `pinned`, `favorite`, and `archived`, and rejects an empty object. Dedicated commands also
cover tags, visibility, pinning, favorites, archive/restore, and deletion.

`import-x` shares the desktop FxTwitter import and creates a private favorite by default.
Pass `public` explicitly for a public Memo. Social publication setup belongs to
[Development](DEVELOPMENT.md#memo-social-publication-configuration).

## Knowledge

The my-knowledge Worker authorizes against D1 before reading KV or R2. Updates and deletion require
the current `expectedHash` and `expectedUpdatedAt`; a stale copy fails instead of overwriting a newer
article. Desktop preserves both values while editing. Complex payloads use the API's JSON contract.

```sh
vesper knowledge page --file filters.json
vesper knowledge get <id>
vesper knowledge create --file article.json
vesper knowledge update-documents <id> --file changes.json
vesper knowledge delete <id> <expected-hash> <expected-updated-at>
```

`page` accepts `cursor`, `limit` (1–100), up to five `tags`, and `visibility`, returning
`{ articles, cursor }` without bodies. `list` returns `{ documents, cursor }` summary projections.
Use `get` for source, `contentHash`, and `updatedAt` before editing; it accepts a UUID or canonical
UUID article URL. Writes use the returned UUID, never a URL or title. `update-draft` and `visibility`
also accept JSON payloads. REST detail/create/content updates wrap an article in `{ article }`;
visibility wraps a body-free summary. Summaries carry `editions.zh.title` and `summary`, tags,
visibility, hash and timestamps; details add Markdown and current translations. REST omits a terminal
cursor; Rust exposes it as `null`. MCP keyword search uses `{ articles }` with the same summaries,
without scores or excerpts. New articles start public on the Knowledge server. Shared Markdown syntax belongs to [Markdown](MARKDOWN.md).

## Moment

`upload-photo` is the coordinated desktop and CLI path: Rust normalizes an image, uploads its original
and thumbnail to R2, then registers their exact keys with the my-moment API. D1 owns photo metadata.

```sh
vesper moment upload-photo --file metadata.json photo.heic
vesper moment query --file filters.json
vesper moment update <id> --file changes.json
vesper moment delete <id>
```

The upload JSON uses the `Upload` contract. PNG, JPEG, WebP, AVIF, and HEIC sources are limited to
20 MB. Rust applies orientation, fills omitted date/coordinates from EXIF where available, and
produces normalized PNG, JPEG thumbnail, and ThumbHash. Upload failures before metadata registration
trigger cleanup of objects written by the operation. Once registration starts, retain uploaded
objects for reconciliation because the server may already have committed. Inspect partial results
before retrying or removing objects.

Low-level `upload <r2-key> <local-path>` and `create <json>` separate transfer from registration for
explicit recovery. An upload alone does not create metadata. If registration fails, retry it or
remove the unreferenced object with `remove-object <r2-key>`. Never remove an object referenced by an
existing photo. Normal `delete <id>` delegates metadata and image removal to the consumer API.

`query` accepts `fromDate`/`toDate` as `YYYY-MM-DD`, `tags`, and `limit` (1–100, default 20), or `search`.
Search cannot combine with date/tag filters. It returns `{ photos }` without a cursor; `list` returns
at most 100 recent photos. `get <id>` reads one record. In updates, omitted `date`/`geo` preserves the
value and explicit JSON `null` clears it. Tags, search, and object download have dedicated commands.

## Todo

Desktop and CLI share dated tasks. Commands default to today; `todo --date YYYY-MM-DD` selects a day.
Storage, ordering, daily carry-forward, and derived calendar completion belong to
[Persistence](PERSISTENCE.md#planner).

`database-path` and `schedule-path` print storage locations. `import-ics` validates every source
before atomically replacing each managed file; an installation failure may leave earlier files
installed and reports that partial result. `sync-ics` reads only ICS files. Recurrences materialize
once per source, UID, and date; local deletion suppresses recreation. Replacing a source is additive
and retains existing tasks. Zoned events use the device time zone.

Run `ntn login` before connecting Notion. The saved view link must include its view ID; `list` and
`sync` preserve view filters and local completion without modifying Notion pages. `notion status`
reports configuration; `notion disconnect` clears it. Public Codex Resets subscriptions are enabled
in Desktop Settings and synchronize through the same Todo boundary without credentials.

`check-ins <habit-id>...` returns dated completion, editability, streak, total, and 28-day history.
`check-in <habit-id>` and `undo-check-in <habit-id>` write the selected date and reject future days.
Use stable habit IDs from Planner configuration, not display names or placement IDs. These commands
never synchronize calendars; Desktop owns habit names and membership.

## Ledger

Ledger shares Desktop's local GBP store. Use `ledger --date YYYY-MM-DD` for another day. Amounts are
decimal GBP strings; quote categories containing spaces.

Create/update accept an optional description. Omitting it on update preserves the note; an empty
string clears it. Each response includes the day's entries and total, month total, category totals,
daily totals, and category suggestions. Update/delete require the entry's own date. Validation and
integer-pence aggregation are transactional. Desktop observes CLI writes on focus or periodic refresh.

## Additional read commands

`vesper status` reads UGOS and AI sources concurrently with independent success/failure results.
`status <source>` reads only that source and fails its exit code on error. Sources include `ugos`,
`claude`, `codex`, `copilot`, `grok`, `opencode`, `deepseek`, and `cherryin`; output excludes credentials.
Additional status commands cover weather, astronomy, stocks, exchange, GitHub, quotations, and
services. `status service-catalog` lists valid service IDs.

`game notes <game>`, `game archive <game>`, `game sync <game>`, and `game steam` use saved accounts.
Interactive game verification, window layout, native island, and local playback remain in Desktop.
See [Dashboard](DASHBOARD.md) and [Games](GAMES.md) for provider behavior and failure handling.
