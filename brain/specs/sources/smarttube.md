# Source spec: SmartTube / MediaServiceCore

Facts-only clean-room specification. No code, identifiers or comments from the source are reproduced; behaviour is described in prose. Protocol values (URLs, JSON paths, header names, client names, public keys, `params` blobs) are facts about YouTube.

## Source, licence, commits

| Repo | Licence | Commit read | Date |
|---|---|---|---|
| yuliskov/SmartTube (`ST`) | MIT | `a0c319a8` | 2026-11-22 |
| yuliskov/MediaServiceCore (`MSC`), submodule of ST, pinned at its HEAD | MIT (licence file added 2026-10-24, `cb24f1da`) | `bad6cb57` | 2026-11-22 |
| yuliskov/SharedModules (`SM`), submodule of both | (not checked; only consulted for the JS engine build) | `e12618a0` | 2026-11-18 |

Almost all YouTube logic is in MSC. ST adds the player side: choosing DASH / HLS / SABR / progressive, and an ExoPlayer SABR module.

Path conventions:
- `yt/` = `MSC:youtubeapi/src/main/java/com/liskovsoft/youtubeapi/`
- `gc/` = `MSC:youtubeapi/src/main/java/com/liskovsoft/googlecommon/`
- `assets/` = `MSC:youtubeapi/src/main/assets/`

Every fact is **current** at the commits above unless it is marked **historical**. Observations marked "observed 2026-11-23" come from 9 anonymous requests to YouTube on that date: 4 by a sub-researcher and 5 for this spec.

## Summary of the current strategy

SmartTube behaves like the YouTube TV ("leanback") app and uses the TV client for everything it can.
- **Catalogue and login:** home, subscriptions, history, playlists, search and channel pages use the `TVHTML5` Innertube client with TV `tvAppInfo` extras. Sign-in is the TV OAuth device-code flow, using the client id and secret that YouTube embeds in its own `/tv` page scripts.
- **Playback:** it walks an ordered list of clients and keeps the last one that worked. The order is:
  1. TVHTML5 7.x with a Fire TV Cobalt user agent; the only client that supports login, and the default;
  2. TVHTML5 with a Samsung Tizen user agent;
  3. WEB, through a YouTube.js-style session;
  4. VISIONOS;
  5. a "downgraded" TVHTML5 5.x client with an old Cobalt user agent, which gets the non-TCL player, 5-digit STS and plain URL formats;
  6. WEB_EMBEDDED_PLAYER, WEB with a Safari user agent, iOS, a geo-fix WEB, MWEB, and ANDROID_VR.
- **Signature and n:** the player JS is downloaded and solved inside an embedded **V8 (J2V8)**.
  - Regular players use yt-dlp's bundled "ejs" solver, on a meriyah parser plus the astring code generator.
  - The new TV "TCL" player variant uses SmartTube's own AST solver.
- **PO tokens:** minted on the device by running Google's real BotGuard in an **Android WebView**, with no cloud service.
  - Web clients get a token bound to the video id.
  - TV clients get a session token bound to the `LIVING_ROOM_PO_TOKEN_ID` scraped from `/tv`.
  - The same token goes into `/player` (`serviceIntegrityDimensions.poToken`), into every stream URL (`pot=`) and into caption URLs.
- **Player feed:** SmartTube builds its own **DASH MPD** from `adaptiveFormats`: SegmentBase for VOD, SegmentTemplate for OTF and live. It can merge in the iOS/visionOS HLS manifest for "extended" high-bitrate formats. A native **SABR/UMP** implementation in ExoPlayer is used when formats come without URLs.

---

## 1. Clients

### 1.1 Client table

All clients below come from `yt/innertube/utils/Constants.kt` (versions, user agents, numeric ids) and `yt/common/helpers/AppClient.kt` (screen, extra context fields, flags) [current]. The numeric id is sent as `X-Youtube-Client-Name`. User agents are listed in 1.2.

| Role in source | `clientName` | `clientVersion` | id | User agent | Referer base | Extra `context.client` fields |
|---|---|---|---|---|---|---|
| Default TV, auth-capable | `TVHTML5` | `7.20260901.15.00` | 7 | Fire TV Cobalt UA | `https://www.youtube.com/tv` | browse only: `tvAppInfo{appQuality:"TV_APP_QUALITY_FULL_ANIMATION", zylonLeftNav:true}`, `webpSupport:false`, `animatedWebpSupport:true` |
| TV, legacy UI | same as default TV | same | 7 | same | same | `appQuality:"TV_APP_QUALITY_LIMITED_ANIMATION"`: Shorts merged into Subscriptions |
| Samsung TV | `TVHTML5` | `5.20260707` | 7 | Tizen 2.4 UA | `/tv` | os Tizen 2.4.0, make Samsung, model UKS9800 |
| Downgraded TV | `TVHTML5` | `5.20260901` | 7 | old Cobalt UA | `/tv` | none |
| TV Simply | `TVHTML5_SIMPLY` | `1.0` | 74 | Fire TV UA | `/tv` | none; source says no auth |
| TV embedded | `TVHTML5_SIMPLY_EMBEDDED_PLAYER` | `2.0` | 85 | Fire TV UA | `/tv` | `clientScreen:"EMBED"` |
| TV Kids (historical, source says merged into TV) | `TVHTML5_KIDS` | `3.20231113.03.00` | none | Fire TV UA | `/tv/kids` | none |
| WEB | `WEB` | `2.20260907.06.00` | 1 | Chrome 124 Windows | `https://www.youtube.com` | none |
| WEB, Safari UA | `WEB` | `2.20260907.06.00` | 1 | Safari 15.5 Mac | same | none |
| Web embedded | `WEB_EMBEDDED_PLAYER` | `2.20260908.01.00` | 56 | Safari 15.5 Mac | same; per-video referer `…/embed/<id>?html5=1` | `clientScreen:"EMBED"`, `thirdParty.embedUrl` set to any non-YouTube URL (the source uses a reddit URL), and `encryptedHostFlags` from the embed page config when available |
| Web creator | `WEB_CREATOR` | `1.20241203.01.00` | 62 | Chrome | `https://studio.youtube.com` | none |
| Web music | `WEB_REMIX` | `1.20260707.12.00` | 67 | Chrome | `https://music.youtube.com` | none |
| Mobile web | `MWEB` | `2.20260907.05.00` | 2 | iPad Safari 16.6 | `https://m.youtube.com` | none |
| Web kids | `WEB_KIDS` | `2.20260205.00.00` | 76 | Chrome | none | none |
| Android | `ANDROID` | `21.26.364` | 3 | `com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip` | none | `androidSdkVersion:30`, `osName:"Android"`, `osVersion:"11"` |
| Android VR | `ANDROID_VR` | `1.65.10` | 28 | `com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip` | none | sdk 32, osVersion 12, `deviceMake:"Oculus"`, `deviceModel:"Quest 3"` |
| iOS | `iOS` | `21.26.4` | 5 | `com.google.ios.youtube/21.26.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)` | none | `deviceModel:"iPhone16,2"`, `osVersion:"18.3.2.22D82"` |
| visionOS | `VISIONOS` | `1.02` | 101 | Safari 26 on macOS 15.7.3 | none | `deviceModel:"RealityDevice17,1"`, `osVersion:"26.5.23O471"` |
| Android creator (defined, unused) | `ANDROID_CREATOR` | `22.43.101` | 14 | none | none | none |
| Android music (defined, unused) | `ANDROID_MUSIC` | `5.34.51` | 21 | none | none | none |

API keys:
- **Innertube key, current:** `AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8`. It is sent as the `key=` query parameter only on anonymous requests. Source: `yt/common/helpers/AppConstants.kt`, `gc/common/helpers/RetrofitOkHttpHelper.kt`.
- **Old key, historical:** `AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8`.
- **WEB static visitor id:** `6zpwvWUNAco`, listed in the client constants.

Innertube base URLs listed: `https://www.youtube.com/youtubei/` (used), `https://youtubei.googleapis.com/youtubei/` (used for the reel endpoint), and several sandbox hosts (`green-`, `release-`, `test-youtubei.sandbox.googleapis.com`, `uytfe.sandbox.google.com`) that are unused.

### 1.2 User-agent facts

Source: `gc/common/helpers/DefaultHeaders.kt` [current].

**TV user agent, used for the default TV client and for every catalogue call:**
`Mozilla/5.0 (Linux armeabi-v7a; Android 7.1.2; Fire OS 6.0) Cobalt/22.lts.3.306369-gold (unlike Gecko) v8/8.8.278.8-jit gles Starboard/13, Amazon_ATV_mediatek8695_2019/NS6294 (Amazon, AFTMM, Wireless) com.amazon.firetv.youtube/22.3.r2.v66.0`
- The source's notes on alternatives:
  - A Samsung SamsungBrowser 9.2 UA gives "no buffering" but only 320x180 thumbnails.
  - A synthesised Android TV Cobalt UA "buffers badly".
  - A Chrome desktop UA causes throttling, "probably Chrome engine checks".

