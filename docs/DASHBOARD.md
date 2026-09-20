# Dashboard Integrations

Rust owns provider protocols, credentials, polling, and typed projections. Svelte retains settled
source state and renders independent loading and errors. Storage rules belong to
[Persistence](PERSISTENCE.md); account setup belongs to [Development](DEVELOPMENT.md).

## Data flow

The Dashboard session activates the Rust runtime and receives source events. Reads run concurrently
behind per-source locks: scheduled reads skip busy sources, while explicit refresh waits and reads
again. Leaving Dashboard cancels its queued and active reads. Missing widgets do not trigger
credentials, CLI processes, or OAuth renewal; structural layout errors fail before provider I/O.

| Source                                        | Active-route refresh                        |
| --------------------------------------------- | ------------------------------------------- |
| Current device and configured UGOS telemetry  | Every two seconds                           |
| AI usage and service status                   | Every sixty seconds                         |
| Steam activity                                | Every five minutes                          |
| Weather, stocks, exchange, GitHub, quotations | Entry and explicit refresh only             |
| Game daily notes                              | Process cache; explicit refresh bypasses it |
| Pull archives                                 | Local reads; explicit synchronization only  |

Dashboard entry and refresh also read the selected Todo date. Provider failures preserve settled
content and remain local to the affected source.

While unlocked, the sidebar polls `tailscale status --json` every 30 seconds and on refresh,
independently of the active route. It lists `tag:server` peers, including offline servers, and excludes
the local device. Names prefer the short MagicDNS alias, then hostname, then IP. Online indicates
tailnet connectivity, not SSH readiness. Discovery failures retain peers with unknown status and
block new SSH connections until recovery. A local tailnet identity change closes remote terminals.
The separate This device entry above the peer list opens a local terminal independently of discovery.

## Widget layout

The twelve-track layout stores typed placements and a nullable Dynamic Island selection. Rust
validates unique placement and habit IDs, singleton constraints, and island references before a
transactional save. `dashboard-default.json` owns initialization and Restore Default; neither resets
feature records. Unsupported old widget kinds are not converted.

Invalid widget configurations appear as diagnostic cards and survive unrelated edits verbatim.
Database and placement-integrity failures affect the whole layout. Singleton conflicts resolve in
stable placement-ID order, independent of drag order.

Dynamic Island shares rendering and Rust source locks in a separate trusted WebView. Expansion reads
only the pinned widget's sources, without starting Dashboard polling. Composite widgets read their
independent sources concurrently. Expanded Planner refreshes once a minute.
App Lock closes the island; verification WebViews receive no account data.

## Current device

`sysinfo` reports CPU, memory, network, and startup-filesystem capacity in decimal GB. APFS volumes
sharing that capacity are not summed. Vesper reads capacity from the operating system and does not
walk directories or estimate file categories. The storage card opens the operating system's storage
settings on macOS and Windows for category details; other platforms report that users should open
their disk utility. Opening settings is explicit and failures remain visible in the card.

## Knowledge

Knowledge reads authenticated summary and detail responses by article ID. Canonical links use
UUIDs; opening one extracts its ID and reads the detail endpoint directly. Article cards resolve
metadata through the authorized summary index.

## Public feeds

`crates/quotes` owns these unauthenticated reads. Failed locations or symbols remain independent.

