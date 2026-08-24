---
name: vesper-cli
description: Operate Vesper's typed CLI for local builds, Todo, R2 publication, and authenticated Memo, Knowledge, and Moment workflows.
---

# Vesper CLI

Use this skill when an AI agent needs to build or publish local content, operate Todo, inspect
consumer data, or perform a Memo, Knowledge, or Moment mutation through `vesper`.

## Before running commands

- Run commands from the Vesper repository unless the task explicitly names another working tree.
- Use `vesper help` as the source of truth for the installed command surface.
- Never pass credentials as command arguments or print them. Debug builds read the selected variables
  documented in `.env.example`; release builds use the operating-system credential store.
- Treat `publish --live`, every `create`, `update`, `visibility`, and `delete` command, Moment uploads,
  and `moment remove-object` as mutations. Obtain clear user intent before running them.
- Prefer read commands and `vesper publish` preview while investigating.
- Read `docs/WORKFLOW.md` before changing delivery order or recovery behavior.

## Todo

Todo commands operate on the same local calendar-day JSON file as the desktop and need no
credential. `list` and `get` are reads; the remaining commands mutate today's list. The desktop
advances to a new empty daily list at local midnight while retaining earlier dates.

```sh
vesper todo list
vesper todo get <id>
vesper todo create <text>
vesper todo update <id> <text>
vesper todo complete <id>
vesper todo reopen <id>
vesper todo delete <id>
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

## Memo

Memo reads and writes go through the my-memos REST API so the consumer can coordinate R2 bodies, D1
metadata, and KV invalidation. List and search responses already include the mirrored Markdown body.

```sh
vesper memo tags
vesper memo list [limit]
vesper memo page '<json>'
vesper memo search <query>
vesper memo create <markdown>
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
vesper moment upload-photo '<json>' <original.png> <thumbnail.jpg>
vesper moment upload <r2-key> <local-path>
vesper moment create '<json-with-r2Key-and-thumbnailR2Key>'
vesper moment update <id> '<json>'
vesper moment download <r2-key> <local-path>
vesper moment delete <id>
vesper moment remove-object <r2-key>
```

`upload-photo` uses the desktop's coordinated Rust workflow. Its JSON follows
`cms_core::api::moment::Upload`; the files must already be a PNG original and JPEG thumbnail. Rust
generates keys and rolls back objects written by the operation when a later step fails.

If metadata creation fails after an upload, retry metadata creation before removing anything.
`remove-object` is only for a verified orphan and can break an existing photo if its key is still
referenced. Use the Rust input types in `crates/cms-core/src/api/moment.rs` as the local contract.

## Output and failures

Consumer commands print JSON on success and write an error to stderr with a failing exit status.
Do not interpret a successful R2 upload as successful metadata registration. Report partial success
and the affected object keys without exposing credentials.