**Downgraded TV user agent:** `Mozilla/5.0 (DirectFB; Linux x86_64) Cobalt/4.13031-qa (unlike Gecko) Starboard/1`.
- With this UA, YouTube serves the old 5-digit signature timestamp and a non-TCL player.
- The commit "fix restricted videos by downgrading UA" (`86c87883`, 2026-09-08) introduced it.

**Samsung client UA:** `Mozilla/5.0 (SMART-TV; Linux; Tizen 2.4.0) AppleWebKit/538.1 (KHTML, like Gecko) Version/2.4.0 TV Safari/538.1`.

**Web UAs:**
- Chrome: `Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36`.
- Safari: `Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.5 Safari/605.1.15,gzip(gfe)`.
- MWEB: an iPad iOS 16.7.10 Safari UA, also ending in `,gzip(gfe)`.

**`browserName` / `browserVersion` in `context.client`:** derived from the UA. The source checks for "SamsungBrowser", then "LG Browser", "Cobalt", "Chrome", "Safari", and takes the token after `<name>/`. So the default TV client sends `browserName:"Cobalt"`. A note says Cobalt is what makes Shorts appear in feeds. Source: `yt/common/helpers/AppClient.kt`.

### 1.3 Common request shape

Source: `yt/common/helpers/QueryBuilder.kt` and `gc/common/helpers/RetrofitOkHttpHelper.kt` [current].

**Every Innertube call is a POST with a JSON body.**
- `context.client` holds:
  - `clientName`, `clientVersion`, `clientScreen` (`WATCH`, or `EMBED` for the embedded clients) and `userAgent`;
  - the optional browser and device fields;
  - `acceptLanguage`, `acceptRegion` and `utcOffsetMinutes`, sent as a string. There is no `hl`/`gl`;
  - `visitorData`.
- `context.user` is `{enableSafetyMode:false, lockedSafetyMode:false}`.
- `context.clickTracking.clickTrackingParams` is added when the item came from a feed.
- `racyCheckOk:true` and `contentCheckOk:true` are always at the top level.

**Interceptor additions for `youtubei/v1` URLs:**
- `prettyPrint=false`.
- `key=<api key>`, only when no auth header is present.
- `t=<random 12-char string>`, only on `/browse`, `/next`, `/reel` and `/playlist`.
- `X-Goog-Visitor-Id: <visitorData>` on `/browse`, `/search`, `/player`, `/reel/`, `/next` and `/api/stats/`. The source says this fixes an empty Home for anonymous users.
- `Accept-Encoding: gzip, deflate`. The source avoids `br` because of RAM use.
- By default, the TV UA and `Referer: https://www.youtube.com/tv`.

**`/player` calls also send** `X-Youtube-Client-Name`, `X-Youtube-Client-Version` and `User-Agent` for the chosen client (`yt/videoinfo/V2/VideoInfoApi.java`).

### 1.4 Why TV, and what each client is good for

Per the source's own annotations: `yt/service/internal/FormatInfoWrapper.kt` and `yt/common/helpers/AppClient.kt`, history `63a7d61c` 2026-11-13 "make TV client default one again", and `1087a014` 2026-11-14.

**Only TV clients can carry the OAuth token.** Auth is accepted on TV, TV legacy, TV embedded, TV Kids, downgraded TV and Samsung TV.
- TV Simply is explicitly marked as not supporting auth.
- History sync, "please sign in" fixes, Premium formats and restricted videos all need the TV client.
- If an anonymous client produced the playable formats, SmartTube re-fetches `/player` with the downgraded TV client *with* auth before reporting watch history.

**PO token needs**, flags in `AppClient.kt`:
- **Web token:** WEB, MWEB, WEB_EMBEDDED, WEB-Safari, and the two internal WEB copies (initial-response and geo).
- **TV token:** the default TV client and the Samsung TV client.
- **No token:** the downgraded TV client, TV Simply, TV embedded, Android, Android VR, iOS and visionOS.

**Clients flagged "playback broken until SABR is implemented":** every web variant, Android VR, MWEB, TV embedded and iOS. They are still in the rotation, because SmartTube now has a SABR player.

**Per-client notes:**

| Client | Note |
|---|---|
| Samsung TV | "Still contains DASH formats" |
| WEB | Fixes "video blocked in current location" |
| visionOS | Returns no URL formats; used for its HLS manifest |
| Web embedded | Meant for 18+ videos, annotated "not working" |
| MWEB | Single audio language |
| TV embedded | Single audio language; commented out |
| Android VR | No PO token and no cipher, but "often hangs". A note says `clientVersion` above 1.65 may return SABR-only streams, and that "made for kids" videos are unavailable |
| Android reel (`/reel/reel_item_watch`) | "No pot and no cipher" but "hangs on all engines"; disabled |
| Android without SDK version | Disabled ("hangs on Cronet") |
| TV Simply | Disabled ("hangs?") |

**Geo fix:** a WEB request with `params:"CgIQBg%3D%3D"` in the `/player` body bypasses geo-blocking (`yt/common/helpers/QueryBuilder.kt`).

**Observed 2026-11-23**, anonymous, public video `jNQXAC9IVRw`, `/tv` visitorData, no PO token:
- **TVHTML5 `7.20260901.15.00` with the Fire TV UA and STS `20774001`:**
  - `playabilityStatus.status` was `OK`.
  - `streamingData` had `expiresInSeconds` (21540), `formats`, `adaptiveFormats`, `probeUrl` and `serverAbrStreamingUrl`.
  - **The 14 adaptive formats had no `url` and no `signatureCipher`**, so they are SABR-only.
  - Progressive itag 18 had a ready `url` with `c=TVHTML5`, `sig` already present (no cipher) and an `n` parameter.
- **TVHTML5 `5.20260901` with the old Cobalt UA and STS `20774`:**
  - `OK`, with no `serverAbrStreamingUrl`.
  - **Every adaptive format had a plain `url`**, with `sig` present and no cipher.
  - A `range=0-9999` GET of the audio URL with the **unsolved `n`** returned **HTTP 403**.

### 1.5 Client rotation logic

Source: `yt/service/internal/FormatInfoWrapper.kt` [current; reworked 2026-11-11 to 2026-11-14].
- **Start:** with the last client that produced a playable result (persisted), or the first in the list.
- **Each attempt:** first reset the client playback nonce (CPN). The source calls this a "history/pot fix": the CPN must be unique per `/player` fetch.
- **Three passes over the list:**
  1. the first result that is not "unplayable";
  2. the first that has progressive URL formats ("480p fallback");
  3. the first result of any kind, to show the error.
- **On a player error (403 etc.)** the app asks the wrapper to move on:
  - first it resets the PO-token cache of the current client, at most once per 60 s per token family, and retries the same client;
  - then it moves to the next client.
  - The 60 s throttle was briefly 3 min and reverted on 2026-11-21 (`3c733fef`).
- **Caching:** one format result is cached in memory. It counts as fresh while it has media, the player JS cache is current, and the client's PO token has not expired.
- **Rental content:** if the response is unplayable but has `playabilityStatus.errorScreen.playerLegacyDesktopYpcTrailerRenderer.trailerVideoId`, the trailer is played instead.

---

## 2. Catalog

### 2.1 Endpoints and clients

Sources: `yt/browse/v2/BrowseApi.kt`, `BrowseApiHelper.kt`, `BrowseService2.kt` [current].

All catalogue calls are POSTs to `https://www.youtube.com/youtubei/v1/<endpoint>`. The endpoints are `browse`, `next`, `search` and `guide`; the reel endpoints `reel/reel_item_watch` and `reel/reel_watch_sequence`; and the playlist endpoints `playlist/get_add_to_playlist`, `browse/edit_playlist`, `playlist/create`, `playlist/delete`, `like/like` and `like/removelike`. There is also a mobile browse at `https://m.youtube.com/youtubei/v1/browse`.

**TV client, TVHTML5 `7.20260901.15.00` with the TV browse extras:**
- Home: `browseId:"default"`. WEB Home was abandoned because it is empty when anonymous.
- Subscriptions: `FEsubscriptions`.
- History: `FEhistory`.
- My videos: `FEmy_videos`.
- My playlists: `FEplaylist_aggregation`.
- Topics: `FEtopics_sports`, `FEtopics_live`, `FEtopics_movies`, `FEtopics_music`, `FEtopics_news`, `FEtopics_gaming`.
- Channel pages, the uploads grid, playlists (`VL<id>`), search, and every TV continuation.
- Liked music: a continuation request with the fixed token `4qmFsgIWEhRGRW11c2ljX2xpa2VkX3ZpZGVvcw%3D%3D`.

