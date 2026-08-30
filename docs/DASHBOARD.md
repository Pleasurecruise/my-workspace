# Dashboard Integrations

Dashboard is a local aggregation surface that does not mutate provider account data. Protocol code,
request ordering, and polling live in Rust. Each external source has an independent request revision
and result event. CherryIN token refresh is the narrow exception to local read-only credential
access: a successful refresh may update the existing Cherry Studio OAuth session.

## Data flow

```text
DashboardView.svelte
  <- typed state in App.svelte
  <- typed source events and refresh commands
  <- dashboard runtime in apps/desktop/src-tauri
  ├─ crates/ugos
  ├─ apps/desktop/src-tauri/weather.rs
  ├─ apps/desktop/src-tauri/stocks.rs
  ├─ apps/desktop/src-tauri/github.rs
  └─ crates/useage
       ├─ codex.rs
       ├─ opencode.rs
       ├─ deepseek.rs
       └─ cherryin.rs
```

An unavailable credential or failed source does not block the other cards. Rust starts unified
Dashboard reads concurrently and emits each result as it settles. A per-source lock prevents
overlapping reads; scheduled refreshes skip a source that is still running, while an explicit refresh
waits for that source and then obtains fresh data. Polling exists only while Dashboard is active and
retains settled data while refreshing: UGOS telemetry runs every two seconds and subscription data
every sixty seconds. Entering Dashboard or using its refresh action reads every source and the
selected Todo date. Weather, stocks, and GitHub have no timer.

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
system-style library with search, widget metadata, and a
selected-widget preview. The canvas keeps the
same twelve-track arrangement at every window width; a narrow window scrolls horizontally and never
projects a different order or column count. The layout is validated and stored locally in
`dashboard-layout.json`; a missing file creates the default, while invalid stored data is reported
without a silent fallback. The stored document is exactly `{ widgets }`; placements contain only
their unique ID and typed widget configuration. Unknown fields are errors, and there is no layout
version or compatibility reader. Existing card-level narrow-screen breakpoints remain intact.

The Usage widget contains Codex and OpenCode Go quota cells above DeepSeek and Cherry balance cells.
Weather widgets accept a city, region-qualified place, or postal code. Rust resolves each saved
query through the [Open-Meteo Geocoding API][open-meteo-geocoding], then reads its forecast and
timezone from [Open-Meteo][open-meteo]. Each card shows a local clock and the next six hourly
forecasts; clocks advance locally without another weather request. One unresolved place remains a
card-local failure and does not discard other weather cards.

## Stocks

Each stock widget stores a validated ticker symbol. Rust reads its recent daily closes from Yahoo
Finance's chart endpoint with bounded concurrency and no credentials. One failed symbol remains a
card-local error and does not discard successful quotes. Stocks refresh on Dashboard entry or an
explicit refresh and have no timer.

## GitHub

Dashboard uses the locally installed, authenticated GitHub CLI rather than storing a GitHub token.
The Rust `github.rs` boundary starts `gh api graphql`, applies a fifteen-second timeout, parses the
typed response, and returns no credentials or raw provider errors to Svelte. `GITHUB_CLI_BINARY` can
override CLI discovery; otherwise Vesper searches `PATH` and the user's login shell. Users must run
`gh auth login` outside Vesper before this card can load.

One GraphQL request loads the viewer's contribution calendar and recent commit, pull-request, and
pull-request-review contributions. The calendar renders the last year as semantic success-color
tiles. Rust maps an approved review to `approve`, other review states to `review`, merges those with
pull requests and repository commit groups, sorts by occurrence time, and exposes only the latest
three activities. GitHub participates in the unified refresh on the first Dashboard load, every
later entry into Dashboard, and the explicit refresh action; it does not poll in the background.

## Calendar Todo list

The Todo card shows one complete month with Monday-first weekday columns, previous and next month
controls, a marker for the current day, and a distinct selected date. Selecting a date reads its own
list; adding, completing, reopening, and deleting items all apply to that selected date. Settled data
is replaced only when the selected date's response arrives, so rapid date changes cannot display an
older request as the current list.

The shared Rust `cms_core::todo` module stores date-keyed lists in `todos.json` below the application
data directory for `me.you-find.vesper`. If the new file is absent, the previous single-day
`today-todos.json` file is ignored without fallback or migration. No SQL database or ORM is involved.
Desktop and `vesper todo` share the file through a sidecar lock. The CLI operates on the current
local date by default and accepts `vesper todo --date YYYY-MM-DD <action>` for another calendar day.
At local midnight Rust advances a view of today to the new empty list without deleting history.

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
response types; the Tauri layer only exposes the result to the frontend.

| Module        | Source                                    | Credential resolution                                         | Values shown                                         |
| ------------- | ----------------------------------------- | ------------------------------------------------------------- | ---------------------------------------------------- |
| `codex.rs`    | Local `codex app-server --stdio` JSON-RPC | Existing `codex login`; optional `CODEX_BINARY` path override | Plan, default limits, and GPT-5.3 Codex Spark limits |
| `opencode.rs` | `https://opencode.ai/zen/go/v1/usage`     | pi auth entry `opencode-go`                                   | Rolling, weekly, and monthly Go-plan windows         |
| `deepseek.rs` | `https://api.deepseek.com/user/balance`   | pi auth entry `deepseek`                                      | Availability and currency balances                   |
| `cherryin.rs` | CherryIN OAuth balance endpoint           | Cherry Studio `cherryin` OAuth session                        | Account balance shown under Cherry                   |

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
6. Add quota data to the upper row or balance data to the lower row without changing other
   providers' loading state or the lower-left Todo area.
7. Cover response parsing with a unit test. Keep authenticated network tests ignored and opt-in.
8. Document the credential identifier, endpoint ownership, units, and failure behavior here.

## Motion and feedback

Dashboard motion uses CSS animations and transitions only. Cards use a restrained lift, the entrance
sequence is staggered, and progress widths use a fast decelerating curve. These choices adapt the
micro-transition principles from the Amicro reference without adding its React or Motion
dependencies. All nonessential motion is disabled when the operating system requests reduced motion.

[deepseek-balance]: https://api-docs.deepseek.com/api/get-user-balance
[open-meteo]: https://open-meteo.com/en/docs
[open-meteo-geocoding]: https://open-meteo.com/en/docs/geocoding-api
