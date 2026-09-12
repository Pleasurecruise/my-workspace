# Dashboard Integrations

Dashboard aggregates provider reads. Rust owns protocols, credentials, source locks and polling;
Svelte holds settled projections and renders independent loading and error states. Vesper-owned
release credentials use the operating-system store, while external CLI sessions retain their own
storage. CherryIN renews the existing Cherry Studio OAuth session when needed. Telegram and X
publication remain outside the Dashboard runtime. See
[Development](DEVELOPMENT.md#credential-resolution) for credential setup.

## Data flow

```text
DashboardView.svelte
  <- source projections in components/dashboard/session.svelte.ts
  <- typed source events and refresh commands
  <- dashboard runtime in apps/desktop/src-tauri
  ├─ current-device telemetry
  ├─ crates/ugos
  ├─ crates/quotes
  │    ├─ exchange.rs
  │    ├─ quotations.rs
  │    ├─ weather.rs
  │    ├─ stocks.rs
  │    ├─ status.rs
  │    └─ github.rs
  └─ crates/useage
       ├─ claude.rs
       ├─ codex.rs
       ├─ copilot.rs
       ├─ grok.rs
       ├─ opencode.rs
       ├─ deepseek.rs
       └─ cherryin.rs
```

The shell passes route activation to the Dashboard view session, which owns source-event listeners,
refresh feedback, Todo selection, and cleanup. An unavailable credential or failed source does not
block the other cards. Codex, OpenCode, Claude, Grok, Copilot, DeepSeek, CherryIN and GitHub read
only when their corresponding widget is saved. An absent widget emits a ready `null` projection
without reading credentials, launching a CLI or renewing OAuth; a structural layout error fails before
provider I/O. Rust starts unified Dashboard reads concurrently and emits each result as it settles.
A per-source lock prevents overlapping reads; scheduled refreshes skip a source that is still
running, while an explicit refresh waits for that source and then obtains fresh data. Leaving
Dashboard cancels queued and in-flight Dashboard runtime reads, including scheduled UGOS requests.
Polling exists only while Dashboard is active and retains settled data while refreshing: configured
UGREEN NAS telemetry and current-device telemetry run every two seconds, while subscription data and
configured service status run every sixty seconds. Entering Dashboard or using its refresh action
reads every source and the selected Todo date. Steam activity refreshes every five minutes while
Dashboard is active. Game daily notes only load once per game and login during the application
process; subsequent reads reuse the cache. Their panels also read when mounted. Pull archives load
locally and sync only on an explicit action. Weather, stocks, exchange rates, GitHub, and random
quotations have no timer.

## Games

Each game has a combined daily-status and pull-archive card; Steam has a library/activity widget.
Panels invoke the Rust games runtime directly. Dashboard entry reuses daily cache entries, while
explicit Dashboard refresh also refreshes daily status. Steam polls every five minutes; archive
sync remains an explicit action. Removed split daily/archive widget kinds are rejected.

[GAMES.md](GAMES.md) owns account binding, QR authorization, cache ordering, human verification,
archive transactions, Steam projections, and protocol references. [DESIGN.md](DESIGN.md#game-interaction)
owns card layout and settings interactions.

## ntfy notifications

`components/inbox/session.svelte.ts` projects notifications and passes route activation to Rust.
Notification delivery is scoped to Inbox, independently from Dashboard polling:

```text
Upstream producers ──> ntfy.you-find.me/mail-summary ── authenticated SSE ──> Vesper Inbox
```

- Vesper does not connect to or configure upstream producers.
- The transport is the self-hosted `https://ntfy.you-find.me` service. The current fixed topic is
  `mail-summary`; its ACL must grant the configured token read permission.
- Vesper connects only when Inbox is active and an ntfy token is configured. Leaving Inbox cancels
  the stream and reconnect loop; saving credentials while another route is active does not connect.
- Vesper subscribes in Rust, reconnects with ntfy's `since=<last-id>` behavior, and keeps the newest
  200 messages locally for Inbox rendering.

## Widget layout

Dashboard cards occupy a twelve-track canvas. Edit mode supports dragging, removing, pinning, and
restoring the Rust-owned default. Dragging within a row targets individual cards; dragging across
rows inserts at a row boundary. Narrow windows scroll the canvas without changing saved order.

The bundled `apps/desktop/src-tauri/src/dashboard-default.json` defines the default order:
AAPL, TSLA, Device CPU, Device Storage, Exchange, GitHub service status, Quotation,
Ningbo/Nottingham/Shanghai weather, Daily Planner, Spending, GitHub activity, Codex, OpenCode Go,
CherryIN, Arknights, and Star Rail. Daily Planner retains placement ID `calendar`, the two habits
Eat Breakfast 🍚 and Work Out 🏋️ with their stable IDs, and the Dynamic Island selection.
The same configuration is used for an uninitialized layout and Restore Default.

Rust validates and transactionally replaces `{ widgets, islandWidgetId }` in `vesper.sqlite3`.
Placements have unique IDs and typed configurations; a non-null island selection references one
placement. Habit IDs and names are unique within the Planner. Unknown fields, unsupported kinds,
duplicate placements, and dangling selections fail validation. An uninitialized layout uses the
default. Invalid widget configurations appear as individual diagnostic cards showing only that
widget’s raw configuration and expandable error details; valid cards remain usable. Editing
other cards preserves the invalid configuration verbatim. Conflicting singleton configurations are
validated in stable placement-ID order, so dragging cards cannot switch which configuration is
active. Removing the active conflict allows the remaining valid configuration to load normally.
Remove a diagnostic card to discard its configuration, or use Restore Default to rebuild the layout. Old Calendar, TodoList, and CheckIn kinds are unsupported;
no old widget conversion or legacy file import runs. Database and layout-integrity failures remain
layout-level errors.

The widget library uses a category rail without search. Personal contains Daily Planner and Spending. Devices contains
UGREEN CPU/Memory/Storage/Network and Device CPU/Memory/Storage/Network. AI Services contains Codex,
OpenCode Go, Claude, Grok, Copilot, DeepSeek, and Cherry. The remaining categories
are Online Services and Games. Existing singleton widgets remain visible with an Added state.

The macOS Dynamic Island shares WidgetContent rendering, saved layout, and Rust source locks with
Dashboard, but has a separate WebView session. Expansion reads only its selected source without
activating Dashboard polling. Expanded Daily Planner rereads tasks and habits once a minute;
Notion reads use the shared five-minute snapshot. Game events reach both trusted UI windows,
while verification webviews receive no account data. App Lock closes the island.

## Current device

`sysinfo` supplies CPU, memory, network and startup-filesystem capacity. Device Storage reports
only the startup filesystem, so macOS APFS system and Data volumes do not add the same container
capacity twice. External drives and mounted images are outside this card's scope. Capacity uses
decimal GB; a missing filesystem or invalid capacity produces an unavailable state.

On Unix, the Storage card separately requests a file-category scan when mounted and exposes an
explicit rescan action. Rust uses `walkdir` on the current user's home and selected system folders,
restricted to the startup filesystem (and macOS Data volume). It does not follow symlinks or cross
nested mounts, deduplicates hard links, and counts allocated blocks. Directory conventions identify
system files, applications, documents, development dependencies and build output, media, application
data, and other files. Build-directory names such as `target` and `dist` require a parent project
manifest; dependencies and tool stores retain their category throughout their descendants.

Scans run outside the live telemetry sampler, with a twenty-second traversal budget and a one-million
entry limit. Results remain in memory for five minutes; the card's rescan action bypasses this cache.
Permission failures and budget exhaustion yield explicitly partial estimates. APFS shared blocks,
snapshots, other users and unscanned folders can prevent category totals from matching disk usage;
Rust reports the unclassified remainder only when it can be calculated without a negative result.
These are file-based estimates, not macOS System Settings categories. Windows retains capacity
reporting and explicitly reports that category scanning is unavailable.

## Weather

Weather widgets accept a city, region-qualified place, or postal code. Rust resolves each saved
query through the [Open-Meteo Geocoding API][open-meteo-geocoding], then reads its forecast and
timezone from [Open-Meteo][open-meteo]. Each card shows a local clock and the next six hourly
forecasts; clocks advance locally without another weather request. One unresolved place remains a
card-local failure and does not discard other weather cards.

## Service status

Service-status widgets store a Rust-validated catalog ID for GitHub, Codex, or DeepSeek. Rust reads
public Statuspage summaries for [GitHub][github-status], [OpenAI][openai-status], and
[DeepSeek][deepseek-status] with a fifteen-second timeout and bounded concurrency. GitHub and
DeepSeek include non-group components and page-wide active incidents. Codex includes only components
whose names identify Codex and unresolved incidents linked to their IDs, including monitoring
incidents after a component recovers.

The projection contains overall health, the operational percentage, active-incident count, and the
names and states of affected components, including maintenance and unknown states. Overall health
uses the most severe matching component; the percentage measures current component health rather
than historical uptime. Svelte renders this projection without reclassifying provider responses.

Reads run on Dashboard entry, explicit refresh, and the sixty-second polling interval. An endpoint
failure remains local to its configured card.

## Stocks

Each stock widget stores a validated ticker symbol. Rust reads its recent daily closes from Yahoo
Finance's chart endpoint with bounded concurrency and no credentials. One failed symbol remains a
card-local error and does not discard successful quotes. Stocks refresh on Dashboard entry or an
explicit refresh and have no timer.

## Exchange rates

The shared `quotes::exchange` boundary reads the latest two working days of official ECB euro
reference rates for EUR, USD, CNY, GBP, JPY, CHF, HKD, SGD, CAD, and AUD. It exposes each currency as
units per euro, the daily change, and cross-rate conversion without requesting another provider.
These are daily reference rates rather than live trading quotes. The Add Widget library registers
one optional singleton exchange card. It shows USD/CNY, GBP/CNY, and EUR/CNY cross rates with their
change between the latest two ECB working days. Exchange rates refresh on Dashboard entry or an
explicit refresh and have no timer. Vesper does not request ECB data when the current layout has no
exchange card.

## Random quotation

`quotes::quotations` reads one quotation from [FreeAPI][freeapi] and narrows the response to the
fields rendered by the card. The optional singleton widget reads this source on Dashboard entry and
explicit refresh only.

## GitHub

Dashboard uses the locally installed, authenticated GitHub CLI. The Rust `quotes::github` boundary
runs contribution and notification requests concurrently with a fifteen-second timeout per request.
It returns typed projections without credentials or raw provider errors. `GITHUB_CLI_BINARY` can
override CLI discovery; otherwise Vesper searches `PATH` and the user's login shell. Users sign in
through `gh auth login` outside Vesper.

GraphQL supplies the contribution calendar and recent commit, pull-request and review contributions.
Rust distinguishes approved reviews, merges activity kinds, sorts by occurrence time and returns
the latest three entries. The calendar initially scrolls to the newest dates and remains aligned to
the right when resized; users can scroll left to inspect history.

A separate REST notifications request reads up to twenty latest unread threads for the same GitHub
account. The card displays the notification reason, subject, repository and update date, highlighting
review requests. A twenty-first result indicates that more notifications are available; the displayed
count is not a claimed account-wide total. Known issue, pull-request and commit subjects link directly
to GitHub; other subjects remain visible and can be accessed through Open inbox. Reads do not mark
threads as read or change review assignments. The API requires `notifications` or `repo` scope; a
failure stays within the Notifications section without hiding contributions and activity.

GitHub refreshes on Dashboard entry and explicit refresh, without background polling. Neither
contribution activity nor GitHub notifications are persisted in the local ntfy Inbox.

## Daily check-in

Daily Planner displays multiple named habits beside Calendar and Todo. Manage accepts names
separated by commas or newlines and removes individual habits. Each row provides an icon action
for the selected date's check-in or undo, an ongoing streak, total checked-in days through that
date, and the 28 days ending on it. Calendar, Todo, and habits share one Planner `selectedDate`.
Historical dates allow check-in and undo; future dates are read-only. Rust validates the future-date
restriction inside the write transaction. A write keeps its submitted date even if midnight or
calendar navigation occurs before it finishes. An unfinished selected day retains the previous
day's streak.

Records are local in `check_ins`, keyed by habit ID and date. Layout changes and restarts preserve
history; removing and re-adding a habit creates a new ID. No credentials or providers are involved.
The panel reads all displayed habits on mount, focus, once per minute, and cross-window updates.
Each write targets one habit and date. Responses carry habit IDs, so rows never depend on response
order. Pending writes defer reads; changing dates clears the old projection and invalidates pending
responses, then rereads the latest selection after any write settles. Automatic reads preserve write
errors until another write or a change of habit/date.

## Spending

Spending is a singleton in Personal, offered in the widget library and included in new/default
layouts after Daily Planner. Existing saved layouts can add it through Edit dashboard → Add widget.
Daily Planner's Calendar and Spending share the selected date. Spending offers previous/next-day
buttons and Today around a text date, without a second calendar or native date picker. Its charts
remain read-only and there is no refresh button. The Dynamic Island can pin Spending in a stacked
layout with its own WebView date; it does not start Dashboard polling. Date changes, successful
writes, expense events, focus, and the existing minute timer update data.

Quick entry requires only a positive GBP amount and a category; the selected date is implicit.
An optional note (up to 500 characters) can be saved, edited, and viewed below the category.
Amounts range from £0.01 to £999,999.99 with at most two decimal places. The initial suggestions are
Coffee, Subscriptions, Eating out, Groceries, Transport, Shopping, and Other; custom categories are
accepted through the Custom category option and become suggestions. The category dropdown uses
the shared UI Select component rather than a native select or datalist. Case and repeated whitespace do not create duplicate categories.
This is an expense ledger, with no income, conversion, bank connection, or automatic import.
Entries can be edited or deleted. A successful write updates the list, selected-day total, monthly
total, category donut, and daily bars together. The charts always cover the selected date's calendar
month. Categories and Daily spending switch within one fixed-height chart region; changing chart
view does not reread data or change the entry draft. Empty months show an explicit empty state and zero daily bars. Amount/category/note drafts are
preserved on failed writes and when edited during saving; changing the selected date starts a new
form for that date.

Reads run on mount, selected-date changes, focus, once per minute while mounted, and cross-window
expense events for the displayed month. Pending writes defer reads; request generations prevent
old dates/months from replacing the current projection. Failed background reads preserve settled
data, and write errors survive automatic refreshes until another write or date change. Entries
remain in local SQLite when the widget is removed. No credentials or provider settings are needed.

## Calendar and Todo

Each Todo has a default-off carry-forward checkbox with small explanatory text at the bottom of its details. Rust moves checked unfinished tasks
to the actual local day, repeating daily until completion or opt-out; reopening after several days
catches up directly. The local-date loop checks every thirty seconds and Todo reads also catch up.
Every changed date emits `todo-updated`, including past/future Notion projections removed during
consolidation. The transaction preserves existing destination order and rejects/rolls back failed
writes. Notion multi-day events become one local follow-up retaining read-only original metadata;
source calendars are never modified, and refresh cannot recreate that event from its source day onward.
If another dated projection is already complete, it remains on its original date and no follow-up is
created; subsequent source refreshes preserve that completed history.

Todo drag and keyboard ordering is local to a date, persists in SQLite, and emits `todo-updated`
to the other surface. Notion refresh preserves positions of surviving tasks and appends new tasks;
local ordering never writes to the source calendar. Concurrent membership changes reject a stale
reorder atomically, leaving the saved list intact and showing an error for retry after refresh.

Daily Planner has one selected date shared by tasks and habits. Calendar renders a complete Sunday-first month; Todo
creates, edits, completes, reopens, and deletes that day's items. Quick add accepts a title and an
expandable description. Manual-task editing preserves date and completion while changing title
and description. Imported items retain source-owned content. Titles are limited to 120 characters
and manual descriptions to 4,000. Selecting a title opens a fixed-size detail view with date,
status, description, and available calendar/time/location metadata. Long details scroll internally;
Back restores the list without changing the Planner layout.

Svelte retains settled data during reads and accepts a response only for the current request and
selected date. Reads requested during a write are coalesced and run after it commits, retaining an
explicit refresh request. Write errors survive automatic reads until the next write or date change.
Rust checks the local date every thirty seconds and emits `planner-date-changed` independently of
ICS, Notion, or SQLite reads. A view following today advances; an explicitly selected historical or
future day remains selected. Planner refresh, active-window focus, and island expansion also read
the local date, recovering missed events and sleep/time-zone changes. A failed task read cannot
hold back the Calendar or habit date. Cross-window Todo mutations invalidate the other surface's
list after its current operation finishes. Calendar navigation remains available during task reads
and writes; responses can only populate their own selected day.

Rust stores tasks and occurrence keys in SQLite. The `ics/` directory remains the source for local
calendar files. Imports validate all files before installing each through atomic replacement.
Recurrence keys prevent duplicate tasks; deleting an imported occurrence suppresses it. Floating
times remain local, and UTC/IANA TZID values are projected into the device time zone. Unsupported
recurrence semantics produce an explicit error. Desktop and CLI share the store constructor and
retain the roaming ICS directory on Windows.

Notion integration uses a view link saved in Settings and the official `ntn` CLI, authenticated with
`ntn login`. Vesper stores only the link and never reads CLI tokens. Calendar views must select a
Date property; other view types work when their data source has exactly one Date property. Formula
and creation-time properties are unsupported. Rust retrieves the view, queries its saved filters
and sorting, paginates page references, and queries data-source pages until all view members are
found. Empty views need no data-source query. Incomplete membership, pagination errors, or timeouts
fail the synchronization instead of installing a partial projection.

A successful read installs one complete in-memory view snapshot for five minutes. Switching dates,
reentering Dashboard, and expanding the island reuse that snapshot; all-day ranges and zoned dates
are projected onto the selected local date. Explicit Dashboard refresh and CLI synchronization
bypass it. Configuration saves clear the cache, and another view URL cannot reuse its contents.
The initial read may scan many data-source pages for sparse views and remains subject to the
90-second query budget. Each CLI process has a twenty-second deadline and is killed on cancellation.

Calendar reads and configuration writes share a cross-process lock. A queued SQLite commit retains
that lock even if its requester is cancelled, preventing an old view's commit after a completed
configuration save. Failed reads preserve saved tasks and expose the synchronization error beside
them. Notion page IDs preserve local completion across refreshes; successful reconciliation updates
titles and removes entries no longer present for the day. Local completion and deletion never
mutate Notion. Clearing the view link disconnects the calendar.

Codex Resets uses a separate enable switch next to Notion in Settings. The fixed public endpoint
`https://codex-resets.com/api/v1/resets` requires no login or API key. Rust queries the selected
local day's UTC bounds, follows cursor pagination, and maps announcements to local date/time.
Regular resets and banked credits have distinct titles; details retain the announcement and source
URL. These are public announcements, not personal quota schedules; forecasts are not imported.

Successful dated reads are cached for five minutes (up to 32 dates); explicit refresh bypasses the
cache. Disabling the source clears the cache and removes its projection as dates are read, without
removing manual or Notion tasks. Reads use bounded HTTP operations and incomplete/failed responses
retain the previous source projection with a sync error. Source IDs preserve local completion,
ordering, deletion and opt-in carry-forward across refreshes, using the same reconciliation and
configuration lock as Notion. Completion and deletion only affect Vesper. The enable preference is
stored through the same typed build-specific configuration backend as the Notion view link.

## UGOS Pro

Remote NAS reads require both the active Dashboard route and at least one saved UGREEN widget.
Current Device widgets do not enable NAS requests; startup and credential saves on other routes do
not poll it. CPU, memory, and network use bounded in-memory histories; storage is a capacity
snapshot. [UGOS.md](UGOS.md) owns connection, certificate trust, login, metric fields, and failure
behavior. The UGOS error boundary strips request URLs because their query strings contain session
tokens; logs and frontend errors retain the error category without the token.

## AI usage providers

The crate is named `useage` by project decision. Each module owns one provider's transport and
response types; callers only expose its typed result.

| Module        | Source                                           | Credential resolution                                                | Values shown                                         |
| ------------- | ------------------------------------------------ | -------------------------------------------------------------------- | ---------------------------------------------------- |
| `codex.rs`    | Local `codex app-server --stdio` JSON-RPC        | Existing `codex login`; optional `CODEX_BINARY` path override        | Plan, default limits, and GPT-5.3 Codex Spark limits |
| `claude.rs`   | Anthropic OAuth usage endpoint                   | Existing Claude Code OAuth session                                   | Five-hour and seven-day subscription windows         |
| `copilot.rs`  | GitHub `GET /copilot_internal/user` via `gh api` | Existing `gh auth login`; optional `GITHUB_CLI_BINARY` path override | Chat, completions, and premium-request quotas        |
| `grok.rs`     | Authenticated Grok runtime billing JSON-RPC      | Existing Grok device login; optional `GROK_BINARY` path override     | Current subscription window                          |
| `opencode.rs` | `https://opencode.ai/zen/go/v1/usage`            | pi auth entry `opencode-go`                                          | Rolling, weekly, and monthly Go-plan windows         |
| `deepseek.rs` | `https://api.deepseek.com/user/balance`          | pi auth entry `deepseek`                                             | Availability and currency balances                   |
| `cherryin.rs` | CherryIN OAuth balance endpoint                  | Cherry Studio OAuth session                                          | Account balance shown under Cherry                   |

Claude, Copilot, and Grok are independent AI Services widgets and Dashboard sources in addition to their
CLI status checks. Claude reuses Claude Code's OAuth session and reads the five-hour and seven-day
subscription windows. Debug builds read its local credential file only; macOS release builds may
also read the existing Claude Code Keychain item. Copilot reuses the authenticated GitHub CLI and reads the same typed user and
quota snapshot consumed by the official Copilot CLI, including unlimited flags and the account-level
reset date. The Dashboard omits unlimited Chat and Completions rows and presents the metered Premium
Requests quota; a zero row-level reset timestamp falls back to the account reset date. Grok launches
the authenticated official runtime and reads its private billing snapshot.

OpenCode Go and CherryIN are separate integrations. Codex, Grok and Copilot use five-minute
in-memory caches with request coalescing; both success and failure are cached to avoid repeated
requests against private interfaces. Cancelled reads release the gate without caching unfinished
results. External CLI account changes become visible after the cache expires.
Supported REST reads, including DeepSeek and Steam, request the API on refresh. Each provider owns
its request construction and timeout.

### API-key resolution

For API-key-backed providers, credential resolution is:

1. The provider entry in `${PI_CODING_AGENT_DIR}/auth.json`, or `~/.pi/agent/auth.json` when the
   override is absent.
2. The provider entry in the matching pi `models.json` for custom model providers.

Provider identifiers are matched without case sensitivity. Pi auth entries must use type `api_key`;
custom model providers must contain a non-empty `apiKey`. Secrets are passed in a Bearer header and
are never serialized to Svelte, application files, or logs.

### Codex

The Codex integration starts the locally installed CLI as `codex app-server --stdio`, performs the
JSON-RPC initialization handshake with `experimentalApi`, then calls `account/rateLimits/read`. It
uses the CLI's existing authenticated session and terminates the child process after reading the
response. Protocol I/O has a fifteen-second timeout. The backward-compatible `rateLimits` bucket
feeds the main Codex card. Spark is selected from `rateLimitsByLimitId` when its map key, limit ID, or
limit name identifies Spark, so accounts that do not receive a Spark bucket remain valid.

### OpenCode Go

The OpenCode integration reuses the `opencode-go` credential stored by pi.
The returned percentage is usage, so Dashboard renders remaining capacity as `100 - percent`. No new
provider configuration is written by Vesper.

### DeepSeek

DeepSeek uses the official [`GET /user/balance` endpoint][deepseek-balance] with Bearer
authentication. It returns decimal balances as strings, which remain strings across the
Rust/TypeScript boundary to preserve provider precision. Currency is displayed from the response
rather than inferred, except that the API's `CNY` code is labeled `RMB` in the card. Dashboard shows
only the total available account balance without a composition breakdown or chart.

### CherryIN

CherryIN requires an existing Cherry Studio sign-in. Vesper reads the `user_provider` OAuth
session from `CherryStudio/Data/cherrystudio.sqlite`. It refreshes expired access tokens or tokens
within sixty seconds of expiry through `/oauth2/token`, using the existing refresh token. A balance
request returning 401 forces one refresh and retry. Missing or rejected refresh tokens require
signing in again in Cherry Studio; Settings has no separate CherryIN login flow.

Successful refreshes conditionally replace the existing session only when its saved JSON still
matches the value read before refresh, preserving unrelated fields and concurrent account changes.
Vesper serializes its CherryIN reads within the process. It does not create the Cherry Studio
database. CherryIN has no five-minute result cache, so a manual retry rereads the session immediately.

Balance reads call `GET /api/v1/oauth/balance` and divide account quota by `500000` for USD display.
Model-token limits are not account balance.

## Adding a provider

1. Add one provider module below `crates/useage/src` and export it from `lib.rs`.
2. Keep endpoint constants, wire response types, parsing, request lifecycle, and errors in that file.
3. Reuse `auth::api_key` only when the provider uses a pi API-key record or custom model provider.
4. Add the provider to the Rust Dashboard source enum and unified refresh runtime.
5. Add the matching TypeScript event variant and an independent `QueryState` entry.
6. Add the provider as its own widget under AI Services without changing other
   providers' loading state or the Todo area.
7. Cover response parsing with a unit test. Keep authenticated network tests ignored and opt-in.
8. Document the credential identifier, endpoint ownership, units, and failure behavior here.

## Motion and feedback

Dashboard motion uses CSS animations and transitions only. Cards use a restrained lift, the entrance
sequence is staggered, and progress widths use a fast decelerating curve. These choices adapt the
micro-transition principles from the Amicro reference without adding its React or Motion
dependencies. All nonessential motion is disabled when the operating system requests reduced motion.

[deepseek-balance]: https://api-docs.deepseek.com/api/get-user-balance
[deepseek-status]: https://status.deepseek.com/api/v2/summary.json
[freeapi]: https://freeapi.app/
[github-status]: https://www.githubstatus.com/api/v2/summary.json
[open-meteo]: https://open-meteo.com/en/docs
[open-meteo-geocoding]: https://open-meteo.com/en/docs/geocoding-api
[openai-status]: https://status.openai.com/api/v2/summary.json

## Component ownership

Dashboard and Dynamic Island share placement dispatch in `WidgetContent`. `PlannerPanel` owns the
Calendar/Todo/habit composition; `TelemetryPanel` owns metric labels and charts. Usage and telemetry
receive only the selected source’s typed state. Source events share the same loading/error update
policy and preserve settled data after failed reads; each provider remains independent.