**WEB client:**
- Trending: `FEtrending` with `params:"6gQJRkVleHBsb3Jl"`.
- Channel Live tab, channel search, channel sort chips.
- Channel fallback when TV fails.
- The Shorts feed.
- Subscribed channels by name, via the `guide` endpoint.

**WEB_REMIX client:** `FEmusic_new_releases_albums` and `FEmusic_new_releases_videos`.

**Defined but unused:** `FEwhat_to_watch`, `FEmy_youtube`, `FElibrary`, `VLLM`, `FEhype_leaderboard`, and Watch Later as `FEmy_youtube` with `params:"cAc%3D"`. Fixed playlist ids: Watch Later `VLWL` / `WL`, Liked `VLLL` / `LL`, Favourites prefix `VLFL`.

### 2.2 Channel id resolution

Source: `yt/utils/UtilsService.kt`, `UtilsApi.kt` [current, last touched 2025-08-03].
- Only `@handle` inputs are handled. The source strips `@`, GETs `https://www.youtube.com/<name>`, and reads the `UC…` id from the page's canonical link (`<link rel="canonical" href="https://www.youtube.com/channel/UC…">`).
- `navigation/resolve_url` is not used.
- It is unverified whether the URL without `@` still resolves every handle.

### 2.3 Channel browse

The request is `browseId:"UC…"` plus optional `params`. Source: `yt/browse/v2/BrowseApiHelper.kt`. Tab params:

| Tab | `params` |
|---|---|
| Home / featured | `EghmZWF0dXJlZPIGBAoCMgA%3D` |
| Videos | `EgZ2aWRlb3PyBgQKAjoA` |
| Shorts | `EgZzaG9ydHPyBgUKA5oBAA%3D%3D` |
| Live (streams) | `EgdzdHJlYW1z8gYECgJ6AA%3D%3D` |
| Playlists | `EglwbGF5bGlzdHPyBgQKAkIA` |
| Releases | `EghyZWxlYXNlc_IGBQoDsgEA` |
| Community | `Egljb21tdW5pdHnyBgQKAkoA` |
| Channel search | `EgZzZWFyY2jyBgQKAloA`, plus a top-level `query` |

**TV channel page:**
- A TV browse of a bare `UC…` returns shelves at `contents.tvBrowseRenderer.content.tvSurfaceContentRenderer.content.sectionListRenderer.contents[].shelfRenderer`.
- Each shelf's `endpoint` has `params` whose prefix says what the shelf is:
  - `EgZ2aWRlb3MYAyACOAJwA…` = live now
  - `EgZ2aWRlb3MYAyAAcA…` = videos
  - `EgZ2aWRlb3MYAyACOARwA…` = past streams
- The Shorts shelf has no endpoint and is recognised by its tile style. Promo shelves are skipped. The ordering fix is `8b5d23a5`, 2026-04-05.

**Uploads grid:** TV with the Videos params. The fallback is WEB Videos plus Live.

**WEB channel layout:**
- Tabs at `contents.twoColumnBrowseResultsRenderer.tabs[].tabRenderer` (or `expandableTabRenderer`).
- Content under `content.richGridRenderer.contents` or `content.sectionListRenderer`.
- Sort chips (Latest / Popular / Oldest): a continuation token in `chipCloudChipRenderer…continuationCommand.token`, or in the view-model variant at `tapCommand.innertubeCommand.continuationCommand.token`.

**Redirect:** if a WEB response has `onResponseReceivedActions[].navigateAction.endpoint.browseEndpoint.browseId`, browse that id instead.

### 2.4 Subscriptions and home (TV)

Source: `yt/browse/v2/gen/BrowseHelper.kt` [current].

**`FEsubscriptions` left nav:**
- Path: `contents.tvBrowseRenderer.content.tvSecondaryNavRenderer.sections[0].tvSecondaryNavSectionRenderer.tabs[]`. Each tab is a `tabRenderer` or `expandableTabRenderer`.
- The tab whose `endpoint.browseEndpoint.browseId` is `FEsubscriptions` is "All".
- Every other tab is one subscribed channel. It has a title and thumbnail, and `presentationStyle.style:"NEW_CONTENT"` when the channel has new uploads.
- A channel tab carries a reload continuation at `content.tvSurfaceContentRenderer.continuation…` rather than a channel id. The source notes that this makes an offline backup of the list impossible.

**Items:** `content.tvSurfaceContentRenderer.content.gridRenderer.items`, or `…sectionListRenderer` shelves for Home rows.

**Pagination:** SmartTube keeps following continuations until it has more than 60 items, at most 10 extra requests, so it can float LIVE items to the top.

### 2.5 Continuations

Sources: `yt/browse/v2/gen/*`, `yt/next/v2/gen/WatchNextHelper.kt` [current].

**Request:** the same client context plus `"continuation":"<token>"`, POSTed to `/browse`. Search continuations go to `/search` and watch-next continuations to `/next`.

**Token location in a TV first response**, in the order the source tries them:
1. the subscriptions tab grid or list;
2. `content.twoColumnRenderer.rightColumn.playlistVideoListRenderer.continuations`;
3. `content.sectionListRenderer.continuations`;
4. the first shelf's continuation;
5. `content.gridRenderer.continuations`, or the last grid item's continuation.

**Token field** inside a `continuations[]` entry: `nextContinuationData.continuation`, `nextRadioContinuationData.continuation` or `reloadContinuationData.continuation`.

**TV continuation response:**
- `continuationContents.horizontalListContinuation`, `.gridContinuation`, `.playlistVideoListContinuation` or `.tvSurfaceContentContinuation.content.gridRenderer` hold `items` and `continuations`.
- `continuationContents.sectionListContinuation` holds shelves.

**WEB continuation response:**
- Items at `onResponseReceivedActions[].appendContinuationItemsAction.continuationItems`, or `reloadContinuationItemsCommand`.
- The next token is in the last item, at `continuationItemRenderer.continuationEndpoint.continuationCommand.token`.

### 2.6 Item fields (TV `tileRenderer`)

Source: `yt/common/models/gen/CommonHelper.kt` [current].

**Ids, title, thumbnail:**
- Title: `metadata.tileMetadataRenderer.title`.
- Subtitle, views and age: text lines at `metadata.tileMetadataRenderer.lines[].lineRenderer.items[].lineItemRenderer`. There is no structured publish date.
- Video id: `onSelectCommand.watchEndpoint.videoId`, or `onSelectCommand.reelWatchEndpoint.videoId`.
- Playlist id: `onSelectCommand.watchEndpoint.playlistId`, or `watchPlaylistEndpoint.playlistId`.
- Channel: `onSelectCommand.browseEndpoint.browseId` plus `params`.
- Thumbnail: `header.tileHeaderRenderer.thumbnail.thumbnails[]`. Music tiles use `header.trackTileHeaderRenderer`. Animated previews are in `movingThumbnail` or `onFocusThumbnail`.

**Duration and badges:**
- Duration or badge text: `header.tileHeaderRenderer.thumbnailOverlays[].thumbnailOverlayTimeStatusRenderer.text`.
- That overlay's `style` gives the kind: `LIVE`, `UPCOMING` or `SHORTS`.
- A tile `style` of `TILE_STYLE_YTLR_SHORTS` also marks a Short.
- `icon.iconType` `MIX` marks a mix.
- A movie has the badge `BADGE_STYLE_TYPE_YPC` and a browseId instead of a videoId.
- Watched percentage: `thumbnailOverlayResumePlaybackRenderer.percentDurationWatched`.
- Context menu and feedback tokens: `onLongPressCommand.showMenuCommand.menu`.

**WEB renderers also handled:**
- `gridVideoRenderer`, `videoRenderer`, `compactVideoRenderer`, `reelItemRenderer`, `playlistVideoRenderer`, and the channel and playlist renderers.
- `lockupViewModel`: title at `metadata.lockupMetadataViewModel.title`, video id via `rendererContext`.
- `shortsLockupViewModel`.
- Live on WEB: the badge `BADGE_STYLE_TYPE_LIVE_NOW`, or the overlay style `LIVE`.

### 2.7 Playlists, Watch Later, history

Source: `yt/browse/v2/*`, `yt/playlist/PlaylistApiHelper.java` [current].

- **Library list.** A TV browse of `FEplaylist_aggregation`. Watch Later is taken as the first item; Liked is the item whose browse id starts with `VLLL`.
  - **Historical** (before `aa132c5b`, 2026-06-01): when Watch Later was missing, the source took the second subsection of `FElibrary`.