| Module       | Source and meaning                                                                         |
| ------------ | ------------------------------------------------------------------------------------------ |
| `weather`    | [Open-Meteo geocoding][geocoding] and [forecast][weather]; local time and six hourly cells |
| `stocks`     | Yahoo Finance chart endpoint; validated tickers with bounded concurrency                   |
| `exchange`   | Latest two ECB working days; units per euro and derived cross rates, not live prices       |
| `quotations` | One quotation from [FreeAPI](https://freeapi.app/)                                         |

## Service status

`quotes::status` reads public Statuspage summaries for [GitHub][github-status], [OpenAI][openai-status],
and [DeepSeek][deepseek-status] with bounded concurrency and a fifteen-second deadline. Codex selects
only matching components and linked unresolved incidents. GitHub and DeepSeek include non-group
components and page-wide incidents. Health reflects the worst matching current component; the
percentage is current operational capacity, not historical uptime. Rust owns this classification.

## GitHub

`quotes::github` uses authenticated `gh api`, with `GITHUB_CLI_BINARY` as an optional path override
and fifteen-second request deadlines. GraphQL provides contributions and recent activity; a separate
REST read returns up to twenty unread notification threads and whether more exist. Notification
scope failures remain separate from contribution results. Reads do not mark threads read or change
assignments. Tokens and raw provider errors never reach the UI; GitHub notifications do not enter ntfy Inbox.

## Calendar and Todo

Planner shares a selected date across Calendar, Todo, and habits. Rust checks the local date every
thirty seconds; focus and refresh recover missed events. Following today advances after midnight,
while explicit selections remain selected. Writes retain their submitted date; stale responses
cannot replace a newer selection, and cross-window events invalidate affected projections.

Manual tasks support titles up to 120 characters and descriptions up to 4,000. Imported content
remains source-owned. Completion, deletion, and ordering are local; reconciliation retains surviving
state and appends new tasks. Stale reorder membership fails atomically.

Opt-in carry-forward moves unfinished tasks to the actual local day until completion or opt-out,
catching up after downtime. Remote events become one local follow-up retaining original metadata;
occurrence markers prevent source recreation. An already completed related projection stays on its
original date and prevents a new follow-up. Future browsing never advances tasks.

A non-future day is marked complete when all stored tasks and all currently configured habits are
complete, including days with no tasks or habits. This monthly projection uses saved data without
fetching all remote dates or persisting another completion flag. See [Persistence](PERSISTENCE.md#planner).

### Calendar sources

ICS synchronization reads local `ics/` files. Imports validate all files before atomic per-file
installation. Recurrence identities deduplicate and suppress deleted occurrences. Floating times
stay local; UTC and IANA TZID times use the device zone. Unsupported recurrence fails explicitly.

Notion uses a saved view link and the official `ntn` CLI's existing login. Calendar views select a
Date property; other views require exactly one data-source Date property. Rust applies saved filters
and sorting and paginates complete view membership. Partial results fail without replacing tasks.
Each process has a twenty-second deadline within a ninety-second query budget. Successful view
snapshots cache for five minutes; changing configuration invalidates them.

Codex Resets uses the public `https://codex-resets.com/api/v1/resets` endpoint without credentials.
Rust queries local-day UTC bounds, follows cursors, and retains announcement text and source links.
These are public announcements, not personal quota schedules; forecasts are excluded. Successful
reads cache for five minutes, bounded to 32 dates, with twenty-second HTTP and thirty-second total
budgets. Disabling removes projections as their dates are read; local follow-ups remain.

Explicit refresh bypasses remote caches. Configuration and reconciliation share a cross-process lock
through commit, including cancellation. Notion and Codex read independently; a failed source retains
its stored projection and reports an error without blocking a successful source's refresh.

## Daily check-in

Habit history uses stable IDs. Historical check-in and undo are allowed; future writes are rejected
inside the transaction. Removing and re-adding a habit creates a new ID. The selected day's summary
includes streak, total days, and 28-day history; an unfinished day retains the preceding streak.

## Spending

`crates/ledger` owns local GBP expenses and monthly totals independently of Planner. It shares the
selected date, refreshes on writes, focus, minute ticks, and cross-window events, and retains records
when its widget is removed. Amounts are £0.01–£999,999.99 with at most two decimal places; optional
notes are limited to 500 characters. Categories normalize case and whitespace. Writes and their
projections commit together. No bank connection, currency conversion, or automatic import is involved.

## Games

Panels use the games runtime directly and share daily-status and archive cards. Authentication,
verification, account binding, and archive protocols are maintained in [Games](GAMES.md).

## UGOS Pro

NAS polling requires an active Dashboard and a saved UGREEN widget. Device widgets do not enable it.
[UGOS Pro](UGOS.md) owns authentication, certificate trust, metrics, and failure behavior. Errors strip
request URLs that can contain session tokens.

## ntfy notifications

Inbox independently activates Rust SSE only on its active route with a configured read token.
The fixed source is `https://ntfy.you-find.me/mail-summary`; Vesper does not configure producers.
Reconnect uses `since=<last-id>` and retains the newest 200 messages locally. Leaving Inbox cancels
the stream and reconnect loop. Replays populate Inbox without producing new system notifications.

## AI usage providers

`crates/useage` owns account usage and balance integrations; the spelling is intentional.

| Module         | Source and authentication                                     |
| -------------- | ------------------------------------------------------------- |
| `codex.rs`     | Local `codex app-server --stdio`; existing Codex login        |
| `claude.rs`    | Anthropic usage endpoint; existing Claude Code OAuth          |
| `copilot.rs`   | `gh api /copilot_internal/user`; existing GitHub CLI login    |
| `grok.rs`      | Official Grok runtime billing JSON-RPC; existing device login |
| `opencode.rs`  | `opencode.ai/zen/go/v1/usage`; pi `opencode-go` API key       |
| `deepseek.rs`  | `api.deepseek.com/user/balance`; pi `deepseek` API key        |
| `cherryin.rs`  | CherryIN balance API; existing Cherry Studio OAuth            |
| `tokenflux.rs` | `tokenflux.dev/v1/usage`; pi `tokenflux` API key              |
| `dimagent.rs`  | Local `dim usage --json`; DimAgent Desktop or CLI login       |

Codex, Grok, Copilot, and DimAgent coalesce requests and cache success and failure for five
minutes; cancelled reads are not cached. CLI path overrides are `CODEX_BINARY`, `GROK_BINARY`,
`GITHUB_CLI_BINARY`, and `DIM_BINARY`.
Claude debug reads its local credential file; macOS release may also read Claude Code's Keychain item.
Copilot preserves unlimited quotas and falls back to the account reset date when a row has none.

OpenCode reports percentage used; remaining capacity is `100 - percent`. DeepSeek decimal balance
strings preserve precision across Rust/TypeScript; label CNY as RMB without converting the currency.

### API-key resolution

Resolve case-insensitive provider IDs from pi `auth.json`, then custom providers in `models.json`,
under `${PI_CODING_AGENT_DIR}` or `~/.pi/agent`. Require `api_key` entries or nonempty `apiKey` values.
Bearer secrets must never enter frontend projections, Vesper files, or logs.

### Codex and Claude

Codex initializes JSON-RPC with `experimentalApi`, calls `account/rateLimits/read`, and terminates the child.
Protocol I/O has a fifteen-second deadline. The card uses the primary and secondary windows from
`rateLimits`; additional model-specific limits are not projected. This account integration is
separate from Codex Resets.

The widget picker offers a combined Codex & Claude card; saved standalone cards remain readable.
Both providers retain independent data and error states, including when pinned to Dynamic Island.

### TokenFlux and DimAgent

TokenFlux reads the pi API key without storing another credential. A null daily limit retains daily
usage with an explicit unavailable-limit label instead of a percentage; a null weekly limit hides
that window. Monthly quota and balance remain visible. Failed refreshes label retained values as
the last successful data. DimAgent delegates authentication
to its installed CLI; `DIM_BINARY` selects an explicit executable, otherwise discovery checks the
Desktop bundle and PATH. Both reads have fifteen-second deadlines. DimAgent rejects missing or
malformed credit totals; optional metadata may be absent. Provider error bodies and CLI stderr
are not forwarded to the WebView.

### CherryIN

Read `user_provider` from `CherryStudio/Data/cherrystudio.sqlite`. Refresh expired or nearly expired
OAuth tokens through `/oauth2/token`; a balance 401 permits one forced refresh and retry. Replace the
session only if its saved JSON still matches the pre-refresh value, preserving concurrent changes
and unrelated fields. Never create the external database. Reads serialize without a result cache.
`GET /api/v1/oauth/balance` divides account quota by `500000` for USD; model-token limits are separate.

[geocoding]: https://open-meteo.com/en/docs/geocoding-api
[weather]: https://open-meteo.com/en/docs
[github-status]: https://www.githubstatus.com/api/v2/summary.json
[openai-status]: https://status.openai.com/api/v2/summary.json
[deepseek-status]: https://status.deepseek.com/api/v2/summary.json
