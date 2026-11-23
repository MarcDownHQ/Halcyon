# Source: Grayjay YouTube plugin (FUTO)

## Source, licence, commits read

- **Repository:** https://gitlab.futo.org/videostreaming/plugins/youtube (default branch `master`).
- **Licence:** Source First License 1.1 (file `LICENSE`). Not MIT compatible. This document contains facts only; no code, no source identifiers, no copied comments or regexes.
- **Commit read:** `640bd863f48869e6b6cc4e11175a1e53755215f5`, 2026-11-21 ("Verisioning"), plugin version **373** (`YoutubeConfig.json`). All facts below are from this commit unless stated otherwise.
  - Main file: `YoutubeScript.js` (about 15,400 lines; the last ~260 lines are bundled third-party code: google-protobuf messages for UMP/SABR and a custom JSDOM build).
  - Manifest: `YoutubeConfig.json` (settings, allowed hosts, login config, changelog).
- **Host engine, read-only context:** Grayjay.Engine (github.com/futo-org/Grayjay.Engine) at `34340b593db2553bd995ebed4d30ef3e33f3fa6c` (2026-11-02, "Share rotated auth cookies across plugin clients"). Files: `Grayjay.Engine/GrayjayPlugin.cs`, `Grayjay.Engine/PluginDescriptor.cs`, `Grayjay.Engine/Packages/PackageBrowser.cs`, `Grayjay.Engine.Tests/AuthCookieTests.cs`, `Grayjay.Engine.Tests/YoutubeTest.cs`.
- Recent history relevant to churn (git log, all `master`): 2026-11-21 "iOSProducer Streams", "Require browser attestation for native UMP and bound token cache lifetimes", "Remove need for HEAD calls"; 2026-11-13/14 "Fix for N decryption", "wip samsung source", "Batch verify support"; 2026-09-04 "Disable browse API for now"; 2026-08-24 "VisionOS support"; 2026-08-22 "disable android vr", "Native ump default"; 2026-07-14 "Native ump"; 2026-06-02 "Added Android VR".
- No live requests to YouTube were made for this report; every fact is from the source as of the commit above.

## Summary of the current strategy

For a video, the plugin first loads `https://www.youtube.com` (home page HTML) once per session to read `ytcfg` (API key, `INNERTUBE_CONTEXT`, `VISITOR_DATA`/`EOM_VISITOR_DATA`, `DATASYNC_ID`, player JS URL, experiment flags), downloads the player JS, and gets the signature and n functions for that player **from a FUTO-run remote service** (`solver.grayjay.app` / `solutions.grayjay.app`) that names which function in the player to call; it then **executes the entire player JS inside a JSDOM window in V8** and calls that function. It mints a **PO token with BotGuard**: preferably inside a real hidden WebView that loads youtube.com and captures the page's own minter function; otherwise by running the BotGuard VM itself (fetched from `jnn-pa.googleapis.com` Waa `Create`, integrity token from `GenerateIT`) inside JSDOM. With the PO token it calls `/youtubei/v1/player` as the **WEB** client (for metadata and the SABR URL), and in the same batch calls `/player` as **TVHTML5 (Samsung TV)**, **VISIONOS** and **IOS_PRODUCER**. For VOD it takes the first of those three whose adaptive-format URLs pass a live byte-range probe (in that order), otherwise falls back to **SABR/UMP** (`serverAbrStreamingUrl`) driven natively by the host. For live it takes the first `hlsManifestUrl` among Samsung TV, VisionOS, iOS Producer, and appends `pot=` to the HLS URL. Login is a **browser cookie capture** (no TV OAuth); rotated cookies are written back by the host engine.

**What this means for us:** every challenge path depends on host capabilities we do not have (V8 + JSDOM to run the player, a real WebView for BotGuard, a remote solver). The only parts directly reusable without a JS engine are: the client table, the "try several non-WEB clients and probe" approach, and the HLS path for live.

## Host (Grayjay engine) capabilities the plugin depends on

Declared in `YoutubeConfig.json`: `packages` = `Http`, `Utilities`, `DOMParser`; `packagesOptional` = `Browser`; `allowEval` false (it still uses the JS `Function` constructor and `eval` inside V8).

- **V8 JavaScript runtime** (ClearScript V8 in the engine). Needed to run YouTube's full player `base.js` and the BotGuard VM.
- **JSDOM** bundled into the script (custom build), giving `window`, `document`, `navigator`, `XMLHttpRequest`. The player JS is run with these globals bound to the JSDOM window.
- **Http package:** synchronous GET/POST/HEAD, a **batch** API that runs several requests in parallel, per-request "use auth" flag (attaches stored login cookies and the captured `authorization` header), cookie jar per client.
- **Utilities:** MD5 of a string (used to key the player JS to the remote solver).
- **DOMParser:** used to parse the RSS XML feed.
- **Browser package (optional):** a hidden WebView with `load(url)`, `waitTillLoaded(ms)`, `run(js, callbackId, cb)`, `addScriptOnLoad(js)` (document-start injection). Used for BotGuard.
- **Native UMP/SABR player** (host feature `UMPSource`), DASH-manifest-from-raw sources, request modifiers per source, `setTimeout`, `bridge.sleep`.
- **Login capture**: an in-app WebView login that records cookies and request headers.

