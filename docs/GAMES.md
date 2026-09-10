# Games Implementation

Games combines daily status and a local pull archive for Genshin Impact, Honkai: Star Rail, Zenless
Zone Zero, Arknights, and Arknights: Endfield. Steam has its own library/activity widget. Svelte owns
interaction and charts; Rust owns account authorization, provider protocols, caching, verification,
and archive calculations.

## Components and boundaries

| Source                                                                                    | Responsibility                                                                                                |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| [GamePanel.svelte](../apps/desktop/src/lib/components/games/GamePanel.svelte)             | Combined per-game frame with daily and archive panels; bounded miHoYo panel height and narrow-layout stacking |
| [GameNotesPanel.svelte](../apps/desktop/src/lib/components/games/GameNotesPanel.svelte)   | Daily snapshot, explicit refresh, verification actions, and stale-response rejection                          |
| [GachaPanel.svelte](../apps/desktop/src/lib/components/games/GachaPanel.svelte)           | Local account/archive selection, manual sync, rarity rings, and recent records                                |
| [GameConnections.svelte](../apps/desktop/src/lib/components/games/GameConnections.svelte) | QR dialogs, miHoYo logins and per-game selections, and Steam credential form                                  |
| [SteamGamesPanel.svelte](../apps/desktop/src/lib/components/games/SteamGamesPanel.svelte) | Steam presence, library totals, recent activity, and ranked games                                             |
| [gaming.rs](../apps/desktop/src-tauri/src/gaming.rs)                                      | Typed Tauri commands, settings prefill, and account/daily events                                              |
| [gaming/](../apps/desktop/src-tauri/src/gaming)                                           | Isolated verification windows and restricted browser bridge                                                   |
| [crates/games](../crates/games/src/lib.rs)                                                | Runtime, provider operations, daily cache, verification lifecycle, and archive access                         |
| [credentials/games](../crates/credentials/src/games/mod.rs)                               | Typed provider secrets, account collection, selections, and SQLite/system-store persistence                   |

```text
Settings -> QR / account binding -> credentials
Dashboard -> GamePanel -> daily command -> Rust cache -> provider API
                      -> archive read -> vesper.sqlite3
                      -> manual sync -> provider history -> validated SQLite transaction
Steam widget -> Steam command -> read-only Steam Web APIs
```

The contracts include `Account`, `Meter`, `Task`, optional `TaskProgress`, `Notes`, and `Pull`.
Daily reads return `ready`, `failed`, `verificationRequired`, or `refreshRequired`. Genshin commissions, expeditions,
and remaining weekly discounts expose typed counts and render filled/outlined stars for small
positive totals up to five. Resin remains numeric; accessible labels and tooltips retain the actual
metric and count, including the meaning of discounts remaining.

## Authorization and account selection

miHoYo QR creation and polling use the account passport API with the same device ID and client
profile. The dialog directs users to Miyoushe. Confirmation yields SToken; Rust exchanges the
additional session tokens without exposing them to the WebView. It does not use a GameToken-based
QR flow. Pending QR IDs scope polling/cancellation so superseded completions cannot save a login.

miHoYo logins are indexed by account ID and each game selects its login independently. Reauthorizing
an existing account replaces that account's session. Removing a login clears its selections and
runtime record session while preserving pull history. The account collection has a separate
read-modify-write lock and accepts only the current collection format; see [PERSISTENCE.md](PERSISTENCE.md) for the exact storage and lock paths.

