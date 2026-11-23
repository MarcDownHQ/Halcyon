# Chapters, quality variants and subtitles: the YouTube side

Clean-room notes for three player features. Facts only: no code, no pseudo-code, no identifiers from the sources. Protocol facts (JSON paths, enum values, query parameters, itags) are verbatim because they are facts about YouTube. This file adds to `SPEC.md` (sections 5.3 to 5.7 and 9) and does not repeat it; where it corrects or extends SPEC, it says so.

## Sources and provenance

| Tag | Source | Licence | Commit | Date |
|---|---|---|---|---|
| `ytdlp` | yt-dlp, `yt_dlp/extractor/youtube/_video.py`, `_base.py`, `extractor/common.py`, `YoutubeDL.py`, `utils/_utils.py` | Unlicense | `1507b26` | 2026-11-22 |
| `st` | SmartTube `a0c319a` (`common/.../exoplayer/selector/`, `common/.../misc/AppDataSourceManager.java`) + MediaServiceCore `bad6cb57` (`youtubeapi/.../next/v2/`, `.../innertube/utils/PlayerResultExtensions.kt`, `.../videoinfo/`, `.../formatbuilders/mpdbuilder/`) | MIT | see left | 2026-11-22 |
| `gj` | Grayjay YouTube plugin, `YoutubeScript.js` | Source First 1.1 (facts only) | `640bd86` | 2026-11-21 |
| `ytjs` | YouTube.js, `src/parser/classes/`, `src/utils/StreamingInfo.ts`, `src/utils/FormatUtils.ts`, `src/Innertube.ts` | MIT | `77703fa` | 2026-11-18 |
| `obs-a` | Earlier live probe, `specs/observations/2026-11-23-clients.md`, raw data not kept | - | - | 2026-11-23 09:51 to 10:52 UTC |
| `obs-b` | This research's live probe, raw data not kept | - | - | 2026-11-23 12:28 to 12:30 UTC |

Every fact is **current** at the commit above unless marked **(hist.)**.

**`obs-b` method.** 16 anonymous requests from the same residential IPv6 as `obs-a`, no cookies, no PO token, plain curl. The client shapes, user agents and `visitorData` were those of `obs-a` (WEB used `clientVersion` `2.20261119.07.00`, TVHTML5 used `7.20260707.07.00` with the Cobalt UA from `ytdlp` `_base.py`). Videos:
- `aircAruvnKk`: 3Blue1Brown, "But what is a neural network?", 1120 s, description chapters, dubbed audio.
- `LXb3EKWsInQ`: "COSTA RICA IN 4K 60fps HDR", HDR.
- `Unzc731iCUY`: MIT "How to Speak", multi-language audio.

No bot check appeared. The last request (a translated caption) returned **HTTP 429**, so probing stopped there (see 3.6).

**Strategy in one paragraph.**
- **Chapters** come from `/youtubei/v1/next`, never from `/player`. YouTube serves them in three shapes depending on the client and the date, so every source tries several paths in turn. `ytdlp` alone falls back to parsing the description.
- **Quality:** every source builds its own list from `adaptiveFormats`, keyed on height, fps, codec and HDR. `ytdlp` ranks by YouTube's `quality` string first. `st` lets the user pick a "preset" ceiling and switches live inside a DASH manifest it builds itself.
- **Subtitles:** all sources read `captions.playerCaptionsTracklistRenderer`. They differ on format (`ytdlp` all formats, `st` TTML or VTT, `gj` the default XML converted to VTT) and on translation.

---

## 1. Chapters

### 1.1 Where YouTube exposes them

Chapters are **not** in the `/player` response.
- **Observed** (`obs-b`): the visionos `/player` for `aircAruvnKk` had top-level keys `responseContext playabilityStatus streamingData playbackTracking captions videoDetails playerConfig storyboards trackingParams attestation adBreakHeartbeatParams`, and no key containing "chapter", "marker" or "heat".
- **Sources:** `ytdlp` reads chapters only from "initial data", which is the watch page's `ytInitialData`, else a `/next` call. `gj` and `st` read them from `/next` only.

The `/next` response carries chapters in up to **four places**, in three generations of shape.

**A. `playerOverlays`, the "markers map" (WEB, current).**
- Path: `playerOverlays.playerOverlayRenderer.decoratedPlayerBarRenderer.decoratedPlayerBarRenderer.playerBar.multiMarkersPlayerBarRenderer.markersMap[]`. The double `decoratedPlayerBarRenderer` nesting is real [obs-b, gj, st]. `gj` also accepts the single-level form, `decoratedPlayerBarRenderer.playerBar…`, as a fallback.
- **Entries:** each entry is `{key, value}`.
  - `key` values seen: `DESCRIPTION_CHAPTERS`, `AUTO_CHAPTERS` [gj], `QUIZ_MARKERS` and `ANIMATION_ANNOTATION_MARKERS` [obs-b].
  - **Order is not fixed.** On `aircAruvnKk` (WEB, obs-b) the order was `QUIZ_MARKERS`, `DESCRIPTION_CHAPTERS`, `ANIMATION_ANNOTATION_MARKERS`. So "take the first entry" (as `st` does) can pick the wrong one. Select by key.
- **Chapters:** `value.chapters[].chapterRenderer` with `title.simpleText`, `timeRangeStartMillis` (a number, ms), `thumbnail.thumbnails[]` and `onActiveCommand`. `value` also has `trackingParams` and `onChapterRepeat` [obs-b, ytjs].
- **Quiz markers:** `value.markers[].markerRenderer` with `title`, `timeRangeStartMillis` and `thumbnail`. These are not chapters [obs-b].
- `multiMarkersPlayerBarRenderer.visibleOnLoad.key` names the default entry (`DESCRIPTION_CHAPTERS`) [obs-b].
- **Preference:** `gj` takes `DESCRIPTION_CHAPTERS`, else `AUTO_CHAPTERS`.

