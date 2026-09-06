# UGOS Pro Implementation

UGOS supplies read-only NAS telemetry to the UGREEN Dashboard widgets. It is separate from local
Device widgets, which use desktop `sysinfo` telemetry. Vesper does not implement NAS administration,
file management, process control, or a cloud relay.

## Components and data flow

```text
DashboardView UGREEN widgets
  <- components/dashboard/session.svelte.ts: typed state and source events
  <- desktop Dashboard runtime: active route, saved widgets, polling, cancellation
  <- ugos::task_manager(): authenticated snapshot and in-memory histories
  <- https://ugreen:9443/ugreen/v1 over Tailscale MagicDNS
```

| Source                                                                                | Responsibility                                                                              |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| [DashboardView.svelte](../apps/desktop/src/lib/components/pages/DashboardView.svelte) | CPU, memory, network trends and storage utilization presentation                            |
| [dashboard.rs](../apps/desktop/src-tauri/src/dashboard.rs)                            | Source scheduling, independent result events, request serialization, and route cancellation |
| [lib.rs](../crates/ugos/src/lib.rs)                                                   | Credentials, cached authenticated client, snapshot projection, and bounded histories        |
| [client.rs](../crates/ugos/src/client.rs)                                             | Desktop version discovery and authenticated GET requests                                    |
| [auth.rs](../crates/ugos/src/auth.rs)                                                 | Verification request, RSA password encryption, and login                                    |
| [tls.rs](../crates/ugos/src/tls.rs)                                                   | Certificate probing, fingerprint verification, and direct HTTP client construction          |
| [types.rs](../crates/ugos/src/types.rs)                                               | NAS response envelopes and typed Task Manager/volume fields                                 |

Desktop requests require both an active Dashboard route and at least one saved UGREEN widget.
Startup or credential saves on another route do not start NAS polling. Polling runs every two
seconds while enabled; leaving Dashboard cancels queued and in-flight Dashboard reads. Each source
settles independently so NAS failure does not block unrelated Dashboard cards. The CLI can also
request the same Rust snapshot explicitly through its status command, outside the desktop route gate.

## Connection and login

1. Resolve the configured UGOS username and password through `crates/credentials`.
2. Connect directly to the fixed host `ugreen`, port `9443`. Both certificate probing and API clients
   use `no_proxy()` and a thirty-second HTTP request timeout.
3. Debug probes the current certificate whenever it creates a new client, then pins that fingerprint
   for the client. Release loads the stored SHA-256 fingerprint, or probes and saves it on first use.
   Release does not silently replace a stored fingerprint when the NAS certificate changes.
4. Fetch `/desktop/?os=ugospro` and parse `window.clientNumberVersion`. The NAS frontend supplies
   the client version; the code does not hard-code a firmware version.
5. POST the username to `/ugreen/v1/verify/check`. Decode its Base64 `x-rsa-token` header into the
   RSA public key. The parser accepts PKCS#1, SPKI, and the observed mislabeled SPKI form.
6. Encrypt the password using RSA PKCS#1 padding and Base64-encode the ciphertext. POST it to
   `/ugreen/v1/verify/login` with the client/device/version headers and login flags.
7. Keep the returned token in the Rust client. Subsequent GETs send it as the API's query parameter,
   together with `UG-Agent`, `Client-Id`, `Client-Version`, and language/cache headers.

The cached client is reused until configuration resets it or the application process exits.
`configure()` saves credentials and clears the client and histories. The current GET implementation
reports API/authentication failure without an automatic re-login/retry loop.

Certificate trust is fingerprint-based first-use trust, not ordinary public-CA/hostname validation.
The custom verifier checks the leaf fingerprint and explicitly verifies TLS 1.3 handshake signatures;
its current TLS 1.2 signature callback returns acceptance. This describes the existing code boundary,
not an assertion that the custom verifier is equivalent to standard Web PKI validation.

## Snapshot projection

The primary request is `taskmgr/stat/get_all`. If its `vol` array is empty, Rust requests
`storage/volume/list?start=0&size=50` as a fallback.

| UI metric | Authoritative response fields         | Projection                                          |
| --------- | ------------------------------------- | --------------------------------------------------- |
| CPU       | `cpu.series`                          | Usage percentage, temperature, and server timestamp |
| Memory    | `mem.series`                          | Usage percentage and server timestamp               |
| Network   | `net.series` entries named `overview` | Aggregate receive/send rates and server timestamp   |
| Storage   | Volume `used` and `total`             | Sum used / sum total × 100, clamped to 0–100        |

The initial `overview.cpu` and `overview.mem` summary does not feed live trends. Individual network
interfaces are not substituted for the aggregate overview series. Missing or zero total volume
capacity yields no storage sample instead of 0% usage.

CPU, memory, and network histories retain at most 60 points each. An older timestamp is ignored;
an equal latest timestamp replaces the last sample; a newer sample appends, dropping the oldest
when full. Storage is a current snapshot with local observation time rather than a trend history.
Svelte draws CPU usage and temperature as separate scales, network send/receive lines, and a storage
capacity bar. Raw login credentials and tokens are not part of the snapshot contract.

## Persistence and failures

Username/password follow the shared credential policy. Release persists the certificate fingerprint
through the credential store; debug uses its newly observed fingerprint without that stored-pin path.
Client token, cookies, and all sample histories remain in memory. There is no telemetry database or
background history archive. See [PERSISTENCE.md](PERSISTENCE.md).

Errors distinguish missing credentials, HTTP failures, envelope decoding, encryption, and NAS API
errors. Configuration resets histories, while ordinary reads do not fabricate zero samples to hide a
failure. Compatibility currently covers the observed Task Manager and volume payloads; processes,
services, fans, machine identity, and firmware inventory are outside this implementation.

## Verification and limits

[Unit fixtures](../crates/ugos/tests/unit) cover version parsing, RSA encodings, API envelopes,
volume/sample decoding, and TLS helpers; the crate also tests history ordering. Desktop
[UGOS layout tests](../apps/desktop/src-tauri/tests/unit/ugos_layout.rs) exercise widget gating.
Use `cargo test -p ugos` plus the relevant desktop tests when changing this path. Unit fixtures do
not establish compatibility with another firmware or NAS certificate. Live NAS and certificate compatibility require explicit device verification.
