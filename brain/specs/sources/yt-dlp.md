# Source spec: yt-dlp (YouTube extractor)

## Source, licence, commits

- **yt-dlp**, https://github.com/yt-dlp/yt-dlp, licence: Unlicense (public domain dedication).
  Commit read: `1507b26936b46d8613b3df197184a1fa1530afac` (2026-11-22). Shallow history fetched back to 2026-03-01 to date churn.
  Area read: `yt_dlp/extractor/youtube/` (`_base.py`, `_video.py`, `_tab.py`, `_redirect.py`, `_search.py`, `jsc/`, `pot/`), plus `yt_dlp/downloader/http.py`, `yt_dlp/downloader/common.py`, `README.md` (extractor-args section).
- **yt-dlp-ejs** (EJS challenge solver), https://github.com/yt-dlp/ejs, licence: Unlicense.
  Commit read: `d48c6303a5a6455a42ef9e0b445149cadea51279` (2026-10-26). yt-dlp@1507b26 vendors EJS script version **0.8.0** (`jsc/_builtin/vendor/_info.py`).
- **yt-dlp wiki**, https://github.com/yt-dlp/yt-dlp/wiki, commit `ee29d169f44e2185bf1e05419ba3d15864c0367e` (2026-09-27): pages "PO Token Guide", "EJS", "Extractors" (YouTube section).
- **Live observations**: 8 anonymous requests from a residential IPv6 address on 2026-11-23 (no login, no cookies), noted as `[obs 2026-11-23]`.

Provenance shorthand used below: `Y:` = yt-dlp@1507b26, path relative to `yt_dlp/extractor/youtube/` unless stated; `E:` = ejs@d48c630; `W:` = wiki@ee29d16. Every fact is **current** (present in the latest commit) unless marked **historical**.

## Summary of the current strategy

yt-dlp makes no attempt to generate PO tokens or to speak SABR. It gets formats by fanning out Innertube `/youtubei/v1/player` calls over several clients and merging what comes back. Logged out, with a JS runtime available, the default clients are `tv_samsung` (TVHTML5 with a Samsung Tizen user agent and an old `5.x` version), `visionos` (VISIONOS, no JS player needed), and `web` (WEB). `web` mostly yields SABR-only formats without URLs, so in practice it contributes metadata, captions and the player JS URL. Without a JS runtime only `visionos` is used. Logged in (cookies), the defaults are `tv_samsung, web_embedded, web`. For Premium accounts they are `web_creator, tv_samsung, web`. Signature and n challenges are no longer solved by a Python JS interpreter. Instead yt-dlp hands the **whole player JavaScript** to an external JS engine (deno, node, quickjs/quickjs-ng, or bun) running the EJS script. EJS parses the player with a JS parser, keeps only its top-level declarations and assignments, executes them under minimal browser shims, then finds and drives the player's own URL-rewriting object. PO tokens only come from the user (pasted) or from third-party plugins (BotGuard via BgUtils, or a real browser). By default a client whose format URLs need a GVS PO token that is missing has those formats **dropped**. The defaults were chosen because they need no PO token. The client list changes every few weeks: `android_vr` left the defaults on 2026-08-18, `visionos` arrived on 2026-07-09, `tv_samsung` on 2026-10-10, `ios_producer` (fallback) on 2026-11-20.

---

## 1. Clients

### 1.1 Request plumbing shared by all clients

- Endpoint: `POST https://{host}/youtubei/v1/{endpoint}?prettyPrint=false`. `host` is `www.youtube.com` except `web_music` (`music.youtube.com`). **No `key` query parameter is sent by default.** yt-dlp states "By default, no API key is used" (`Y: _base.py` request builder; `README.md` `innertube_key`). Alternative hosts that still work if forced: `studio.youtube.com`, `youtubei.googleapis.com` (README).
- Body: `{"context": <client context>, ...endpoint fields}`. The client context is the client's `INNERTUBE_CONTEXT` below, with `client.hl` forced to `en` (or the user's language), `client.timeZone: "UTC"`, `client.utcOffsetMinutes: 0` (`Y: _base.py`).
- Headers sent on every Innertube call (`Y: _base.py`):
  - `Content-Type: application/json`
  - `X-YouTube-Client-Name: <numeric id>`
  - `X-YouTube-Client-Version: <clientVersion>`
  - `Origin: https://<host>`
  - `X-Goog-Visitor-Id: <visitorData>` when known
  - `User-Agent: <client userAgent>` when the client defines one
  - Logged-in headers: see section 7.
- Before the player calls, yt-dlp downloads a "client config" page for some clients and parses `ytcfg.set({...});` out of it. This gives the real `INNERTUBE_CONTEXT` (including `visitorData` and `configInfo`), `PLAYER_JS_URL`, `STS`, `WEB_PLAYER_CONTEXT_CONFIGS`, `VISITOR_DATA`, `SESSION_INDEX`, `DATASYNC_ID`, `LOGGED_IN`. The config pages (`Y: _base.py`):

  | yt-dlp label | Config page |
  |---|---|
  | `web`, `web_safari` | `https://www.youtube.com` |
  | `mweb` | `https://m.youtube.com` |
  | `web_music` | `https://music.youtube.com` |
  | `web_creator` | `https://studio.youtube.com` (only fetched when logged in) |
  | `web_embedded` | `https://www.youtube.com/embed/{videoId}?html5=1`, with `Referer: https://www.reddit.com/` |
  | `tv` | `https://www.youtube.com/tv` |
  | others (`tv_samsung`, `tv_downgraded`, `tv_simply`, `visionos`, `android*`, `ios*`) | none: the static context is used |

  Each config page is fetched with the client's own user agent. Retries happen on network errors, but not on HTTP 403 or 429.