Skland uses its scan-login exchange, then signed API requests and role-specific history tokens.
The [official login client](https://web.hycdn.cn/user-center/index.fa9aa713.js) defines QR states.
Polling maps 100 to waiting, 101 to phone confirmation, and 102 to expired. Steam uses an API key
and SteamID64 entered in Settings (Keychain in macOS release builds). Debug builds prefer
`STEAM_API_KEY` and `STEAM_ID` from the environment / root `.env`, falling back to the development credential table
only when both variables are absent. Only `read_steam_settings` returns those stored values to the
trusted form. General connection and activity projections omit secrets.

## Daily cache and request ordering

`Runtime` owns separate operation mutexes for miHoYo, Skland, and Steam. Daily reads acquire the
provider operation lock, then use the memory cache keyed by game, account, and login identity.
The cache also has per-key locking and stores both successes and failures, including failures while
initializing a record session. It has no expiry. Per-key cache isolation does not remove the outer
serialization of operations using the same provider.

Mounting the panel or entering Dashboard reads this cache. Explicit daily refresh or explicit
Dashboard refresh bypasses it. Navigation, WebView reload, and closing verification do not force a
provider request. Restarting the Rust process clears it; returning to an unchanged login can reuse
its entry. Component request generations and account-change events prevent obsolete responses from
replacing a newer view. Failed refreshes remain visible while previously displayed data can remain.

## miHoYo record and human verification

miHoYo uses the account QR API with a consistent device identifier across creation and polling.
Confirmation returns SToken directly, without the legacy GameToken exchange. LToken exchange uses
passport client headers and a DS v2 signature with `{}` as the GET signature body. Errors identify
the failed stage without exposing response bodies. Risk rejection (-3503) stops polling; the user
can create a new code after checking login in the Miyoushe app.

Genshin and Star Rail daily notes use Hutao's CookieToken/LToken, DS v2 (X4), client type `5`,
client version `2.95.1`, device fingerprint, and the official record Referer. Genshin also uses
Hutao's `x-rpc-tool_verison: v5.0.1-ys` header. Role lookup is cached in the record session.
Star Rail uses its own `hkrpg/api/note` endpoint; Hutao itself supports Genshin, so its shared
verification flow is adapted to Star Rail's game ID `6` and note URL. ZZZ retains the existing
Starward record client profile. Device identity remains persistent, with on-demand fingerprint
renewal after three days; there is no background registration task.

Codes `1034`, `5003`, `10035`, `10041` and `10053` become a typed verification state. Numeric codes
remain diagnostic data while the card explains the required action in plain language.
Genshin registers a challenge through `card/wapi/createVerification?is_high=true`; Star Rail
uses the RPG client's `toolcomsrv/risk/createGeetest` with its application key. Both include
`x-rpc-challenge_game` and `x-rpc-challenge_path`
bound to the selected game's daily endpoint. The triggering daily response's `x-trace-id` is
retained per game and login, then forwarded as `x-rpc-challenge_trace` on both registration and
proof submission, matching the official record client. Only numeric return codes, game, stage,
and trace presence enter English diagnostics; cookies, challenge values, trace values and response
bodies never enter logs. Known failures distinguish login expiry, challenge expiry, captcha
rejection, invalid parameters and rate limiting. A dedicated isolated window loads Geetest's widget;
it receives only the challenge and has no account cookies or main-window Tauri capabilities.
After the user solves it, Rust submits the proof to Genshin's `card/wapi/verifyVerification` or
Star Rail's `toolcomsrv/risk/verifyGeetest` with the same session and device. The returned `x-rpc-challenge` is retained for one manual daily read of that
game and login. Opening, solving, failing or closing verification never automatically rereads daily
notes. Cancelled, expired, duplicate and superseded verification results cannot install a challenge.

ZZZ continues to open the official record page with its restricted miHoYo browser bridge. Its
login action routes to Settings → Games. The macOS cookie check reads the whole isolated store
because Wry's URL getter incorrectly excludes parent-domain cookies. This official-page flow is
not used for Genshin or Star Rail. Provider failures remain visible and preserve settled card data.

Provider operations remain scoped to the selected login. A no-captcha response without a token
does not confirm access or clear a cached restriction. Verification diagnostics record game, stage,
numeric result, trace presence, and timing in the single `game_diagnostic` database row, excluding secret values.

## Pull archive and Steam

Manual pull sync has a five-minute deadline and bounded pagination. Genshin generates an AuthKey
from SToken; ZZZ uses cookie history. Skland derives its game's history authorization.

Star Rail uses the official miHoYo activity flow: `common/badge/v1/login/account` exchanges the
record CookieToken/LToken for an ephemeral `e_hkrpg_token`, then `event/rpg_gacha_record/pool_stat`
and `five_star_list` return six pools' totals, current pull counts since the last five-star, and
paginated five-star records. It does not call `genAuthKey`, read game caches, or ask for links.
The activity cookie is confined to that explicit sync and never changes the daily record session.

This API does not expose individual three- and four-star records. Vesper stores a separate typed
report in the shared `game_reports` table, preserves older five-star records and existing
full pull archives, and displays the coverage explicitly. It never invents filler pulls, dates,
or rarity distributions. Opening the card reads the saved report only; the refresh icon syncs.
The protocol was checked against
[Axiu-Plugin's Star Rail service](https://github.com/AxiuCN/Axiu-Plugin/blob/master/model/srGacha.js)
and verified with live badge login, all six pools and a temporary archive.

Rust validates all downloaded records before merging accounts and pulls in one `vesper.sqlite3`
transaction. Identity is `(game, uid, record id)`. Sync adds available history without deleting older
saved records. Missing miHoYo item IDs can be filled later; conflicting identity/content fails the
transaction. Arknights normalizes rarity, and Endfield skips bonus events as pulls while still
advancing pagination. Free/new flags remain attached to actual records.

Archive summaries derive pool rarity counts, total pulls, distance since the last top-rarity item,
and observed intervals. Rings display those local statistics, not guaranteed pity or cross-banner
carry-over. Provider history that has already expired cannot be recovered by a new sync.

Steam reads player presence, owned games, and recent games. Rust sums minutes across the returned
collections before selecting top-five lists; Svelte displays hours. Private/missing counts remain
unavailable. Steam snapshots are uncached and refresh every five minutes while Dashboard is active.
Neither daily snapshots nor Steam activity are persisted as an archive.

## Reference repositories and adopted behavior

| Repository/source                                                                             | Recorded role                                                                                                                                                                           |
| --------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Snap.Hutao.Remastered](https://github.com/SnapHutaoRemasteringProject/Snap.Hutao.Remastered) | Account QR/passport requests, Genshin record headers and signing, AuthKey flow, and human-verification reference; Vesper adapts shared behavior to Star Rail and retains manual refresh |
| [Scighost/Starward](https://github.com/Scighost/Starward)                                     | Game-record device profile and browser bridge; current ZZZ record-window path retains this approach                                                                                     |
| [UIGF-org/mihoyo-api-collect](https://github.com/UIGF-org/mihoyo-api-collect)                 | Recorded miHoYo protocol reference collection; not a runtime dependency                                                                                                                 |
| [seriaati/genshin.py](https://github.com/seriaati/genshin.py)                                 | Recorded HoYoLAB/Miyoushe API reference; the workspace does not execute this Python client                                                                                              |
| [FrostN0v0/nonebot-plugin-skland](https://github.com/FrostN0v0/nonebot-plugin-skland)         | Recorded Skland/Arknights/Endfield integration reference; implemented here in Rust rather than loaded as a NoneBot plugin                                                               |
| Official Skland login client and miHoYo RPG record client                                     | Response-state and Star Rail challenge protocol references                                                                                                                              |
| [Steam Web API](https://partner.steamgames.com/doc/webapi)                                    | Official provider contract                                                                                                                                                              |

The pinned
[Starward record source](https://github.com/Scighost/Starward/tree/3e2da5ffecde252211edb74b850ee13d6b93f6dd/src/Starward/Features/GameRecord)
and [Hutao verification source](https://github.com/SnapHutaoRemasteringProject/Snap.Hutao.Remastered/blob/0977f377f20a359aae34e07ea249d76be59ff926/src/Snap.Hutao.Remastered/Snap.Hutao.Remastered/Service/Geetest/GeetestService.cs)
are preserved from existing project documentation. Other rows are recorded protocol references;
that is not evidence that code was copied from every listed repository.

## Verification and limits

`cargo test -p games --lib` covers archive transactions, conflicting records, daily-cache behavior,
QR states/cancellation, verification request scoping, provider parsing, and Steam projections.
Frontend tests under [games/**tests**](../apps/desktop/src/lib/components/games/__tests__) cover
panels, QR interactions, accessible counters, and verification lifecycle. Desktop verification tests
cover the isolated window boundary. Live provider tests remain ignored and opt-in; fixtures do not
prove current account access, real-device scanning, or captcha acceptance. This document describes
the checked source and recorded references without running authenticated provider workflows.