**A'. `chapteredPlayerBarRenderer` (hist.).** `ytdlp` still tries `playerOverlays.playerOverlayRenderer.decoratedPlayerBarRenderer.decoratedPlayerBarRenderer.playerBar.chapteredPlayerBarRenderer.chapters[].chapterRenderer` first. The WEB response in `obs-b` did not have it, only `multiMarkersPlayerBarRenderer`. `ytdlp` still gets chapters because it falls through to B.

**B. Engagement panel "macro markers" (WEB, current).**
- Path: `engagementPanels[].engagementPanelSectionListRenderer.content.macroMarkersListRenderer.contents[].macroMarkersListItemRenderer` [ytdlp, st, ytjs, obs-b].
- The panel has `panelIdentifier` / `targetId` `engagement-panel-macro-markers-description-chapters` [obs-b]. Auto chapters plausibly use a different identifier; this was not observed.
- **Item fields:**
  - `title.simpleText`;
  - `timeDescription.simpleText`, a display string such as `0:00` or `1:07`, with `accessibility.accessibilityData.label` such as "0 seconds";
  - `thumbnail.thumbnails[]`;
  - `onTap.watchEndpoint.startTimeSeconds`, an integer number of seconds;
  - `trackingParams`, `carouselType`, `timeDescriptionA11yLabel`, `shareButton`.
  - `ytjs` also reads `layout` and `isHighlighted`.
- **Start time:** `ytdlp` parses the display string `timeDescription`. `st` uses `onTap.watchEndpoint.startTimeSeconds × 1000`. The integer field avoids parsing a localised string.
- **Not chapters:** `contents[]` can also hold `macroMarkersInfoItemRenderer` entries, a header or info row [ytjs]. Skip them.

**C. `frameworkUpdates` entity (TV, current).**
- Path: `frameworkUpdates.entityBatchUpdate.mutations[].payload.macroMarkersListEntity.markersList` where `markerType == "MARKER_TYPE_CHAPTERS"` [st, obs-b].
- **Markers:** `markers[]` with `title.simpleText`, `startMillis` (a decimal **string**), `durationMillis` (string), `thumbnailDetails.thumbnails[]`, `onActive` and `onTap` [obs-b, st].
- **TVHTML5 has only this shape** (obs-b on `aircAruvnKk`):
  - no `chapterRenderer` anywhere;
  - the TV `markersMap` was only `{visibleOnLoad:{key:<entity key>}, trackingParams}`;
  - the TV description panel used `macroMarkersListViewModel{macroMarkerListEntityKey, context:"MACRO_MARKERS_LIST_VIEW_MODEL_CONTEXT_DETAIL_VIEW", markersEngagementPanelSyncEntityKey}`, which points at the entity instead of inlining items.
- **Entity key:** `EhRERVNDUklQVElPTl9DSEFQVEVSUyCSAigB`, which is base64url of a small protobuf containing the string `DESCRIPTION_CHAPTERS`. So the auto/description distinction survives in the key, not in `markerType` [obs-b].
- **Same mutations list:** it also carries `MARKER_TYPE_HEATMAP` and `MARKER_TYPE_TIMESTAMPS` (the quiz markers) entities, on both WEB and TV [obs-b].

**D. The description (fallback, `ytdlp` only).** See 1.4.

**Per-client summary for `/next`** (obs-b, `aircAruvnKk`, logged out):

| Client | A (`markersMap` chapters) | B (panel items) | C (`MARKER_TYPE_CHAPTERS`) | Heatmap entity |
|---|---|---|---|---|
| WEB `2.20261119.07.00` | yes (12) | yes (12) | no | yes |
| TVHTML5 `7.20260707.07.00` | no (key reference only) | no (view-model reference only) | yes (12) | yes |
| VISIONOS `1.02` | no | no | no | no |

### 1.2 The `/next` request

- **Endpoint:** `POST https://www.youtube.com/youtubei/v1/next?prettyPrint=false`. The headers are those of `/player` (SPEC 3.1).
- **Minimal body:** `{"context":{"client":{…}},"videoId":"<id>","contentCheckOk":true,"racyCheckOk":true}` [ytdlp]. This worked in obs-b for WEB, TVHTML5 and VISIONOS.
- **Extra fields some sources send:**
  - `ytjs`: `playlistId`, `playlistIndex`, `params` (from a navigation endpoint), plus `playbackContext.contentPlaybackContext{vis:0, splay:false, lactMilliseconds:"-1", signatureTimestamp}`;
  - `gj`: `playbackContext.contentPlaybackContext{lactMilliseconds:"-1", vis:0}`, plus `Accept-Language: en-US` and `Cookie: PREF=hl=en&gl=US`;
  - `st`: `videoId` (empty when loading by playlist index), `playlistId`, `playlistIndex` and `params`, with `X-Goog-Visitor-Id`, on its **TV** client [st `WatchNextService`].
- **Clients used:** `ytdlp` uses its webpage client, `web` by default (`_DEFAULT_WEBPAGE_CLIENT`), and prefers the watch page's embedded `ytInitialData` when it has `contents`. `gj` uses the WEB context taken from the homepage. `st` uses TV. `ytjs` uses the session client (WEB by default).
- **Logged out:** chapters and the heatmap are returned to logged-out requests. `responseContext.serviceTrackingParams[].params` had `logged_in=0` for WEB and TV [obs-b], and `st`'s 2023 test fixture with engagement-panel chapters was also `logged_in=0`.
- **Response size:** about 750 KB for WEB, 225 KB for TV and 457 KB for VISIONOS on one video [obs-b]. The WEB response is large because of comments, related videos and the onesie configuration. Fetch it once per play, never per catalogue item.
- **For a TV-only plugin:** TVHTML5 needs no JS for `/next`, so a plugin that already avoids WEB can read path C.

