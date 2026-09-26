---
name: vesper-cli
description: Use vesper to inspect providers, manage Todo and Ledger, publish local content, and read or edit Memo, Knowledge, and Moment data.
---

# Vesper CLI

## Execution

- Work from the Vesper repository unless the user names another working tree. Use `vesper --help`
  and `<command> --help` for installed syntax; do not duplicate the command catalog here.
- Never pass credentials as arguments or print them. Debug credentials follow `.env.example`;
  release builds use the OS store.
- Writes require clear user intent, including local records, imports, visibility, deletion,
  uploads, object removal, and `publish --live`. Investigate with reads and publication previews.
- Content payloads accept quoted inline Markdown/JSON, `--file <path>`, or `--stdin`; prefer files
  for multiline input. Parsing finishes before requests. Use `--` before literal dash-prefixed input.
- Data commands return JSON; build and publish commands print status text. Failures use stderr
  and a nonzero exit. Report partial success without secrets.
- Read [Workflow](../../../docs/WORKFLOW.md) for delivery/recovery changes,
  [Development](../../../docs/DEVELOPMENT.md) for credentials, and
  [Markdown](../../../docs/MARKDOWN.md) before authoring embeds.

## Knowledge

Use the API through `vesper`; never bypass its metadata and cache coordination with direct R2 writes.
Identity is the returned UUID `id`, with canonical URL `https://knowledge.you-find.me/articles/{id}`.
There is no `slug` or title alias. `get` accepts a UUID or canonical URL; writes require the UUID.
New articles are public.

```sh
vesper knowledge page --file filters.json
vesper knowledge get <id-or-url>
vesper knowledge create --file article.json
vesper knowledge update-draft <id> --file changes.json
vesper knowledge update-documents <id> --file changes.json
vesper knowledge visibility <id> --file visibility.json
vesper knowledge delete <id> <expected-hash> <expected-updated-at>
```

| Read            | Output                   | Content                                            |
| --------------- | ------------------------ | -------------------------------------------------- |
| `page`          | `{ articles, cursor }`   | Summary `editions.<locale>.{title,summary}`        |
| `list [cursor]` | `{ documents, cursor }`  | Flattened Chinese summaries and `newspaperEdition` |
| `get`           | Article, without wrapper | `editions.<locale>.{title,summary,markdown}`       |

Articles/summaries include `id`, `tags`, `visibility`, `contentHash`, `createdAt`, and `updatedAt`.
Locales include required `zh` and optional `en`/`ja`; never assume a translation exists. CLI output
has no desktop `source`, `html`, `toc`, or `stats`. `newspaperEdition` is developer, personal, or null.
`page` filters: `cursor`, `limit` (1–100, default 20), up to five `tags`, and `visibility`
(public/private). Tags use AND matching with descendants; no tag filter excludes daily articles.
Use `tags: ["daily"]` to read those separately. A null cursor ends pagination.

| Write            | JSON fields                                                              |
| ---------------- | ------------------------------------------------------------------------ |
| Create draft     | `title`, `summary`, `body`, `tags`                                       |
| Create documents | `documents: { zh, en?, ja? }` (complete Markdown strings)                |
| Update draft     | Draft fields, `expectedHash`, `expectedUpdatedAt`, optional `visibility` |
| Update documents | `documents`, `expectedHash`, `expectedUpdatedAt`                         |
| Visibility       | `visibility`, `expectedHash`, `expectedUpdatedAt`                        |

Before mutations, `get` the article and copy `contentHash` → `expectedHash` and `updatedAt` →
`expectedUpdatedAt` from that same response, exactly. Timestamp checks detect visibility-only changes.
On conflict, reread and reconcile; never blindly retry with a new version. Confirm installed
`delete --help` accepts all three positional arguments. Create/content updates return Article,
visibility returns Summary, and deletion returns `{ id, deleted: true }`.
Use [Rust input types](../../../crates/consumers/src/api/knowledge.rs); do not invent fields.

## Markdown and publication

Keep semantic Markdown in Knowledge, not compiled HTML or generated SVG.
[Markdown](../../../docs/MARKDOWN.md) owns supported `embed:*` kinds, fields, examples, SVG vocabulary
and safety.
Use GitHub/stock cards when the repository/ticker matters to the explanation, not as decoration.

