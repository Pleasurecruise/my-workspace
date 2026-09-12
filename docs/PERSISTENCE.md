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

Corrupt databases fail without replacing their contents. Application startup neither imports retired
file stores nor falls back to them; normal reads and writes use the shared database.

| Tables                                        | Feature owner           | Contents                                                            |
| --------------------------------------------- | ----------------------- | ------------------------------------------------------------------- |
| `dashboard_widgets`, `dashboard_layout`       | Desktop widgets         | Ordered placements and nullable island selection                    |
| `todo_items`, `todo_occurrences`              | Todo                    | Dated tasks and suppressed/imported occurrence keys                 |
| `todo_sources`                                | Todo                    | Public calendar source enable preferences keyed by source name      |
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

Ledger validates decimal amounts and categories, then commits each expense mutation and its selected-month projection in one immediate transaction. Ledger reads
use a consistent transaction; failed validation or aggregation leaves the stored entries intact.
Removing a widget preserves its feature records.

Inbox commits its messages and replay cursor together before publishing the new in-memory state.
Game imports validate provider records and reject conflicts with archived identity before committing
an entire batch. Existing records survive a failed transaction.

## Planner

`todo_items` stores tasks keyed by date and ID, with explicit position, completion, and default-off
`rollover` fields. Source metadata belongs to the task projection. `todo_occurrences` records
imported or suppressed occurrence identities: deleting an imported task does not allow the next
sync to recreate it. Remote IDs are source-prefixed; a dated `rollover:<source ID>` marker suppresses
that event from its source date onward after it becomes a local follow-up.

`todo_sources` stores public calendar source preferences as `name` and `enabled`; an absent
`codex-resets` row means disabled. It contains no credentials or API payloads. Notion's existing
view-link configuration and CLI authentication remain separate. Provider response caches are
memory-only, while successfully reconciled tasks survive restart and failed provider reads.

Task mutations reload the day in an immediate transaction. Reordering validates the entire dated
ID set before rewriting positions. Carry-forward moves all eligible dates in one transaction,
retains destination order, and consolidates remote projections without moving completed history.
Repeated or concurrent runs cannot create duplicate follow-ups. Remote reconciliation preserves
completion, ordering, and carry-forward preferences while replacing source-owned content. Failed
reads do not replace a provider's saved projection. Configuration changes and reconciliation retain
a feature lock through commit across Desktop and CLI.

ICS source files remain in `ics/` and are read on synchronization. Imports validate every file
before staging and atomically replacing each one; a later installation failure can leave earlier
files installed and is reported explicitly. SQLite occurrence keys retain local deletion history.

`check_ins` is keyed by stable habit ID and date; the Dashboard layout owns the current habit IDs
and names. Historical writes are allowed, and future writes are rejected inside the transaction.
Removing or reordering habits does not delete or reassign history. A newly added habit gets a new ID.
Monthly completion is calculated from tasks and the current configured habit IDs in a consistent
read: every task complete, every configured habit checked, at least one task or habit, and no future
date. No completion table or persisted summary can drift from those records. The calculation uses
stored task projections, without fetching additional calendar dates.

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
calendar imports, notification persistence, credential isolation, and game-history conflicts.
