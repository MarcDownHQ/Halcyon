# Player features: quality, subtitles, chapters (Planned)

Three player features the user asked for, grouped in M5 of the [roadmap](../../ROADMAP.md). Each needs work on the
Nightglass side as well as in Halcyon, so each carries an open decision. The facts on what YouTube offers for each
were gathered by a facts-only researcher in [chapters-quality-subs](../specs/sources/chapters-quality-subs.md); read it
before deciding.

## Subtitle selector

- **Halcyon**: caption tracks from `captions.playerCaptionsTracklistRenderer.captionTracks[]` become
  `playback.tracks`: the `baseUrl` made absolute, `xosf` removed, `fmt=vtt` added. `language` is the
  `languageCode`, `title` the track name plus " (auto)" for auto-generated ones. Manual tracks first, about 10 at
  most. Tracks whose `exp` asks for a PO token are skipped. Translation (`tlang`) is later.
- **Nightglass**: the contract carries them, and the player's subtitle menu picks one, or none.
- Observed 2026-11-23: `visionos` captions came back as valid WebVTT with no token.

## Quality selector

- Today's plan (M1) picks one video format under the `quality` cap.
- **Open decision**: let the viewer switch by either
  - returning several video-only formats, one per height, as tracks (mpv opens each at load and then reads only the
    chosen one; a track's `title` becomes its label in the menu), or
  - returning the client's HLS master, whose variants the player switches between (`visionos` lists H.264 and VP9
    up to 2160p60 with separate audio groups and a subtitle rendition). The cost: HLS variant choice is the
    player's, and dubbed audio and captions come only as the master names them.
- **Nightglass**: since contract 0.4.0 (PR #46), a Quality menu over mpv's video tracks, labelled by the track's
  title or its height, frame rate, codec and HLS bitrate; the chosen height is remembered.

## Chapters

- **Contract**: `playback.chapters`, each a `start` in seconds and a `title` (Nightglass 0.4.0, PR #46). The app sorts
  them, drops empty titles and non-finite starts, and shows them on the seek bar, in a Chapters menu and with
  Page ↑ ↓.
- **Halcyon**: chapters come from `/next`, not `/player`, and `visionos` returns none, so an extra `/next` call
  with a client that has them (see [chapters-quality-subs](../specs/sources/chapters-quality-subs.md)).

## Sources

None yet; lands with M5.