- **Opening a playlist.** A TV browse of `VL<playlistId>`. Items are at `…tvSurfaceContentRenderer.content.twoColumnRenderer.rightColumn.playlistVideoListRenderer.contents`.
- **History.** A TV browse of `FEhistory`, as a grid.
- **Edits** (TV body):
  - `playlist/get_add_to_playlist` with `{videoIds:[id]}`.
  - `browse/edit_playlist` with `{playlistId, actions:[…]}`. Action types: `ACTION_ADD_VIDEO` (field `addedVideoId`), `ACTION_REMOVE_VIDEO_BY_VIDEO_ID` (`removedVideoId`), `ACTION_SET_PLAYLIST_NAME` (`playlistName`), `ACTION_SET_PLAYLIST_VIDEO_ORDER`.
  - `like/like` or `like/removelike` with `{target:{playlistId}}`.
  - `playlist/create` with `{title[, videoIds]}`.
  - `playlist/delete` with `{playlistId}`.

### 2.8 Watch next

Source: `yt/next/v2/*` [current].
- **Request:** `/next` with `videoId`, `playlistId`, `playlistIndex` and an optional `params`.
  - A negative `playlistIndex` returns an error.
  - Loading by index needs an empty `videoId`.
- **TV response:** `contents.singleColumnWatchNextResults`.
  - Metadata: `results.results.contents[0]`.
  - Playlist panel: `playlist.playlist`.
  - Autoplay: `autoplay.autoplay.sets[0]`.
  - Chapters: `playerOverlays.playerOverlayRenderer.decoratedPlayerBarRenderer`.

### 2.9 RSS

Source: `yt/rss/RssService.kt`, `YouTubeRssParser.java` [current].
- **URL:** `https://www.youtube.com/feeds/videos.xml?channel_id=UC…`.
- **Fields per `entry`:** `yt:videoId`, `yt:channelId`, `title`, `published`, `updated`, `author/name`, `media:group/media:thumbnail@url`, `media:description`, and `media:community/media:statistics@views`.
- **Limits:** no duration, no live flag, no Shorts flag.
- **How SmartTube compensates:** it fetches up to 100 channels in parallel, merges them, sorts by `published`, and cross-checks against the TV uploads grid. It copies live, upcoming and watched state from the grid and drops entries that are not in the grid, which mostly removes Shorts.

### 2.10 Shorts feed (WEB)

Source: `yt/browse/v2/*` [current; parsing changed in `4be5193d`, 2026-11-09].
- **First page:** `reel/reel_item_watch` with `{disablePlayerResponse:true, inputType:"REEL_WATCH_INPUT_TYPE_SEEDLESS", params:"CA8%3D"}`.
- **Next pages:** `reel/reel_watch_sequence` with `{sequenceParams:<token>}`.
- **Entries:** `entries[].command.reelWatchEndpoint.{videoId, thumbnail, overlay.reelPlayerOverlayRenderer.style}`. The style `REEL_PLAYER_OVERLAY_STYLE_SHORTS` marks a Short.

### 2.11 Search (the contract has none; noted for completeness)

Sources: `yt/search/v2/*`, `yt/search/SearchFilterHelper.java` [current].
- **Request:** `/search` with the TV client and `{query[, params]}`.
- **Results:** `contents.sectionListRenderer.contents[0].shelfRenderer`.
- **Filter `params`:**
  - Upload date: last hour `EgQIARAB`, today `EgQIAhAB`, week `EgQIAxAB`, month `EgQIBBAB`, year `EgQIBRAB`.
  - Duration: under 4 min `EgQQARgB`, 4 to 20 min `EgQQARgD`, over 20 min `EgQQARgC`.
  - Type: video `EgIQAQ%3D%3D`, channel `EgIQAg%3D%3D`, playlist `EgIQAw%3D%3D`, movie `EgIQBA%3D%3D`.
  - Features: live `EgJAAQ%3D%3D`, 4K `EgJwAQ%3D%3D`, HDR `EgPIAQE%3D`.
  - Sort: by upload date `CAI%3D`, by views `CAMSAhAB`, by rating `CAESAhAB`.
- **Suggestions:** GET `https://clients1.google.com/complete/search?client=youtube-lr&ds=yt&xhr=t&oe=utf-8&xssi=t&q=…&hl=…&gl=…`.
- **Failure handling:** an authenticated search that fails is retried anonymously (`6c02bd1b`, 2026-09-19).

---

## 3. The player request

Sources: `yt/common/helpers/QueryBuilder.kt`, `yt/videoinfo/V2/VideoInfoApi.java`, `VideoInfoService.java` [current; reload token added 2026-10-07 `1b43ec76`, TCL/TV fixes 2026-10-01 to 2026-10-04].

**Endpoint:** POST `https://www.youtube.com/youtubei/v1/player`.

**Headers:**
- `Content-Type: application/json`;
- `X-Goog-Visitor-Id`;
- `User-Agent` of the client;
- `X-Youtube-Client-Name` (numeric) and `X-Youtube-Client-Version`;
- when signed in, `Authorization` and possibly `X-Goog-Pageid` (see 7).

**Body:**
- `context.client`: the common fields from 1.3, plus:
  - for TV clients, `tvAppInfo.livingRoomPoTokenId` when known;
  - `visitorData`. For PO-token clients it is the visitorData the token was minted against; otherwise the one from `/tv`.
- `context.clickTracking.clickTrackingParams`, when the video came from a feed.
- `context.thirdParty.embedUrl`, embedded clients only.
- `context.user` as in 1.3.
- `playbackContext.contentPlaybackContext`:
  - `html5Preference:"HTML5_PREF_WANTS"`;
  - `lactMilliseconds:60000`;
  - `isInlinePlaybackNoAd:true`, which skips pre-roll waits;
  - `signatureTimestamp:<STS>` (see 5);
  - `encryptedHostFlags` for embedded clients.
- `playbackContext.devicePlaybackCapabilities`: `supportsVp9Encoding:true`, and `supportXhr` **false for TV clients, true otherwise**. The source cites a report that `supportXhr:false` brings URLs back for TV.
- `playbackContext.reloadPlaybackContext.reloadPlaybackParams.token`: only after a SABR ReloadPlayerResponse for this video (see 4.4).
- `serviceIntegrityDimensions.poToken`, when the client needs a token.
- `racyCheckOk:true`, `contentCheckOk:true`, `videoId`, and `cpn` (a 16-character client playback nonce, regenerated for each `/player` call).
- `params`: only the geo-fix blob `CgIQBg%3D%3D`, on the geo client.
- **Reel endpoint** (disabled): the same video fields nested under `playerRequest` and POSTed to `https://youtubei.googleapis.com/youtubei/v1/reel/reel_item_watch`.

**Response paths read** (`yt/videoinfo/models/VideoInfo.java`, `VideoDetails.java`):
- **Playability:** `playabilityStatus.status`, `.reason`, `.playableInEmbed`, `.errorScreen.playerErrorMessageRenderer.subreason`, `.errorScreen.playerLegacyDesktopYpcTrailerRenderer.trailerVideoId`, and `.paygatedQualitiesMetadata.restrictedAdaptiveFormats[]` (Premium-gated formats).
- **Streaming data:** `streamingData.formats[]`, `.adaptiveFormats[]`, `.hlsManifestUrl`, `.dashManifestUrl`, `.serverAbrStreamingUrl`.
- **SABR config:** `playerConfig.mediaCommonConfig.mediaUstreamerRequestConfig.videoPlaybackUstreamerConfig`.
- **Other player data:** `playerConfig.audioConfig.loudnessDb`, `captions.playerCaptionsTracklistRenderer.*`, `microformat.playerMicroformatRenderer.uploadDate` and `.liveBroadcastDetails.startTimestamp`, `playbackTracking.videostatsPlaybackUrl.baseUrl` and `.videostatsWatchtimeUrl.baseUrl`, `paidContentOverlay`.
- **`videoDetails`:** `videoId`, `title`, `author`, `channelId`, `lengthSeconds`, `viewCount`, `shortDescription`, `isLive`, `isLiveContent`, `isLowLatencyLiveStream`, `isOwnerViewing`, `thumbnail.thumbnails[]`.

**`playabilityStatus.status` classes:**

| Status | Source's reading |
|---|---|
| `OK` | playable |
| `UNPLAYABLE` | unknown reason, or a temporary ban |
| `ERROR` | removed or hidden |
| `LOGIN_REQUIRED`, `AGE_CHECK_REQUIRED`, `CONTENT_CHECK_REQUIRED` | age-restricted |
| `AGE_VERIFICATION_REQUIRED` | defined but not classified |
| `LIVE_STREAM_OFFLINE` | defined; an empty result shows a message and is re-polled every 30 s (ST `common/…/VideoLoaderController.java`) |

The source also treats a response as unplayable when it is live and its first adaptive format has neither `url` nor cipher (SABR-only live), unless the user enabled experimental SABR live.