Nightglass has none of these. Our sandbox has outbound HTTP with optional browser TLS fingerprint, storage, and no JS engine.

## 1. Clients

All `/player` calls for non-WEB clients go to `https://youtubei.googleapis.com/youtubei/v1/player?key=<INNERTUBE_API_KEY>&prettyPrint=false&t=<12 random chars>&id=<videoId>`, with the key taken from the WEB `ytcfg` (`INNERTUBE_API_KEY`). The WEB player call goes to `https://www.youtube.com/youtubei/v1/player?prettyPrint=false`. Body common fields for non-WEB clients: `videoId`, `cpn` (16 random chars), `contentCheckOk`, `racyCheckOk` (some older client builders send a misspelled `racyCheckOn` with string values; probably ignored by YouTube), `context.user.lockedSafetyMode=false`, `context.client.hl`=`en`, `gl`=`US`, `utcOffsetMinutes`=0. Unless noted, headers are `Content-Type: application/json`, `X-Goog-Api-Format-Version: 2`, `Accept-Language: en-US, en;q=0.9`.

| Client | `clientName` / version | `X-YouTube-Client-Name` | User-Agent | Context extras | Status |
|---|---|---|---|---|---|
| WEB | from home-page `ytcfg` (`INNERTUBE_CONTEXT`); hard-coded fallbacks seen: `2.20251030.01.00`, `2.20250923.08.00` | 1 | Windows Chrome 110 UA when authenticated | whole `INNERTUBE_CONTEXT` from the page, with `hl`/`gl` forced and `visitorData` cleared on enable | **current**, primary for metadata, PO token, SABR |
| TVHTML5 ("Samsung TV") | `TVHTML5` / `5.20260707` | 7 | `Mozilla/5.0 (SMART-TV; Linux; Tizen 2.4.0) AppleWebKit/538.1 (KHTML, like Gecko) Version/2.4.0 TV Safari/538.1` | `deviceMake` Samsung, `deviceModel` UKS9800, `userAgent` (same UA), `osName` Tizen, `osVersion` 2.4.0; body adds `playbackContext.contentPlaybackContext.html5Preference=HTML5_PREF_WANTS` and `signatureTimestamp` (from the player JS) | **current**, first choice for VOD direct URLs (added 2026-11-13). Headers also `x-youtube-client-version`, `Origin: https://www.youtube.com`, `X-Goog-Visitor-Id`. May be sent with login. **Needs sig/n deciphering** (it sends STS). |
| VISIONOS | `VISIONOS` / `1.04` | 101 (declared; header **not** sent) | `com.google.visionos.youtube/1.04 (RealityDevice17,1; U; CPU visionOS 26_6_0 like Mac OS X; US) gzip` | `deviceMake` Apple, `deviceModel` RealityDevice17,1, `osName` visionOS, `osVersion` 26.6.0; no `platform` | **current** (2026-08-24). Second choice. |
| IOS_PRODUCER | `IOS_PRODUCER` / `0.147.945` | 92 | `com.google.ios.youtubeproducer/0.147.945 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)` | `deviceMake` Apple, `deviceModel` iPhone16,2, `osName` iPhone, `osVersion` 18.3.2.22D82 | **current** (2026-11-21). Third choice. Always anonymous (the source says this explicitly: it never uses login even when others do), only the anonymous `visitorData`. Headers add `x-youtube-client-name`, `x-youtube-client-version`, `X-Goog-Visitor-Id`. |
| IOS | `IOS` / `19.14.3`, device `iPhone15,4`, `osVersion` 17.4.1.21E237, `platform` MOBILE | 5 | `com.google.ios.youtube/19.14.3(iPhone15,4; U; CPU iOS 17_4_1 like Mac OS X; US)` | | **historical**: request is disabled in the batch, its enabling setting no longer exists in the manifest. Kept as a live-HLS fallback that is switched off when the "HTML5 livestreams" setting is on (default on). |
| ANDROID | `ANDROID` / `21.03.36`, `osVersion` 15, `androidSdkVersion` 35, `clientScreen` WATCH, `platform` MOBILE | 3 (not sent) | `com.google.android.youtube/21.03.36 (Linux; U; Android 15; US) gzip` | sent to `https://youtubei.googleapis.com/youtubei/v1/reel/reel_item_watch` with `playerRequest{videoId,cpn,contentCheckOk,racyCheckOn}` and `disablePlayerResponse=false`; response's `playerResponse` holds streaming data | **historical**: disabled in batch; setting removed. An older ANDROID `17.31.35` (Android 12, SDK 31, `params` `8AEB`) builder also exists, unused. |
| ANDROID_VR | `ANDROID_VR` / `1.65.10`, `deviceMake` Oculus, `deviceModel` Quest 3, `osVersion` 12L, `androidSdkVersion` 32, `platform` MOBILE | 28 | `com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; en_US; Quest 3 Build/SQ3A.220605.009.A1) gzip` | | **historical**: added 2026-06-02, "disable android vr" 2026-08-22; request disabled, setting removed. Its live-HLS path code still exists. |
| TVHTML5_SIMPLY_EMBEDDED_PLAYER | `TVHTML5_SIMPLY_EMBEDDED_PLAYER` / `2.0`, `clientScreen` EMBED, `platform` TV; `context.thirdParty.embedUrl` = watch URL; `playbackContext.contentPlaybackContext.signatureTimestamp` + `referer` | not sent | Chromecast UA `Mozilla/5.0 (CrKey armv7l 1.5.16041) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/31.0.1650.0 Safari/537.36` | | Still called, only in the **old (non-session) code path** as the age-gate / "controversial content" bypass. Treated as best effort; the code expects "sign in" reasons back. Effectively **historical**. |
| `visitor_id` endpoint | `https://youtubei.googleapis.com/youtubei/v1/visitor_id?prettyPrint=false` with a client context, returns `responseContext.visitorData` | | | | present, not used in the current flow. |

