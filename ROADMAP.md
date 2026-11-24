# Roadmap

The agreed backlog, in delivery order, with the decisions taken on each item so they are not lost. It follows the
milestones of [brain/specs/SPEC.md](brain/specs/SPEC.md) section 14, adjusted by the decisions since. Vocabulary
follows [brain/glossary.md](brain/glossary.md); the reasons behind the shape are in
[brain/decisions.md](brain/decisions.md). Small fixes and open questions go in [TODO.md](TODO.md).

## M0 - Foundation

- [x] Single Rust crate building a `wasm32-wasip2` component against Nightglass's `wit/provider.wit` (vendored,
      `nightglass:provider@0.4.0`), with the logic in plain Rust and wit-bindgen only in `src/wasm.rs`.
- [x] CI on GitHub-hosted runners: format, clippy (host and wasm), tests, the component builds.
- [x] 98% line coverage floor (`scripts/coverage.sh --check`, a CI job).
- [x] Docs: `CLAUDE.md`, README, roadmap, TODO, changelog, and the brain.
- [x] Clean-room specs: the per-source fact notes, the merged [SPEC](brain/specs/SPEC.md) and the first live probe
      ([2026-11-23](brain/specs/observations/2026-11-23-clients.md)).
- [x] Name chosen: Halcyon (see [decisions](brain/decisions.md)).
- [x] Project skills: `brain`, `feature-treatment`, `release`.
- [x] Release workflow: a `v*` tag attaches `halcyon.wasm` and its SHA-256 to a GitHub release.

## M1 - VOD through visionos, RSS catalog

The smallest useful slice. No JS engine, no PO token, no relay.

- [ ] Fields `channels` and `videos`, with the free-text list parsing of SPEC 2.2.
- [ ] Resolve `@handle` and channel URLs to a `UC…` id (HTML canonical link or `externalId`), kept in storage.
- [ ] Each channel becomes a series built from its RSS feed: exact `aired`, thumbnail, title, plot, no duration.
      Videos become movies.
- [ ] visitorData bootstrap from an anonymous Innertube answer, kept in storage, refreshed on a bot check.
- [ ] `episode-url` and `movie-url`: `visionos` `/player`, the best video-only format under `quality`, the audio
      languages as tracks, the client's User-Agent in `headers`, a tail-range probe on the audio URL. Fallbacks:
      `ios_producer`, then the `visionos` `hlsManifestUrl`. Decided: `visionos` first, `ios_producer` second (the
      probe of 2026-11-23 played both end to end); no relay, since mpv can read the URLs directly.
- [ ] Errors: the `playabilityStatus` classification (SPEC 5.2) mapped to the contract's errors (SPEC 11.2).
      Decided: the bot check backs off and reports, it never cycles through clients.
- [ ] Exit criterion: a 10-minute video plays to the end and seeks in mpv.

## M2 - Full catalog

- [ ] Uploads from the `/videos` and `/streams` HTML tabs plus `/browse` continuations; the `lockupViewModel` parser
      with the older renderers as fallbacks; duration badges; relative dates merged with the RSS dates.
- [ ] `max-episodes` and `tabs`.
- [ ] Playlists (`playlists` field).
- [ ] The `UU` uploads playlist as the fallback when the tab layout breaks.
- [ ] Self-healing tab `params`; the first page kept about 15 minutes so `episodes` right after `load-catalog`
      does not fetch it again.

## M3 - Live

- [ ] **Investigation first.** The probe of 2026-11-23 found every live refused to every client with the bot check,
      and twice the whole IP was bot-checked on VODs right after live calls. Before building anything, find out
      when an anonymous live `/player` answers, which clients return a live `hlsManifestUrl`, and whether its
      playlists carry `n` or need a token. Write it up in `brain/specs/observations/`.
- [ ] The `live` field; `chlive:` channels resolved at play time through `/channel/<UC>/live`; live-now items from
      the streams tab. Decided: never probe a live at catalog time.