**Additional fix-up requests:**
- **HLS for extended formats.** When the user enables "extended HLS" formats, the video is not live, there is no `hlsManifestUrl` and the first adaptive format is 1080p, SmartTube makes an extra anonymous `/player` call with **visionOS** to get `hlsManifestUrl`. It also takes a missing storyboard spec from that response.
- **Translation languages.** When the client returned fewer than 100 translation languages, an extra anonymous WEB call fetches the full list, which is cached.

**`get_video_info`** (`https://www.youtube.com/get_video_info?html5=1&c=TVHTML5&ps=leanback…`) is **historical**.

**WEB path** (`yt/innertube/*`, a port of LuanRT/YouTube.js, current):
1. GET `https://www.youtube.com/sw.js_data` for session and device info.
2. POST `https://www.youtube.com/youtubei/v1/config` for `configData`, `coldConfigData`, `coldHashData` and `hotHashData`, which go into `context.client.configInfo`.
3. Call `/player` with that context.

The session is cached per player id and invalidated when a result is unplayable.

---

## 4. Formats

### 4.1 Format fields used

Source: `yt/videoinfo/models/formats/*` [current].
- **Identity and URL:** `itag`, `url`, `cipher` (historical name), `signatureCipher`, `mimeType`, `type` (`FORMAT_STREAM_TYPE_OTF` marks OTF streams), `lastModified`.
- **Size and bitrate:** `contentLength`, `bitrate`, `averageBitrate`, `approxDurationMs`.
- **Video:** `width`, `height`, `fps`, `quality`, `qualityLabel`, `projectionType`.
- **Audio:** `audioSampleRate`, `audioQuality`, `audioTrack.id`, `audioTrack.audioIsDefault`, `audioTrack.isAutoDubbed`, `isDrc`.
- **Byte ranges:** `initRange.start/end`, `indexRange.start/end`.
- **Live:** `targetDurationSec`, `maxDvrDurationSec`.
- **`xtags`.**

**Audio language from `xtags`:** the URL's `xtags` value is parsed as `key=value` pairs separated by `:`. `lang` is the language, and `acont` is one of `original`, `descriptive`, `dubbed`, `dubbed-auto` or `secondary`; it is shown as "lang (acont)". For SABR formats the label comes from `audioTrack.id` plus the auto-dubbed and default flags instead. Source: `yt/videoinfo/models/VideoUrlHolder.kt`.

**DRC audio:** DRC formats get a distinct MPD Representation id, `<itag>-drc`, because they share the itag with the normal track (`yt/formatbuilders/mpdbuilder/YouTubeMPDBuilder.java`).

### 4.2 Format URL parameters

Source: `yt/videoinfo/models/VideoUrlHolder.kt`, `yt/videoinfo/VideoInfoServiceBase.java` [current].
- **Ciphered formats.** `signatureCipher` (or `cipher`) is a query string: `url` is the base URL and `s` is the scrambled signature. **The deciphered value is always written as `sig`**; the cipher's `sp` field is ignored.
- **`n`:** replaced by the solved value. Every format's `n` is solved in one batch, because all formats share one value.
- **`pot`:** set to the PO token on every format URL, when the client has a token.
- **`cpn` and `cver`:** the source can set them on URLs.
- **Other parameters recognised:** `lsig` (marks a pre-signed URL), `ei`, `itag`, `clen`, `dur`, `lmt`, `xtags`.
- **OTF streams:** the init segment is the format URL plus `&sq=0`; media segments are the URL plus `&sq=<n>`.
- **Live segments:** the URL plus `&sq=<n>`.
- No chunk-size or `range` limit is set in MSC: ExoPlayer requests byte ranges from the MPD SegmentBase index.

### 4.3 DASH building

Source: `yt/formatbuilders/mpdbuilder/YouTubeMPDBuilder.java`, `YouTubeOtfSegmentParser.java` [current]. SmartTube writes its own MPD from `adaptiveFormats` instead of using `dashManifestUrl` for VOD.

**MPD root:**
- `xmlns="urn:mpeg:DASH:schema:MPD:2011"` and `xmlns:yt="http://youtube.com/yt/2012/10/10"`;
- `minBufferTime="PT1.500S"`;
- `profiles="urn:mpeg:dash:profile:isoff-on-demand:2011"`.
- VOD: `type="static"`, `mediaPresentationDuration="PT<lengthSeconds>S"`, and a Period with the same duration.
- Live: `type="dynamic"`, `minimumUpdatePeriod="P100Y"` (never refreshed), and Period `start="PT0S"`.

**Adaptation sets:**
- One per mime type and audio language; subtitles are added as separate sets.
- High-quality representations are written first.
- Role descriptors use `urn:mpeg:DASH:role:2011`.

**Representation attributes:**
- `id` (the itag), `codecs` (from the `mimeType` codecs parameter), `startWithSAP="1"`, `bandwidth` (`bitrate`);
- video: `width`, `height`, `frameRate`, `maxPlayoutRate="1"`;
- audio: `audioSamplingRate`, derived from the itag.

**VOD (non-OTF) segments:**
- `BaseURL` holds the format URL, with a `yt:contentLength` attribute when `clen` is known.
- `SegmentBase indexRange="<indexRange.start>-<indexRange.end>" indexRangeExact="true"`, with `Initialization range="<initRange>"`.

**OTF segments:**
1. Fetch the init URL (`&sq=0`). Its response contains a text header line `Segment-Durations-Ms: …`, a comma list of durations in ms with optional repeat counts.
2. Write a `SegmentTemplate` with `media="<url>&sq=$Number$"`, `initialization="<url>&sq=0"`, `startNumber="1"`, and a SegmentTimeline built from those durations.

**Live:**
- `SegmentTemplate` with `timescale=1000000`, `media="<url>&sq=$Number$"`, `startNumber=<current head segment>`, `presentationTimeOffset = segmentDuration × startNumber`.
- One `S` element with `t` equal to that offset, `d` equal to the segment duration, and `r` set to cover 48 h. The source uses a 48-hour window for live and premieres.
- **Head segment, segment duration and start time**, measured by requesting the smallest audio format URL. The source tries three methods in order:
  1. response headers `X-Sequence-Num` (or `X-Head-Seqnum`), `X-Head-Time-Millis` and `X-Walltime-Ms`;
  2. the body as a DASH manifest, reading `yt:earliestMediaSequence`, `startNumber` and `minimumUpdatePeriod`;
  3. the media body, reading text lines `Sequence-Number:`, `Ingestion-Walltime-Us:` and `Stream-Duration-Us:`.

  If the result looks invalid, it retries once. The segment duration falls back to `targetDurationSec`.

### 4.4 Player selection, HLS and SABR

Source: `ST:common/src/main/java/com/liskovsoft/smartyoutubetv2/common/app/models/playback/controllers/VideoLoaderController.java` [current]. In order:
1. **Unplayable:** show the reason and skip to the next video after 5 s.
2. **DASH formats present** (and SABR live not preferred): play the self-built MPD. If "high bitrate formats" is on and an HLS manifest is available for a non-live 1080p video, **merge** the DASH formats with the HLS manifest from the visionOS/iOS call.
3. **SABR formats present** (adaptive formats without URLs, or `serverAbrStreamingUrl` plus the ustreamer config): play through the SABR module.
4. **Live with `dashManifestUrl`:** play YouTube's own DASH manifest, unless the user forces HLS.
5. **Live with `hlsManifestUrl`:** play YouTube's HLS.
6. **Progressive URLs only:** a plain URL list, highest quality first; "always LQ" (`yt/formatbuilders/hlsbuilder/YouTubeUrlListBuilder.java`).
7. **Nothing:** treated as an upcoming stream; re-polled every 30 s.

**SABR module** (`ST:exoplayer-amzn-2.10.6/library/sabr/`, current):
- POSTs `Content-Type: application/x-protobuf` to `serverAbrStreamingUrl`, with an `rn=<request number>` query parameter that goes up by one per request.
- The request is a VideoPlaybackAbrRequest protobuf, and responses are UMP.
- Protobuf schemas bundled: buffered_range, client_abr_state, crypto_params, format_initialization_metadata, innertube_request, live_metadata, media_capabilities, media_header, next_request_policy, onesie_*, playback_cookie, reload_player_response, sabr_context_sending_policy, sabr_context_update, sabr_error, sabr_redirect, sabr_seek, streamer_context, stream_protection_status, time_range, video_playback_abr_request.
- Subtitles are not fetched over SABR; they are plain GETs.
- **Reload:** when the server sends a ReloadPlayerResponse, the app stores its token for that video and re-requests `/player` with `reloadPlaybackContext.reloadPlaybackParams.token`. The source says no media flows until this is done.
- **Live:** SABR live is experimental and off by default. It was disabled for live on 2026-10-24 (`ba369250`, "some streams still broken") and made a global preference on 2026-11-06 (`4010903f`).
- **Which clients get SABR:**
  - Observed 2026-11-23: TVHTML5 7.x anonymous without a PO token returns SABR-only adaptive formats.
  - Per the source, Android VR above 1.65 does the same.

