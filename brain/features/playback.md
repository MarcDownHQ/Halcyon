# Playback (Planned)

What `episode-url` and `movie-url` do, from [SPEC](../specs/SPEC.md) sections 3, 5, 10 and 11 and the
[probe of 2026-11-23](../specs/observations/2026-11-23-clients.md). Lands with M1.

## The client order

Anonymous VOD, stopping at the first client whose URLs pass the probe:

1. `visionos`: no JavaScript, no PO token, signed URLs, HLS available.
2. `ios_producer`: the same, H.264 and AAC only; the fallback for made-for-kids videos.
3. From M4: TVHTML5 5.x Samsung, then downgraded, once `n` and the signature can be solved.
4. Otherwise the most informative `playabilityStatus`, as an error.

The last client that worked is kept and tried first; the order is re-tested from the top after a day. Clients are
asked one at a time.

## The request

`POST https://www.youtube.com/youtubei/v1/player?prettyPrint=false` with the client's context (name, version, device
fields, `hl:"en"`, `timeZone:"UTC"`), `visitorData` in the context and in `X-Goog-Visitor-Id`, a fresh 16-character
`cpn`, `contentCheckOk` and `racyCheckOk`. No API key.

## Building the playback

- **Formats** without a URL (SABR-only), with DRM, OTF, live adaptive, unsolvable `n`, or damaged are dropped.
- **Video**: the tallest video-only format under the `quality` cap, then the highest fps, then VP9 before AVC before
  AV1. AI-upscaled and HDR formats only on request. With no video-only format, the best muxed one, then the HLS
  manifest.
- **Audio**: one format per language (the part of `audioTrack.id` before the dot), DRC, voice-boost and descriptive
  variants dropped, the highest bitrate kept. Order: the original language, other human dubs, auto-dubs last. Each
  is a `track` with an ISO 639 `language` and `audioTrack.displayName` as `title`.
- **Headers**: the client's User-Agent, the only header the probe needed.
- **Subtitles** join the tracks in M5 ([player-features.md](player-features.md)).

## The probe

Before handing URLs to mpv, request a small range near the end of the chosen audio format (from `contentLength` or
`clen`). A 2xx means the client's URLs are good; a 403 moves to the next client. A googlevideo 403 is otherwise
silent until mpv hits it.

## Errors and the bot check

| Answer | Action |
| --- | --- |
| `OK` | Continue |
| `LOGIN_REQUIRED` "Sign in to confirm you're not a bot" | Refresh visitorData once, retry the same client once, then **stop and report** `bad-response`. Never cycle through clients: it can be IP-wide and lasts up to half an hour |
| Age or content gate | `auth-failed` (from M6, a signed-in TV client) |
| `UNPLAYABLE` | Next client (made-for-kids on `visionos` goes to `ios_producer`) |
| `ERROR` | `bad-response(reason)`, no other client |
| `LIVE_STREAM_OFFLINE` | `bad-response("Live starts at …")` |
| "This content isn't available, try again later", a captcha | `http(429)`, no other client |

SPEC 5.2 moves on to the next client after a bot check; the probe overrules it, since the check hit every client
alike and moving on only spent requests while it lasted.

The reason shown is YouTube's own text (reason, then subreason), with "This helps protect our community. Learn
more" stripped.

## Sources

None yet; lands with M1.
