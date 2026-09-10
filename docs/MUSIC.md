# Music Implementation

Music is a local player with two collections: Spotify Liked Songs and QQ Music Daily 30. Svelte
renders the library, controls, artwork, and lyrics; Rust owns authorization, provider requests,
queue order, decoding, and audio output. There is no music proxy server in this workspace.

## Components and data flow

```text
SettingsView -> settings/session.svelte.ts ──┐
MusicView ──────────────────────────────────┤
  -> typed Tauri commands in desktop music.rs
  -> MusicState: lazily initialized Spotify and QQ runtimes
  -> crates/music: provider APIs, lyrics, and local audio
  -> crates/credentials: renewable provider credentials
```

| Source                                                                              | Responsibility                                                                                                    |
| ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| [MusicView.svelte](../apps/desktop/src/lib/components/pages/MusicView.svelte)       | Provider selection, collection, playback controls, progress, and lyric presentation                               |
| [SettingsView.svelte](../apps/desktop/src/lib/components/pages/SettingsView.svelte) | Credential forms and QQ QR dialog; settings/session.svelte.ts owns login command callbacks                        |
| [Desktop music.rs](../apps/desktop/src-tauri/src/music.rs)                          | Typed commands, lazy runtime ownership, login cancellation, and pausing the other provider when starting playback |
| [spotify/mod.rs](../crates/music/src/spotify/mod.rs)                                | Liked Songs, token refresh, track/cover projections, and player access                                            |
| [spotify/auth.rs](../crates/music/src/spotify/auth.rs)                              | PKCE browser grants, loopback callback validation, and token exchange                                             |
| [spotify/player.rs](../crates/music/src/spotify/player.rs)                          | librespot session, audio output, playback events, and queue advancement                                           |
| [qq/mod.rs](../crates/music/src/qq/mod.rs)                                          | Daily 30 discovery, session renewal, media and lyric requests, and queue advancement                              |
| [qq/audio.rs](../crates/music/src/qq/audio.rs)                                      | Rodio worker ownership, cancellation, decoder seeks, and the loaded audio snapshot                                |
| [qq/qr.rs](../crates/music/src/qq/qr.rs)                                            | QQ QR creation, polling, redirect validation, and authorization exchange                                          |
| [lyrics.rs](../crates/music/src/lyrics.rs)                                          | LRCLIB lookup and shared LRC parsing                                                                              |

The transport exposes `Track`, `Playback`, `PlaybackOrder`, and `Lyrics`. Track IDs and cover keys
refer to the selected provider's loaded collection; UI code does not choose arbitrary media URLs.
Commands cover reading tracks/playback/lyrics, playing, pausing, resuming, seeking, and changing
sequential, repeat-one, or shuffle order. Rust returns expected failures through the tagged command
response and removes request URLs from transport errors to keep login and media query credentials
out of the UI and logs.

## Spotify lifecycle

### Authorization

Settings connects the Web API and local playback through two sequential PKCE grants. The Web API
uses the shared application by default or the personal Client ID entered in Settings. Playback uses
its independent application identity and requires Spotify Premium. A successful library read does
not establish playback eligibility. Each authorization checks callback state and has a ten-minute
deadline.

