# Dashboard Integrations

Dashboard is a local aggregation surface that does not mutate provider account data. Protocol code,
request ordering, and polling live in Rust. Each external source has an independent request revision
and result event. CherryIN token refresh is the narrow exception to local read-only credential
access: a successful refresh may update the existing Cherry Studio OAuth session.

Telegram and X are outbound Memo publication providers, not Dashboard sources. Their configuration,
authorization, and token refresh paths remain outside the Dashboard runtime so its read-only provider
contract does not expand.

## Data flow

```text
DashboardView.svelte
  <- typed state in App.svelte
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

An unavailable credential or failed source does not block the other cards. Rust starts unified
Dashboard reads concurrently and emits each result as it settles. A per-source lock prevents
overlapping reads; scheduled refreshes skip a source that is still running, while an explicit refresh
waits for that source and then obtains fresh data. Polling exists only while Dashboard is active and
retains settled data while refreshing: UGREEN NAS telemetry and configured current-device telemetry
run every two seconds, while subscription data and configured service status run every sixty
seconds. Entering Dashboard or using its refresh action reads every source and the selected Todo
date. Weather, stocks, exchange rates, GitHub, and random quotations have no timer.

## ntfy notifications

Notification delivery is independent from Dashboard polling:

```text
Upstream producers ──> ntfy.you-find.me/mail-summary ── authenticated SSE ──> Vesper Inbox
```

- Vesper does not connect to or configure upstream producers.
- The transport is the self-hosted `https://ntfy.you-find.me` service. The current fixed topic is
  `mail-summary`; its ACL must grant the configured token read permission.
- Vesper subscribes in Rust, reconnects with ntfy's `since=<last-id>` behavior, and keeps the newest
  200 messages locally for Inbox rendering.

Dashboard cards occupy a user-configurable fixed desktop widget canvas. In edit mode, users drag a
card itself with the four-way move pointer to reorder it, delete it from the small upper-right
button, or restore the Rust-owned default; there is no separate drag handle or card-level component
menu. Reordering within one row targets individual cards, while crossing rows inserts at the row
boundary so a full-width card cannot split a populated row. The Add Widget action opens a
category-based library with widget metadata and a selected-widget preview. The canvas keeps the same
twelve-track arrangement at every window width; a narrow window scrolls horizontally and never
projects a different order or column count. The layout is validated and stored locally in
`dashboard-layout.json`; a missing file creates the default, while invalid stored data is reported
without a silent fallback. The stored document is exactly `{ widgets }`; placements contain only
their unique ID and typed widget configuration. Unknown fields are errors, and there is no layout
version. The compatibility projection replaces former combined `usage`, `quota`, and `balance`
placements with independent Codex, OpenCode Go, DeepSeek, and Cherry provider placements. Existing
card-level narrow-screen breakpoints remain intact.

The widget library uses a category rail without a search input. System Status contains both the
explicitly named UGREEN CPU, UGREEN Memory, UGREEN Storage, and UGREEN Network widgets and the
Device CPU, Device Memory, Device Storage, and Device Network widgets backed by local telemetry.
Quota contains separate Codex, OpenCode Go, Claude, Grok, and Copilot widgets. Balance contains
separate DeepSeek and Cherry widgets. Existing singleton widgets remain visible and are marked as
added instead of disappearing from the library.
Weather widgets accept a city, region-qualified place, or postal code. Rust resolves each saved
query through the [Open-Meteo Geocoding API][open-meteo-geocoding], then reads its forecast and
timezone from [Open-Meteo][open-meteo]. Each card shows a local clock and the next six hourly
forecasts; clocks advance locally without another weather request. One unresolved place remains a
card-local failure and does not discard other weather cards.

## Service status

Each service-status widget stores one Rust-validated catalog ID selected by name in the Add Widget
dialog. The initial catalog contains GitHub, Codex, and DeepSeek; arbitrary status URLs are not
accepted. Rust reads the public Statuspage summaries for [GitHub][github-status],
[OpenAI][openai-status], and [DeepSeek][deepseek-status] with a fifteen-second timeout and bounded
concurrency. GitHub and DeepSeek summarize their non-group components. Codex uses only OpenAI status
components whose names identify Codex, so an unrelated ChatGPT incident does not mark Codex
unavailable.

The card's progress bar is the current percentage of matching components reported operational; it
is not historical uptime. The most severe matching component determines the displayed state, and
the card also shows the number of active incidents returned by the status page. One failed endpoint
remains local to its configured card. Status reads run on Dashboard entry, explicit refresh, and the
sixty-second Dashboard polling interval.

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

Dashboard uses the locally installed, authenticated GitHub CLI rather than storing a GitHub token.
The Rust `quotes::github` boundary starts `gh api graphql`, applies a fifteen-second timeout, parses the
typed response, and returns no credentials or raw provider errors to Svelte. `GITHUB_CLI_BINARY` can
override CLI discovery; otherwise Vesper searches `PATH` and the user's login shell. Users must run
`gh auth login` outside Vesper before this card can load.

One GraphQL request loads the viewer's contribution calendar and recent commit, pull-request, and
pull-request-review contributions. The calendar renders the last year as semantic success-color
tiles. Rust maps an approved review to `approve`, other review states to `review`, merges those with
pull requests and repository commit groups, sorts by occurrence time, and exposes only the latest
three activities. GitHub participates in the unified refresh on the first Dashboard load, every
later entry into Dashboard, and the explicit refresh action; it does not poll in the background.

