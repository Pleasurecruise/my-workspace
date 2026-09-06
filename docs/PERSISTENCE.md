# Local Persistence

Vesper persists data by feature: JSON files for small local collections and configuration, SQLite
for game pull history, the build-specific credential store for secrets, and WebView local storage
for presentation preferences. Rust owns business-data reads and writes. Svelte accesses local
storage only for interface preferences and invokes typed Tauri commands for application data.

This document describes the current implementation, including recovery and concurrency limits.
See [ARCHITECTURE.md](ARCHITECTURE.md) for application boundaries and
[DEVELOPMENT.md](DEVELOPMENT.md#credential-resolution) for credential setup.

## Storage locations

The application identifier is `me.you-find.vesper`. Desktop business files use Tauri's
`app.path().app_data_dir()`. The shared Todo CLI uses `dirs::data_dir()` plus the identifier;
credential files and their locks use `dirs::data_local_dir()` plus the identifier.

Persisted filenames describe their data or provider without a build-mode prefix. Debug and release
credential separation is selected by the compiled backend. Legacy filenames are not read or migrated.

On macOS these resolve to `~/Library/Application Support/me.you-find.vesper/`. On other platforms,
use the owning resolver rather than hard-coding the macOS path; in particular, Windows roaming
application data and local application data are different locations. Debug and release builds use
the same identifier and business-data paths; their credential backends differ.

The main files are created as their features need them, rather than all being installed at startup:

| Path                               | Owner                       | Persisted data                                             |
| ---------------------------------- | --------------------------- | ---------------------------------------------------------- |
| `todos.json`                       | `crates/todo`               | Date-keyed tasks and calendar-import deduplication records |
| `ics/`                             | `crates/todo`               | Imported calendar source files                             |
| `layout.json`                      | Desktop `widgets.rs`        | Ordered widget instances and their configurations          |
| `notifications.json`               | Desktop `notifications.rs`  | Pending Inbox messages and replay cursor                   |
| `games.sqlite3`                    | `crates/games`              | Accounts and accumulated pull history                      |
| `mihoyo-verification.json`         | `crates/games`              | Latest saved verification diagnostic                       |
| `telegram.session`                 | `crates/social`             | MTProto authorization and session state                    |
| `credentials.json`                 | `crates/credentials`, debug | Shared development credential entries                      |
| `spotify.json`                     | `crates/credentials`, debug | Spotify renewable credentials                              |
| `qq-music.json`                    | `crates/credentials`, debug | QQ Music renewable session                                 |
| `games-{mihoyo,skland,steam}.json` | `crates/credentials`, debug | Game accounts, sessions, or provider settings              |

Temporary files may also appear during writes. WebView storage is managed by the platform's browser
engine and is not a Vesper JSON file in this inventory.
Lock files coordinate access rather than store business records. The implementation uses two
scopes; these are naming patterns for existing feature-owned locks, not a shared lock registry.

| Scope             | Filename pattern   | Protected operation                                                           |
| ----------------- | ------------------ | ----------------------------------------------------------------------------- |
| File access       | `<name>.json.lock` | Individual credential JSON reads, temporary-file recovery, and writes         |
| Feature operation | `<name>.lock`      | A feature's complete read-modify-write sequence or coordinated backend access |

A file-access lock ends when its individual read or write finishes. A feature-operation lock stays
held while the owning feature reads the current collection, changes it, and saves it, preventing
concurrent updates from overwriting each other. Features that replace complete values and features
that update shared collections do not necessarily need the same scope.

For example, Todo coordinates its calendar and ICS operations through `todos.lock`. The shared
credential store uses one `credentials.lock` path: debug protects the JSON map update, while macOS
release protects Keychain map access and its cache revision. Account collections apply the same
operation-level principle within their owning feature; see [Games](GAMES.md#authorization-and-account-selection).
These locks are separate from SQLite's database locking and can also protect operations whose
values reside in the system credential store.

## Todo and calendar sources

[`model.rs`](../crates/todo/src/model.rs) defines the on-disk calendar:

- `days`: a map from `YYYY-MM-DD` to an ordered array of tasks.
- Each task has `id`, `text`, `completed`, and nullable `details`.
- `details` contains `calendar`, `startDate`, `startTime`, `endDate`, `endTime`, `location`, and
  `description`; all fields except calendar and start date can be null.
- `imported_occurrences`: a map from date to a set of imported occurrence keys, preventing repeated
  calendar reads from adding the same event again.

[`store.rs`](../crates/todo/src/store.rs) serializes operations with an in-process mutex and an
exclusive `todos.lock` file lock shared by desktop and CLI. Mutations reload the calendar under the
lock, modify it, serialize the complete document to `todos.json.tmp`, sync the temporary file, and
replace `todos.json`.

If the primary file is missing but the temporary file exists, a read first restores the temporary
file. If neither exists, the calendar starts empty. Invalid JSON and I/O errors are reported rather
than converted to an empty calendar. The former `today-todos.json` format is not migrated.

The sibling `ics/` directory contains the source calendars. Import validates the input files before
installing each through a synced temporary file. Installation of multiple files is not one
transaction: earlier files can remain installed if a later installation fails. Calendar sync adds
matching occurrences to the requested day's tasks and saves their deduplication keys alongside them.
Historical days remain stored when the active date changes.

## Dashboard layout

[`widgets.rs`](../apps/desktop/src-tauri/src/widgets.rs) stores
`{ "widgets": [{ "id": "...", "widget": { "kind": "..." } }] }` in `layout.json`.
Array order controls placement. The tagged widget value carries configuration such as a weather
location, stock symbol, service catalog ID, or game. Card spans come from application policy,
not a separate saved geometry field.

Reads and writes use an in-process mutex. Writes validate widget identities, configuration, and
duplicates, then write and sync a same-directory `NamedTempFile` before replacing the destination.
There is no cross-process layout file lock. A missing file selects the default layout; malformed
or invalid data produces an explicit error. The decoder also normalizes legacy separate game-note
and gacha widgets into the current combined game widget when reading the existing layout format.

## Inbox notifications

[`notifications.rs`](../apps/desktop/src-tauri/src/notifications.rs) stores two top-level fields:
`last_id` and `notifications`. Each notification contains `id`, `topic`, `source`, nullable `title`,
`message`, `timestamp`, and `tags`.

Rust loads this file at startup, deduplicates incoming IDs against retained messages, and keeps at
most the newest 200 entries. The SSE subscription resumes from the saved message ID. Marking an
entry as read deletes it from the local collection; there is no separate read-history table.

Updates hold the in-memory write lock, build the next collection and cursor together, and persist
them through a synced same-directory temporary file. Only a successful replacement advances the
in-memory state. There is no cross-process file lock. Missing storage starts empty; unreadable or
invalid storage disables consumption and shows an Inbox error without preventing application startup
or overwriting the file with empty data.

## Game archive

[`archive.rs`](../crates/games/src/archive.rs) owns `games.sqlite3`, currently schema version 2 via
SQLite `user_version`. A database with a newer version is rejected.
Star Rail activity reports are stored in `official_reports(game, uid, payload)`, separately from
individual pull rows. Only complete manual syncs replace report statistics; older five-star entries
remain saved. No badge cookie, AuthKey or session cookie is stored in the archive.

| Table      | Primary key       | Other columns                                                                 |
| ---------- | ----------------- | ----------------------------------------------------------------------------- |
| `accounts` | `(game, uid)`     | `name`, `region`, `role_id`, `synced_at`                                      |
| `pulls`    | `(game, uid, id)` | `pool`, `pool_name`, `item_id`, `name`, `rarity`, `time`, `is_free`, `is_new` |

Pulls reference their owning account through a foreign key. Connections enable foreign keys,
`synchronous=FULL`, and a five-second busy timeout. Sync fetches every remote page before merging;
the account update and record inserts commit in one database transaction. Existing history is not
deleted when it disappears from the provider's limited history window. Conflicting records fail the
merge; missing miHoYo item IDs can later be filled without changing record identity. Unknown item
IDs are stored as empty strings and projected to consumers as null.

Counts, rarity distributions, and recent-record projections are calculated from the archive rather
than persisted as another summary. Removing login credentials leaves the archive readable offline.
Game login sessions and account selections belong to the credential store, not these tables.

`mihoyo-verification.json` is a separate diagnostic containing `game`, `stage`, `retcode`,
`has_trace`, and `timestamp`. It excludes account identity, cookies, and captcha values. Unlike the
business stores, this diagnostic uses a direct file write and is not a transactional archive.

## Credentials and authorization sessions

[`crates/credentials`](../crates/credentials/src/lib.rs) selects its backend at compile time:

- Release on macOS stores an `entries` map inside one Keychain item, service
  `me.you-find.vesper`, account `credentials`. A process mutex and `credentials.lock` serialize
  access. A random revision in the lock file invalidates another process's cached map after a
  change. Failed Keychain writes do not install the attempted values in the cache.
- Release on Windows and Linux uses per-provider operating-system credential entries.
- Debug uses development files and provider-specific environment resolution; it never falls back
  to the system credential store. The shared JSON file maps credential names to string values,
  which may themselves encode typed provider records. A file lock protects read-modify-write.
  Music and games use the separate typed session files listed above.

Development file reads and writes hold the sibling `.json.lock` file lock. Writes use a synced
`.json.tmp` file and replacement; a missing primary can be recovered from that temporary file under
the same lock. Unix secret files are restricted to mode `0600`. Music token rotation uses provider
locks; shared account collections also hold their owning feature’s operation lock across
read-modify-write. The shared credential map holds `credentials.lock` across that complete operation.

Debug startup may load the ignored repository-root `.env`, with inherited environment variables
taking precedence. Settings writes do not rewrite `.env` or the running environment. Exact
provider precedence is documented in [DEVELOPMENT.md](DEVELOPMENT.md#credential-resolution).

[`telegram.rs`](../crates/social/src/telegram.rs) is the separate secret-file boundary:
`telegram.session` is serialized JSON despite its extension, holding MTProto session data including
authorization keys and peer state. Writes use `telegram.session.tmp`, sync, and replacement, with
Unix mode `0600` and missing-primary recovery. Session access uses an in-process mutex. Login codes
and two-factor passwords are not persisted. Telegram API configuration remains in the credential
store, as do X access and refresh grants.

App Lock's configured password is a credential-store value, not a content encryption key. The
current locked/unlocked state stays in Rust memory: a WebView reload preserves it, while an
application restart starts unlocked. Only the typed Settings prefill boundary may return stored
credentials to the trusted local form.

## WebView preferences and memory-only state

[`App.svelte`](../apps/desktop/src/App.svelte) and
[`theme.ts`](../apps/desktop/src/lib/theme.ts) use these local-storage keys:

| Key                     | Value                                |
| ----------------------- | ------------------------------------ |
| `app-theme`             | `dark` or `light`                    |
| `vesper.profile.name`   | Local display name                   |
| `vesper.profile.avatar` | Cropped avatar image data            |
| `vesper.sidebar.width`  | Sidebar width serialized as a string |

These values belong to the WebView origin/profile, are not shared with the CLI, and do not represent
an authenticated application account. Changing WebView origins or clearing browser storage can
change which preferences are available.

Memo and Knowledge editor drafts survive page navigation in Svelte session state, but not a WebView
reload or app restart. Consumer first-page results, Moment image buffers, music collections, lyrics,
daily game results, and telemetry histories are runtime state rather than a persistent offline
database. The Moment image cache is bounded to 64 objects and 128 MiB; consumer startup first-page
reuse lasts up to 30 seconds, and music collections cache for five minutes.

Memo and Knowledge content is authoritative in their authenticated APIs. Moment metadata uses its
API and image objects use R2. Desktop reads and edits do not create a retained local content mirror.
Explicit CLI downloads are user-selected output files. Repository `content/` is authoring input;
publication builds use disposable system-temporary `vesper-publish-*` directories. Shared logging
initializes `tracing` output without configuring a persistent log-file store.

External clients such as Codex, Claude Code, GitHub CLI, and Cherry Studio own their existing local
sessions. Their paths and behavior are described in [DASHBOARD.md](DASHBOARD.md); they are not part
of Vesper's application-data inventory. CherryIN refresh can conditionally update the matching
Cherry Studio OAuth record.

## Durability and recovery boundaries

There is no application-wide storage transaction, migration framework, or automatic local-data
backup. Each feature owns its validation, synchronization, and compatibility rules.

For Todo, development credentials, and Telegram sessions, non-Windows replacement uses rename;
Windows currently removes the old destination before renaming the temporary file. The Windows
sequence has a missing-file window and is not an atomic replacement. Temporary-file recovery helps
on the next read but does not make that sequence atomic. Layout and Inbox use `NamedTempFile`
replacement. File syncing in these JSON writers does not include an explicit parent-directory sync,
so it should not be described as a universal guarantee against every power-loss scenario.

A local backup must distinguish business files, secret stores, and WebView preferences. Copying
business JSON and SQLite files alone does not include Keychain credentials or browser preferences.
For a consistent manual copy, close desktop and CLI writers first. Preserve corrupt files for
inspection: missing data may initialize defaults, but invalid existing business data is normally
reported as an error. App Lock does not encrypt these files.