### 1.3 Auto-generated chapters and key moments

- **Auto chapters:** `markersMap` key `AUTO_CHAPTERS` [gj]. `gj` uses them only when `DESCRIPTION_CHAPTERS` is absent. None of the sources has a fixture, and obs-b did not find an auto-chaptered video, so their TV/entity form is unverified (see open questions).
- **Heatmap ("most replayed"), current shape** [ytdlp, ytjs, obs-b]:
  - Path: `frameworkUpdates.entityBatchUpdate.mutations[].payload.macroMarkersListEntity` with `markersList.markerType == "MARKER_TYPE_HEATMAP"`.
  - `markersList.markers[]` has `startMillis` (string), `durationMillis` (string) and `intensityScoreNormalized` (a float from 0 to 1).
  - `markersList.markersMetadata.heatmapMetadata{maxHeightDp:40, minHeightDp:4, showHideAnimationDurationMillis:200}`.
  - `markersList.markersDecoration.timedMarkerDecorations[]` has `visibleTimeRangeStartMillis`, `visibleTimeRangeEndMillis`, `decorationTimeMillis` and `label.runs[0].text` = "Most replayed". These are the "key moments" peaks.
  - **Observed:** exactly 100 markers of equal duration (11200 ms × 100 = 1120 s, the video length); the last marker had intensity 0 [obs-b].
  - The entity also has `key` and `externalVideoId` [ytjs].
- **Heatmap, `ytjs` (hist.) shape:** a `Heatmap` node with `heatMarkers[]` inside the `markersMap` value under key `heatmap`, which `ytjs` still parses.
- **How the sources use it:** `ytdlp` turns each heat marker into `{start = startMillis/1000, end = (startMillis + durationMillis)/1000, value = intensityScoreNormalized}`. Neither `gj` nor `st` uses the heatmap.
- **Quiz markers** (`MARKER_TYPE_TIMESTAMPS` / `QUIZ_MARKERS`) are point events with `startMillis` and `durationMillis` [obs-b]. No source uses them.

### 1.4 Description timestamps (`ytdlp` only)

`ytdlp` parses the description only when both JSON paths gave nothing. The description is `videoDetails.shortDescription`.

- **Line rule:** a line is a chapter if it starts with a timestamp, then a word boundary, any non-word characters (a dash, a bullet, a parenthesis), whitespace and the title. If no line matches that way round, it tries the reverse: the title, then a timestamp at the end of the line.
- **Timestamp shape:** an optional run of hour digits and a colon, one or two minute digits, a colon, and exactly two second digits. So `M:SS`, `MM:SS`, `H:MM:SS` and `HH:MM:SS` all match.
- **Lines:** leading and trailing whitespace is ignored, and the match is anchored per line.
- **Edge cases handled:**
  - Timestamps in the middle of a line are ignored. `aircAruvnKk` has "Correction 14:45 - The final index…" [obs-b], which is correctly not a chapter.
  - Description chapters are **sorted** by start time; JSON chapters are not (see 1.5).
  - A start past the video duration is dropped.
  - Without a known duration, no chapters are returned at all.
- **Not handled:**
  - timestamps in brackets at line start (the word-boundary rule rejects `[`);
  - `1h02m03s` styles;
  - the bare-seconds form.
- **YouTube's own rule for when description timestamps become chapters** (first timestamp 0:00, at least three, each at least 10 s) is not encoded in any source. When the rule is met, YouTube itself returns path A/B/C, so the fallback mostly matters for descriptions YouTube rejected.

### 1.5 Turning markers into a chapter list (how the sources do it)

- **`ytdlp`:**
  - Builds `{start_time, title}` for each entry.
  - Walks them in response order. It keeps an entry only if its start is at or after the previous kept start and at or before the duration. Others are dropped with a warning, and an entry with no start is dropped.
  - A final pass (common to all extractors):
    1. inserts an untitled chapter at 0 if the first one starts later;
    2. sets each chapter's end to the next chapter's start, and the last one's end to the video duration;
    3. names untitled chapters "<Untitled Chapter N>".
- **`gj`:**
  - Keeps `chapterRenderer` entries that have a title and a `timeRangeStartMillis` that is truthy or exactly 0.
  - Converts to whole seconds by truncation.
  - The end is the next chapter's start, or a sentinel of 999999 s for the last.
  - It merges SponsorBlock segments into the chapters, which is out of scope here.
  - For `/clip/` URLs it builds three synthetic chapters from the watch page's `clipConfig.startTimeMs`/`endTimeMs`.
- **`st`:** takes path A (first `markersMap` entry), else B, else C. It reads title, start in ms and thumbnail; on C it also has `durationMillis`. In obs-b, the TV C entries' last `startMillis + durationMillis` was exactly the video length (1023000 + 97000 = 1120000).
- **`ytjs`:** exposes the raw nodes (`player_overlays`, `engagement_panels`, `heat_map`) and builds no list.

**Fields a plugin can fill, and where from:**
- **Start (ms):** `chapterRenderer.timeRangeStartMillis` (number), `startMillis` (string, C), or `onTap.watchEndpoint.startTimeSeconds × 1000` (B).
- **Title:** `title.simpleText` everywhere. Treat `runs[]` as a fallback, since `ytjs`/`st` accept both.
- **End:** only C gives `durationMillis`; otherwise use the next start, or the duration (`videoDetails.lengthSeconds`) for the last chapter.

### 1.6 Thumbnails per chapter