Notes:
- Mobile web (m.youtube.com) pages are fetched with an iPad Chrome UA (`Mozilla/5.0 (iPad; CPU OS 13_3 like Mac OS X) ... CriOS/87.0.4280.77 ...`) for logged-in context; `X-YouTube-Client-Name: 2` (MWEB) in auth headers when "mobile".
- Every page fetch and the WEB `/player` call sends `Cookie: PREF=hl=en&gl=US` and `Accept-Language: en-US`.
- Which clients need what, per the source's own behaviour (not independently verified):
  - **WEB** `/player`: sent with `serviceIntegrityDimensions.poToken`; its formats are only used via SABR. Its direct `adaptiveFormats` path (append `pot=` to each URL) exists but is disabled (`if(false && ...)`).
  - **TVHTML5, VISIONOS, IOS_PRODUCER**: no PO token is sent in `/player` and none is appended to their media URLs. Their URLs are n-deciphered (and sig-deciphered if `signatureCipher`), then **verified by an actual range request** before use, which tells you the source considers them unreliable per video/IP.
  - Which ones return HLS: the code reads `streamingData.hlsManifestUrl` for live from TVHTML5, VISIONOS, IOS_PRODUCER (and historically ANDROID_VR, IOS).
  - Login: only WEB and TVHTML5 (and VISIONOS if the "use login for details" setting is on) are sent with cookies.

## 2. Catalog

### Channel id resolution
- Direct `/channel/UC...` URLs: the id is the path segment after `/channel/` if it starts with `UC`.
- Otherwise (`@handle`, `/c/`, `/user/`): **current path** fetches the channel page HTML itself (`<url>/videos`, etc.) and reads `ytInitialData`; channel id is at `header.c4TabbedHeaderRenderer.channelId` (old layout) or `metadata.channelMetadataRenderer.externalId` (new `pageHeaderRenderer` layout). Vanity URL: `metadata.channelMetadataRenderer.vanityChannelUrl`; name `metadata.channelMetadataRenderer.title`.
- **Disabled alternative (2026-09-04 "Disable browse API for now")**: POST `https://www.youtube.com/youtubei/v1/navigation/resolve_url?key=<key>&prettyPrint=false` with `{context, url}`; id at `endpoint.browseEndpoint.browseId`. Code is complete but switched off by a constant; the commit history gives no reason.
- Alerts: if `alerts[].alertRenderer.type == "ERROR"`, the channel is treated as unavailable with that text.

### Channel tabs, params, continuations
- Known `browse` `params` per tab (current constants):
  - Videos: `EgZ2aWRlb3PyBgQKAjoA`
  - Shorts: `EgZzaG9ydHPyBgUKA5oBAA%3D%3D`
  - Live/streams: `EgdzdHJlYW1z8gYECgJ6AA%3D%3D`
  - Playlists: `EglwbGF5bGlzdHPyBgoKCEIGCgIQaCIA`
  - Featured/home: no params.
