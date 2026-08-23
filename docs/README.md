# Documentation

Maintainer-facing documentation is split by responsibility:

| Document                                     | Scope                                                                                         |
| -------------------------------------------- | --------------------------------------------------------------------------------------------- |
| [Architecture](ARCHITECTURE.md)              | Workspace boundaries, data flow, storage, desktop consumers, and current limitations          |
| [Dashboard integrations](DASHBOARD.md)       | UGOS protocol, AI usage providers, credentials, polling, and failure isolation                |
| [Development and operations](DEVELOPMENT.md) | Commands, local setup, R2 publication, credential handling, and verification                  |
| [Local-to-consumer workflow](WORKFLOW.md)    | Local artifacts, R2 uploads, consumer API synchronization, rollback, and ownership boundaries |
| [Markdown pipeline](MARKDOWN.md)             | Current compilation boundary, rendering safety, and lessons adopted from Waku                 |
| [Design system](DESIGN.md)                   | UI token layers, component ownership, themes, and accessibility                               |
| [Code style](STYLEGUIDE.md)                  | Rust, TypeScript, Svelte, naming, errors, helpers, packages, and review rules                 |

Keep the root [README](../README.md) short. Update the owning document when runtime behavior,
directory responsibilities, credentials, external protocols, or engineering rules change.
