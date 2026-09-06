# Documentation

Maintainer-facing documentation is split by responsibility:

| Document                                     | Scope                                                                                         |
| -------------------------------------------- | --------------------------------------------------------------------------------------------- |
| [Architecture](ARCHITECTURE.md)              | Workspace boundaries, data flow, storage, desktop consumers, and current limitations          |
| [Dashboard integrations](DASHBOARD.md)       | Dashboard scheduling, widget sources, AI providers, and failure isolation                     |
| [Development and operations](DEVELOPMENT.md) | Commands, local setup, R2 publication, credential handling, and verification                  |
| [Local-to-consumer workflow](WORKFLOW.md)    | Local artifacts, R2 uploads, consumer API synchronization, rollback, and ownership boundaries |
| [Markdown pipeline](MARKDOWN.md)             | Current compilation boundary, rendering safety, and lessons adopted from Waku                 |
| [Design system](DESIGN.md)                   | UI token layers, component ownership, themes, and accessibility                               |
| [Code style](STYLEGUIDE.md)                  | Rust, TypeScript, Svelte, naming, errors, helpers, packages, and review rules                 |

Feature implementation guides:

| Document                            | Scope                                                                                             |
| ----------------------------------- | ------------------------------------------------------------------------------------------------- |
| [Music](MUSIC.md)                   | Spotify and QQ authorization, collections, local playback, lyrics, and dependency repositories    |
| [UGOS Pro](UGOS.md)                 | NAS connection, certificate handling, RSA login, telemetry projection, and Dashboard lifecycle    |
| [Games](GAMES.md)                   | Components, accounts, daily cache, verification, pull archives, Steam, and reference repositories |
| [Local persistence](PERSISTENCE.md) | Data files, credentials, locks, recovery, and memory-only state                                   |

Architecture describes boundaries and data flow; feature guides own protocol details; Design owns
visible interaction; Development owns setup and verification; Persistence owns durable formats and
recovery. Link to the owner instead of repeating its implementation details. When behavior changes,
rewrite the affected section to describe the final state and remove obsolete or duplicate wording.
Keep the root [README](../README.md) short.

## Release review

The refactor uses one shared Diesel SQLite schema. ICS source files remain active; obsolete local
JSON stores and compatibility readers have been removed. Rust owns provider reads, credentials and
persistence; Svelte owns views and request presentation. The CLI shares the same Rust capabilities.

The 2026-09-07 commit review covers the shared storage, Notion calendar, native island,
provider caches, CLI additions, UI changes and their documentation. It checks Rust formatting,
Clippy, frontend lint/type checks, workspace tests, and CLI/macOS Desktop Release builds.
The cancellation regression verifies that a queued Notion commit retains its calendar lock until
completion; Settings tests cover edits made while saving a Notion link.

This pass is a source review with automated checks, not an independent review or a new live-provider
and native-display acceptance run. Authenticated tests remain opt-in, and other platform bundles
remain the release workflow's responsibility. Storage upgrade behavior is documented in
[Persistence](PERSISTENCE.md#location-and-schema).