- In the (disabled) browse-API path: POST `/youtubei/v1/browse` with `{context, browseId: "UC...", params}`. If the returned tab under `contents.twoColumnBrowseResultsRenderer.tabs[]` (or `singleColumnBrowseResultsRenderer`) whose `tabRenderer.endpoint.commandMetadata.webCommandMetadata.url` ends with the tab suffix is not `selected`, the source treats the hard-coded params as stale, browses the channel root without params, reads that tab's `tabRenderer.endpoint.browseEndpoint.params`, caches them, and browses again. Good self-healing idea for us.
- **Current path:** GET `https://www.youtube.com/<channel>/videos` (`/streams`, `/shorts`, `/playlists`) HTML, parse `ytInitialData`. Tabs are `contents.twoColumnBrowseResultsRenderer.tabs[].tabRenderer` (or `expandableTabRenderer`) with `title` ("Videos", "Live", "Shorts", "Playlists", "Home") and `content.richGridRenderer` or `content.sectionListRenderer`.
- Rich grid `contents[]` items: `richItemRenderer.content` holds the item; `richSectionRenderer.content.richShelfRenderer` holds shelves; `continuationItemRenderer.continuationEndpoint.continuationCommand.token` is the next-page token. Newer layouts use `continuationItemViewModel.continuationCommand.innertubeCommand.continuationCommand.token`.
- Next page: POST `https://www.youtube.com/youtubei/v1/browse?key=<key>&prettyPrint=false` with `{context, continuation: token}`. Items at `onResponseReceivedActions[0].appendContinuationItemsAction.continuationItems` (or `onResponseReceivedCommands`). If no items come back and the context had no `visitorData`, the source copies `responseContext.visitorData` into the context and retries once.
- Browse retries once on HTTP 408 or 500; a 500 when logged in is reported as "login might have expired".
- Item renderers handled: `lockupViewModel` (current main layout), `videoRenderer`, `richItemRenderer`, `compactVideoRenderer`, `videoWithContextRenderer`, `shortsLockupViewModel`, `channelFeaturedContentRenderer`; `adSlotRenderer` and `reelItemRenderer` are skipped.
- `lockupViewModel` paths:
  - id: `rendererContext.commandContext.onTap.innertubeCommand.watchEndpoint.videoId`
  - title: `metadata.lockupMetadataViewModel.title`
  - thumbnails: `contentImage.thumbnailViewModel.image.sources[]`
  - duration: text badge in `contentImage.thumbnailViewModel.overlays[].thumbnailBottomOverlayViewModel.badges[].thumbnailBadgeViewModel.text` (or `thumbnailOverlayBadgeViewModel.thumbnailBadges[]`), "h:mm:ss"
  - views / date: free text in `metadata.lockupMetadataViewModel.metadata.contentMetadataViewModel.metadataRows[].metadataParts[].text.content` ("N views", "N watching", "N waiting", "3 days ago", "Streamed ...", "Scheduled for ...", "Premieres ..."). The source interprets absolute dates as US Pacific display time, and forces `en`/`US` to keep this English.
  - author (when not on a channel page): first metadata part text; channel id at `metadata.lockupMetadataViewModel.image.decoratedAvatarViewModel.rendererContext.commandContext.onTap.innertubeCommand.browseEndpoint.browseId`.
  - live: overlay badge with text `LIVE` or `badgeStyle` `THUMBNAIL_OVERLAY_BADGE_STYLE_LIVE`, or a metadata row badge `badgeViewModel.badgeStyle == BADGE_STYLE_TYPE_LIVE_NOW`.
  - members-only: row badge `BADGE_MEMBERS_ONLY` (older renderers: `metadataBadgeRenderer.label` "Members only"/"Members first").
- `videoRenderer` paths: `videoId`, `title`, `thumbnail.thumbnails[]`, `lengthText.simpleText`, `publishedTimeText.simpleText`, `viewCountText`; live from `badges[].metadataBadgeRenderer.label == "LIVE"`, `style == BADGE_STYLE_TYPE_LIVE_NOW`, or `thumbnailOverlays[].thumbnailOverlayTimeStatusRenderer.style == "LIVE"`; upcoming from `upcomingEventData.startTime` (unix seconds).
- `shortsLockupViewModel`: id `onTap.innertubeCommand.reelWatchEndpoint.videoId`, title `overlayMetadata.primaryText.content`, views `overlayMetadata.secondaryText`; no date or duration.

### RSS
- `https://www.youtube.com/feeds/videos.xml?channel_id=<UC...>` (current; a setting makes it the only channel source, "inferior"). Fields used: feed `author/name`, `author/uri`; per `entry`: `yt:videoId`, `link@href`, `updated` (used as the upload date), `media:group/media:title`, `media:group/media:thumbnail@url`, `media:group/media:community/media:statistics@views`. No duration and no live flag from RSS.

### Playlists
- Current: GET the playlist page HTML (`/playlist?list=...`, via m.youtube.com when "mobile pages" applies, with cookies), parse `ytInitialData`.
  - Header: `header.playlistHeaderRenderer` (`title`, `ownerText.runs`, `numVideosText`, `playlistId`) or new `header.pageHeaderRenderer` (`pageTitle`; playlist id from a `buttonViewModel.onTap.innertubeCommand.watchEndpoint.playlistId`; owner from `metadataRows[].metadataParts[].avatarStack.avatarStackViewModel`; count from a "N videos" text part).
  - Items: `playlistVideoListRenderer.contents[].playlistVideoRenderer`; next page token from `continuationItemRenderer.continuationEndpoint.continuationCommand.token`, fetched via `/browse` `{continuation}`. Some playlists render as `richGridRenderer` or `sectionListRenderer` instead.
- Disabled browse alternative: `browseId` = `VL` + playlist id.
- Logged-in library playlists: GET `https://m.youtube.com/feed/library` with an Android Chrome UA; playlist URLs from `compactPlaylistRenderer` / `playlistCardRenderer` `navigationEndpoint`; Liked (`list=LL`) and radio mixes are excluded.
- Subscriptions (login): GET `https://m.youtube.com/feed/channels`; `sectionListRenderer.targetId == "browse-feedFEchannels"`, items `channelListItemRenderer.navigationEndpoint`; fallback `channelListSubMenuRenderer` / `channelListSubMenuAvatarRenderer`. Disabled browse alternative `browseId` `FEchannels`.
- History (login): `https://m.youtube.com/feed/history`, continuation via browse with `X-Goog-Pageid` = `DELEGATED_SESSION_ID`.
- Watch Later: not handled explicitly.
- Other browse ids seen: `FEtrending`, `FEwhat_to_watch`, `FEsubscriptions`.