- `tv` workaround: when logged out, yt-dlp deletes `INNERTUBE_CONTEXT.client.configInfo.appInstallData` from the TV ytcfg before use. This is a workaround for an experiment that DRMs every TV-client format (yt-dlp issue #12563). The source notes it does not help when logged in (`Y: _base.py`).
- Embedded clients: yt-dlp always sets `context.thirdParty.embedUrl` to a non-YouTube URL (it uses `https://www.reddit.com/`; "any valid non-YouTube URL"). On player requests it also copies `encryptedHostFlags` from the embed page's `WEB_PLAYER_CONTEXT_CONFIGS.WEB_PLAYER_CONTEXT_CONFIG_ID_EMBEDDED_PLAYER.encryptedHostFlags` into `playbackContext.contentPlaybackContext.encryptedHostFlags`. The source explains: an experiment named `embeds_enable_encrypted_host_flags_enforcement` may require it, and sending it is harmless (`Y: _video.py`, `_base.py`; added 2026-03-11, Safari UA 2026-09-16).

### 1.2 The full client table (`Y: _base.py`, verbatim protocol values)

Defaults applied to every entry unless overridden: host `www.youtube.com`, **requires JS player = yes**, **supports cookies = no**, **requires auth = no**, all PO-token policies "not required, not recommended", `hl=en`.

| yt-dlp label | `clientName` | `clientVersion` | id | Other `context.client` fields | User agent | JS player | Cookies | Auth |
|---|---|---|---|---|---|---|---|---|
| `web` | `WEB` | `2.20260708.00.00` | 1 | none | (none set; the downloader's default browser UA is used) | yes | yes | no |
| `web_safari` | `WEB` | `2.20260708.00.00` | 1 | none | `Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.5 Safari/605.1.15,gzip(gfe)` | yes | yes | no |
| `web_embedded` | `WEB_EMBEDDED_PLAYER` | `2.20260708.00.00` | 56 | `thirdParty.embedUrl` (top-level context) | same Safari UA as `web_safari` | yes | yes | no |
| `web_music` | `WEB_REMIX` | `1.20260707.12.00` | 67 | host `music.youtube.com` | (none) | yes | yes | no |
| `web_creator` | `WEB_CREATOR` | `1.20260708.06.00` | 62 | none | (none) | yes | yes | **yes** |
| `android` | `ANDROID` | `21.26.364` | 3 | `androidSdkVersion: 30`, `osName: "Android"`, `osVersion: "11"` | `com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip` | **no** | no | no |
| `android_vr` | `ANDROID_VR` | `1.65.10` | 28 | `deviceMake: "Oculus"`, `deviceModel: "Quest 3"`, `androidSdkVersion: 32`, `osName: "Android"`, `osVersion: "12L"` | `com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip` | **no** | no | no |
| `ios` | `IOS` | `21.26.4` | 5 | `deviceMake: "Apple"`, `deviceModel: "iPhone16,2"`, `osName: "iPhone"`, `osVersion: "18.3.2.22D82"` | `com.google.ios.youtube/21.26.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)` | **no** | no | no |
| `ios_producer` | `IOS_PRODUCER` | `0.147.945` | 92 | same device fields as `ios` | `com.google.ios.youtubeproducer/0.147.945 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)` | **no** | no | no |
| `visionos` | `VISIONOS` | `1.02` | 101 | `deviceMake: "Apple"`, `deviceModel: "RealityDevice17,1"`, `osName: "visionOS"`, `osVersion: "26.5.23O471"` | `Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15` | **no** | no | no |
| `mweb` | `MWEB` | `2.20260708.05.00` | 2 | none | `Mozilla/5.0 (iPad; CPU OS 16_7_10 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1,gzip(gfe)` | yes | yes | no |
| `tv` | `TVHTML5` | `7.20260707.07.00` | 7 | none | `Mozilla/5.0 (ChromiumStylePlatform) Cobalt/25.lts.30.1034943-gold (unlike Gecko), Unknown_TV_Unknown_0/Unknown (Unknown, Unknown)` | yes | yes | no |
| `tv_downgraded` | `TVHTML5` | `5.20260707` | 7 | none | `Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version` | yes | yes | no |
| `tv_samsung` | `TVHTML5` | `5.20260707` | 7 | `deviceMake: "Samsung"`, `deviceModel: "UKS9800"`, `osName: "Tizen"`, `osVersion: "2.4.0"` | `Mozilla/5.0 (SMART-TV; Linux; Tizen 2.4.0) AppleWebKit/538.1 (KHTML, like Gecko) Version/2.4.0 TV Safari/538.1` | yes | yes | no |
| `tv_simply` | `TVHTML5_SIMPLY` | `1.0` | 75 | none | (none) | yes | **no** | no |

Other per-client flags:
- `mweb` and `web_music` are the only clients that support sending `playbackContext.adPlaybackContext: {"pyv": true}`. yt-dlp sends it only on opt-in. The README says it skips pre-roll ads and so the forced wait (section 3.4), but loses Premium formats when used with a Premium account.
- `PLAYER_PARAMS` (the player request's `params` field): yt-dlp sets none for any client at this commit. The field is supported as a user override, and when set it is also passed on the watch page as `pp=`.
- yt-dlp also uses the user-agent string when downloading the watch page (section 3.1) and client config pages.

Source comments attached to clients (all current, `Y: _base.py`):
- `web_safari`: the Safari UA "returns pre-merged video+audio 144p/240p/360p/720p/1080p HLS formats". **"Since 2026.07, HLS formats are only returned with some logged-in or 'trusted' sessions."**
- `web_creator`: "now requires sign-in for every video".
- `android_vr`: "Made for kids" videos are not available. "Using a clientVersion>1.65 may return SABR streams only". "Since 2026.07, intermittent/selective POT enforcement has been observed for non-HLS formats". **"Since 2026.08.17, ALL formats (including live HLS and itag 18) are 403'd with version 1.65.10."** So `android_vr` is effectively dead without a PO token. It was removed from the defaults on 2026-08-18 (commit `dae52d8`).
- `ios`: has HLS live streams. The device model is set to get 60 fps formats.
- `ios_producer`: "Only H.264 and mp4a formats are available".
- `visionos`: "Made for kids" videos are not available.
- `mweb`: has "ultralow" formats. With this iPad UA it "previously did not require PO Token" (**historical**).

### 1.3 Client priority and default order (`Y: _video.py`, `README.md`)

- Base-client priority, high to low, used for `all` ordering: `tv`, `web`, `mweb`, `android`, `ios`. Variants (`web_safari`, `tv_samsung`, …) inherit their base's priority.
- Default sets, chosen in this order of precedence:

  | Situation | Clients, in request order |
  |---|---|
  | Premium subscriber detected (logged in) | `web_creator`, `tv_samsung`, `web` |
  | Logged in (cookies) | `tv_samsung`, `web_embedded`, `web` |
  | Logged out, no JS runtime available | `visionos` only (yt-dlp warns that JS-less extraction "has been deprecated") |
  | Logged out, JS runtime available | `tv_samsung`, `visionos`, `web` |

  The README text is out of step with the code for logged-in users. It says `tv_samsung,web_embedded,web` for free accounts (matching the code) and `web_creator,tv_samsung,web` for Premium (matching the code). The commit diff from 2026-10-10 shows the previous order was `web_embedded, tv_downgraded, web`.
- When logged in, clients that do not support cookies (`android`, `android_vr`, `ios`, `ios_producer`, `visionos`, `tv_simply`) are dropped with a warning. `web_music` is appended for `music.youtube.com` URLs.
- Premium detection: the `/next` response's `topbar.desktopTopbarRenderer.logo.topbarLogoRenderer.iconImage.iconType == "YOUTUBE_PREMIUM_LOGO"`, or the tooltip text containing "premium" (`Y: _video.py`).
- Change history of the defaults (from `git log`): `visionos` added to logged-out defaults and the default moved back from `web_safari` to `web` (2026-07-20, `69ea200`). `android_vr` removed (2026-08-18). `tv_samsung` replaced `tv_downgraded` (2026-10-10, `8973412`). `web_embedded` given the web PO-token policies and deprioritized (2026-11-14, `ee25591`).

### 1.4 Dynamic fallbacks during extraction (`Y: _video.py`)

Run after each player response:
- If the client is `android_vr` or `visionos`, the status is `UNPLAYABLE` or `ERROR`, and the watch page contains the text "made for kids": add `web_embedded` (only if a JS runtime exists), and add `ios_producer` when logged out.
- If the response looks age-gated (section 3.3) and the client is not already embedded: add the embedded variant (`web_embedded`).
- If logged in and the response is age-gated, or `web_embedded` returned `UNPLAYABLE` ("embedding disabled"): add `web_creator`. The source says it "may work around age-verification for all videos but requires PO token". EU accounts need age verification.
- A player response whose `videoDetails.videoId` differs from the requested id is discarded. If every response is like that, the error is "All player responses are invalid. Your IP is likely being blocked by Youtube" (see NewPipe issue 8713).

### 1.5 Which clients avoid which hurdle (current, combining `Y: _base.py` and `W: PO Token Guide`)

- **No JS player needed** (direct `url`, no `signatureCipher`, no `n`): `android`, `android_vr`, `ios`, `ios_producer`, `visionos`. `[obs 2026-11-23]` `visionos` returned 16 adaptive formats with plain `url`s carrying `sig` (already signed) and no `n` parameter, plus an `hlsManifestUrl` and a `serverAbrStreamingUrl`, but no progressive `formats` (no itag 18) for `jNQXAC9IVRw`. A 64 KiB `range=0-65535` GET on an audio URL returned 200 with no PO token.
- **No GVS PO token by policy**: `tv`, `tv_downgraded`, `tv_samsung`, `ios_producer`, `visionos` (default "not required"). The wiki table also lists `web_embedded` and `android_vr` as "Not required", but the code now gives `web_embedded` the web policies (2026-11-14) and gives `android_vr` a required GVS policy (2026-07-20). The wiki is stale on both.
- **HLS**: `ios` (live and VOD), `web_safari` (pre-merged progressive-quality HLS, now only for logged-in or "trusted" sessions), `web_embedded` (HLS deprioritized because "pre-merged m3u8 formats may have lower quality audio"), `visionos` `[obs]`.
- **Login required**: `web_creator`.
- **Age gate / embed workarounds**: `web_embedded` (embeddable videos only), `web_creator` (logged-in age verification).
- **TV DRM**: the wiki notes "`tv`: All formats DRM'd if cookies (logged-in or active guest) aren't passed. Only SABR formats available in some cases". yt-dlp skips DRM formats (any non-empty `drmFamilies`) and warns about the TV DRM experiment.

---

## 2. Catalog

### 2.1 Channel URL, handle, custom URL to channel id (`Y: _tab.py`, `_base.py`)

- Patterns: handle `@` plus 3–30 of `[A-Za-z0-9_.-]`. Channel id `UC` plus 22 of `[A-Za-z0-9_-]`. URL forms accepted: `/channel/<id>`, `/c/<name>`, `/user/<name>`, `/browse/<id>`, `/@handle`, and bare `/<name>` (a vanity URL), unless the name is in a reserved list (`watch`, `shorts`, `playlist`, `feed`, `results`, `live`, `embed`, `hashtag`, …).
- yt-dlp does not resolve handles separately. It fetches the channel page itself, `https://www.youtube.com/<form>/videos`; with no tab given it appends `/videos`. It parses `ytInitialData` from the HTML (either the `ytInitialData = {...}` assignment or a `<script id="yt-initial-data">` element). The channel id is `metadata.channelMetadataRenderer.externalId`, falling back to the id inside `metadata.channelMetadataRenderer.channelUrl`. The handle comes from `vanityChannelUrl` or `ownerUrls[]`.
- API fallback (when the webpage is skipped or fails): `POST /youtubei/v1/navigation/resolve_url` with body `{"url": "<full youtube URL>"}`. The response's `endpoint.browseEndpoint` (`browseId`, `params`) is then posted as-is to `/youtubei/v1/browse`; or `endpoint.watchEndpoint` goes to `/next`. Error if neither: "Failed to resolve url (does the playlist exist?)".
- A page that redirected to the home feed (`selected tab.tabIdentifier == "FEwhat_to_watch"`) means "The channel/playlist does not exist".
- Regional redirects: the browse response may carry `onResponseReceivedActions[].navigateAction.endpoint.commandMetadata.webCommandMetadata.url`. yt-dlp follows it.
- Incomplete `ytInitialData` (no `contents`, `currentVideoEndpoint` or `onResponseReceivedActions`) is retried.

### 2.2 Channel tabs, uploads, live, shorts, playlists (`Y: _tab.py`)

- yt-dlp **does not hard-code browse `params` blobs for channel tabs** at this commit. It loads the tab page by URL (`/videos`, `/streams`, `/shorts`, `/playlists`, `/featured`, `/membership`, …) and reads the selected tab from `contents.twoColumnBrowseResultsRenderer.tabs[].tabRenderer` (or `expandableTabRenderer`) where `selected == true`. Tab ids come from the tab's `endpoint.commandMetadata.webCommandMetadata.url` (the last path segment), else from `tabIdentifier` (`TAB_ID_SPONSORSHIPS` maps to membership), else from the tab title (`home`→featured, `live`→streams).
- "All uploads" for a bare channel URL means the videos tab plus the `streams` and `shorts` tabs when present. They are returned as separate lists. yt-dlp suggests the single uploads playlist instead: **`UU` + channel id without its `UC` prefix** (`https://www.youtube.com/playlist?list=UU…`). Topic channels with no videos tab are redirected to that `UU…` playlist. `UUMO…` is mentioned in a test as a members-only uploads playlist prefix for any channel.
- `/live` on a channel is expected to redirect to the live video. If it does not, the result is "not live".
- `/about` is no longer a tab.
- Item renderers handled, by response path:
  - Tab content: `tab.content.richGridRenderer.contents[].richItemRenderer.content` holds one of:
    - `lockupViewModel` (videos and playlists, newer layout);
    - `videoRenderer`;
    - `reelItemRenderer`;
    - `playlistRenderer`;
    - `shortsLockupViewModel`.
  - Section layout: `tab.content.sectionListRenderer.contents[].itemSectionRenderer.contents[]` holds `gridRenderer`, `shelfRenderer`, `playlistVideoListRenderer`, `reelShelfRenderer`, `videoRenderer`, `playlistRenderer`, `channelRenderer`, `richGridRenderer` or `lockupViewModel`.
- Video fields, classic `videoRenderer`, `gridVideoRenderer` and `playlistVideoRenderer`:
  - `videoId`; title in `title` (or `headline`); description in `descriptionSnippet`.
  - Duration: `lengthSeconds`, else `lengthText`, else the overlay `thumbnailOverlays[].thumbnailOverlayTimeStatusRenderer.text`, else parsed from the accessibility label.
  - Channel id: `shortBylineText.runs[].navigationEndpoint.browseEndpoint.browseId`. Channel name: `ownerText` / `shortBylineText`. Handle: `…browseEndpoint.canonicalBaseUrl` or `commandMetadata.webCommandMetadata.url`.
  - Thumbnails: `thumbnail.thumbnails[]` (`url`, `width`, `height`). For `maxresdefault` URLs, strip the query string.
  - Relative date: `publishedTimeText` (or `videoInfo`, e.g. "50K views • 10 years ago").
  - Scheduled start: `upcomingEventData.startTime` (epoch seconds), meaning upcoming.
  - Live: `thumbnailOverlayTimeStatusRenderer.style == "LIVE"`, or a badge with style `BADGE_STYLE_TYPE_LIVE_NOW`. "streamed" in the time text means was-live. Overlay style `SHORTS` or a `/shorts/` navigation URL means a short.
  - Views: `viewCountText` / `shortViewCountText`.
  - Badges: icon types `PRIVACY_UNLISTED`/`PRIVACY_PRIVATE`/`PRIVACY_PUBLIC`/`CHECK_CIRCLE_THICK`/`OFFICIAL_ARTIST_BADGE`/`CHECK`. Styles `BADGE_STYLE_TYPE_MEMBERS_ONLY`/`_PREMIUM`/`_LIVE_NOW`/`_VERIFIED`/`_VERIFIED_ARTIST`.
- Video fields, `lockupViewModel`:
  - `contentId` and `contentType` (`LOCKUP_CONTENT_TYPE_VIDEO`, `LOCKUP_CONTENT_TYPE_PLAYLIST`, `LOCKUP_CONTENT_TYPE_PODCAST`).
  - Title: `metadata.lockupMetadataViewModel.title.content`.
  - Thumbnails: `contentImage.thumbnailViewModel.image.sources[]`. For playlists: `contentImage.collectionThumbnailViewModel.primaryThumbnail.thumbnailViewModel.image.sources[]`.
  - Duration: the text of `contentImage.thumbnailViewModel.overlays[].thumbnailBottomOverlayViewModel.badges[].thumbnailBadgeViewModel`, or of `…thumbnailOverlayBadgeViewModel.thumbnailBadges[]…`. The text "upcoming" means upcoming. Badge style `THUMBNAIL_OVERLAY_BADGE_STYLE_LIVE` means live.
  - Channel: `metadata.lockupMetadataViewModel.metadata.contentMetadataViewModel.metadataRows[].metadataParts[].text`, with `commandRuns[0].onTap.innertubeCommand.browseEndpoint.browseId` / `canonicalBaseUrl`.
  - Views and relative time: the last metadata row whose last part has an `accessibilityLabel`. Part 0 is views and the last part is the relative time.
  - Members-only: `BADGE_MEMBERS_ONLY` in `metadataRows[].badges[].badgeViewModel.badgeStyle`.
- Shorts, `shortsLockupViewModel`:
  - Video id: `onTap.innertubeCommand.reelWatchEndpoint.videoId`.
  - Title: `overlayMetadata.primaryText.content`, else parsed from `accessibilityText` ("<title>, N views - play Short").
  - Views: `overlayMetadata.secondaryText.content`.
  - Thumbnail: `thumbnailViewModel.thumbnailViewModel.image.sources[]`.
- Exact publish dates are **not** in tab listings. yt-dlp only gives approximate timestamps by parsing relative text, and only on opt-in. Relative units it understands include abbreviations (`s/sec/min/h/hr/d/w/wk/mo/y/yr`); this was fixed 2026-06-27.

### 2.3 Continuations (`Y: _base.py`, `_tab.py`)

- Token locations, searched in this order:
  - `continuations[0].nextContinuationData` (`continuation`, `clickTrackingParams`);
  - `continuation.reloadContinuationData`;
  - in `contents[]` / `items[]` / `rows[]`:
    - `continuationItemRenderer.continuationEndpoint`, or its `button.buttonRenderer.command`;
    - `continuationItemViewModel.continuationCommand.innertubeCommand`;
  - inside an endpoint: `continuationCommand.token`, also nested under `commandExecutorCommand.commands[]`.
- Next page: `POST /youtubei/v1/browse` with `{"context":…, "continuation": "<token>", "clickTracking": {"clickTrackingParams": "<ctp>"}}`. `clickTracking` is only sent when a ctp exists. Headers include `X-Goog-Visitor-Id`, using the **latest** `responseContext.visitorData` from each page. The source says this "is required to prevent an infinite extraction loop".
- Continuation responses: items at `onResponseReceivedActions[].appendContinuationItemsAction.continuationItems` or `onResponseReceivedEndpoints[]…`, or the legacy `continuationContents` (`playlistVideoListContinuation`, `gridContinuation`, `sectionListContinuation`, `itemSectionContinuation`).
- A repeated token means the end (feed looping). A page with only a continuation item is followed (fixed 2026-06-13, "Fix pagination").
- Retries in the API call helper: network errors, HTTP 5xx and 404 are retried. HTTP 403 and 429 are not retried. A 200 response whose `alerts` contain an error with "unknown error" is retried. A response missing the expected keys counts as "Incomplete data received" and is retried, then becomes a warning.

### 2.4 RSS feed

yt-dlp does **not** use `/feeds/videos.xml`. `[obs 2026-11-23]` `GET https://www.youtube.com/feeds/videos.xml?channel_id=UC4QobU6STFB0P71PMvOGN5A` returned 200, an Atom feed with `yt:` and `media:` namespaces, `<yt:channelId>` holding the id **without** its `UC` prefix (`4QobU6STFB0P71PMvOGN5A`), and `<id>yt:channel:…</id>`.

### 2.5 Playlists, Watch Later, subscriptions (`Y: _tab.py`, `_redirect.py`)

- Playlist: the page `https://www.youtube.com/playlist?list=<id>`, or the API with `browseId: "VL<playlistId>"`. Items are `playlistVideoListRenderer.contents[].playlistVideoRenderer`.
- To include unavailable or hidden videos, yt-dlp re-fetches `/browse` with `{"browseId": "VL<id>", "params": "wgYCCAA="}`. Warnings to ignore: "Unavailable videos will be hidden during playback", "Unavailable videos are hidden".
- Playlist id shape: prefixes `PL|LL|EC|UU|FL|RD|UL|TL|PU|OLAK5uy_` followed by 10 or more of `[0-9A-Za-z-_]`, or the specials `RDMM|WL|LL|LM`.
- Watch Later: `playlist?list=WL`. Liked: `playlist?list=LL`. Both need login.
- Subscriptions and history feeds: `https://www.youtube.com/feed/subscriptions`, `/feed/history` (login), `/feed/recommended`.
- Inline playlist from a watch URL: `/next` with `playlistId` and `params` (defaulting to `OAE%3D`).

### 2.6 Search (not in the contract; for reference)

`POST /youtubei/v1/search` with `{"query": "...", "params": "EgIQAfABAQ=="}`. yt-dlp labels that `params` value "Videos only". Results are at `contents.twoColumnSearchResultsRenderer.primaryContents.sectionListRenderer.contents`, then `onResponseReceivedCommands[0].appendContinuationItemsAction.continuationItems` (`Y: _search.py`, `_tab.py`).

---

## 3. The player request

### 3.1 Sequence for one video (`Y: _video.py`)

1. Set cookies: `PREF` with `hl=en&tz=UTC` merged in, and `SOCS=CAI` (consent "accept all"; the source says it is required for mixes) unless logged in or already consented (`Y: _base.py`).
2. GET `https://www.youtube.com/watch?v=<id>&bpctr=9999999999&has_verified=1` with the `web` user agent. Parse `ytcfg`, `ytInitialPlayerResponse` and `ytInitialData`. The webpage player response is kept for metadata only, with its `streamingData` stripped. The source notes Android responses lack `microformat`.
3. If `ytInitialData` is missing or lacks `contents`: `POST /youtubei/v1/next` with `{"videoId": id, "contentCheckOk": true, "racyCheckOk": true}`.
4. For each client in order: fetch the client config (1.1). Get the player JS URL once (section 5.1). Fetch a player PO token if the policy wants one. Then `POST /youtubei/v1/player`.

### 3.2 Body (`Y: _video.py`)

The body holds `context`, the fields below, `contentCheckOk: true` and `racyCheckOk: true`.
- `videoId`: the video id.
- `params`: optional; see 1.2.
- `serviceIntegrityDimensions: {"poToken": "<player PO token>"}`: only when a player token is available.
- `playbackContext`:
  - `contentPlaybackContext`:
    - `html5Preference: "HTML5_PREF_WANTS"`;
    - `signatureTimestamp: <sts>`: sent only when a player JS URL is known, i.e. for JS-player clients;
    - `encryptedHostFlags`: embedded clients only.
  - `adPlaybackContext: {"pyv": true}`: opt-in, `mweb`/`web_music` only.

Headers are the shared set plus any logged-in headers, with `X-Goog-Visitor-Id` set from the first `visitorData` found. It is looked up in the ytcfg (`VISITOR_DATA` or `INNERTUBE_CONTEXT.client.visitorData`), then in the webpage player response's `responseContext.visitorData`. `[obs 2026-11-23]` Without any visitor data, `VISIONOS` and `TVHTML5` (tv_samsung shape) both returned `LOGIN_REQUIRED` "Sign in to confirm you’re not a bot". The same `VISIONOS` request with `X-Goog-Visitor-Id` and `context.client.visitorData` taken from a prior anonymous `WEB` `/player` response's `responseContext.visitorData` returned `OK`. So carry visitor data from a first anonymous call into the next ones.

### 3.3 `playabilityStatus` handling (`Y: _video.py`)

- Statuses treated as normal: `OK`, `LIVE_STREAM_OFFLINE`, `AGE_CHECK_REQUIRED`, `AGE_VERIFICATION_REQUIRED`. Any other status is logged per client, and that client simply contributes no formats.
- **Age gate** is detected by any of:
  - a present `playabilityStatus.desktopLegacyAgeGateReason`;
  - `status` or `reason` containing `confirm your age`, `age-restricted`, `inappropriate`, `age_verification_required` or `age_check_required`.
- `UNPLAYABLE` and `ERROR` drive the made-for-kids fallback (1.4).
- Paid content trailer: `playabilityStatus.errorScreen.playerLegacyDesktopYpcTrailerRenderer.trailerVideoId` means yt-dlp redirects to the trailer.
- When **no formats** come back from any client, the message is built from `errorScreen.playerErrorMessageRenderer.reason` (text runs), else `playabilityStatus.reason`, else `errorScreen.playerInterstitialRenderer.content.interstitialViewModel.title.content`. It is followed by `subreason`, or the interstitial `description.content`. The mapping (last touched 2026-10-22, `a51ef56`):

  | YouTube text or shape | yt-dlp interpretation |
  |---|---|
  | subreason starts with "The uploader has not made this video available in your country" | geo-restricted; countries from `microformat.playerMicroformatRenderer.availableCountries` or the `regionsAllowed` meta |
  | reason contains "sign in" (e.g. "Sign in to confirm you’re not a bot", "Sign in to confirm your age") | login or cookies needed; yt-dlp strips the trailing "This helps protect our community. Learn more" |
  | `errorScreen.playerCaptchaViewModel` present | "YouTube is requiring a captcha challenge before playback" |
  | "This content isn't available, try again later" | **rate limit** of the session or account "for up to an hour". The wiki gives limits of ~300 videos/hour (~1000 webpage+player requests) for guests and ~2000 videos/hour (~4000 requests) for accounts, and recommends 5–10 s between videos |
  | `streamingData.licenseInfos` present and nothing playable | DRM |

- "Unknown error" alerts in 200 responses are retried (2.3).

### 3.4 `streamingData` and response fields used (`Y: _video.py`)

- `formats[]`, `adaptiveFormats[]`, `hlsManifestUrl`, `dashManifestUrl`, `licenseInfos`. `serverAbrStreamingUrl` is **not used** (no SABR support). `expiresInSeconds` is not read. `[obs 2026-11-23]` `expiresInSeconds` was `21540` for WEB.
- `videoDetails`:
  - `videoId`, `title`, `shortDescription`, `lengthSeconds`, `keywords`, `channelId`, `viewCount`, `isPrivate`;
  - live fields: `isLive`, `isLiveContent`, `isUpcoming`, `isPostLiveDvr`.
- `microformat.playerMicroformatRenderer`:
  - `uploadDate`, `lengthSeconds`, `isFamilySafe`, `isUnlisted`, `availableCountries`, `category`, `ownerProfileUrl`, `externalChannelId`, `isShortsEligible`;
  - `liveBroadcastDetails` with `isLiveNow`, `startTimestamp`, `endTimestamp`.
- `storyboards.playerStoryboardSpecRenderer.spec`.
- `captions.playerCaptionsTracklistRenderer`: see section 8.
- **Pre-roll forced wait** (added 2026, fixed 2026-06-26, `3c279b3`). yt-dlp computes how long the download must wait before it starts, and applies the wait only to non-live videos. The wait sums each pre-roll ad's duration (or its skip offset when the ad is skippable). Ads come from:
  - `adPlacements[]` whose `adPlacementRenderer.config.adPlacementConfig.kind == "AD_PLACEMENT_KIND_START"`, then `adPlacementRenderer.renderer.instreamVideoAdRenderer`;
  - `adSlots[]` whose `adSlotRenderer.adSlotMetadata.triggerEvent == "SLOT_TRIGGER_EVENT_BEFORE_CONTENT"`, then `adSlotRenderer.fulfillmentContent.fulfilledLayout.playerBytesAdLayoutRenderer.renderingContent.instreamVideoAdRenderer`. The renderer can also sit inside `playerBytesSequentialLayoutRenderer.sequentialLayouts[].playerBytesAdLayoutRenderer.renderingContent`.

  The ad length is `playerVars` (a query string) → `length_seconds`, or `skipOffsetMilliseconds`. yt-dlp then sleeps until now + total before downloading. This implies YouTube refuses or blocks media fetched before the ad time has elapsed, though the source does not state the failure mode.
- Thumbnails: besides the response's lists, yt-dlp probes `https://i.ytimg.com/vi{_webp}/<id>/<name>{_live}.{webp|jpg}` for names `maxresdefault, hq720, sddefault, hqdefault, 0, mqdefault, default, sd1-3, hq1-3, mq1-3, 1-3`.

---

## 4. Formats

### 4.1 Per-format fields used (`Y: _video.py`)

- Core: `itag`, `url` or `signatureCipher`, `mimeType` (`type/subtype; codecs="..."`), `bitrate`/`averageBitrate`, `contentLength`, `approxDurationMs`, `width`, `height`, `fps`, `quality`, `qualityLabel`, `audioQuality`, `audioSampleRate`, `audioChannels`.
- `audioTrack`:
  - `id`: the language comes from the part before the first `.`;
  - `displayName`: containing "original" means the original track, containing "descriptive" means audio description;
  - `audioIsDefault`.
- `isDrc`: the DRC variant shares the itag, so yt-dlp ids it `<itag>-drc`. `[obs]` Duplicated audio itags 139/140/249/250/251 on `visionos`.
- `drmFamilies`: DRM, skipped. `projectionType`, `spatialAudioType`.
- `type == "FORMAT_STREAM_TYPE_OTF"`: OTF; see 4.3. `targetDurationSec`: a live adaptive format.
- `xtags`, a URL parameter that is itself a query string: `sr=1` marks an "AI-upscaled" super-resolution format.
- Known itag notes: `17` is 3gp and worst. `22` is "likely to be damaged". `616` and `235` are marked "Premium". A format whose `approxDurationMs` is less than half the video duration is "possibly damaged". `18` is exempt from the GVS PO-token requirement in yt-dlp's policy check.
- Quality order used: `tiny`, `audio_quality_ultralow`, `audio_quality_low`, `audio_quality_medium`, `audio_quality_high`, `small`, `medium`, `large`, `hd720`, `hd1080`, `hd1440`, `hd2160`, `hd2880`, `highres`.

### 4.2 Building a usable https URL (`Y: _video.py`)

1. `url` present: use it. Otherwise parse `signatureCipher` as a query string: `url`, `s` (the encrypted signature), `sp` (the parameter name; default `signature`). If `url` or `s` is missing, the format has no usable URL. On `web`/`web_safari` yt-dlp reads this as "YouTube is forcing SABR streaming for this client". On other clients it reads it as "the SABR-only streaming experiment for your account/the current session" (issue #12482).
2. Signature: append `&<sp>=<solved s>`.
3. `n`: if the URL has `n=`, replace its value with the solved n. **If n cannot be solved, the format is dropped.** yt-dlp does not keep throttled URLs.
4. PO token: append `pot=<GVS token>` when one is available.
5. No `ump`, `alr`, `rn` or `cpn` parameters are added by yt-dlp.

### 4.3 Download rules for googlevideo (`Y: _video.py`, `yt_dlp/downloader/http.py`)

- Chunking: every https format is downloaded in **10 MiB (10 × 2^20 bytes) chunks**, through the HTTP `Range: bytes=a-b` header (the downloader's `http_chunk_size`). The request also sends `Accept-Encoding: identity`.
- Optional "dashy" mode (opt-in) turns a format with a known `contentLength` into fragments `…&range=<start>-<end>` of 10 MiB each, using the `range` **query parameter**. The source code caps the end at `filesize` rather than `filesize-1`.
- `[obs 2026-11-23]` A `range=0-65535` query on a `visionos` URL returned 200 with exactly 65536 bytes.
- OTF (`FORMAT_STREAM_TYPE_OTF`) https formats are **skipped**. Per the source, they need the init segment fetched with `&sq=0`, then its `emsg` box parsed for the segment count, then segments fetched with `&sq=N`.
- Live adaptive formats (those with `targetDurationSec`) are skipped unless the user asks for live-from-start, or the video is post-live (4.5).
- DASH manifest: the `/n/<challenge>/` path segment is rewritten with the solved n, and `/pot/<token>` is appended to the path. File size comes from `/clen/<n>` in segment URLs.
- HLS manifest: the same `/n/…/` rewrite. `/pot/<token>` is inserted before a trailing `/file/index.m3u8` or `/playlist/index.m3u8`. If the manifest has an n challenge that cannot be solved, the manifest is skipped. The itag comes from `/itag/<n>` in variant URLs.
- Recently uploaded videos (2 days or less) get their HLS formats marked for testing before use. Freshly processed HLS can be incomplete.

### 4.4 SABR

yt-dlp has **no SABR or UMP implementation**. `serverAbrStreamingUrl` is ignored. A format with neither `url` nor `signatureCipher` is SABR-only and gets skipped.
- `web` and `web_safari` are effectively forced onto SABR. The wiki says "Only SABR formats available" for `web`. `[obs 2026-11-23]` Anonymous `WEB` `/player` for `jNQXAC9IVRw` returned 14 formats with neither `url` nor `signatureCipher` (fields: `itag`, `mimeType`, `bitrate`, `width`, `height`, `initRange`, `indexRange`, `contentLength`, `approxDurationMs`, …) plus `serverAbrStreamingUrl`.
- `tv` is SABR-only "in some cases" (wiki).
- `android_vr` versions above 1.65 "may return SABR streams only" (`_base.py`).
- Any client can be put under a per-session SABR-only experiment.

### 4.5 Live, post-live, upcoming (`Y: _video.py`; churn: live adaptive support added 2026-07-05 `8c1f07d`, fixed 2026-07-20, post-live reworked 2026-11-15/16)

- Live status derivation: `isPostLiveDvr` gives post-live; otherwise `isLive` (or `liveBroadcastDetails.isLiveNow`) gives live; `isUpcoming` gives upcoming; `isLiveContent` gives was-live; otherwise not-live.
- **Live, not from start**: the source of truth is HLS (`hlsManifestUrl`), parsed as a live m3u8. DASH is skipped for live. Live adaptive https formats are skipped as "incomplete".
- **Post-live** (`isPostLiveDvr`): the stream is still being processed. Post-live DASH and m3u8 manifests hold only "the last ~2 hours", so they are deprioritized. HLS is skipped by default. DASH is skipped when the duration is over 2 h.
- **Live from start and post-live via adaptive formats**: the base URL is the live adaptive format URL. yt-dlp sends `HEAD <base_url>` and reads the response header **`X-Head-Seqnum`** (the newest sequence number). On `tv_samsung` the HEAD can return HTTP 500 but still carry the header (issue #18031). It then fetches segments `<base_url>&sq=<i>` for i from the known index up to `X-Head-Seqnum`. The loop polls every 5 s. YouTube keeps **at most 120 hours (432000 s)** of segments. The format URL is refreshed by re-running the extraction after 5 h, or after 5 s when segments stop arriving. Post-live stops after one pass.
- Upcoming: there is no special path in the player code. `LIVE_STREAM_OFFLINE` is a normal status. The start time comes from `liveBroadcastDetails.startTimestamp`. Tab listings use `upcomingEventData.startTime`.
- Live HLS PO token: in the `ios` client policy, "HLS Livestreams require POT 30 seconds in". The wiki says "HLS live streams do not require a PO Token (excluding `ios` client)". For `android_vr`, all formats including live HLS are 403'd since 2026-08-17 (section 1.2).
- Thumbnails for live videos use the `_live` suffix.

---

## 5. Player JavaScript and challenges

### 5.1 Finding the player (`Y: _video.py`)

- Taken from a ytcfg: `PLAYER_JS_URL`, else `WEB_PLAYER_CONTEXT_CONFIGS.*.jsUrl`.
- Fallback: `GET https://www.youtube.com/iframe_api`, then search for an 8-hex-digit player id after `player/` (slashes may be backslash-escaped). `[obs 2026-11-23]` The page contained `player\/6b3d0364\/`.
- Player id: 8 or more hex digits in `/s/player/<id>/`.
- **yt-dlp always rewrites the URL to the `main` variant** (the default; others are for debugging): `https://www.youtube.com/s/player/<id>/player_ias.vflset/en_US/base.js`. Known variants and their paths (the locale segment `en_US` may differ in real URLs):

  | Variant | Path after `/s/player/<id>/` |
  |---|---|
  | main | `player_ias.vflset/en_US/base.js` |
  | tcc | `player_ias_tcc.vflset/en_US/base.js` |
  | tce | `player_ias_tce.vflset/en_US/base.js` |
  | es5 | `player_es5.vflset/en_US/base.js` |
  | es6 | `player_es6.vflset/en_US/base.js` |
  | es6_tcc | `player_es6_tcc.vflset/en_US/base.js` |
  | es6_tce | `player_es6_tce.vflset/en_US/base.js` |
  | tv | `tv-player-ias.vflset/tv-player-ias.js` |
  | tv_es6 | `tv-player-es6.vflset/tv-player-es6.js` |
  | phone | `player-plasma-ias-phone-en_US.vflset/base.js` |
  | house | `house_brand_player.vflset/en_US/base.js` ("used by Google Drive") |

  `[obs]` The `main` base.js for `6b3d0364` was 3,060,278 bytes.
- **Historical**: on 2026-03-03 yt-dlp forced a pinned player (`9f4cc5e4`, commit `d3165e8`) and skipped the webpage player response so that `web` would use it. At this commit the default is the actual player again; pinning is now a user option in the form `STS@HASH`.

### 5.2 Signature timestamp (`Y: _video.py`)

The order is: `STS` from the ytcfg; else search the player JS for the key `signatureTimestamp` (or `sts`), followed by optional whitespace, a colon, and a **5-digit** number. The value is cached in memory per player. `[obs 2026-11-23]` The current player `6b3d0364` contains `signatureTimestamp:20774`. Note the regex is 5 digits fixed; YouTube will outgrow it at 100000.

### 5.3 What the challenges are

- **sig**: `signatureCipher.s` must be transformed by player code into the `sp` parameter (usually `sig`/`signature`). yt-dlp's key observation: the transform is a fixed **permutation and deletion by position** that depends only on the input length. To learn it, yt-dlp solves an input made of the characters with codes 0, 1, 2, …, L-1. It reads the output back as a list of source indices and caches that list per (player, length) **on disk**. Each real signature of length L is then solved by indexing, with no JS run. The EJS test vectors confirm the output is a reordering and subset of the input (e.g. player `631d3938`: a 103-character input of codes 0x00–0x66 maps to 100 characters).
- **n**: the URL's `n` value (and the `/n/<value>/` segment in manifest paths) must be replaced with a transform of itself. Unsolved n means throttling, so yt-dlp drops the format (4.2). Results are cached in memory per (player, input).
- yt-dlp collects every distinct n value and every distinct signature length across all clients' responses, and solves them in one bulk JS invocation per player.

### 5.4 How EJS solves them (`E: src/yt/solver/*.ts`; churn below)

What it does to the player script (in prose):
1. Parse the entire player with a full ES parser (EJS uses meriyah 6.1.4 and regenerates code with astring 1.9.0).
2. Locate the player's body. Two outer shapes are accepted:
   - a single top-level statement that calls a member of an anonymous function expression (a `(function(){…}).call(this)`-style wrapper);
   - two top-level statements where the second is an immediately-invoked function expression. Here the first statement inside that function (which declares a local `window` bound to `this`) is dropped.

   Anything else is an "unexpected structure" failure.
3. Inside that body, drop every top-level expression statement that is not an assignment and not a bare literal (side-effecting calls). Keep all declarations, assignments and directives, so the player's functions and global tables get defined without starting the player.
4. Find candidate functions: any top-level function declaration, `var` declarator initialised with a non-async function expression, or assignment of a non-async function expression to a name or member. A candidate's **own top-level statements** must contain a call of the form `<identifier>.<anything>("alr", "yes")`. That is a URL-builder/stream-URL routine that sets `alr=yes`.
5. For each candidate, build a solver that:
   - calls the candidate with a dummy watch URL, the parameter name `"s"`, and the URL-encoded signature (or undefined), which yields a URL object;
   - sets `n` on that object;
   - calls the **first method** on its prototype other than `constructor`, `set`, `get` and `clone`, which applies the player's own deciphering;
   - reads back `s` (URL-decoded) and `n`.
6. Every candidate is tried. The results must agree: zero results gives a "no solutions" error, and more than one distinct result gives "invalid solutions". The single agreed value is the answer.
7. The setup prelude, prepended to the program, defines only what is missing:
   - `XMLHttpRequest` as an object with an empty `prototype`;
   - `location`: a `URL` for `https://www.youtube.com/watch?v=…`, or a plain object with `hash`, `host`, `hostname`, `href`, `origin`, `password`, `pathname`, `port`, `protocol`, `search` and `username` when `URL` does not exist;
   - `document` and `navigator` as empty null-prototype objects;
   - `self` and `window` bound to the global object.
8. The transformed program is run with the global `Function` constructor, which receives a result object. The results come back as `{n: fn, sig: fn}`.

Input and output format, as yt-dlp feeds the runtime: the lib script (bundled meriyah plus astring) is assigned onto `globalThis`, the core script is appended, and the program prints a JSON object to stdout. The request is either `{type:"player", player:<js>, requests:[{type:"n"|"sig", challenges:[…]}], output_preprocessed:true}`, or a `preprocessed` form that carries a previously transformed player. The response is `{type:"result", responses:[{type:"result", data:{challenge: answer}} | {type:"error", error}], preprocessed_player?}`. yt-dlp has the preprocessed-player disk cache **disabled** ("files are large and we do not support rotation").

JavaScript requirements:
- A real ES2015+ engine is required. It must parse the full ~3 MB player and the ~hundreds-KB parser bundle, and run with `Function`, `Set`, arrow functions, template literals, `Object.getPrototypeOf`/`getOwnPropertyNames`, `encodeURIComponent`, and optionally `URL`.
- **No DOM** is needed beyond the stubs above.
- No network and no file system access.
- **QuickJS works**: supported are QuickJS ≥ 2023-12-09 and any QuickJS-NG (`E: README.md`, `W: EJS`). Versions older than QuickJS 2025-04-26 or QuickJS-NG 0.12.0 "are missing optimizations which can lead to execution times of several minutes" (`W: EJS`, `Y: jsc/_builtin/quickjs.py`). yt-dlp runs QuickJS through a temp file (`qjs --script <file>`) because QuickJS cannot read the script from stdin.
- Runtime preference in yt-dlp: deno 1000, node 900, quickjs 850. Bun has a lower preference and is deprecated.
- Minimum versions: deno ≥ 2.3.0, node ≥ 22, bun 1.2.11–1.3.14. Deno and node are run with restricted permissions, and optionally JIT-less.
- Script integrity: yt-dlp pins EJS by major.minor version and SHA3-512 hash. It loads the script from the Python package, then a disk cache, then a vendored copy, then (opt-in) GitHub release assets `https://github.com/yt-dlp/ejs/releases/download/<ver>/yt.solver.{core,lib}.min.js`.

Test vectors (facts about YouTube's player output, `E: src/yt/solver/test/tests.ts`; the comments give the STS):

| Player id (STS) | n input → output |
|---|---|
| `74edf1a3` (20522) | `IlLiA21ny7gqA2m4p37` → `9nRTxrbM1f0yHg`; `eabGFpsUKuWHXGh6FR4` → `izmYqDEY6kl7Sg` |
| `901741ab` (20523) | `BQoJvGBkC2nj1ZZLK-` → `UMPovvBZRh-sjb` |
| `e7573094` (20524) | `IlLiA21ny7gqA2m4p37` → `3KuQ3235dojTSjo4` |
| `76ad2fe8` (20528) | `V8pdMGD0Sz_M0DRT` → `rBnTGCWGFpwByA` |
| `631d3938` (20529) | `KBx1qz7jMhxELa8c` → `ttPvh7WIptsgSw` |

Every vector is tested against all nine variants (main, tcc, tce, es6, tv, tv_es6, phone, es6_tcc, es6_tce). The file also has sig vectors, including index-probe inputs. `[obs]` The live player on 2026-11-23 is STS 20774, well past the newest vector. `[obs]` The literal pair `"alr","yes"` occurs 3 times in the `6b3d0364` main player.

Churn in EJS (git log): the sig fix for player `638ec5c6` (2025-10-22). es6/tv_es6 n fix (2025-10-28). tce sig fix for `c1c87fb0` (2026-01-19). "More robust solver extraction" (2026-02-08). main sig fix for `4e51e895` (2026-02-16). tce/es6 sig fix (2026-02-21). "Solve new player variants" and "Fix sig value encoding" (2026-03-13). "Expose `window` global as `self`" (2026-03-17); the tests note that players 20528/20529 tv and tv_es6 "needed self.location.origin fix". No solver-logic change from 2026-03-17 to 2026-10-26: the "run the player, drive its URL object" approach has held for 8 months. **Historical**: before EJS, yt-dlp extracted the n and sig functions with regexes and ran them in a Python JS interpreter, or used PhantomJS. The wiki says "EJS replaces the prior JSInterp and PhantomJS based approach. For YouTube both are no longer used."

### 5.5 Cache keys (`Y: _video.py`, `jsc/_builtin/ejs.py`)

- Player code: in memory, keyed `<playerId>-<variant>`. An unknown variant's path is sanitised into the key.
- STS: in memory, keyed by section "sts" plus the player key. Not on disk; the source notes it is waiting on preprocessed-player caching.
- Signature index map: **on disk**, keyed by section "sigfuncs" plus `<playerId>-<variant>` plus the signature length. The value is the list of source indices. The minimum cache version is `2025.07.21`.
- n results: in memory, keyed by section "n" plus the player key plus the challenge string.
- EJS scripts: on disk, section `challenge-solver`, entries `lib` and `core`, each `{version, variant, code}`.

---

## 6. PO tokens and BotGuard

### 6.1 What yt-dlp knows (`W: PO Token Guide`, `Y: _base.py`, `pot/`)

- A PO token is produced by BotGuard (web), DroidGuard (Android) or iOSGuard (iOS). Tokens are not portable across platforms.
- Contexts: **GVS** (the googlevideo media request), **Player** (`/player` `serviceIntegrityDimensions.poToken`), **Subs** (timedtext).
- Content binding:
  - GVS when logged out: visitor data, or the **visitor id** (by default yt-dlp caches by visitor id, decoded from `visitorData` as base64url bytes 2–12, an 11-character id);
  - GVS when logged in: the account DataSync id (`DATASYNC_ID` or `responseContext.mainAppWebResponseContext.datasyncId`; the user session id part when the form is `delegated||user`);
  - GVS, alternatively, the **video id** when the ytcfg experiment `html5_generate_content_po_token=true` is set. It appears in `WEB_PLAYER_CONTEXT_CONFIGS.*.serializedExperimentFlags`, a query string;
  - Player and Subs: the **video id**;
  - `WEB_REMIX` binds like GVS in every context.

  The wiki says "Most PO Tokens (such as for web GVS/Player) are bound to the video ID, so a new token is required for each video", and "Manually extracting PO Tokens is no longer recommended".
- Clients whose tokens are web (BotGuard) tokens: `WEB`, `MWEB`, `TVHTML5`, `WEB_EMBEDDED_PLAYER`, `WEB_CREATOR`, `WEB_REMIX`, `TVHTML5_SIMPLY`, `TVHTML5_SIMPLY_EMBEDDED_PLAYER`.
- Lifetime: the source says "Integrity token response usually states it has a ttl of 12 hours (43200 seconds)", and yt-dlp caches for 6 h. The cache key includes the binding, the binding type, `context.client.remoteHost` (IP), the source address and the proxy. The wiki notes "possibly as short as 12 hours… some reports suggest… several months".
- Cold-start tokens, `/att/get`, `/api/jnn/v1/Create`, `GenerateIT`, waa: **not covered**. yt-dlp contains no BotGuard code. It points to BgUtils (LuanRT) and to the plugins bgutil-ytdlp-pot-provider (BgUtils-based) and yt-dlp-getpot-wpc (a real browser).
- Token format: yt-dlp normalises user tokens by URL-unquoting, base64url-decoding and re-encoding them.
- Placement: `pot=` query on https URLs; `/pot/<token>` path segment on HLS/DASH manifests; `pot=<token>&potc=1&c=<clientName>` on caption URLs; `serviceIntegrityDimensions.poToken` in `/player`.

### 6.2 Policy per client and context (`Y: _base.py`, current)

"Required" means formats are skipped without a token. "Recommended" means yt-dlp tries to get one anyway. Exceptions: "not for Premium" means the token is not required for Premium subscribers. "Not with player token" means it is not required if a player PO token was sent.

| Client | GVS https/DASH | GVS HLS | Player | Subs |
|---|---|---|---|---|
| `web`, `web_safari`, `web_embedded` | required, recommended, not for Premium | not required, recommended | not required | not required (see below) |
| `web_music`, `web_creator`, `mweb` | as web | as web | not required | not required |
| `android`, `android_vr` | required, recommended, **not with player token** | not required, recommended, not with player token | not required, recommended | not required |
| `ios` | https required, recommended, not with player token (no DASH entry, so DASH gets the default "not required") | **required**, recommended, not with player token ("HLS Livestreams require POT 30 seconds in") | not required, recommended | not required |
| `tv_simply` | required, recommended | not required, recommended | not required | not required |
| `tv`, `tv_downgraded`, `tv_samsung`, `ios_producer`, `visionos` | not required | not required | not required | not required |

- Subtitles: the policy is "not required", with this comment: "In rollout, currently detected via experiment. Premium users DO require a PO Token for subtitles". The detection: a caption `baseUrl` whose `exp` query values include `xpe` or `xpv` requires a token. Without one, that client's captions are discarded.
- itag `18` is never treated as needing a GVS token.
- With default settings, yt-dlp only fetches tokens from providers when required. Policy "recommended" only matters when the user set "always".
- The wiki enforcement table (2026-09) says: `web` Subs+GVS ("Only SABR formats available"); `web_safari` GVS (its HLS does not need one); `mweb` GVS; `tv` not required; `tv_simply` GVS; `web_music` GVS; `web_creator` GVS; `android` and `ios` "GVS or Player". GVS is never needed for Premium.

### 6.3 visitorData and `VISITOR_INFO1_LIVE`

- `visitorData` comes from the ytcfg (`VISITOR_DATA`, `INNERTUBE_CONTEXT.client.visitorData`) or any response's `responseContext.visitorData`. It is sent as `X-Goog-Visitor-Id`.
- The wiki explains that passing visitor data without cookies requires skipping the webpage, "so that the `VISITOR_INFO1_LIVE` cookie does not interfere". It calls this approach "not recommended… less stable extraction".
- `[obs]` An anonymous `/player` call returns a fresh `responseContext.visitorData`, and reusing it unblocked `visionos` (3.2).

---

## 7. Login

- **OAuth / TV device code: removed.** A username starting with `oauth` raises "Login with OAuth is no longer supported" (`Y: _base.py`). The wiki says "Due to new restrictions enacted by YouTube, logging in with OAuth no longer works with yt-dlp." No endpoints, client ids or scopes remain in the source. Password login is also unsupported.
- **Cookies** only:
  - "Logged in" means a `LOGIN_INFO` cookie plus one of `SAPISID`, `__Secure-1PAPISID` or `__Secure-3PAPISID`. The source notes "YouTube doesn't appear to clear 3PSAPISID when rotating cookies (as of 2025-04-26)", whereas `LOGIN_INFO` is cleared. yt-dlp re-checks after every request and warns that "cookies are no longer valid… likely been rotated".
  - `Authorization` holds up to three space-separated values: `SAPISIDHASH <v>` (from `SAPISID`, or `__Secure-3PAPISID` when `SAPISID` is missing), `SAPISID1PHASH <v>` (from `__Secure-1PAPISID`) and `SAPISID3PHASH <v>` (from `__Secure-3PAPISID`).
  - Each `<v>` is `<unix seconds>_<sha1 hex>`. The SHA-1 input joins with single spaces: `[u-value if known]`, the timestamp, the cookie value and the origin (`https://www.youtube.com`).
  - When a user session id is known, it is the first hashed part, and the literal suffix `_u` is appended to `<v>`.
  - Also sent: `X-Origin: <origin>` with the Authorization header; `X-Goog-AuthUser: <SESSION_INDEX or 0>`; `X-Goog-PageId: <delegated session id>` for brand or secondary channels; `X-Youtube-Bootstrap-Logged-In: true` when the ytcfg says `LOGGED_IN`.
  - DataSync id: `"delegated||user"` for secondary channels, `"user||"` for the primary.
- Export advice (wiki): use a private window, open `https://www.youtube.com/robots.txt`, export the cookies, then close the window so the browser never rotates them. Accounts risk bans.
- Login is needed for: age-restricted videos (beyond the `web_embedded` workaround), members-only and private content, Watch Later, Liked, subscriptions, history and Premium formats. `web_creator` needs it for every video.

---

## 8. Captions (`Y: _video.py`)

- Track list path: `captions.playerCaptionsTracklistRenderer`:
  - `captionTracks[]` with `baseUrl`, `languageCode`, `vssId`, `name`, `kind` (`asr` means auto-generated), `isTranslatable`;
  - `translationLanguages[]` with `languageCode` and `languageName`. Only web and mweb clients provide translation languages.
- Language key: `vssId` without its leading `.`, with `.` replaced by `-` (an ASR track looks like `a-en`). Falls back to `languageCode`.
- URL: `baseUrl` (made absolute against `https://www.youtube.com`) plus `fmt=<json3|srv1|srv2|srv3|ttml|srt|vtt>`, with **`xosf` removed**. The source says `xosf=1` yields unwanted positioning data (issue #13654).
- Translation: add `tlang=<code>`. Never set `tlang` equal to the track's own `lang`, which "returns damaged subtitles".
- An ASR track whose language is translatable is treated as the original audio language.
- Caption requests are made with browser-impersonated TLS.
- PO token: see 6.2. The token is fetched once per client, and the caption URL gets `pot`, `potc=1` and `c=<innertube clientName>`.
- Live chat: from `/next` `contents.twoColumnWatchNextResults.conversationBar.liveChatRenderer.continuations[0].reloadContinuationData.continuation`.

---

## 9. Live

See 4.5. Summary of the facts:
- Live playback uses the HLS manifest. `ios` provides live HLS but needs a GVS token about 30 s in. Live HLS on web clients needs no token. `android_vr` live HLS has been 403'd since 2026-08-17.
- DVR or timeshift: live adaptive https formats are addressed by `sq=<n>`, with the newest number in the `X-Head-Seqnum` header of a HEAD on the base URL. Up to 120 h is available. Post-live HLS and DASH manifests only hold about 2 h. A post-live stream stays post-live until processing ends.
- Fields: `videoDetails.isLive`, `isLiveContent`, `isUpcoming`, `isPostLiveDvr`; `microformat…liveBroadcastDetails.{isLiveNow,startTimestamp,endTimestamp}`. When missing, duration is computed as end minus start.
- Low-latency: not covered.
- Upcoming premieres: listings use `upcomingEventData.startTime`. The player returns `LIVE_STREAM_OFFLINE`, which yt-dlp does not treat as an error status.

---

## 10. Failure modes and churn

| Area | What breaks, as seen in the source | Last change |
|---|---|---|
| Client list and versions | Clients get PO-token or SABR enforcement in waves. Versions are bumped together (all at 2026-07-08/09) | 2026-11-20 (`ios_producer`), 2026-11-14 (`web_embedded` policies), 2026-10-10 (`tv_samsung`) |
| `android_vr` | Fully 403'd since 2026-08-17 | 2026-08-18 |
| `web_safari` HLS | Only for logged-in or "trusted" sessions since 2026-07 | 2026-07-20 |
| `tv` DRM experiment | All formats DRM'd for some sessions | (workaround older) |
| SABR-only experiment | Formats without URL on any client | ongoing |
| n/sig solving | Player structure and variant changes; 7 EJS fixes from 2025-10 to 2026-03 | EJS 2026-03-17, yt-dlp bumped to 0.8.0 on 2026-03-17 |
| Pinned player | Forced player in 2026-03, later reverted | 2026-03-03 |
| Pre-roll wait | Forced wait before download | 2026-06-26 |
| Live adaptive and post-live | Reworked several times | 2026-11-15/16/19 |
| Tab metadata and pagination | Renderer migration to `lockupViewModel`, `pageHeaderViewModel` ("slow rollout began April 2024") | 2026-06-13/15/21/24, 2026-08-20 |
| Error messages | Rate-limit and captcha texts | 2026-10-22 |

Retry and IP behaviour:
- 403 and 429 are never retried on page or API fetches. 5xx, 404 and network errors are retried.
- Mismatched `videoId` in player responses means the IP is being blocked.
- "Sign in to confirm you’re not a bot" is the bot check. `[obs]` It went away once visitor data was supplied.
- Guest rate limit: ~300 videos/h. Account: ~2000 videos/h.

---

## Open questions

- Does `tv_samsung` (TVHTML5 5.x, Tizen UA) give plain or ciphered URLs without PO tokens and without SABR, and why was it preferred over `tv_downgraded`? The commit messages give no reason, and my one anonymous test hit the bot check before visitor data was added. Not re-tested to stay under the request budget.
- Do `visionos` formats survive full-length downloads without a PO token? Only a 64 KiB range was tested. The source says intermittent enforcement was seen on `android_vr` before it went dark, and `visionos` may follow.
- The minimum JS engine size and time budget: EJS on QuickJS for a 3 MB player under a 5-minute, 256 MiB sandbox. The source only says old QuickJS "can take several minutes". The cost of parsing the player, regenerating it with astring, and re-parsing it via `Function` is not measured anywhere.
- The source does not settle whether the EJS approach (finding an `"alr","yes"` call inside a URL-object factory) will keep working: it depends on that literal pair staying in the player. It held from 2026-03 through STS 20774 `[obs]`, but there are no test vectors past 20529.
- The PO token generation protocol (BotGuard, `/att/get`, `GenerateIT`, waa) is entirely absent from yt-dlp.
- What happens if media is fetched before the pre-roll wait elapses (403? SABR backoff?). The source only enforces the wait.
- Live low-latency and `isLowLatencyLiveStream` are not handled.
- Channel-tab browse `params` blobs and RSS are not used by yt-dlp. Take them from another source.
- `expiresInSeconds` is ignored. yt-dlp re-extracts live URLs every 5 h instead.
