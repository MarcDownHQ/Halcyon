# The host contract

`wit/provider.wit` (package `nightglass:provider@0.3.0`, world `provider`) is vendored from Nightglass; its header
names the source commit. Nightglass is MIT. Each export below is marked with what it does today and, where it is
still to build, what it is **(Planned)** to do and when.

## Exports

| Export | Today | Planned |
| --- | --- | --- |
| `describe` | Id `halcyon`, name "Halcyon", no fields, `resolves-streams: true`, no preset | The fields of SPEC 2.2 (M1); maybe one example preset |
| `load-catalog` | `bad-response("not implemented yet")` | Configured channels and playlists as series, lives as channels, videos as movies (M1-M3) |
| `episodes` | not implemented yet | A series' uploads or playlist items (M1 from RSS, M2 from the tabs) |
| `movie-url`, `episode-url` | not implemented yet | The shared resolver: `/player`, then a URL plus tracks (M1) |
| `stream-url` | not implemented yet | A live's HLS manifest (M3) |
| `catchup-url` | not implemented yet | `http(501)` until M8, then a synthesised `sq` playlist |
| `filter` | Returns the media unchanged | Only if a live HLS playlist ever carries `n` (SPEC 9.4), or for the M8 catch-up playlist |

Catalog ids stay stable across refreshes (the app keys favourites and progress on them): `ch:<UCid>` and
`pl:<playlistId>` for series, `<videoId>` for episodes and movies, `live:<videoId>` and `chlive:<UCid>` for
channels. The `stream` of a playable item is opaque (`v:<videoId>`, `chlive:<UCid>`) and read back at play time.

## Imports

- `http.send`: http(s) only, at most 128 MiB per answer. Any status is a response; only a failed connection is an
  error. Redirects are followed (googlevideo often answers 302 to a sibling edge host). A request may present a
  browser's TLS and HTTP/2 handshake (`chrome`, `edge`, `firefox`, `safari`); so far nothing at YouTube has needed
  one.
- `storage.get/set/delete`: text values, an optional time to live in seconds, optionally kept on disk. Keys are at
  most 256 bytes, values 1 MiB, 16 MiB in all, shared by every source of the plugin. During `describe`, `get`
  returns nothing and `set` fails.
- `log.log`: planned for every client attempt at `debug` and each fallback at `info`, so a user can report a
  breakage with context.

## The relay and its filter

A `playback` with a `relay` makes the player open a 127.0.0.1 address: the app fetches the stream itself and rewrites
HLS playlists so everything they name comes back through it. It rewrites HLS, not DASH. Without `filter`, answers
stream through with `Range` passed along; with `filter`, every answer is read whole, without `Range`, and goes
through the `filter` export. The relay asks a 403, 502 or 503 once more.

Halcyon plans no relay: the probe of 2026-11-23 showed mpv can read googlevideo URLs directly. A relay with no filter
is the fallback if random 403s show up mid-stream.

## Playback

`playback.url` is what the player opens; `playback.tracks` are played alongside it, in order of preference, each with
an ISO 639 `language` and a `title` (mpv joins them in an EDL). `headers` go with every request of the stream and its
tracks. A video-only stream with its audio languages and subtitles as tracks is the shape Halcyon plans to return.
The contract has no chapters and no notion of quality variants; see
[features/player-features.md](../features/player-features.md).

## Limits

- Each call runs in a fresh instance with 256 MiB of memory; `describe` has 10 s, `filter` 30 s, the rest 5 min.
- Stream addresses must be network schemes (http, https, rtmp, rtsp, rtp, udp, srt, mms): no `data:` or `edl://`.
- With `resolves-streams`, a movie's or an episode's `stream` can be any text; the app hands it back byte for byte.
- No search export, no UI hook and no background task: a plugin speaks only through return values and `log`.

## Sources

- [wit/provider.wit](../../wit/provider.wit)
- [src/wasm.rs](../../src/wasm.rs)
- [src/provider.rs](../../src/provider.rs)