### Search (not in our contract)
- POST `/youtubei/v1/search?key=...` with `{context, query, params}`; default `params` `8AEB`; channels `EgIQAg%3D%3D`; playlists `EgIQAw%3D%3D`; continuation with `{context, continuation}`. Suggestions: `https://suggestqueries-clients6.youtube.com/complete/search?client=youtube&gs_ri=youtube&ds=yt&q=`.

### Page-scraping facts
- `ytInitialData` is found after `var ytInitialData = ` in a script tag; sometimes it is a single-quoted string with `\xNN` hex escapes that must be decoded before JSON parsing. Alternative location: a `<script id="yt-initial-data">` tag. `ytInitialPlayerResponse` similarly.
- `ytcfg.set({...})`: the right call is the one whose JSON contains `INNERTUBE_CONTEXT`.
- EU consent: if the HTML contains a form posting to `https://consent.youtube.com/save`, the source POSTs `gl=US&m=0&app=0&pc=yt&continue=<url>&x=6&bl=boq_identityfrontenduiserver_20231017.04_p0&hl=en&src=1&cm=2&set_eom=true` (form-encoded) to that URL to refuse consent.
- Expired login: a short page (<100 KB) containing `accounts/answer/61416` means "cookies expired".
- Captcha: HTTP 429 with a body containing "captcha".

## 3. The player request (WEB)

POST `https://www.youtube.com/youtubei/v1/player?prettyPrint=false`, body:
- `videoId`, `context` = page `INNERTUBE_CONTEXT`
- `playbackContext.contentPlaybackContext`: `signatureTimestamp` (integer STS), `html5Preference: HTML5_PREF_WANTS`, `currentUrl: /watch?v=<id>`, `referer: https://www.youtube.com/watch?v=<id>`, `autoCaptionsDefaultOne: false`, `autonavState: STATE_OFF`, `lactMilliseconds: "-1"`, `splay: false`, `vis: 0`, `watchAmbientModeContext {hasShownAmbientMode: true, watchAmbientModeEnabled: true}`
- `racyCheckOk` / `contentCheckOk`: false normally, true on the retry after age/content verification
- `serviceIntegrityDimensions.poToken` when a token exists.

Age / content-check retry: POST `https://www.youtube.com/youtubei/v1/verify_age` with `{context, nextEndpoint.watchEndpoint{videoId, playerParams: "QAA%3D", racyCheckOk: true, contentCheckOk: true}, setControvercy: true}` using login, then repeat `/player` with both checks true. Requires login.

`playabilityStatus` handling:
- `LOGIN_REQUIRED` or `CONTENT_CHECK_REQUIRED`: retry with login (verify_age + player) if logged in and allowed; otherwise "login required" with `playabilityStatus.reason`.
- `UNPLAYABLE`: error with reason; members-only content ("member-only content" in the reason) triggers the login retry.
- Age gate detected by `reason` containing "your age"; controversial content by `errorScreen.playerErrorMessageRenderer.reason.simpleText` containing "following content may".
- Error text: `errorScreen.playerErrorMessageRenderer.subreason`.
- Upcoming/offline live: `playabilityStatus.liveStreamability.liveStreamabilityRenderer.offlineSlate.liveStreamOfflineSlateRenderer.scheduledStartTime` (unix seconds). If the scheduled time is later than one hour ago and no live source was found, the video is returned with no sources (waiting state).
- Bot check: not handled by name; a 429 with "captcha" raises a captcha flow.

`streamingData` fields used: `adaptiveFormats`, `hlsManifestUrl`, `dashManifestUrl`, `serverAbrStreamingUrl`. `formats` (muxed) and `expiresInSeconds` are not used. SABR needs `playerConfig.mediaCommonConfig.mediaUstreamerRequestConfig.videoPlaybackUstreamerConfig`. Metadata: `videoDetails.{videoId,title,thumbnail.thumbnails,channelId,author,lengthSeconds,viewCount,isLive,shortDescription}`, `microformat.playerMicroformatRenderer.{viewCount,lengthSeconds}`, `storyboards.playerStoryboardSpecRenderer.spec`.

Storyboard spec (for seek thumbnails): `|`-separated; the first part is a URL template with `$L` (level) and `$N` (sheet name); each later part is `#`-separated: width, height, frame count, columns, rows, interval ms, name template (containing `$M` = sheet index), signature. The signature is appended as `sigh=`.

## 4. Formats