Shared article cards use 1–50 canonical UUID URL lines or one `url` field, preserving order.
Vesper's `id`, title and description overrides are local-only; do not submit them to my-knowledge.
Use authorized `page`/`get` IDs, never title-derived addresses. Unresolved cards stay disabled.

`vesper build` validates `content/` into disposable artifacts; `vesper publish` previews uploads.
Only `vesper publish --live` uploads under R2 `blog/`; destination-only objects remain. Validate
before publication and use the [recovery rules](../../../docs/WORKFLOW.md#static-publication).

## Memo

The API coordinates R2, D1 and KV; list/search already contain Markdown. `list [limit]` accepts 1–25.
`page` JSON accepts `cursor`, `limit`, `search`, `tags`, `sortByUpdated`, `archivedOnly`,
`favoritesOnly`; the final two are mutually exclusive. `patch` accepts optional `content`,
`visibility`, `tags`, `pinned`, `favorite`, `archived` and rejects an empty object.
`import-x <url> [public|private]` creates a favorite and defaults to private.
See `vesper memo --help` and [input types](../../../crates/consumers/src/api/memos.rs).

## Moment

Prefer `vesper moment upload-photo --file metadata.json photo.heic`: Rust applies orientation and
EXIF defaults, creates normalized PNG/JPEG thumbnail/ThumbHash, uploads to R2, then registers metadata.
PNG, JPEG, WebP, AVIF and HEIC inputs are limited to 20 MB. JSON follows
[Upload](../../../crates/consumers/src/api/moment.rs).

Before registration, failed uploads clean up their objects. Once registration starts, retain objects
for reconciliation: the server may have committed. An upload alone is not publication. Retry failed
registration before removing anything; `remove-object` is only for verified unreferenced objects.
Normal `delete <id>` delegates metadata/image removal to the API.

`query` accepts `fromDate`/`toDate` (YYYY-MM-DD), `tags`, `limit` (1–100, default 20), or `search`;
search cannot combine with dates/tags. Output is `{ photos }`, without a cursor. Update omission
preserves `date`/`geo`; explicit null clears them. See `vesper moment --help` for recovery commands.

## Todo and habits

Local records are shared with Desktop. Default date is today; use `todo --date YYYY-MM-DD`.
`list`/`sync` may update calendar projections; `sync-ics` reads only ICS. Successful list responses
may contain `syncError` while preserving prior data. `description` is nullable; imported `details`
contains calendar/timing/location, while manual items have null. Title updates preserve descriptions.

`import-ics <path>...` validates all inputs before atomic per-file replacement; partial installation
is reported. Zoned events use local time, unsupported recurrence fails, and occurrence keys preserve
local deletion/completion. `database-path`/`schedule-path` report SQLite/managed ICS locations.

Run `ntn login` before `notion connect <view-url>`; the URL must include `v`, and the account needs
database access. Completion/deletion stay local. Notion owns authentication; `notion status` reports
configuration and the URL. Codex Resets is enabled in Desktop Settings without credentials.

`check-ins <habit-id>...` reads state/history; `check-in` and `undo-check-in` write the selected date.
Use stable Planner habit IDs, not names or placement IDs. Historical writes are allowed, future
writes fail; these commands never sync calendars. Desktop manages habit membership.

## Ledger and provider reads

Ledger shares Desktop's GBP records. Use `ledger --date YYYY-MM-DD`; quote categories with spaces.
Amounts allow two decimal places. Update/delete require the entry's date. Optional descriptions
are preserved when omitted and cleared by an empty string. Responses contain day/month totals,
category/day aggregates, entries and category suggestions. Writes are atomic; Desktop refreshes on focus.

`vesper status` reads configured UGOS/AI sources concurrently with independent ready/failed results.
`status <source>` selects one and fails on read errors; existing renewal policies apply. Additional
reads cover feeds and service status; `status service-catalog` lists service IDs.
Use `vesper status --help` and `vesper game --help`; game reads/sync use saved accounts.
Interactive verification, layout and playback remain in Desktop.