## Calendar and Todo

Calendar and Todo are independent widgets with one selected date. Calendar renders a complete
Monday-first month; Todo creates, completes, reopens, and deletes items for the selected day.
Selecting a title replaces the list with a fixed-size detail view showing status and date plus the
calendar, time, location, and description available on imported items. Long details scroll inside
the card, and Back restores the list without changing the dashboard layout.

Each date read has its own request revision. The view keeps settled data while loading and accepts a
response only if it still matches the selected date, preventing a slower earlier request from
replacing a newer selection. Stored legacy `todo` placements expand to separate Calendar and Todo
placements while decoding the dashboard layout.

Rust stores the date-keyed calendar in `todos.json`, shares it with `vesper todo` through a sidecar
lock, and ignores the former `today-todos.json` format. Reads also sync the optional sibling `ics`
directory. Floating DTSTART values remain local, while UTC and IANA TZID-qualified times are
converted to the device time zone before their date and `HH:MM` prefix are selected. The documented
RRULE subset and EXDATE are materialized once per source file, UID, and source occurrence date.
Invalid structure, unknown time zones, and unsupported recurrence fields fail the Todo read rather
than silently changing meaning. At local midnight a view still showing today advances and syncs the
new date without deleting history.

## UGOS Pro

### Connection and authentication

- Fixed address: `https://ugreen:9443` through Tailscale MagicDNS.
- UGOS clients bypass the operating-system HTTP proxy and connect directly to the Tailscale address.
- Required local configuration: UGOS username and password saved through Settings.
- On first connection, Vesper probes the NAS certificate and stores its SHA-256 fingerprint in the
  operating-system credential store.
- Later clients trust only the recorded fingerprint. Changing the NAS certificate requires an
  explicit credential-record update rather than silent trust replacement.
- The login client loads `/desktop/?os=ugospro` and extracts `window.clientNumberVersion` at runtime.
- The authenticated API root is `/ugreen/v1`.

The current implementation reads real-time CPU, memory, network, and volume samples from the
configured device. The current Task Manager response exposes the live values under the top-level
`cpu.series`, `mem.series`, and `net.series` fields; its `overview.cpu` and `overview.mem` values are
an initial summary and must not feed the trend lines. Network history selects the aggregate series
whose name is `overview`, rather than an individual interface. Vesper retains the latest 60 unique,
chronologically increasing server-timestamped samples in
memory for the CPU, memory, and network trend lines. The CPU chart renders usage and temperature as
independently scaled primary and secondary lines. Storage utilization is calculated from volume
`used` and `total` capacity and uses a capacity bar because it is a slow-changing snapshot. A missing
or zero total volume capacity produces no storage sample instead of a misleading 0%. The history is
not persisted. It does not currently query processes, services, fan data, machine identity, or
firmware information.

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
| `cherryin.rs` | CherryIN OAuth balance endpoint                  | Cherry Studio `cherryin` OAuth session                               | Account balance shown under Cherry                   |

Claude, Copilot, and Grok are independent Quota widgets and Dashboard sources in addition to their
CLI status checks. Claude reuses Claude Code's OAuth session and reads the five-hour and seven-day
subscription windows. Copilot reuses the authenticated GitHub CLI and reads the same typed user and
quota snapshot consumed by the official Copilot CLI, including unlimited flags and the account-level
reset date. The Dashboard omits unlimited Chat and Completions rows and presents the metered Premium
Requests quota; a zero row-level reset timestamp falls back to the account reset date. Grok launches
the authenticated official runtime and reads its private billing snapshot.

Vesper does not create or register an OpenCode provider named `cherry-opencode-go`. OpenCode Go and
CherryIN are separate integrations. Vesper reads OpenCode Go from pi. It reuses CherryIN's existing
OAuth session from Cherry Studio and only updates that session when an access-token refresh succeeds.
Each provider owns its request construction and timeout so one integration cannot change another
provider's connection policy.

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

Dashboard follows Cherry Studio's CherryIN integration: it reads the existing `cherryin` OAuth access
and refresh tokens from Cherry Studio's `Data/cherrystudio.sqlite`, calls `/api/v1/oauth/balance`, and
converts the returned account `quota` with CherryIN's `500000` quota unit. An access token that is
expired or within sixty seconds of expiry is refreshed through `/oauth2/token`; a balance request
that returns `401` forces one refresh and retry. Refreshed tokens are conditionally written back only
when the stored refresh token still identifies the same session, so a concurrent Cherry Studio
logout or login is not overwritten. Vesper serializes CherryIN reads so two Dashboard refreshes do
not rotate the same refresh token concurrently. If the refresh token is absent or rejected, Vesper
asks the user to sign in again in Cherry Studio. It never uses pi's model token, `/api/usage/token/`,
or the billing subscription endpoints, so an unlimited model token cannot be mistaken for account
balance.
The resulting balance is displayed as US dollars with an explicit `USD` label.

## Adding a provider

1. Add one provider module below `crates/useage/src` and export it from `lib.rs`.
2. Keep endpoint constants, wire response types, parsing, request lifecycle, and errors in that file.
3. Reuse `auth::api_key` only when the provider uses a pi API-key record or custom model provider.
4. Add the provider to the Rust Dashboard source enum and unified refresh runtime.
5. Add the matching TypeScript event variant and an independent `QueryState` entry.
6. Add the provider as its own widget under Quota or Balance without changing other
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