- [ ] HLS from `visionos`, then `ios_producer`.
- [ ] Offline, upcoming and post-live messages; an optional `programmes` entry with the stream title.

## M4 - Player script and challenges

- [ ] rquickjs (QuickJS-NG) in sloppy mode, with memory and stack limits.
- [ ] Player id from `iframe_api`, the STS, per-player caches in storage (SPEC 6.7).
- [ ] The n and signature solver. Decided: our own clean-room code, a Rust-side ES parser that finds the player's
      stream-URL builder plus rquickjs to run it, not yt-dlp's EJS vendored in QuickJS.
- [ ] A self-test per new player id; a failing player is set aside for an hour and the JS-free clients carry on.
- [ ] TVHTML5 5.x (Samsung, then downgraded) join the client order; `/n/` rewriting in HLS manifests.

## M5 - Captions, quality and chapters

Replaces SPEC's captions-only M5 with the three player features asked for. What YouTube offers for each is in
[chapters-quality-subs](brain/specs/sources/chapters-quality-subs.md). The Nightglass side landed in contract 0.4.0
(nightglass PR #46): a Quality menu over the video tracks, `playback.chapters`, and the existing subtitle menu.

- [ ] **Subtitles.** Caption tracks as `playback.tracks` (VTT, `xosf` removed, `exp`-gated tracks skipped, manual
      before auto-generated, about 10 at most). Nightglass's existing subtitle menu picks among them.
- [ ] **Quality selector.** Nightglass's Quality menu lists the video tracks mpv sees. Open decision: hand it
      several video-only formats as tracks (mpv opens each at load but reads only the chosen one), or the HLS
      master (H.264 and VP9 only, no AV1). The `quality` field stays as the cap.
- [ ] **Chapters.** `playback.chapters` (start in seconds, title), from `/next` (not `/player`; visionos returns
      none), shown by Nightglass on the seek bar, in a menu and with Page ↑ ↓.
- [ ] Audio-track polish: auto-dubbed languages last, an option for DRC audio.

## M6 - Login

- [ ] TV device-code sign-in with token refresh. Decided: login comes now, not earlier; public videos keep the
      anonymous path even when signed in.
- [ ] The sign-in code shown through the `action-required` prompt of contract 0.4.0 (message, address, code,
      `retry-in`), which the Sources screen shows and retries. Decided: not the `catalog.account.status` and
      placeholder-series workaround of SPEC 8.2.
- [ ] Brand accounts through `accounts_list` and `X-Goog-Pageid`.
- [ ] A logged-in TV client for age-gated, members-only and private videos.
- [ ] Watch Later, Liked, library playlists and subscriptions as series.
- [ ] Cookies (`SAPISIDHASH`) as the alternative.

## M7 - BotGuard spike (gated)

- [ ] Only if the decision tree of SPEC 7.6 reaches it: a token-gated client that returns plain URLs with a token,
      while the token-free ones no longer cover a case. Homepage challenge, DOM and navigator stubs, host timers,
      `GenerateIT`, mint, a GVS probe after each mint, a re-mint on failure. Decided: no BotGuard in v1.

## M8 - Catch-up and DVR

- [ ] `catchup-window` from `maxDvrDurationSec`.
- [ ] A synthesised `sq` playlist served through `relay.filter`, if a probe shows the segments play.

## Later

- [ ] Search, once the contract has a search export (the endpoints are in SPEC 4.9).
- [ ] A Shorts feed beyond the `shorts` tab of a channel.
- [ ] Subtitle translation (`tlang`) as extra tracks; Premium formats through a signed-in TV client.

## Not planned

- SABR/UMP playback. mpv cannot play it and the relay does not speak it; it would take an app-side bridge. Formats
  without a URL are dropped.
- Writing to YouTube: watch history, likes, playlist edits, watch-time reports.
- Comments and live chat.
- YouTube Music.
- DRM content.