- **Where:** each chapter carries its own still.
  - A and B: `thumbnail.thumbnails[]`, observed at 168×94 and 336×188.
  - C (TV): `thumbnailDetails.thumbnails[]`, observed one at 640×360.
  - [obs-b, st, ytjs]
- **URL form:** `https://i.ytimg.com/vi/<id>/hqdefault_<ms>.jpg?sqp=…&rs=…`. `<ms>` is a frame time near, but not equal to, the chapter start: for example 162300 for a chapter at 162000, and 88833 for one at 67000. `sqp`/`rs` look like signed resize parameters, so keep the URL whole [obs-b].
- **Choice:** `st` picks the "optimal resolution" entry of the list.

---

## 2. Quality variants

### 2.1 Format fields that matter (beyond SPEC 5.3)

Observed on visionos `adaptiveFormats` [obs-a raw files, obs-b]:
- **Video keys:** `itag url mimeType bitrate averageBitrate width height fps quality qualityLabel qualityOrdinal colorInfo projectionType initRange indexRange contentLength approxDurationMs lastModified highReplication`.
- **Audio keys:** the same, plus `audioQuality audioSampleRate audioChannels loudnessDb trackAbsoluteLoudnessLkfs isDrc xtags audioTrack`.

The fields one by one:
- **`quality`:** one of `tiny small medium large hd720 hd1080 hd1440 hd2160 hd2880 highres`. Audio formats also say `tiny` [obs-b]. `ytdlp` orders them in that sequence, with `audio_quality_ultralow/low/medium/high` placed between `tiny` and `small`. For audio, use `audioQuality` (`AUDIO_QUALITY_LOW`/`MEDIUM`/`HIGH`).
- **`qualityLabel`:**
  - It is `<height>p`, then the fps when above 30 (`2160p60`, `720p60`), then ` HDR` for HDR formats (`2160p60 HDR`) [obs-b]. YouTube's own label, ready for display.
  - Premium formats contain the word "Premium" [ytdlp: a format whose label has "Premium" gets a large source bonus].
  - **The label is display text, not the frame size.** Portrait and ultrawide videos label by the shorter side. `st` recomputes labels from `width`/`height`: it buckets the height into the standard tier when the aspect is under 2:1, and buckets the width when the aspect is 2:1 or wider. It adds "2K", "4K" or "8K" prefixes for 1440, 2160 and 4320. Its examples are 2560×1182, 2560×1088 and 2560×1066 videos that should read as 1440p.
- **`qualityOrdinal`:** `QUALITY_ORDINAL_144P` … `QUALITY_ORDINAL_2160P` [obs-b]. No source reads it; it is a cleaner sort key than parsing labels (open question for higher tiers).
- **`fps`:** typically 24, 25, 30, 50 or 60, and low-resolution VP9 can be 12 or 15 [obs-a HLS]. `ytdlp` treats `fps <= 1` as missing ("wrongly returned as 1").
- **`bitrate` vs `averageBitrate`:** `bitrate` is the peak. `ytdlp` uses `averageBitrate`, else `bitrate`, for its `tbr`; `gj` and `ytjs` use `bitrate`.
- **`highReplication: true`:** present on some formats (`134`, `140`) [obs-b]. No source uses it.
- **`colorInfo`:**
  - Shape: `{primaries, transferCharacteristics, matrixCoefficients}` with the enum values `COLOR_PRIMARIES_BT709`/`_BT2020`, `COLOR_TRANSFER_CHARACTERISTICS_BT709`/`_BT2020_10`/`_SMPTEST2084` (PQ)/`_ARIB_STD_B67` (HLG), and `COLOR_MATRIX_COEFFICIENTS_BT709`/`_BT2020_NCL` [ytjs, obs-b].
  - Present on VP9 and AV1 formats, **absent on AVC** [obs-b; ytjs: "h264 streams always seem to be SDR"].
  - `ytjs` maps them to H.273 code points for DASH (primaries 1/9, transfer 1/14/16/18, matrix 1/14). It puts formats with different `colorInfo` in different adaptation sets, because "HDR videos have both SDR and HDR vp9 formats".

### 2.2 Itags and codecs seen (2026-11-23)

| Kind | Itags | `mimeType` codecs |
|---|---|---|
| AVC (H.264) | 160 (144p), 133 (240p), 134 (360p), 135 (480p), 136 (720p), 298 (720p60), 137 (1080p), 299 (1080p60) | `avc1.4D400C` … `avc1.640028`/`64002A`. **No AVC above 1080p.** |
| VP9 SDR | 278, 242, 243, 244, 247, 302 (720p60), 248, 303 (1080p60), 271/308 (1440p, p60), 313/315 (2160p, p60) | `vp9` |
| VP9 HDR (VP9.2) | 330 to 337 (144p to 2160p, 60 fps) | `vp9.2` |
| AV1 SDR | 394 to 401 (144p to 2160p) | `av01.0.00M.08` … `av01.0.13M.08` (8-bit) |
| AV1 HDR | 694 to 701 (144p to 2160p) | `av01.0.13M.10.0.110.09.16.09.0` (10-bit, BT.2020, PQ) |
| AAC | 139 (HE-AAC `mp4a.40.5`, ~48 kbps), 140 (AAC-LC `mp4a.40.2`, ~128 kbps) | |
| Opus | 249 (~50 kbps), 250 (~65 kbps), 251 (~130 kbps) | `opus` |
| Premium (1080p VP9 high-bitrate) | DASH 356, HLS 616 | `vp09.00.40.08` (see 2.5) |

- **Sources for the table:**
  - itags and codecs: [obs-a, obs-b];
  - HDR itag ranges 330 to 337 (webm) and 694 to 701 (mp4): [st];
  - VP9.2 and AV1 profile-suffix detection: [st, ytdlp];
  - 616 and 235 as "Premium": [ytdlp].