---

## 5. Player JavaScript and challenges

### 5.1 Finding the player

Sources: `yt/app/AppApi.java`, `yt/app/models/AppInfo.java`, `TvConfig.java`, `yt/app/AppServiceCoreCached.java`, `yt/common/helpers/AppConstants.kt` [current].

1. **The `/tv` page.** GET `https://www.youtube.com/tv` with the TV user agent and any stored cookies; response cookies are stored and replayed. From the HTML, read:
   - `"player_url":"…"`, a relative path `/s/player/<hash>/tv-player-es6.vflset/tv-player-es6.js` (or `tv-player-ias`), JSON-unescaped and made absolute;
   - the URL of the "base" script, for OAuth (see 7);
   - `"visitorData":"…"`;
   - `"LIVING_ROOM_PO_TOKEN_ID":"…"`, with `=` unescaped to `=`.

   The page is re-scraped every 10 hours.
2. **`tv_config`.** GET `https://www.youtube.com/tv_config?action_get_config=true&client=lb4&theme=cl`, the call the TV app makes at startup. Inside the object under `WEB_PLAYER_CONTEXT_CONFIG_ID_LIVING_ROOM_WATCH`, `jsUrl` points to a `-tcl` TV player. This commit uses only its BotGuard fields (see 6).
3. **Fallbacks.** If the `/tv` player fails validation, the source tries the last player URL that worked, then the first entry of a hard-coded list of about 50 known player URLs (`/s/player/7460dd14/tv-player-es6.vflset/tv-player-es6.js` first).
   - When falling back, the STS from the newer, failing player is kept, because tracking needs the current STS.
4. **Retries.** Three download tries, 1 s apart (`bad6cb57`, 2026-11-22). A failed download now throws instead of caching a broken extractor (`1814c5ad`, 2026-11-22).

**Player variants seen:**
- TV: `tv-player-es6`, `tv-player-ias`, `tv-player-ias-tcl`, `tv-player-es6-tcl`, `tv-player-es6-tce` (exposes global helpers), `tv-player-es6-tcc` (returned 503).
- Web: `player_ias.vflset/en_US/base.js`, `player_es6…/base.js`, `player_ias_tce`.

**Observed 2026-11-23:** `/tv` returned hash `6b3d0364` to one request and `7f755914` to a later one the same day, so the hash rotates or is A/B-assigned per request. `tv_config` `jsUrl` gave `/s/player/6b3d0364/tv-player-es6-tcl.vflset/tv-player-es6-tcl.js`.

**Which player variant per client:**
- Every TV client except the downgraded one uses the **TCL** variant: the path ending is rewritten to `/tv-player-ias-tcl.vflset/tv-player-ias-tcl.js` on the same hash, and es6 is replaced with ias.
- All other clients, and the downgraded TV client, use the URL as given.
- The rewrite of TV paths to `player_ias.vflset/en_US/base.js` for web clients is dead code (historical).

### 5.2 Signature timestamp

Source: `yt/app/playerdata/CommonExtractor.kt`, `yt/common/helpers/QueryBuilder.kt` [current].
- **Extraction:** the first `signatureTimestamp:` followed by digits in the downloaded player.
- **Format:** the TCL TV player carries an **8-digit** value, the 5-digit web value with `001` appended. Observed 2026-11-23: `20774001` in `tv-player-ias-tcl.js` and `20774` in `tv-player-es6.js` for hash `6b3d0364`.
- **Consequence:** the source says a wrong format triggers a "page should be reloaded" error on TV. So the STS must come from the variant that matches the client.
- **Historical:** appending `001` by hand for TV clients.

### 5.3 Signature and n

Sources: `yt/app/nsigsolver/**`, `assets/nsigsolver/`, `yt/videoinfo/VideoInfoServiceBase.java` [current].

**Engine:** **V8 through J2V8**, vendored from SharedModules (`SM:j2v8`, version 6.2.1, prebuilt for arm64-v8a, armeabi-v7a, x86 and x86_64). The F-Droid flavour uses the Maven `com.eclipsesource.j2v8:j2v8` instead.
- Rhino, Duktape and QuickJS appear only as commented-out dependencies (historical).
- Each solve creates a fresh runtime, runs the call, and releases it, all behind one lock.
- No DOM is needed for sig or n.

**Regular players: yt-dlp's "ejs" solver** (Unlicense, generated by github.com/yt-dlp/ejs), run in V8.
- Bundled scripts: the ejs core script, meriyah 6.1.4 (JS parser), astring 1.9.0 (code generator) and a small polyfill.
- Script sources, in order: a persisted cache, the bundled assets, then a download from `https://github.com/yt-dlp/ejs/releases/download/<ver>/yt.solver.lib.min.js` and `…/yt.solver.core.min.js`. The pinned version label is `0.0.1`.
- **Input:** one JSON object, `{type:"player", player:<full JS>, requests:[{type:"n"|"sig", challenges:[…]}], output_preprocessed:true}`.
- **Output:** `responses[]` (each a `type` and a `data` map from input to output), an optional `error`, and `preprocessed_player`.
- Later calls for the same player send `type:"preprocessed"` with the cached preprocessed text instead of the 2 to 3 MB player.

**TCL players: SmartTube's own AST solver**, about 31 KB, in V8. It was added 2026-09-29 to 2026-10-05 by contributor pixkk.
- **Argument shape:** in the TCL player the n transform takes a URL-parameters object and rewrites its `n` in place; it does not take a bare string. So each challenge is wrapped as `https://www.youtube.com/videoplayback?n=<value>`, and the transformed URL is read back.
- **How the code is located,** in prose:
  1. **Candidate call sites.** Search the player's AST for a guarded call: an `&&` chain ending in an obfuscated property read, whose index is itself a lookup into the player's global string array. The guarded call's result is assigned and takes three arguments: two integer literals, then the URL-params object.
  2. **Confirming the transform.** Unrelated helpers share that shape. The real transform is confirmed as a function that directly throws a new error built from a global-array element.
  3. **The integer arguments** are read from a plain call to that function elsewhere in the player.
  4. **The URL-params constructor** is found as a two-argument `new` whose result is reused by the very next statement.
  5. **Assembly.** All dependencies are pulled in: top-level declarations, prototype methods grouped by constructor, and a namespace object. They become a standalone script, cached as the preprocessed player.
- **Signatures:** the TCL path never deciphers them, on the assumption that TV streams arrive pre-signed. Observed 2026-11-23: true for both TV clients tested.

**Validation:** each new player is self-tested.
- A fixed sample n, and for the regular path a sample signature, must come out changed.
- Regular players need n, sig and STS to pass; TCL players need n and STS.

**Historical:** regex location of the n and signature functions, inlining of the global string array (a `"use strict";` followed by a `var` holding a split string or an array literal), evaluated with a small Kotlin expression interpreter. Dead since late 2025; last "temp bad sig workaround" 2025-09-26.

### 5.4 CPN

Source: `yt/app/playerdata/ClientPlaybackNonceExtractor.kt` [current].
- The source extracts the player's own nonce generator: random characters drawn from a 64-character alphabet, 16 characters long. It runs it in V8 with a stub `window.crypto.getRandomValues`.
- Fallback: a native random 16-character string.
- The CPN is reset before every `/player` call.

### 5.5 Caching per player version

Source: `yt/app/playerdata/PlayerExtractorCache.kt`, `yt/app/nsigsolver/common/CacheService.kt` [current].
- **Persisted per variant** (regular and TCL): player URL, CPN code and STS. When the current URL matches, they are restored without a self-test.
- **Solver cache:** one section per variant, keyed by the full player URL. Each store clears its section first, so only one preprocessed player is kept per variant.
- **Invalidation:**
  - A V8 "Invalid or unexpected token" error, or a TCL solver error, wipes that section.
  - Clearing visitor cookies is documented as the way to get the newest player, because visitorData is tied to player versions.

---

## 6. PO tokens and BotGuard

Sources: `yt/app/PoTokenGate.kt`, `yt/app/potokennp2/**`, `yt/app/models/TvConfig.java`, `assets/potokennp2/po_token2.html` [current unless marked]. The cloud fallback was removed 2026-10-11 (`0b07f952`); the stated reason is that the token must be generated on the device that uses it. The TV token was added 2026-10-04/05 (`2ece0912`, `b85666ae`).

### 6.1 Where tokens go

