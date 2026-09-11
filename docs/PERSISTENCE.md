# Local Persistence

Rust owns durable application data. Diesel maps feature records into the shared `vesper.sqlite3`
database; release credentials remain in the operating-system credential store. Svelte retains only
presentation preferences in WebView local storage. Remote API JSON and publication artifacts are
transport formats, not local database substitutes.

## Location and schema

Desktop uses `app_local_data_dir()` and CLI uses `dirs::data_local_dir()` plus `me.you-find.vesper`.
On macOS this is `~/Library/Application Support/me.you-find.vesper/`. Both applications use the same
`vesper.sqlite3`; Windows local data must not be confused with roaming application data.
ICS keeps its original `dirs::data_dir()/me.you-find.vesper/ics` location, including the roaming
application-data directory on Windows. ORM storage changes do not move or delete those input files.

`crates/database` owns connection setup and the schema. Connections enforce foreign keys, a bounded
SQLite busy timeout, and synchronous commits. Unix database permissions are restricted to the owner.
`schema.sql` is the only schema definition. Opening the database creates missing tables with
`CREATE TABLE IF NOT EXISTS`; it does not upgrade or reset existing tables. This personal application
does not maintain versioned migrations. When a constraint or column changes incompatibly, rebuild
only the affected local tables once using the current schema, outside application startup. Preserve
records where compatible, and keep the rebuild transactional; never reset the whole shared database
for a change confined to one feature. Normal reopen must not erase saved data.

Corrupt
databases are reported without replacing their contents. Legacy JSON files, temporary-file recovery
and the separate game database are not imported or used as fallback storage. When upgrading from the retired file-based storage, the shared
database starts with empty tasks, archives and Inbox, default layout, and no Telegram
session or debug credentials. Old files remain on disk; re-saving credentials or syncing provider
history does not recover records that are no longer available remotely.

| Tables                                        | Feature owner           | Contents                                                            |
| --------------------------------------------- | ----------------------- | ------------------------------------------------------------------- |
| `dashboard_widgets`, `dashboard_layout`       | Desktop widgets         | Ordered placements and nullable island selection                    |
| `todo_items`, `todo_occurrences`              | Todo                    | Dated tasks and suppressed/imported occurrence keys                 |
| `check_ins`                                   | Todo                    | Habit completions keyed by stable habit ID and selected date        |
| `ledger_entries`                              | Ledger                  | Dated GBP expenses in integer pence with category and creation time |
| `notifications`, `notification_cursor`        | Desktop Inbox           | Pending messages and SSE replay cursor                              |
| `game_accounts`, `game_pulls`, `game_reports` | Games                   | Accounts, deduplicated history and official reports                 |
| `game_diagnostic`                             | Games                   | Latest bounded verification metadata, excluding secrets             |
| `telegram_session`                            | Social                  | MTProto session state                                               |
| `credentials`                                 | Credentials, debug only | Development credentials and renewable sessions                      |

Typed provider unions, report payloads and credential values may use JSON inside a database field.
Their owning module validates the payload; ordering, identity and transaction boundaries are database
fields. There is no parallel JSON file writer for these records.

## Transactions and failure behavior

Layout replacement validates references before writing and commits placements and selection together.
Invalid widget configurations retain their original JSON through unrelated layout edits; structural
errors fail the layout read. The bundled default initializes or explicitly resets only the layout,
never Todo, habit, or expense records.
Todo mutations reload the selected day inside an immediate transaction. Read-only lists use a read
transaction and can return committed data while another connection holds a pending write. ICS files remain in the
existing `ics/` directory and are read on each calendar sync. Imports validate every source before
replacing any file, then stage and atomically replace each file. A later installation failure may
leave earlier files installed and is reported explicitly. Calendar occurrence keys in SQLite prevent
repeated imports and keep deleted occurrences from reappearing.

Notion reads the configured calendar view through `ntn api` before replacing that day's Notion
projection. A failed CLI read leaves the stored projection untouched. Stable page IDs preserve local
completion, while remote title/date changes and removals are reflected by a successful refresh.
Todo completion and deletion do not edit the Notion page. Imported titles and descriptions are
source-owned; manual title edits preserve descriptions when the CLI omits that field. A feature lock coordinates configuration changes with reads and commits across Desktop and CLI. See [Dashboard](DASHBOARD.md#calendar-and-todo) for the request lifecycle.

Habit mutations validate their explicit date under the write lock, permit historical changes, and
reject future check-ins. Ledger independently validates decimal amounts and categories, then commits
each expense mutation and its selected-month projection in one immediate transaction. Ledger reads
use a consistent transaction; failed validation or aggregation leaves the stored entries intact.
Removing either widget preserves its feature records.

Inbox commits its messages and replay cursor together before publishing the new in-memory state.
Game imports validate provider records and reject conflicts with archived identity before committing
an entire batch. Existing records survive a failed transaction.

## Credentials and sessions

Vesper-owned credentials use the shared database in debug builds and the operating-system store in
release builds. Feature-specific environment overrides apply only where documented; missing or
invalid debug configuration never falls through to Keychain. Feature locks serialize complete
credential read-modify-write operations. [Development](DEVELOPMENT.md#credential-resolution) defines
resolution order.

CherryIN retains its session in Cherry Studio's existing database. Vesper reads that session and
conditionally writes renewed tokens back after a successful OAuth refresh, preserving concurrent
account changes and unrelated fields. It creates no separate CherryIN credential record or database.
[Dashboard](DASHBOARD.md#cherryin) owns the refresh and retry protocol.

Provider output and logs exclude access tokens, cookies, login codes and passwords. The trusted
Settings prefill response is the narrow exception for fields the user edits locally.

## Backup and verification

Close desktop and CLI writers before copying the database. A database backup does not include OS
credentials, the `ics/` directory or WebView preferences. Back up ICS files separately. App Lock controls access to the interface; it does not encrypt
the database. Preserve corrupt data for inspection instead of silently replacing it with defaults.

Functional tests cover concurrent Todo writes, transaction rollback, layout consistency, duplicate
calendar imports, notification persistence, credential isolation and game-history conflicts. Current
verification scope is recorded in [Documentation](README.md#release-review).