- **HDR detection, three ways:**
  - **itag range** [st];
  - **codec string** [st, ytdlp]:
    - `st` counts as HDR exactly `vp9.2`, an AV1 string ending in `10.0.110.09.18.09.0` (HLG) or `10.0.110.09.16.09.0` (PQ), or one ending in `hdr`;
    - `ytdlp` counts VP9 profile 2 or AV1 with 10-bit depth as "HDR10", and `dvh1`/`dvhe` as Dolby Vision;
  - **`colorInfo.transferCharacteristics`** being `SMPTEST2084` or `ARIB_STD_B67` [ytjs, obs-b]. This is the only one that tells PQ from HLG directly.
- **Shape of an HDR video** (`LXb3EKWsInQ`, obs-b):
  - every tier from 144p to 2160p is offered three times: VP9 SDR, VP9.2 HDR and AV1 HDR, with no AV1 SDR;
  - the HDR tiers are all 60 fps;
  - the SDR tiers below 720p are 30 fps.
- **Super-resolution:** an "AI-upscaled" format is marked by `xtags` containing `sr=1`, and gets the format id suffix `-sr` [ytdlp]. SPEC already says to keep it opt-in.

### 2.3 How the sources rank and present a list

**`ytdlp`:**
- **Sort key**, in order:
  1. YouTube's `quality` tier, minus a half step for DRC audio;
  2. resolution;
  3. fps;
  4. HDR, ranked DV > HDR12 > HDR10+ > HDR10 > HLG > SDR, but capped at "12" by default;
  5. source preference (damaged formats, itag 22 and SABR-missing-POT formats are pushed down; Premium is pushed up);
  6. video codec;
  7. channels, audio codec, size, bitrate, sample rate, protocol and the rest.
- **Codec order:** AV1 > VP9.2 > VP9 > H.265 > H.264 > VP8, with an opt-in to prefer VP9.2.
- **Audio codec order:** FLAC > WAV > Opus > Vorbis > AAC > MP4A > MP3.
- **Disagreement with SPEC 5.5:** SPEC proposes VP9 > AVC > AV1 for decoder cost. `ytdlp` prefers AV1, and `gj` drops AV1 entirely.
- **Display:**
  - The format note joins the audio-track display name ("(default)" if `audioIsDefault`), `qualityLabel`, "DRC", "AI-upscaled", the projection, the spatial-audio type and "DAMAGED"/"MISSING POT".
  - The format id is the itag, with `-drc`/`-sr` suffixes. Duplicates by (itag, protocol, language) are dropped.
- **Pairing:** the default selection is the best video-only plus the best audio-only, else the best muxed. With multiple audio streams disabled, one audio is picked by language preference (original first, SPEC 5.6).

**`gj`:**
- Lists one video source per video-only adaptive format, named `<height>p<fps> <container>` (for example "1080p60 video/webm").
- Drops AV1 ("unsupported") and formats missing `initRange`/`indexRange` end values.
- Lists audio sources separately, named after the audio track display name or the codec, plus " [Normalized]" for DRC, and flagged `original`.
- The app's player pairs them and owns the ranking, so the plugin does no sorting.

**`ytjs`:**
- **Format choice:**
  - Filters by type (video, audio or both), codec substring and container (`mp4` by default).
  - A quality is either an exact `qualityLabel` match or `best`/`bestefficiency`. `best` keeps only the widest formats and takes the highest `bitrate`; `bestefficiency` takes the lowest.
  - Audio-only selection prefers the original track (from `xtags` `acont`) or a named language.
- **DASH output:** one adaptation set per (mime type, codec family, colour info, audio track id, DRC, voice-boost), so the player can switch inside a set.

**`st` (TV player), its quality model:**
- **The manifest:** MediaServiceCore turns `adaptiveFormats` into a **DASH MPD** (or uses SABR, `dashManifestUrl` or HLS for live).
  - One adaptation set per container for video (mp4, webm).
  - One per container and language for audio.
  - Plus subtitle sets.
  - Representations are sorted by height, then bitrate, descending. Their id is the itag, or `<itag>-drc` [st `MediaFormatComparator`, `YouTubeMPDBuilder`].
- **Presets:** the user picks from fixed `width,height,fps,codec` combinations. Codecs are `avc`, `vp9`, `vp9.2`, `av01` and `av01.hdr`, from 144p30 up. AVC presets stop at 1080p; HDR presets start at 720p (AV1) or 1080p (VP9) [st `AppDataSourceManager`].
- **Matching a preset:** the preset is a ceiling. The selector picks the best track with size ≤ preset, within a 5 % size tolerance, and fps ≤ preset. HDR is taken only if the preset is HDR, and a non-HDR preset never picks HDR. Without a preset it compares directly [st `VideoTrack`].
- **"Auto":** clears the override and lets ExoPlayer's adaptive selection run.
- **Switching:** happens **in place** by changing ExoPlayer's track-selection override on the already-loaded manifest. There is no new `/player` call and no reload [st `TrackSelectorManager`].
- **Labels:** resolution label, then fps ("60fps"), bitrate ("4.33Mbps"), short codec (avc, vp9, av1), "HDR" and a high-bitrate mark. Audio labels give channels, sample rate, bitrate, codec and a DRC mark.

### 2.4 Pairing audio with video (beyond SPEC 5.6)

- **Container pairing is not required for mpv.** `ytdlp` muxes any pair through ffmpeg, and `st`/`gj` keep audio and video as separate DASH sets.
- **Loudness:** `loudnessDb` and `trackAbsoluteLoudnessLkfs` are on every audio format [obs-b]. HLS exposes the same value as `YT-EXT-ABSOLUTE-LOUDNESS` (2.5). No source applies them.
- **One DRC flag, two encodings:** `isDrc: true` with `xtags=CggKA2RyYxIBMQ` (protobuf for `drc=1`) [obs-b, ytjs].

