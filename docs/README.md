# Documentation

Each document owns one responsibility. Read the owner rather than repeating its details elsewhere.

| Document                        | Scope                                            |
| ------------------------------- | ------------------------------------------------ |
| [Architecture](ARCHITECTURE.md) | Packages, boundaries and data flow               |
| [Dashboard](DASHBOARD.md)       | Sources, scheduling and failure behavior         |
| [Development](DEVELOPMENT.md)   | Setup, commands, credentials and releases        |
| [Workflow](WORKFLOW.md)         | Content operations, publication and recovery     |
| [Markdown](MARKDOWN.md)         | Compilation and rendering safety                 |
| [Design](DESIGN.md)             | Tokens, components, layout and interaction       |
| [Code style](STYLEGUIDE.md)     | Engineering and review rules                     |
| [Persistence](PERSISTENCE.md)   | Durable records, transactions and schema policy  |
| [Music](MUSIC.md)               | Authentication, library, playback and lyrics     |
| [Games](GAMES.md)               | Accounts, daily notes, verification and archives |
| [UGOS](UGOS.md)                 | NAS connection and telemetry protocols           |

Update documents by rewriting their existing explanation. Remove superseded claims, repeated rules,
and change-by-change implementation notes. Keep setup steps in Development, protocols in feature
guides, and engineering policy in Code style. Link between owners instead of copying paragraphs.
Do not split a feature into more documents merely to move excess text elsewhere.

## Release review

Run the checks in [Development](DEVELOPMENT.md#verification) and use the independent review process
in [Code style](STYLEGUIDE.md#review). Record the actual results and limits in the change handoff,
not as dated review logs in permanent documentation. Automated checks do not replace native UI or
live-provider verification; authenticated tests remain opt-in.