The credential record stores both refresh grants and the optional Web API Client ID. Records without
that ID use shared access. Token refresh uses the application identity belonging to the saved grant,
runs under credential synchronization, and renews before expiry. Access tokens stay in Rust.
Configuration steps and proxy behavior are documented in [DEVELOPMENT.md](DEVELOPMENT.md#spotify-music-configuration).

### Library reads and rate limits

Liked Songs reads `/v1/me/tracks` sequentially in pages of 50. Rust maps tracks and artwork URLs to
provider-owned IDs and cover keys, then replaces the collection only after pagination succeeds.
A failed read leaves the settled collection intact. Completed pages remain available for retry at
the failed offset until fifteen minutes after that refresh started; an expired partial refresh
restarts from the first page.

A refresh mutex serializes pagination and shares the Spotify runtime's cooldown among callers.
HTTP 429 records the complete integer `Retry-After` duration. Missing, invalid, or zero values use
one second. A library read can automatically retry at most three times, each after a wait of at
most thirty seconds. Longer waits return an error with the remaining delay and preserve the full
cooldown. An explicit `QUOTA_EXCEEDED` response returns a quota error without automatic retry.

The error notice offers Retry library. Requests made during cooldown return the remaining delay
without contacting Spotify. Rate-limit logs contain the pagination offset, wait duration, and quota
classification; response bodies and tokens never reach logs or the UI. Library progress and
cooldowns live only in the runtime and end when it is replaced or the application exits.

### Playback and lyrics

Playback creates the local librespot player on demand. Loads are matched to librespot request IDs
in command order, so delayed events from an earlier load cannot change the selected song, including
when the same song is selected twice. Position and seek events preserve pause state; unavailable
tracks surface a playback error. End events advance only while playback is enabled, and resuming
an ended or failed track reloads it. The event reader holds a weak player reference; closing the
runtime stops playback and cancels the reader.

Lyrics use LRCLIB's `/api/get` with artist, title, album, and duration. A 404 means unavailable;
instrumental, synchronized LRC, and plain text are represented separately.

## QQ Music lifecycle

1. Rust requests `ptqrshow`, retaining `qrsig` inside the pending login. The UI receives the QR image
   and polls `ptqrlogin` through Tauri. The QR expires after five minutes.
2. After phone confirmation, Rust validates redirects, follows the QQ authorization exchange, and
   saves the resulting renewable QQ Music cookie. Cancelling or replacing login invalidates older
   completions through the desktop generation counter and operation gate.
3. `musicu.fcg` receives the authenticated recommendation-feed request. Rust finds the `每日30首`
   card and uses its current playlist ID to request tracks, limiting the collection to 30. The ID is
   discovered from the account's feed rather than hard-coded.
4. Before provider requests, a session older than twenty hours can renew on demand. Renewal attempts
   back off for one hour. Failed renewal retains the existing session; if saving a renewed session
   fails, the current implementation logs the failure but still uses the renewed value in memory.
5. Starting a track resolves its audio URL through `music.vkey.GetVkey/UrlGetVkey`. The shared QQ
   RPC response boundary checks HTTP status, JSON structure, and both result codes before decoding
   business data for login, renewal, recommendations, lyrics, and media URLs. Error responses can
   omit success data; failures identify the operation and returned code or format error without
   exposing response values. Existing successful field aliases and optional lyrics remain supported.
   A successful media response selects available FLAC, MP3, or M4A according to account rights.
   Rust downloads bounded media bytes and loads them on the Rodio worker. New play, pause, resume,
   and seek operations invalidate earlier automatic loads; cancelled work cannot install audio after
   a newer operation. The loaded sink owns its track metadata so snapshots remain consistent during
   cancellation. A weak completion monitor advances the queue through the same request path.
   Failed automatic loads stop advancement and appear in playback reads; no song is silently skipped.
   Resuming drained audio reloads the current track. Audio commands have a ten-second reply deadline;
   a timed-out or stopped worker is retired and the next load creates a new worker. Retiring stops
   installed audio and prevents late work from installing another sink. Native calls already running
   cannot be forcibly terminated. Seeking uses a decoder over the downloaded bytes, preserving pause
   and progress offset without waiting for a mixer callback.
6. QQ lyrics use `music.musichallSong.PlayLyricInfo/GetPlayLyricInfo`. The response can be Base64;
   after decoding it passes through the shared LRC parser. QQ lyrics do not use Spotify's LRCLIB path.

These QQ login, recommendation, and media calls are private client protocols. Region, membership,
copyright, and provider-side changes can affect availability independently of a saved login.

## State, timing, and media boundaries

Both collections cache for five minutes in Rust; a refresh mutex coalesces concurrent misses.
MusicView uses request revisions and provider identity to reject stale collection, playback, and
lyric responses. While mounted, it reads playback every five seconds and advances displayed
progress every 500 ms. Unmounting clears these timers; Rust owns the player independently of the view.

Artwork reaches the main WebView through `vesper-music-cover`; Rust checks provider HTTPS domains
and limits each image to 10 MiB. QQ audio is restricted to provider domains and 100 MiB per download.
The desktop pauses the inactive provider when selecting, starting, or resuming another provider.
A shared playback command gate assigns revisions before waiting for runtime initialization and
cancels superseded selection/play/pause/resume/seek work. Its action lock releases cancelled work
before the next command touches either provider. Spotify also invalidates pending library/initialization
requests on pause and serializes the final load/resume with that pause.
Replacing account credentials holds the runtime lock, closes the old player, and waits for existing
credential writes before saving the new login. In-flight old runtimes cannot renew over that login.
A completed Spotify reconnection advances the Settings session revision and remounts any active
MusicView. The new view clears its settled Spotify collection; unmounting invalidates the old
view's requests so late responses cannot restore the previous account's tracks. A closed Spotify
runtime also rejects library responses that arrive after shutdown.

Only credentials persist: debug uses the shared credential table; release uses the system
store. Track collections, queue/player state, cover lookup maps, and lyrics are not an offline
library mirror. See [PERSISTENCE.md](PERSISTENCE.md) for file locks and storage details.

## Repositories and reference provenance

| Source                                                                | Relationship to this implementation                                                                                                                            |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [crmne/fastpotify](https://github.com/crmne/fastpotify)               | Spotify implementation reference, confirmed by the project owner; distinct from the Rust playback dependencies below                                           |
| [librespot-org/librespot](https://github.com/librespot-org/librespot) | Actual Rust dependency: `librespot-core` and `librespot-playback` 0.8, with the Rodio backend and native-root Rustls transport; owns Spotify playback protocol |
| [RustAudio/rodio](https://github.com/RustAudio/rodio)                 | Actual audio dependency; directly used by the QQ audio thread and through librespot's selected backend                                                         |
| Spotify Accounts/Web API                                              | Provider protocol used by `spotify/auth.rs` and `spotify/mod.rs`; not a vendored reference repository                                                          |
| LRCLIB                                                                | Remote lyric service used by `lyrics.rs`; Vesper does not run or embed its server                                                                              |

Fastpotify is the project owner’s confirmed Spotify reference. Librespot and Rodio are runtime
dependencies declared in [Cargo.toml](../crates/music/Cargo.toml). These are different relationships;
listing a reference does not claim that every UI or login detail was copied from it.

## Verification and limits

Rust tests cover provider parsing, callback validation, personal Client ID authorization, legacy
credential decoding, cache expiry, session renewal, lyrics, queue behavior, cancellation, worker
recovery, stale events, runtime release, and media URL restrictions. Local HTTP tests exercise
Spotify's long cooldowns, bounded retries, concurrent reads, quota errors, retained pagination,
expired partial refreshes, and responses arriving after shutdown.

Frontend tests cover Settings prefill and Client ID submission, music controls, library retry,
reconnection cache invalidation, and late responses from a previous account. Run the Music,
credentials, and desktop Rust tests with `cargo test -p music -p vesper-credentials -p vesper --lib`
and the frontend suite through `pnpm test:frontend`. The HTTP tests use loopback listeners and
synthetic credentials.

These checks establish local request and state behavior. Real browser authorization, personal-app
quota availability, token exchange and rotation, account rights, audio devices, and live QQ protocol
compatibility require account-level verification. They have not been established by the mock tests.