### 2.5 HLS from the token-free clients (`visionos`, `ios_producer`)

**Facts from master playlists** fetched logged out: obs-a for `e8FLcGEXsO0` (visionos and ios_producer), `aqz-KE-bpKQ` (visionos) and `dQw4w9WgXcQ` (web_safari, for comparison); obs-b for `LXb3EKWsInQ` and `Unzc731iCUY` (visionos).

- **Header and URLs:**
  - Header: `#EXTM3U`, `#EXT-X-INDEPENDENT-SEGMENTS`.
  - The master URL and every child URL are on `manifest.googlevideo.com/api/manifest/hls_playlist/…` (path-style parameters).
  - Media playlists are `#EXT-X-VERSION:6`, `#EXT-X-PLAYLIST-TYPE:VOD`, `#EXT-X-TARGETDURATION:8` with an `#EXT-X-MAP` init, and segments of about 3.2 s [obs-b, itag 616].
  - A segment URL answered **302** with a `Location` on an `rrN---sn-….googlevideo.com/videoplayback/…` host, then 200 with media [obs-b]. **Follow redirects.**
- **Video variants are muxed-reference variants.** Each `#EXT-X-STREAM-INF` lists the video codec and an audio codec in `CODECS`, and points at a separate audio rendition group with `AUDIO="233"` or `AUDIO="234"`.
- **Variant attributes:** `BANDWIDTH`, `CODECS`, `RESOLUTION`, `FRAME-RATE`, `VIDEO-RANGE` (`SDR` or `PQ`), `YT-EXT-ABSOLUTE-LOUDNESS`, `AUDIO`, `SUBTITLES="vtt"` (when captions exist) and `CLOSED-CAPTIONS=NONE`.
- **HLS itags differ from DASH itags.** The variant URL path has `/itag/<hls itag>/` and an `sgovp/` parameter whose value carries the underlying DASH itag:
  - `229` → 133;
  - `270` → 137;
  - `614` → 248;
  - `616` → 356.
  - Audio uses `sgoap/`: `233` → 139 (HE-AAC), `234` → 140 (AAC-LC). Group 233 is used for the lowest tiers, `234` for the rest.
  - `ytdlp` reads the itag from the `/itag/` segment.

**What visionos lists:**
- **Tiers:**
  - every AVC tier up to 1080p (avc1 up to `640028`/`64002A`);
  - every VP9 SDR tier up to the source maximum (up to 2160p60, `vp09.00.51.08`);
  - **VP9 HDR PQ** variants on HDR videos (`vp09.02.xx.10.01.09.16.09.00`, `VIDEO-RANGE=PQ`, 144p to 2160p, 9 variants on `LXb3EKWsInQ`).
- **Not listed:** **AV1** variants (on any video, including the HDR one) and HDR AVC.
- **Variant counts:** 26 on `LXb3EKWsInQ` (7 AVC SDR, 10 VP9 SDR, 9 VP9 PQ) and 16 on `Unzc731iCUY`.
- **itag 616:**
  - **Present, logged out:** visionos HLS on 1080p videos lists **two VP9 1080p variants**: itag 614 (DASH 248, about 2.9 Mbit/s) and itag **616** (DASH 356, about 5.7 Mbit/s). `ytdlp` labels 616 as "Premium".
  - **Served:** its media playlist and first segment downloaded with HTTP 200 and no token [obs-b, `Unzc731iCUY`; obs-a `e8FLcGEXsO0` also showed two 1080p VP9 variants].
  - This contradicts SPEC open question 20, which assumes 616 needs a logged-in TV client.

**What ios_producer lists:** AVC only, up to 1080p, plus the two AAC groups and the subtitle group. It does so even though its `/player` has **no** `captions` block [obs-a].

**Audio renditions:**
- **Single-audio videos:** two `#EXT-X-MEDIA:TYPE=AUDIO` entries, `GROUP-ID="233"` and `"234"`, `NAME="Default"`, `DEFAULT=YES`, `AUTOSELECT=YES`, no `LANGUAGE`.
- **Multi-language videos** (`Unzc731iCUY`, visionos):
  - one entry **per language per group** (3 languages × 2 groups);
  - attributes: `LANGUAGE="es-US"`, `NAME="es-US - dubbed"` / `"Português (Brasil) - dubbed"` / `"American English - original"`, `YT-EXT-XTAGS` (base64url protobuf of `acont=<original|dubbed>` and `lang=<code>`), `YT-EXT-AUDIO-CONTENT-ID="en-US.4"` (the same id as `audioTrack.id`) and `YT-EXT-ABSOLUTE-LOUDNESS`;
  - **every one with `DEFAULT=NO`**, so the original must be picked from `NAME`/`YT-EXT-XTAGS`, not `DEFAULT`.
  - ios_producer offers only the original track (obs-a, `/player`).
- **Never in HLS:** Opus and DRC audio. There are **no audio-only variants** (`#EXT-X-STREAM-INF` without video) in any master seen.

**Subtitle renditions:**
- **Shape:** `#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="vtt",LANGUAGE=…,NAME=…,DEFAULT=NO,AUTOSELECT=YES`, with URI `https://manifest.googlevideo.com/api/manifest/hls_timedtext_playlist/…/tts_params/<urlencoded query>/…`.
- **`tts_params`:** a signed query with `caps=asr`, `v`, `lang`, `ip`, `ipbits`, `expire`, `sparams`, `signature` and `key=dg_yt0`, plus `name` for named tracks (for example `name=CC (English)`).
- **Only manual tracks are listed.** On `dQw4w9WgXcQ` the master listed en, de-DE, ja, pt-BR and es-419 but not the `a.en` ASR track. On `Unzc731iCUY` it listed zh, zh-Hans, en, pt-BR and es-US but not `a.en`. No translated tracks appear.
- **`NAME`:** the localised track name, for example "English CC (English)" or "Español (Estados Unidos)".
- **Not done:** whether the subtitle playlist and its VTT segments play without a token. No source fetches it; `ytdlp` merges them as subtitles and notes it does not know whether they need `pot`.
- **No subtitle group on `LXb3EKWsInQ`**, which has no captions.