- **`/player` body:** `serviceIntegrityDimensions.poToken`. For TV clients, also `context.client.tvAppInfo.livingRoomPoTokenId`.
- **`context.client.visitorData`:** the visitorData the token was minted against.
- **Every stream URL:** `pot=<token>`. One token serves both `/player` and googlevideo.
- **Caption URLs** (newer path only): `&pot=<token>&c=<client label>`.

### 6.2 Runtime

- **Engine:** an **Android System WebView**, a full browser engine. It loads a local HTML page with base URL `https://www.youtube.com` and a desktop Mac WebKit user agent.
  - All network traffic is done natively and handed into the page; WebView network loads are blocked.
  - Promise and Symbol polyfills are included for old WebViews.
- **What runs in JS:**
  1. Evaluate the BotGuard interpreter script.
  2. Find the VM under the challenge's `globalName`.
  3. Start it with the `program` and a callback that receives the async-snapshot, shutdown, pass-event and check-camera functions.
  4. Poll every 1 ms, for up to 10 s, until the snapshot function exists.
  5. Snapshot with an empty output array that BotGuard fills (the web PO signal output). The result is the `botguardResponse` string.
- **DOM needs:** no element is passed in. When a page `ytcfg` is available, a global `yt` object with `config_` set to that ytcfg is created first, because BotGuard reads `EVENT_ID` from it. The source notes the token is then 120 characters instead of 124.
- **Timeouts:** 20 s for init, 10 s per mint.
- **Historical:** a J2V8 (bare V8) generator with polyfills. The source itself says a browser-like environment cannot be built on bare V8; it is unused.

### 6.3 Web token, end to end

1. **visitorData.**
   - POST `https://www.youtube.com/youtubei/v1/visitor_id` with a plain WEB context.
   - Headers: `Content-Type: application/json`, `Accept-Language: en-US, en;q=0.9`, `Cookie: SOCS=CAE=`, `Origin` and `Referer` `https://www.youtube.com`, a Firefox 128 user agent, `X-Youtube-Client-Name: 1`, `X-Youtube-Client-Version: 2.20250213.05.00`.
   - Read `responseContext.visitorData`.
2. **Challenge, preferred path (since 2026-08).**
   - GET `https://www.youtube.com/` with the Mac UA and `accept-language: en-US,en;q=0.7`.
   - Take the first `ytcfg.set({…})` object, which has `EVENT_ID`.
   - Take the object passed to `window.ytAtN(…)`. It is loose JS, so `\xHH` escapes, single quotes, unquoted keys and trailing commas must be normalised.
   - Its `R` field is a JSON string containing `bgChallenge` with `interpreterHash`, `program`, `globalName`, `clientExperimentsStateBlob` and `interpreterUrl.privateDoNotAccessOrElseTrustedResourceUrlWrappedValue`. That URL is protocol-relative, so prefix `https:`.
   - Download the interpreter script.
   - The source's rationale: challenges handed over without their page's ytcfg are rejected.
3. **Challenge, fallback path.** POST `https://www.youtube.com/youtubei/v1/att/get?prettyPrint=false` with WEB `clientName`/`clientVersion` and `engagementType:"ENGAGEMENT_TYPE_UNBOUND"`. The response carries the same `bgChallenge`.
4. **Run BotGuard** to get `botguardResponse` and the filled output array.
5. **Integrity token.**
   - POST `https://www.youtube.com/api/jnn/v1/GenerateIT` with body `["O43z0dpjhgX20SCx4KAo", <botguardResponse>]`. The first element is the web request key.
   - Headers: `Content-Type: application/json+protobuf`, `x-goog-api-key: AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw`, `x-user-agent: grpc-web-javascript/0.1`, `Accept: application/json`.
   - Response: `[integrityToken (base64url, '.' used as padding), expiresInSeconds, …]`.
   - Alternative endpoint: `https://jnn-pa.googleapis.com/$rpc/google.internal.waa.v1.Waa/GenerateIT`. The source says it gives fewer 403s but can hang.
6. **Minter.** Call the first function in the output array with the integrity token as bytes; it returns the minter.
7. **Mint.** Pass the content-binding identifier, UTF-8 encoded. The resulting bytes are encoded as URL-safe base64.
8. **Binding.**
   - Once per generator, a token bound to **visitorData** is minted. Its use is unknown to the source.
   - Per video, a token bound to the **video id** is minted. This is the one sent in `/player` and as `pot`.
9. **Lifetime.**
   - Expiry is the GenerateIT lifetime minus 600 s.
   - The generator and visitorData are rebuilt on expiry or error.
   - A failed mint is retried once with a new generator.
   - Per-video tokens are cached until expiry.

### 6.4 TV (living-room) token

1. **`tv_config`** (see 5.1). Read:
   - `challengeRequestKey`, the TV BotGuard request key, which differs from the web key;
   - `challengeParams.R`, a ready challenge as escaped JSON. It can be missing for Cobalt user agents.
2. **`/tv` page.** Read `LIVING_ROOM_PO_TOKEN_ID` (example shape `wpeaWfYpTp8=`) and `visitorData` **from the same page load**.
3. **Challenge.** The current code always fetches it via `/youtubei/v1/att/get` with `clientName:"TVHTML5"` and the TV version, engagement type UNBOUND.
4. **BotGuard and GenerateIT** as on web, but with the TV request key.
5. **Mint** once, with the living-room id as the identifier. This is a **session token, not per video**. It is used for `/player` and for `pot`, alongside `tvAppInfo.livingRoomPoTokenId` and the `/tv` visitorData.
6. **Lifetime:** same expiry margin. Only one generator family (web or TV) is kept alive at a time.

### 6.5 Historical PO-token code

- **Cold-start token** (`yt/app/potoken/PoTokenService.kt`), used from 2026-05-22 and now uncalled:
  - layout: tag byte 34, a length byte, a 2-byte random key, a 0 byte, a client-state byte (1), a 4-byte big-endian Unix time, then the UTF-8 identifier (at most 118 bytes);
  - everything after the key is XOR'd with the key;
  - the result is base64-encoded.
- **Older endpoints:** `https://jnn-pa.googleapis.com/$rpc/google.internal.waa.v1.Waa/Create` and `https://www.youtube.com/api/jnn/v1/Create`. The challenge was scrambled: base64 with each byte shifted by 97.
- **Cloud services** (`yt/app/potokencloud*`): GET with `content_binding` and a `/health-check`. The hosts were placeholders. Removed 2026-10-11.

### 6.6 visitorData

- Non-token requests use the `/tv` visitorData.
- The `/tv` response cookies (`VISITOR_INFO1_LIVE`, `VISITOR_PRIVACY_METADATA`) are stored and replayed on later `/tv` fetches.
- The source says visitorData is tied to player versions and can be invalidated.

---

## 7. Login (TV device-code OAuth)

Sources: `yt/auth/V2/*`, `yt/app/models/ClientData.java`, `yt/service/YouTubeSignInService.java`, `yt/service/internal/YouTubeAccountManager.java`. The flow is current; the auth code was last refactored 2025-12-13 (`7c5e8444`), and the client-id extraction is unchanged since 2023-11-01.

**Client id and secret** are not hard-coded. They are scraped at runtime:
1. From the `/tv` HTML, take the URL of the "base" script. The modern page has a `<script id="base-js" src="…">`. Cobalt pages assign `….src = '…m=base'` in an inline script, with a newer form that also sets `id = 'base-js'`.
2. In that script, find the **first** occurrence of a `clientId:"<…>.apps.googleusercontent.com"` property followed directly by another short string property. That second string is the client secret. The source says later occurrences carry other client ids that fail with HTTP 401.

Observed 2026-11-23: the Fire TV UA's `/tv` page referenced `/s/_/kabuki/_/js/k=kabuki.base_cr.en_US…/m=base` (about 1 MB). Its first match was client id `861556708454-d6dlm3lh05idd8npek18k6be8ba3oc68.apps.googleusercontent.com`, with a secret starting `SboV…`. The pattern still works.

**Endpoints.** All are POSTs with `Content-Type: application/json`, and the bodies are JSON, not form-encoded:

| Step | URL | Body |
|---|---|---|
| Device code | `https://www.youtube.com/o/oauth2/device/code` | `{client_id, device_id: <random UUID persisted per install>, model_name:"ytlr::", scope:"http://gdata.youtube.com https://www.googleapis.com/auth/youtube-paid-content"}` |
| Poll for token | `https://www.youtube.com/o/oauth2/token` | `{code:<device_code>, client_id, client_secret, grant_type:"http://oauth.net/grant_type/device/1.0"}` |
| Refresh | `https://www.youtube.com/o/oauth2/token` | `{refresh_token, client_id, client_secret, grant_type:"refresh_token"}` |
| Accounts | `https://www.youtube.com/youtubei/v1/account/accounts_list` | TV context plus `accountReadMask:{returnOwner:true, returnBrandAccounts:true, returnPersonaAccounts:false}` |

