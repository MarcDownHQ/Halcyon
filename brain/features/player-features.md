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
- **Nightglass**: the contract already carries them; the player needs a subtitle menu to pick one, or none.
- Observed 2026-11-23: `visionos` captions came back as valid WebVTT with no token.

## Quality selector

- Today's plan (M1) picks one video format under the `quality` cap.
- **Open decision**: let the viewer switch by either
  - returning several video-only formats, one per height, as tracks the player can choose between (the contract has
    no way to mark a track as a video quality rather than an alternative), or
  - returning the client's HLS master, whose variants the player switches between (`visionos` lists H.264 and VP9
    up to 2160p60 with separate audio groups and a subtitle rendition). The cost: HLS variant choice is the
    player's, and dubbed audio and captions come only as the master names them.
- **Nightglass**: a quality menu in the player either way; perhaps a contract addition to label variants.

## Chapters

- **Open decision**: a contract addition such as `playback.chapters`, a list of titles with start times, and the
  player showing them on the seek bar and in a menu. Where YouTube exposes chapters, and how reliably, is for the
  coming notes.
- **Nightglass**: both the contract change and the player UI.

## Sources

None yet; lands with M5.