**What `ytdlp` says about HLS:**
- It deprioritises web_safari and web_embedded HLS on VOD because "pre-merged m3u8 formats may have lower quality audio streams". The cap is AAC 128 kbps (itag 140); there is no Opus.
- It rewrites the manifest path suffix `/file/index.m3u8` or `/playlist/index.m3u8` when adding a PO token.
- HLS gets a "maybe DRM" flag when `drmFamilies` is present.

**Consequence for quality switching.** One HLS master gives mpv a ready variant list. mpv can choose among its variants by `hls-bitrate` or by track. But the HLS ladder caps at AVC 1080p / VP9, has no AV1, and its audio is only AAC. The `adaptiveFormats` route (SPEC 5.5) reaches AV1, Opus and DRC.

### 2.6 Live

Live variants come only from HLS (SPEC 9). Every live `/player` was refused in obs-a, so no live master was inspected for this file.

---

## 3. Subtitles

### 3.1 Tracks (beyond SPEC 5.7)

From `captions.playerCaptionsTracklistRenderer` [obs-a, obs-b]:
- **`captionTracks[]`:** fields `baseUrl`, `name` (`runs[0].text` or `simpleText`), `vssId`, `languageCode`, `kind` (`asr`, or absent), `isTranslatable` and `trackName` (empty string seen).
  - `ytjs` also types `kind: 'frc'`, which was never seen; its meaning is unknown.
- **`vssId` grammar:**
  - `.<lang>` for a plain manual track;
  - `.<lang>.<suffix>` for a named manual track (`.en.j3PyPqV-e1s`, named "English CC");
  - `a.<lang>` for ASR.
  - `ytdlp` derives its language key from `vssId` with the leading dot removed and further dots turned into hyphens (`en-j3PyPqV-e1s`), else from `languageCode`. This keeps two English manual tracks apart.
- **Track order in the response:** alphabetical by English language name. The ASR track sits right after its language's manual track (`.en`, then `a.en`, then `.fil` on `aircAruvnKk`) [obs-b].
- **`audioTracks[]`** (per audio language): `audioTrackId` (for example `en-US.4`, `fr.3`), `captionTrackIndices[]`, `defaultCaptionTrackIndex`, `hasDefaultTrack`, `captionsInitialState` and, per `ytjs`, `visibility`.
  - **`captionTrackIndices` order:** manual tracks first, with **ASR moved to the end** (`[0,1,…,6,8,…,30,7]`) [obs-b].
  - **`captionsInitialState`:** `CAPTIONS_INITIAL_STATE_ON_RECOMMENDED` for dubbed audio tracks and `…_OFF_RECOMMENDED` for the original [obs-b]. That is YouTube's own "turn subtitles on when listening to a dub" hint.
  - `defaultAudioTrackIndex` points into `audioTracks`.
- **ASR `baseUrl`:** carries `kind=asr` and, on `aircAruvnKk`, `variant=gemini` [obs-b].
- **Dub transcripts:**
  - **What they are:** auto-dubbed videos have an extra ASR track per dub language, with `variant=timing-optimized` [st].
  - **For translation:** `st` skips them as the source of translations.
  - **As tracks:** they show up "when the response doesn't contain the dubbed audio (e.g. android_vr)".
- **Query keys on every caption `baseUrl`** (obs-b, visionos): `v caps opi xoaf xowf hl ip ipbits expire sparams signature key lang`, plus `kind` (and `variant`) for ASR. **No `exp`** key was present, so the `xpe`/`xpv` gate (SPEC 5.7) was not active.

### 3.2 Which clients return which lists

Observed [obs-a raw files]:
- **Full track list:** visionos, web_safari, mweb and tv_samsung all return the full `captionTracks` list.
- **`translationLanguages`:** visionos, web_safari and mweb return **156** entries. tv_samsung returns **18** (ar, zh-Hant, nl, en, fr, de, hi, id, it, ja, ko, pt, ru, es, th, tr, uk, vi) plus `defaultTranslationSourceTrackIndices`.
- **No captions:** ios_producer has no `captions` block at all, although its HLS master has subtitles (2.5).
- **Disagreement:** `ytdlp` says "only web/mweb clients provide translationLanguages", and so takes the list from any response that has it. That is stale for visionos (obs-a/b).

### 3.3 Formats

Values of `fmt`:
- **The `ytdlp` set:** `json3`, `srv1`, `srv2`, `srv3`, `ttml`, `srt` and `vtt`, all offered for every track.
- **`st`:** `ttml` (the current Kotlin path's default) or `vtt` (the older Java path's default, which the user can toggle to TTML). SPEC 5.7 says `st` uses VTT; that holds only for the older path.
- **`gj`:** sends no `fmt` at all and converts the result itself.

What each format looks like (obs-b, all HTTP 200, no `pot`):
- `fmt=vtt` → `text/vtt`: `WEBVTT`, then `Kind: captions` and `Language: en` headers, then plain cues.
- `fmt=srv3` → `text/xml`: `<timedtext format="3"><body><p t="<ms>" d="<ms>">…</p>`.
- `fmt=json3` → `application/json`, with:
  - `wireMagic:"pb3"`, `pens`, `wsWinStyles` and `wpWinPositions`;
  - `events[]` with `tStartMs`, `dDurationMs` and `segs[]{utf8, tOffsetMs}`, which gives **word-level timing** for ASR.
  - The first event is a window definition with no `segs`.