**Response fields:**
- Device code: `device_code`, `user_code`, `expires_in`, `interval`, `verification_url`. The user enters the code at youtube.com/activate.
- Token: `access_token`, `expires_in`, `refresh_token`, `scope`, `token_type`, `error`, `error_description`.

**Polling:** a fixed 3 s interval for up to 200 attempts (10 min). It does **not** use the `interval` returned by the server. Polling stops when a `refresh_token` appears.

**Use:**
- Every Innertube call carries `Authorization: <token_type> <access_token>`, and then the `key=` parameter is dropped.
- The access token is refreshed from the stored refresh token on demand, at most every 60 min. The source notes the access token lives at most 60 min.

**Accounts and switching:**
1. After sign-in, the refresh token is used to call `accounts_list`.
2. Accounts are at `contents[0].accountSectionListRenderer.contents[0].accountItemSectionRenderer.contents[*].accountItem`, with fields `accountName`, `accountByline`, `channelHandle`, `accountPhoto.thumbnails[]`, `isSelected`, `isDisabled`, `hasChannel`, and `serviceEndpoint.selectActiveIdentityEndpoint.supportedTokens[*].pageIdToken.pageId`.
3. Each account, including brand accounts, is stored with the **same refresh token**.
4. Selecting a brand account adds the header **`X-Goog-Pageid: <pageId>`** to every request. The source says this applies the brand account's rights, for example for restricted videos.
5. A note says the `8AEB2AMB` `params` (thought to unlock web premium formats) is not supported while `X-Goog-Pageid` is set.

**Features that need login:** history sync and watch-time reporting, subscriptions, Watch Later and playlists, Premium formats, and some restricted videos. Only the TV clients carry the token for `/player`.

**Cookies:** SAPISIDHASH cookie auth is **not** used anywhere in MSC.

**Historical:** `yt/auth/V1/*`, an older API set.

---

## 8. Captions

Sources: `yt/videoinfo/models/CaptionTrack.java`, `TranslatedCaptionTrack.java`, `yt/innertube/impl/MediaSubtitleImpl.kt`, `yt/innertube/utils/PlayerResultExtensions.kt` [current].

**Track list:** `captions.playerCaptionsTracklistRenderer.captionTracks[]`, with fields `baseUrl`, `languageCode`, `vssId`, `name`, `kind` (`asr` = auto-generated) and `isTranslatable`. Translation targets are in `…translationLanguages[]`.
- A relative `baseUrl` (seen on MWEB) is prefixed with `https://www.youtube.com`.
- TV clients return a limited translation list, so the full list is taken from an extra anonymous WEB call (see 3).

**Format:**
- `fmt` in `baseUrl` is overwritten with `vtt` (mime `text/vtt`, codec `wvtt`). This has been the default since `85908326`, 2026-04-01.
- `ttml` (`application/ttml+xml`, `stpp`) is the user-selectable alternative.

**Translated tracks:**
- Append `&tlang=<code>` to the base URL of one origin track. Skip it when the target language equals the origin's language.
- **Choosing the origin track:** the first ASR track that is neither a dub transcript nor in an auto-dubbed language. Otherwise the first ASR track; otherwise the first track.
  - A dub transcript has `variant=timing-optimized` in its URL.
  - Auto-dubbed languages are the `adaptiveFormats[].audioTrack.id` values where `isAutoDubbed` is true.
  - This rule was added in `4929ff24`, 2026-10-16.

**PO token:** the newer path appends `&pot=<token>&c=<client label>` when a token exists. The older path never does, so a token is not always required.

---

## 9. Live

Sources: `yt/videoinfo/VideoInfoServiceBase.java`, `yt/formatbuilders/mpdbuilder/YouTubeMPDBuilder.java`, `ST:…/VideoLoaderController.java` [current].

**Flags:**
- `videoDetails.isLive` and `isLiveContent` are read. `isLiveContent` with `isLive` false means a past or post-live stream.
- `isLowLatencyLiveStream` and the format `maxDvrDurationSec` and `targetDurationSec` are also read.
- `liveBroadcastDetails.startTimestamp` gives the start time for upcoming and live streams.

**Playback paths:**
- The default for live is SmartTube's own dynamic MPD (4.3), built from `adaptiveFormats` with the head-segment probe. If the start time is unknown, it uses YouTube's `dashManifestUrl` instead.
- User toggles force YouTube's DASH URL or YouTube's HLS. `hlsManifestUrl` for live comes from the TV client response.
- An `hfr/all` in `dashManifestUrl` marks a high frame rate stream.

**Window:** the MPD covers 48 h, so the whole DVR window plus long streams stay seekable.

**Upcoming and premieres:** when there are no formats at all, the app re-polls every 30 s.

**SABR live:** experimental and off by default (see 4.4).

---

## 10. Failure modes and churn

**Recent history (newest first):**

| Date | Commit | Area | Change |
|---|---|---|---|
| 2026-11-22 | `bad6cb57`, `1814c5ad` | Player JS | Download retries raised to 3; a failed download no longer leaves a broken cached extractor |
| 2026-11-21 | `3c733fef` | PO token | Cache-reset throttle reverted from 3 min to 60 s |
| 2026-11-11 to 11-14 | | Clients | Client list reworked: TV is the default again, Samsung TV added, a fallback to an error result added, "pot attestation fix" (CPN per request) |
| 2026-11-06 | `4010903f` | SABR | Live SABR becomes a preference |
| 2026-10-24 | `ba369250` | SABR | Live SABR disabled: "some streams still broken" |
| 2026-10-07 | `1b43ec76` | SABR | Reload token in `/player` |
| 2026-10-01 to 10-05 | | TV player and PO token | TCL TV player, TV PO token and `livingRoomPoTokenId` added, after TV playback broke ("TV client fix (nsig and potoken)") |
| 2026-10-11 | `0b07f952` | PO token | Cloud PO-token fallback removed |
| 2026-09-08 | `86c87883` | Clients | Restricted videos fixed by downgrading the TV UA |
| 2026-08 | | PO token | Homepage ytcfg / `ytAtN` challenge path; WebView generator v4, SABR-compatible |
| 2025-09 to 2026-03 | | Player JS | Frequent ejs solver core updates, V8 lifecycle fixes, a "temp bad sig workaround" |

**What breaks most often:**
- the n transform, especially the newest TCL shape, which the solver's own notes call ambiguous;
- PO-token acceptance (403s on googlevideo);
- client versions and user agents;
- Shorts detection in feeds (2026-04-29, 2026-11-09);
- the Watch Later location (2026-06-01).

**Errors and retries:**
- **A googlevideo 403 or a playback error:** reset the PO-token cache of the current family (at most once per 60 s), then rotate to the next client.
- **Unplayable `/player` responses:** move to the next client.
- **Rental content:** play the trailer.
- **Failed player downloads:** 3 tries, 1 s apart.
- **Failed authenticated search:** retry anonymously.
- **Live head-segment probe:** retried once.
- **SABR ReloadPlayerResponse:** re-fetch `/player` with the reload token.

**Rate limits and IPs:** the source has no specific handling for 429s or IP bans. `UNPLAYABLE` is read as possibly a temporary ban.

---

## Open questions

1. **Login without SAPISID.** Does the TV OAuth token alone get adaptive URLs, rather than SABR-only formats, from TVHTML5 7.x without a living-room PO token? The source always sends the token when it can, and this was not tested (it would need a login).
2. **Downgraded TV client.** How long will TVHTML5 5.x with an old Cobalt UA keep returning plain adaptive URLs without a PO token? Observed working on 2026-11-23, but n still has to be solved: an unsolved n gave a 403.
3. **Player hash.** Does YouTube require the player hash from `/tv` to match the one from `tv_config` `jsUrl`? `/tv` gave two different hashes on the same day.
4. **TCL streams.** Do they ever carry `s`, or a `signatureCipher`? The source assumes never.
5. **BotGuard without a DOM.** Could BotGuard run without a browser DOM, for example in a plain JS engine inside WebAssembly? The source's only working path is a full WebView, and it abandoned bare V8.
6. **The visitorData-bound web token.** What is it for? The source does not know.
7. **The ejs version label.** Is the `0.0.1` pin only a label? The bundled core changed several times under it.
8. **Handle resolution.** Does the bare-name URL still resolve every `@handle`? The code dates from 2025-08.
9. **Range limits.** Range and chunk-size limits for googlevideo are not set anywhere in MSC; they are left to ExoPlayer. This source gives no guidance on throttling thresholds.
10. **Device-code polling.** The source ignores the server's `interval`. Whether YouTube enforces `slow_down` is not settled.
