# Live (Planned)

Lives as Nightglass channels, from [SPEC](../specs/SPEC.md) section 9. Lands with M3, which starts with an
investigation: on 2026-11-23 every live was refused to every client with the bot check, and live calls came right
before both IP-wide blocks ([observations](../specs/observations/2026-11-23-clients.md)).

## What becomes a channel

- A live on air, found in a configured `live` or `channels` entry: `channel` with id `live:<videoId>`, stream
  `v:<videoId>`, named after the channel and the stream title, group "YouTube live".
- A pinned 24/7 channel (a `live` entry naming a channel): id and stream `chlive:<UCid>`. Its live video id changes
  between broadcasts, so it is resolved at play time from `https://www.youtube.com/channel/<UC>/live`.
- Optionally one `programmes` entry: the stream title from `liveBroadcastDetails.startTimestamp`.

Lives are never probed at catalog time.

## Playing

`stream-url` takes `hlsManifestUrl` from `visionos`, then `ios_producer`, and returns it as `playback.url` with the
client's User-Agent. Any `/n/<v>/` path segment is solved (needs M4). If live playlists turned out to carry `n` in
their segments, a filtering relay would rewrite them (SPEC 9.4); no source says they do.

| State | Answer |
| --- | --- |
| Live now | The HLS manifest |
| Upcoming, `LIVE_STREAM_OFFLINE` | `bad-response("Live starts at …")` |
| Post-live, under 2 h | The HLS manifest (it holds the last two hours) |
| Post-live, longer | `bad-response("Stream ended; replay is still processing")` |
| A pinned channel not live | `bad-response("not live")` |

## Catch-up (M8)

`catchup-window` from `maxDvrDurationSec` (at most 120 h), served as a synthesised playlist of `&sq=<n>` segment URLs
through `relay.filter`, if a probe shows such segments play. Until then `catchup-url` answers `http(501)`.

## Sources

None yet; lands with M3.
