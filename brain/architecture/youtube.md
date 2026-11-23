# YouTube, as observed

A short pointer doc. The full protocol is in [the spec](../specs/SPEC.md), merged from facts-only notes on five
projects ([sources](../specs/sources/yt-dlp.md)); what was checked against YouTube itself is in
[observations](../specs/observations/2026-11-23-clients.md). Below are the facts that shape the design today. Add a
new fact here with the date it was observed, and a probe write-up in `brain/specs/observations/` when it took more
than a request or two.

## Clients (observed 2026-11-23, anonymous, no PO token, no JavaScript)

| Client | Verdict | Why |
| --- | --- | --- |
| `visionos` | Plays end to end (VOD) | Adaptive URLs arrive signed, with no `n`. Up to 2160p60 VP9/AV1, Opus and AAC, DRC and dubbed audio tracks, an HLS master with a VTT rendition, `fmt=vtt` captions. No itag 18; refuses made-for-kids videos (per the spec). |
| `ios_producer` | Plays end to end (VOD) | Signed URLs, no `n`, but H.264 up to 1080p and AAC only, the original audio only, no captions block. |
| `tv_samsung` | Not without JS | Every URL carries `n` (403 unsolved); the music video came back as `signatureCipher`. |
| `mweb`, `web_safari` | No | Adaptive formats SABR-only. |
| TVHTML5 5.x downgraded, `web_embedded` | No | "The page needs to be reloaded." / "This video is unavailable". |

## Facts that matter

- **visitorData is mandatory.** Without it, `visionos` answers the bot check; with one taken from any anonymous
  Innertube answer's `responseContext.visitorData`, it answers `OK`. It comes URL-encoded and must be decoded.
  Locally generated ids are rejected (spec, since 2025-03).
- **URLs are bound to the IP** (`ip` in `sparams`) and expire after about 6 h (`expiresInSeconds` 21540).
- **Ranges.** `&range=a-b` and a `Range:` header are equally fast; the header form answers a proper 206. A read
  larger than about 11 to 16 MiB, including a whole file, is never refused but is paced to exactly twice real time.
  10 MiB chunks arrive at 15 to 25 MiB/s. mpv can read the URLs directly; no relay is needed.
- **The bot check can cover the whole IP.** Twice, right after live `/player` calls, every `/player` call got
  "Sign in to confirm you're not a bot" on VODs too, for 12 to 32 minutes; fresh visitorData did not clear it.
  Media URLs, HLS segments and captions handed out before kept working. Hence the back-off rule.
- **Lives were refused to every client** with the bot check, so live HLS is untested (M3 starts with an
  investigation).
- **Captions** from `visionos` and `mweb` worked with `xosf` removed and `fmt=vtt` added, no token, even during the
  bot check.
- **The player** was `6b3d0364` with STS `20774` on 2026-11-23; the id rotates and two can be live at once (spec).

## Sources

- [brain/specs/SPEC.md](../specs/SPEC.md)
- [brain/specs/observations/2026-11-23-clients.md](../specs/observations/2026-11-23-clients.md)
- None in `src/` yet; lands with M1.