- Adaptive formats are kept only if `initRange.end` and `indexRange.end` exist and `approxDurationMs` parses (so OTF/live-only formats are dropped from the direct path). Codec = text inside `codecs="..."` in `mimeType`; container = `mimeType` before `;`.
- AV1 (`av01`) is dropped from the direct path, and from SABR unless the user allows it.
- Audio: `audioTrack.displayName`, `audioTrack.id` (language prefix before `.`), `audioTrack.audioIsDefault`. "Original" track = the one whose `displayName` contains "original" if any track says so, else `audioIsDefault`. `isDrc` marks the "normalized" (DRC) variant. `isVb` marks "voice boost" tracks, which the source avoids when choosing audio. `audioChannels`, `contentLength`. `xtags` and `lastModified` are passed to SABR.
- URL handling: if `url` present, n-decipher it; else parse `signatureCipher` (or legacy `cipher`) as a query string (`url`, `s`, `sp`), decipher `s`, append `&<sp>=<sig>`, then n-decipher. `n` can be in the query (`n=`) or as a path segment `/n/<value>/`. `cpn` (16 random chars) is appended if missing.
- Requests to googlevideo for direct sources:
  - a `Range` header is converted into a `range=` query parameter;
  - `rn` = an incrementing request number per source;
  - `TE: trailers`;
  - User-Agent chosen by the `c=` parameter in the URL (ANDROID, IOS, IOS_PRODUCER, VISIONOS and TVHTML5 each get their own client UA from the table above, otherwise Windows Chrome);
  - for `c=WEB` or `c=TVHTML5_SIMPLY_EMBEDDED_PLAYER` also `Origin`/`Referer: https://www.youtube.com`, `Sec-Fetch-Dest: empty`, `Sec-Fetch-Mode: cors`, `Sec-Fetch-Site: cross-site`.

  Byte skipping is disabled. No explicit chunk-size cap is set in the direct path.
- **Source verification (current):** for each candidate client, take its first audio format; get content length from `contentLength` or the `clen` URL parameter (only fall back to HEAD if both are missing, change of 2026-11-21); request the byte range from (length minus 150) to (length minus 50); if that succeeds the client's sources are used. All candidates are probed in parallel batches. This catches 403s from URLs that look fine (the old comment speaks of "active blocking").
- SABR/UMP:
  - the URL is `serverAbrStreamingUrl` with n deciphered, plus `rn=` and `alr=yes`;
  - the request body is a protobuf VideoPlaybackRequest built from the ustreamer config, the PO token, and client info (WEB, client name 1, version `2.20250923.08.00`, Windows 10.0);
  - if no PO token exists, a **cold-start token** is used (see 6).

  The native-UMP path refuses to run without a browser-minted token (2026-11-21). The source also has a host-driven "DASH from UMP" fallback. Not usable by us (needs a SABR client; the brief says mpv cannot play it and the relay does not rewrite DASH).
- Live versus VOD: decided by `videoDetails.isLive` (SABR also checks `isLiveNow`). `isLiveContent` and `isPostLiveDvr` are **not** used by this source.

## 5. Player JavaScript and challenges

- **Player URL:** from the home page or watch page HTML, the JSON value after `"PLAYER_JS_URL":` (fallback `ytcfg.PLAYER_JS_URL`), a path like `/s/player/<8-hex id>/player_ias.vflset/en_US/base.js`, fetched from `https://www.youtube.com` + path. The player id is the segment after `/s/player/`. The source keeps a list of about 40 known player ids (newest first, latest `99f55c01`) as regression tests.
- **STS:** the first `signatureTimestamp` followed by `:` or `=` and digits in the player JS. Missing STS is a hard error.
- **Preprocessing:** the source detects a global constant string array declared near the top of the player (recognisable because one of its entries is `index.m3u8`; either a literal array, or a long string followed by `.split(...)`), evaluates it, and inlines every indexed reference to that array into the code. This is the "global array" obfuscation layer.
- **Finding the functions:** the source **does not locate them itself anymore**. It hashes the raw player JS with MD5 and asks:
  - `GET https://solutions.grayjay.app/<md5>` (fast cache);
  - else `GET https://solver.grayjay.app/jobs/<md5>`;
  - else `POST https://solver.grayjay.app/jobs` with `{playerJsUrl, body: <whole player JS>}`;
  - then polls up to 15 times, once per second.

  These requests carry Grayjay-specific headers (a static token, a random load-balancer id, platform and version). The status codes are 0 = pending, 1 = solved, 2 = failed, and "no job" is handled as unknown/start. A solved entry gives, for each of sig and n, a global **function name** and a **parameter list** in which one slot is the input string (other slots are constant arguments). It also carries a validation flag for sig; sig is optional (passthrough if missing), n is mandatory.
- **Executing:** the whole player JS is run in V8 with `window`, `document`, `navigator` and `XMLHttpRequest` bound to a JSDOM window. Its IIFE footer is patched so the named functions get exported onto a scope object, falling back to the `_yt_player` object. **So the source needs a full JS engine plus a DOM shim; it does not use a mini-interpreter.** The changelog shows "Sig solver optional" (v336) and "N decryption fix" (v369/371, 2026-11-13), so n is the part that breaks.
- **Caching:** per player URL in memory for the session (decipher functions, STS, raw script); remote solutions per MD5. Nothing is persisted across app restarts beyond the host's saved state (client context).
- Known test vectors kept in source: player `5ec65609` maps n `IMKPt9L6DXEYGGRzJ` to `zL66zSisCR_I4g`; player `6742b2b9` maps n `Cmy1zDmHEfSutp0R` to `Fk0DqlPV0q44cA`. (Historical, useful only if we fetch those players.)

## 6. PO tokens and BotGuard

