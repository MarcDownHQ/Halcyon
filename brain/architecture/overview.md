# Overview

One crate, `halcyon`, built two ways:

- for `wasm32-wasip2` as a `cdylib`: the component Nightglass loads, `halcyon.wasm`;
- natively as an `rlib`: what the tests and the coverage run.

Only `src/wasm.rs` is wasm-specific (`#[cfg(target_family = "wasm")]`). It runs `wit_bindgen::generate!` on
`wit/provider.wit` and implements the generated `Guest` trait by calling `provider`, converting records on the way.
Everything else is plain Rust, so it builds and is tested on the host, and `wasm.rs` stays free of logic: it is not
measured by the coverage.

## Modules today

| Module | Role |
| --- | --- |
| `model` | The WIT records Halcyon produces, as plain Rust: `ProviderInfo`, `Preset`, `Config` |
| `provider` | The exports as plain Rust; so far only `describe` (id `halcyon`, name "Halcyon", `resolves-streams`, no preset) |
| `wasm` | The component boundary; every export but `describe` and `filter` answers `bad-response("not implemented yet")`, `filter` passes media through |

## Modules to come (Planned)

The split follows the spec's topics; names may change when the code lands.

| Module | Role | Milestone |
| --- | --- | --- |
| `host` | `Request`, `Response`, the `Http` and `Storage` traits mirroring the WIT imports, so resolvers run on fakes | M1 |
| `config` | The free-text fields of SPEC 2.2, validated into a typed config | M1 |
| `innertube` | Request plumbing, the client table, visitorData | M1 |
| `player` | `/player`, `playabilityStatus`, format filtering and choice, audio tracks, the probe | M1 |
| `catalog` | Handle resolution, RSS, tabs, renderers, continuations, playlists | M1, M2 |
| `live` | Live detection, `chlive:` resolution, HLS | M3 |
| `challenges` | Player id, STS, the n and signature solver on rquickjs | M4 |
| `captions` | Caption tracks | M5 |
| `auth` | Device code, tokens, brand accounts, cookies | M6 |
| `testing` | `FakeHttp`, `FakeStorage`, fixtures (tests only) | M1 |

## A play, end to end (Planned, M1)

1. The catalog gives every playable item an opaque `stream` such as `v:<videoId>` or `chlive:<UCid>`
   (`resolves-streams` is set, so the host hands it back untouched).
2. `episode-url`, `movie-url` or `stream-url` parses it, loads visitorData from storage, and asks `/player` as the
   first client of the order in [features/playback.md](../features/playback.md).
3. It builds a `playback`: the video-only format as `url`, the audio languages and subtitles as `tracks`, the
   client's User-Agent in `headers`. For a live, the HLS manifest.

## What is kept between calls (Planned)

Each call runs in a fresh instance, so anything worth keeping goes through storage. Every source of the plugin shares
the same keys, and a time to live is never longer than what it keeps stays valid. Losing a value costs requests,
never a play. The keys proposed by the spec (SPEC 3.1, 3.3, 4.1, 4.10, 6.7, 8.3):

| Key | Value | Lifetime | Milestone |
| --- | --- | --- | --- |
| handle to `UC` id | the channel id | no expiry, on disk | M1 |
| visitorData | from an anonymous Innertube answer | about 24 h, on disk; refreshed on a bot check | M1 |
| last good client | the client that last passed the probe | re-tested from the top after 24 h | M1 |
| first page of a channel | the parsed first uploads page | about 15 min | M2 |
| `player:id` | the current player id | 1 h, memory | M4 |
| `sts:<pid>`, `sig:<pid>:<len>` | the STS, a signature position map | 30 days, on disk | M4 |
| `n:<pid>:<n>` | a solved n | 6 h (a URL's lifetime), memory | M4 |
| `js:<pid>:<engine>:<k>` | the prepared player as QuickJS bytecode, in chunks under 1 MiB | 7 days, on disk | M4 |
| `auth:*` | client id and secret, device id, refresh and access tokens, the pending code, accounts | see [features/login.md](../features/login.md) | M6 |

## Sources

- [src/lib.rs](../../src/lib.rs)
- [src/wasm.rs](../../src/wasm.rs)
- [src/model.rs](../../src/model.rs)
- [src/provider.rs](../../src/provider.rs)