- **No `fmt`** → `text/xml`: `<transcript><text start="<s>" dur="<s>">…</text>` (srv1). Entities are double-escaped (`&amp;#39;`). `gj` parses this, ends each cue at the next cue's start when they overlap, and writes VTT.
- **`xosf`:** `ytdlp` removes `xosf` because `xosf=1` adds unwanted positioning to vtt, json3 and srv* output. No `xosf` was in obs-b URLs.
- **ASR `vtt` cue shape** (obs-a sample `captions/mweb-Unzc731iCUY-a.en.vtt`):
  - every cue has `align:start position:0%`;
  - cues alternate between a "live" cue (a blank first line, then words with inline `<hh:mm:ss.mmm><c> word</c>` timestamps) and a 10 ms cue that repeats the finished line above a blank line.
  - So mpv shows rolling, duplicated lines. Converting from json3 or srv3 avoids this. Manual-track VTT is plain cues.

### 3.4 Naming and ordering in the sources

- **`ytdlp`** keeps two dictionaries:
  - **Subtitles (manual):** keyed by the `vssId`-derived code, named by `name`.
  - **Automatic captions:**
    - the ASR track under its language;
    - every translation, under the target code (for manual tracks, `<target>-<source>`, named "<target name> from <source name>");
    - the ASR original **also** under `<lang>-orig`, named "<name> (Original)". " (auto-generated)" is stripped from ASR names.
  - **Spoken language:** the ASR language sets the spoken language of audio formats that have none.
  - **Languages outside `translationLanguages`** (for example Cantonese `yue`) are still added from the ASR track itself.
- **`st`:**
  - appends one translated track per `translationLanguages` entry, built on a chosen ASR origin track, marked as auto;
  - suffixes auto track names with `*`;
  - matches the user's preferred language by name prefix, preferring a non-auto track over an auto one.
- **`gj`:** lists manual and ASR tracks with `name`, `languageCode` and `text/vtt`, and offers no translation.

### 3.5 PO token and `pot` per client

- **`ytdlp`:**
  - The subtitles PO-token policy is **not required** for every client, including visionos and ios_producer (one default, never overridden; "in rollout, currently detected via experiment; Premium users DO require a PO Token for subtitles").
  - A track is treated as needing a token only when its URL's `exp` includes `xpe` or `xpv`.
  - When it has a token it appends `pot=<token>&potc=1&c=<clientName>`.
  - Without one, gated tracks of that client are skipped.
- **`gj`:** appends `pot` (plus `potc=1`, `c` and `cver` when minted through BotGuard). Without a token it fetches plainly.
- **`st`:** appends `&pot=<token>&c=<client>` when it has a token.
- **Observed today (obs-b, visionos URLs, logged out, no `pot`):** manual `vtt`, `srv3`, the default XML and ASR `json3` all returned 200 with content. obs-a also fetched `fmt=vtt` captions from visionos and mweb with no token.

### 3.6 Translated tracks (`tlang`)

- **The mechanism:** append `tlang=<code>` to an origin track's URL [ytdlp, st].
  - Never set it to the track's own language (SPEC 5.7).
  - `ytdlp` omits `tlang` when the target equals the URL's `lang`.
- **Rate limit (obs-b):** `fmt=vtt&tlang=fr` on the ASR track of `aircAruvnKk` returned **HTTP 429** with Google's "Sorry… your computer or network may be sending automated queries" HTML page. The same IP had just fetched four untranslated formats fine. The IP was flagged earlier the same day (obs-a), so this does not prove translations are always limited. It does show translation is the first caption call to be refused. No source handles 429 on subtitles.
- **Recommendation for the spec:** keep translation out of v1 (as SPEC does) and never pre-fetch translations.

---

## Open questions

1. **Auto chapters in the TV/entity shape.** `AUTO_CHAPTERS` is known only from `gj`'s key list. Does path C use a different `markerType` or only a different entity key for auto chapters, and does path B have a separate panel identifier? It needs a probe on an auto-chaptered video.
2. **Does `/next` on VISIONOS ever carry chapters?** It returned none for a video with description chapters. If the plugin uses visionos for `/player`, it needs a second client (WEB or TVHTML5) for `/next`. TVHTML5's 225 KB response is the lighter one.
3. **`chapteredPlayerBarRenderer`.** Is it dead everywhere, or still served to some client or region? `ytdlp` tries it first.
4. **`qualityOrdinal` values above 2160P** (4320p, `highres`) and for Premium formats: unseen.
5. **itag 616 logged out.** It was served on visionos HLS today. Is that stable, region-dependent, or an experiment? Is DASH itag 356 also in `adaptiveFormats` for visionos? It was not checked; the HLS master showed it, the adaptive list was not compared.
6. **HLS subtitle playlists** (`hls_timedtext_playlist`): do they play in mpv without a token, and is their VTT segmented or whole? Not fetched by any source or probe.
7. **HDR in HLS:** only VP9 PQ was seen. Are HLG videos listed with `VIDEO-RANGE=HLG`, and does AV1 ever appear in an HLS master?
8. **`tlang` 429:** is it per-IP reputation (the IP was flagged earlier) or a general limit on translated timedtext for anonymous clients? Retest on a clean IP.
9. **`kind: "frc"`** in `ytjs`'s caption type: what is it, and should it be shown?
10. **`variant=gemini` on ASR tracks:** a new ASR model tag. Does it change the cue format?
11. **YouTube's rule for description chapters** (0:00 first, at least 3, at least 10 s each) is not in any source. Should the plugin's own description fallback enforce it, to match what YouTube shows?
