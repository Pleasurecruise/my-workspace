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
| [qq/mod.rs](../crates/music/src/qq/mod.rs)                                          | Daily 30 discovery, session renewal, media and lyric requests, queue, and Rodio audio thread                      |
| [qq/qr.rs](../crates/music/src/qq/qr.rs)                                            | QQ QR creation, polling, redirect validation, and authorization exchange                                          |
| [lyrics.rs](../crates/music/src/lyrics.rs)                                          | LRCLIB lookup and shared LRC parsing                                                                              |

The transport exposes `Track`, `Playback`, `PlaybackOrder`, and `Lyrics`. Track IDs and cover keys
refer to the selected provider's loaded collection; UI code does not choose arbitrary media URLs.
Commands cover reading tracks/playback/lyrics, playing, pausing, resuming, seeking, and changing
sequential, repeat-one, or shuffle order. Expected failures use the tagged command response.

## Spotify lifecycle

1. Settings opens two PKCE grants in sequence: a Web API grant for library/account reads, then a
   playback grant for librespot. Callback state is checked; authorization has a ten-minute deadline.
2. The two refresh grants are stored as one typed credential record. Access tokens and runtime
   objects stay in Rust. Refresh runs under credential synchronization with an expiry margin.
3. Liked Songs reads `/v1/me/tracks` in pages of 50, maps provider objects to tracks, and records
   artwork URLs behind opaque provider keys. It finishes pagination before replacing the collection.
4. Playback creates the local librespot player on demand. The Rust player handles player events and
   advances the queue; Svelte displays its snapshot. Local Spotify playback requires Premium.
5. Lyrics use LRCLIB's `/api/get` with artist, title, album, and duration. A 404 means unavailable;
   instrumental, synchronized LRC, and plain text are represented separately.

Spotify Web API and playback are distinct grants and paths; a successful library request does not
establish that the account can play audio. Client IDs are application identifiers, not secret keys.

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
5. Audio URL resolution selects available FLAC, MP3, or M4A according to account rights. Rust
   downloads bounded media bytes, then sends commands to a dedicated thread owning Rodio's audio
   device. A generation-scoped completion monitor advances the queue.
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
The desktop pauses the inactive provider when starting or resuming another provider.

Only credentials persist: debug uses `spotify.json` and `qq-music.json`; release uses the system
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

Rust unit tests cover callback state, provider parsing, cache expiry, session renewal, lyric parsing,
queue behavior, and media URL restrictions. Frontend MusicView tests cover request/interaction
behavior. Run `cargo test -p music` and the MusicView tests through Vite Plus when changing this path.
Real browser authorization, account rights, audio devices, and live QQ protocol compatibility still
require explicit manual verification. This document records code behavior, not a fresh account test.
