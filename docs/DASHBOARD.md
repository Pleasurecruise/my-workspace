# Dashboard Integrations

Dashboard is a local, read-only aggregation surface. Protocol code lives in Rust and each external
source has an independent query state, error, refresh lifecycle, and polling interval.

## Data flow

```text
DashboardView.svelte
  <- typed state in App.svelte
  <- Tauri commands in apps/desktop/src-tauri
  ├─ crates/ugos
  ├─ apps/desktop/src-tauri/weather.rs
  ├─ apps/desktop/src-tauri/github.rs
  └─ crates/useage
       ├─ codex.rs
       ├─ opencode.rs
       ├─ deepseek.rs
       └─ cherryin.rs
```

An unavailable credential or failed source does not block the other cards. Initial requests run
independently. Polling retains settled data while refreshing; the explicit spinner is reserved for a
user-requested dashboard refresh. UGOS telemetry polls every two seconds, subscription data every
sixty seconds, and weather every fifteen minutes while Dashboard is selected.

The calendar Todo list occupies the narrower lower-left area. One consolidated panel occupies the wider lower-right area.
Its first row contains Codex and OpenCode Go quota cells; its second row contains DeepSeek and Cherry
balance cells. Codex keeps its default windows and GPT-5.3 Codex Spark in one cell with three progress
bars. DeepSeek and Cherry display account balances as text without donut charts. The lower columns stay side
by side until the viewport reaches the narrow-screen breakpoint.

Weather and local time appear together above the Todo and usage region for Shanghai
(`31.2304, 121.4737`), Ningbo (`29.8683, 121.5440`), and Nottingham
(`52.9548, -1.1581`). Each city uses the timezone returned by Open-Meteo for a 24-hour clock and
shows the next six hourly temperature and weather-code forecasts. Forecasts come from
[Open-Meteo][open-meteo] without an API key. The UI advances all clocks locally every second rather
than polling the weather service for time.

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
three activities. GitHub refreshes on the first Dashboard load, every later entry into Dashboard,
and an explicit Dashboard refresh; it does not poll in the background.

## Calendar Todo list

The Todo card shows one complete month with Monday-first weekday columns, previous and next month
controls, a marker for the current day, and a distinct selected date. Selecting a date reads its own
list; adding, completing, reopening, and deleting items all apply to that selected date. Settled data
is replaced only when the selected date's response arrives, so rapid date changes cannot display an
older request as the current list.

The shared Rust `cms_core::todo` module stores date-keyed lists in `todos.json` below the application
data directory for `me.you-find.vesper`. If the new file is absent, the previous single-day
`today-todos.json` file is ignored without fallback or migration. No SQL database or ORM is involved.
Desktop and `vesper todo` share the file through a sidecar lock, while the CLI operates on the current
local date. At local midnight Rust advances a view of today to the new empty list without deleting
history.

## UGOS Pro

### Connection and authentication

- Fixed address: `https://ugreen:9443` through Tailscale MagicDNS.
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
CherryIN are separate integrations. Vesper reads OpenCode Go from pi and reads CherryIN's existing
OAuth session from Cherry Studio without modifying either credential store.

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
token from Cherry Studio's `Data/cherrystudio.sqlite`, calls `/api/v1/oauth/balance`, and converts the
returned account `quota` with CherryIN's `500000` quota unit. The database is opened read-only. If
the access token has expired, Vesper asks the user to renew the session in Cherry Studio. It never
uses pi's model token, `/api/usage/token/`, or the billing subscription endpoints, so an unlimited
model token cannot be mistaken for account balance.
The resulting balance is displayed as US dollars with an explicit `USD` label.

## Adding a provider

1. Add one provider module below `crates/useage/src` and export it from `lib.rs`.
2. Keep endpoint constants, wire response types, parsing, request lifecycle, and errors in that file.
3. Reuse `auth::api_key` only when the provider uses a pi API-key record or custom model provider.
4. Add one narrow Tauri command returning the provider's public Rust response type.
5. Add the matching TypeScript transport interface and an independent `QueryState` entry.
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