- **Which token:** the WEB `/player` call and SABR get a token. The page `ytcfg.WEB_PLAYER_CONTEXT_CONFIGS.WEB_PLAYER_CONTEXT_CONFIG_ID_KEVLAR_WATCH.serializedExperimentFlags` is parsed as a query string:
  - `html5_generate_content_po_token=true` means a **content-bound** token (bound to the **video id**);
  - otherwise, with `html5_generate_session_po_token=true`, a **session-bound** token (bound to the visitor data, or to `DATASYNC_ID` when logged in);
  - if the config is missing, the source defaults to content-bound.
- The visitor binding comes from `ytcfg.EOM_VISITOR_DATA`, falling back to `ytcfg.VISITOR_DATA`, with `%3D` decoded to `=`. Login uses `DATASYNC_ID`.
- Non-WEB clients (TVHTML5, VISIONOS, IOS_PRODUCER) get **no** token in this source.
- Captions: `&pot=<token>` (and in the async path also `&potc=1&c=<clientName>&cver=<clientVersion>`, using the WEB client values) appended to the caption `baseUrl`, with a content-bound token for the video id. Live HLS: `?pot=<token>` appended to the manifest URL (setting on by default).
- **Token encoding:** the mint output bytes are base64url-encoded (`+` becomes `-`, `/` becomes `_`, padding kept).
- **Cold-start token** (fallback for SABR when no token is minted), byte layout:
  1. byte `0x22`, then the payload length (8);
  2. two random bytes;
  3. one byte "j" (1), then one byte "client state" (1);
  4. the 4-byte big-endian unix time in seconds.

  Every payload byte from index 2 on is XORed with the random byte at (index mod 2). The result is standard base64.
- **BotGuard, engine path (fallback):**
  1. POST `https://jnn-pa.googleapis.com/$rpc/google.internal.waa.v1.Waa/Create`, body JSON `["O43z0dpjhgX20SCx4KAo"]` (request key), headers `content-type: application/json+protobuf`, `x-goog-api-key: AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw`, `x-user-agent: grpc-web-javascript/0.1`, a macOS Chrome-like UA.
  2. Response element [1] is a base64url string (`.` used as padding) whose bytes, each plus 97 mod 256, give a JSON array: message id, wrapped interpreter script, wrapped URL, interpreter hash, program, global name, (unused), client experiments state blob.
  3. Run the interpreter script; take the object at `globalThis[globalName]`; call its `a` function with the program, a snapshot callback, `true`, `null`, a no-op, and `[[],[]]`.
  4. Call the snapshot function with `[[], undefined, results, undefined]`. It resolves the "BotGuard response" string and puts a **minter constructor** at results[0].
  5. POST `.../Waa/GenerateIT` with `[requestKey, botguardResponse]`, which returns `[integrityToken, ttlSeconds]`.
  6. Decode the integrity token (base64url) to bytes and call the constructor with those bytes; it returns the minter.
  7. Minter(UTF-8 bytes of the binding id) gives the token bytes.

  This runs in V8 with JSDOM `window`/`document`. **There is no `/att/get` and no `GenerateIT` from youtube.com in this path.**
- **BotGuard, browser path (preferred, mandatory for native SABR since 2026-11-21):** a hidden WebView loads `https://www.youtube.com` (desktop) or `https://m.youtube.com/` (mobile). A document-start script hooks the page global `trayride` (the object the page's BotGuard loader assigns) to intercept its `a` call chain and capture the minter function the page itself creates. Tokens are minted by calling that captured function inside the page. The source then assumes a **5-minute** lifetime for the minter (unknown expiry, refresh 30 s before), caches tokens per binding kind and id for that generation, and on expiry reloads the page to get a new minter. It waits up to 20 s for the page to load and 10 to 15 s for the minter.
- `X-Goog-Visitor-Id` header and `context.client.visitorData` carry the visitor data for TV/iOS-producer/VisionOS calls. `VISITOR_INFO1_LIVE` is not handled explicitly; it comes in through the cookie jar.

## 7. Login

- **No TV device-code OAuth** anywhere in the current source (searched for oauth/device code; none). Login is a WebView sign-in:
  - start URL `https://accounts.google.com/ServiceLogin?service=youtube&uilel=3&passive=true&continue=https%3A%2F%2Fwww.youtube.com%2Fsignin%3Faction_handle_signin%3Dtrue%26app%3Ddesktop%26hl%3Den%26next%3Dhttps%253A%252F%252Fwww.youtube.com%252Fchannel_switcher&hl=en&ec=65620`, with a mobile Chrome 120 UA;
  - completion when the WebView requests `https://m.youtube.com/youtubei/v1/guide?*`;
  - the host captures the request header `authorization` on `.youtube.com` (the browser's `SAPISIDHASH ...` value) and the cookies (it must see `SIDCC`);
  - the user must pick a profile on `channel_switcher`.
- The captured `authorization` header is **replayed verbatim** on every authenticated request; neither plugin nor engine recomputes SAPISIDHASH (no SAPISID code in either). Authenticated requests also send `x-goog-authuser` (`0` or `SESSION_INDEX`), `x-goog-pageid` (`DELEGATED_SESSION_ID`), `x-origin` (`https://www.youtube.com` or m.), `x-youtube-client-name` 1 or 2.
- **Rotated `__Secure-*PSIDTS` cookies:** handled by the **host engine**, not the plugin:
  - on any response, each `Set-Cookie` whose name already exists in the stored login cookie map for that domain is written back;
  - the encrypted auth blob is persisted, and the new value is shared with every other client instance of the plugin. Engine commit 34340b5 (2026-11-02) and its tests use `__Secure-1PSIDTS` on `.youtube.com` as the example;
  - cookies not already present (e.g. `YSC`) are not persisted, and deletions (empty value) are ignored.

  The plugin never calls `accounts.google.com/RotateCookies` (that host is not even in `allowUrls`); rotation relies on YouTube's own responses setting fresh PSIDTS.
- Features using login (opt-in settings): channel pages, video details (default off), login fallback for age/member/content-check (default on), subscriptions, library playlists, history, watch-time reporting. Logged-in session state is a second context (`ytcfg` from m.youtube.com with the iPad UA).
- Expiry signals: "accounts/answer/61416" in a short page, or a browse 500 while logged in.

## 8. Captions

- Path: `captions.playerCaptionsTracklistRenderer.captionTracks[]` with `baseUrl` (relative `/api/...` gets `https://www.youtube.com` prefixed), `name`, `languageCode`; `kind` read from the `kind=` query parameter (`asr` for auto captions).
- The source fetches `baseUrl` **without** `fmt` and parses the default timedtext XML (`<text start="" dur="">...</text>`), converting it to WebVTT itself, clipping each cue's end to the next cue's start. Tracks whose URL says `kind=vtt` are passed through as VTT.
- PO token: appended (`pot=`, plus `potc=1&c=&cver=` in one path) when available, with a 2-second timeout for minting; otherwise fetched without. The source treats the token as needed for reliability, but does not stop if it is missing.
- Translated tracks (`translationLanguages`, `tlang`): not used.

## 9. Live

- Live is decided by `videoDetails.isLive`. HLS is taken from `streamingData.hlsManifestUrl` of, in order: (ANDROID_VR, disabled), TVHTML5/Samsung TV, VISIONOS, IOS_PRODUCER; then native SABR; then (IOS, disabled by default); then the WEB response's own `hlsManifestUrl`/`dashManifestUrl`. The chosen manifest URL is n-deciphered (n may be a `/n/<v>/` path segment) and gets `?pot=<token>` appended (setting "use HTML5 livestreams POT", default on).
- Upcoming: `upcomingEventData.startTime` in lists; `scheduledStartTime` in the offline slate on the player response. Premieres were fixed in 2026-08 ("Support premiere"). No sources are returned until shortly before the start.
- DVR window, low-latency (`latencyClass`), `isLiveContent`, `isPostLiveDvr`: not handled. Live chat: `https://www.youtube.com/live_chat` and `/youtubei/v1/live_chat/get_live_chat` (out of scope).

## 10. Failure modes and churn (from history and code)

- **n-challenge**: the most frequent break. Fixed 2026-11-13 (v369/371). The source moved its function discovery to a remote service, so FUTO can update it without shipping a plugin release. Errors read like "N decryptor function [...] is not available" or "Remote n decryptor unavailable", and include the MD5 of the player.
- **Direct-URL clients rotate**: in six months, Android VR was added (June) then disabled (August), VisionOS added (August), Samsung TV added (November), iOS Producer added (2026-11-21). IOS and ANDROID are gone. Expect any given client to stop giving usable URLs within weeks; the source compensates by probing several in parallel each time.
- **PO token / attestation**: native SABR now refuses to run without a browser-minted token (2026-11-21). The minter lifetime is unknown to the source (assumed 5 minutes).
- **Browse API**: disabled 2026-09-04 in favour of HTML scraping for first pages; continuations still use `/browse`.
- **Experiments**: the source lists experiment ids 51217102 and 51217476 as "invalid" and 51397094 as "reload required", read from `ytcfg.FEXP_EXPERIMENTS` (old code path only).
- **Retry strategy**:
  - browse: one retry on 408/500;
  - continuation: retry once with fresh `visitorData`;
  - captcha: surfaced to the user (cookie `GOOGLE_ABUSE_EXEMPTION`);
  - SABR: "aggressive recovery" that re-fetches `/player` and re-mints the token.
- The subscription rate limit declared in the manifest is 140.

## Open questions

- Whether TVHTML5 (Samsung, `5.20260707`), VISIONOS (`1.04`) and IOS_PRODUCER (`0.147.945`) direct URLs play **without any PO token** from a typical IP, and for how long. The source only tells us it probes them every time; we should confirm with a few light `/player` calls.
- Whether TVHTML5 URLs need sig deciphering in practice (the source sends STS, implying signatureCipher), and whether VISIONOS and IOS_PRODUCER URLs carry `n` at all. If they don't, those clients would avoid the player JS entirely, which matters most for us.
- How long the minter and integrity token actually live (`GenerateIT` returns a TTL; the browser path does not know it).
- Whether BotGuard can run without a DOM; the source always provides JSDOM or a real browser, so it does not settle this.
- Whether the live HLS `pot=` is required, or only helps.
- Whether a TV OAuth flow still works (not attempted by this source).
- How to locate the n function without a remote service: the source no longer documents this.
