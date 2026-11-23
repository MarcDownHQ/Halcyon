# Source facts: YouTube.js, BgUtils, googlevideo (+ bgutil-ytdlp-pot-provider)

Facts-only clean-room notes. No code from the sources is reproduced here; behaviour is described in prose. Protocol facts (URLs, headers, JSON paths, protobuf field numbers, constants YouTube itself publishes) are given verbatim because they are facts about YouTube.

## Sources, licences, commits read

| Short name | Repo | Licence | Commit read (default branch) | Commit date |
|---|---|---|---|---|
| YTJS | https://github.com/LuanRT/YouTube.js (`main`, package version 18.3.0) | MIT | `77703faa19e0214735be8db905e262c9e4a43a63` | 2026-11-18 |
| BGU | https://github.com/LuanRT/BgUtils (`main`, package 4.0.3) | MIT | `f842f8a14435a35800ff469d315bb1a7e6d7a64a` | 2026-10-23 |
| GV | https://github.com/LuanRT/googlevideo (`main`, package 5.0.1) | MIT | `0fb5b840636ff7dbc645624747998ef8eec601a1` | 2026-11-21 |
| POT | https://github.com/Brainicism/bgutil-ytdlp-pot-provider (`master`, 2.0.2) | **GPL-3.0** (facts only) | `26475e9d3665b972c4aeb83b4dbaafcf64b88438` | 2026-10-07 |

Issue threads read (with `gh`): BGU #3, #19, #31, #48; POT #208, #242, #285. Light live observations against YouTube were made on 2026-11-23 (11 requests, logged out, one residential IP); they are marked **[obs 2026-11-23]**.

"Current" = present in the latest commit listed. "Historical" = kept as a fallback, removed, or reported dead.

## Summary of the sources' current strategy

YouTube.js is an InnerTube client whose default session client is `WEB`; it can switch per request to about fifteen other clients (iOS, Android, Android VR, visionOS, TV variants, embedded, Music, Kids, Creator). It does **not** generate PO tokens itself: the caller passes one in (session-wide, or per video), and YouTube.js puts it in `serviceIntegrityDimensions.poToken` on `/player` and as `pot=` on non-SABR stream URLs. Since v16.0.0 (2025-10-12) it also ships **no JavaScript evaluator**. It parses the player `base.js` with a real ES parser, extracts a self-contained script that holds the n/signature transform and the signature timestamp, and asks a caller-supplied evaluator to run it. BgUtils is the companion PO-token library. It does not emulate BotGuard. It fetches a BotGuard challenge (interpreter JS plus a bytecode "program"), runs the interpreter in the host's JS engine with fake `window`/`document`/`navigator` (jsdom in every example), snapshots the VM, exchanges the snapshot for an integrity token at `GenerateIT`, then uses a VM-provided minter to mint ~110-128-byte WebPO tokens bound to a string (video id or visitor data). The challenge that works for the `WEB`/`MWEB` clients now has to come from the YouTube homepage (`window.ytAtN(...)`) together with that page's `ytcfg` (for `EVENT_ID`). The WAA `Create` challenge only works for some clients (YouTube Music). googlevideo implements SABR/UMP, which the `WEB` client's `/player` response now uses exclusively (observed: no format URLs, only `serverAbrStreamingUrl`).

---

## 1. Clients (YouTube.js)

All from YTJS `src/utils/Constants.ts` and `src/utils/HTTPClient.ts` @77703fa, **current**.

| YTJS key | `clientName` | `clientVersion` | `X-YouTube-Client-Name` | Notes |
|---|---|---|---|---|
| WEB | `WEB` | `2.20260623.01.00` | 1 | API key `AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8`; "static visitor id" `6zpwvWUNAco`. With the default `/sw.js_data` session path, the WEB version is taken live from YouTube rather than this constant. |
| MWEB | `MWEB` | `2.20260205.04.01` | 2 | `platform: MOBILE` |
| YTKIDS | `WEB_KIDS` | `2.20260205.00.00` | 76 | sends a `kidsAppInfo` block (category list and `KIDS_CORPUS_PREFERENCE_YOUNGER`) |
| YTMUSIC | `WEB_REMIX` | `1.20250219.01.00` | 67 | |
| ANDROID | `ANDROID` | `21.03.36` | 3 | `androidSdkVersion` 36, UA `com.google.android.youtube/21.03.36(Linux; U; Android 16; en_US; SM-S908E Build/TP1A.220624.014) gzip`, `osName Android`, `osVersion 13`, `platform MOBILE` |
| ANDROID_VR | `ANDROID_VR` | `1.65.10` | 28 | `androidSdkVersion` 32, `osVersion 12L`, `deviceMake Oculus`, `deviceModel Quest 3`, UA `com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip`. Added in 17.0.0 (2026-03-16); UA header override added in 18.0.0. |
| IOS | `iOS` | `20.11.6` | 5 | UA `com.google.ios.youtube/20.11.6 (iPhone10,4; U; CPU iOS 16_7_7 like Mac OS X)`, `deviceMake Apple`, `deviceModel iPhone10,4`, `osName iOS`, `osVersion 16.7.7.20H330`; browserName/Version removed |
| VISIONOS | `VISIONOS` | `1.02` | 101 | `deviceMake Apple`, `deviceModel RealityDevice17,1`, `osName visionOS`, `osVersion 26.5.23O471`, Safari-style UA; added 18.0.0 (2026-08-13) |
| YTSTUDIO_ANDROID | `ANDROID_CREATOR` | `22.43.101` | 14 | `clientFormFactor SMALL_FORM_FACTOR` + Android fields |
| YTMUSIC_ANDROID | `ANDROID_MUSIC` | `5.34.51` | 21 | as above |
| TV | `TVHTML5` | `7.20260311.12.00` | 7 | UA `Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version` |
| TV_SIMPLY | `TVHTML5_SIMPLY` | `1.0` | 74 | added 14.0.0 (2025-06-08) |
| TV_EMBEDDED | `TVHTML5_SIMPLY_EMBEDDED_PLAYER` | `2.0` | 85 | `clientScreen EMBED`, `thirdParty.embedUrl https://www.youtube.com` |
| WEB_EMBEDDED | `WEB_EMBEDDED_PLAYER` | `1.20260206.01.00` | 56 | `clientScreen EMBED`, `thirdParty.embedUrl https://www.google.com/` |
| WEB_CREATOR | `WEB_CREATOR` | `1.20241203.01.00` | 62 | |

- Request URL: `https://www.youtube.com/youtubei/v1/<endpoint>?prettyPrint=false&alt=json`. YTJS does not add `key=`. (YTJS `HTTPClient.ts`, current)
- Common headers on every InnerTube call: `Accept: */*`, `Accept-Language: *`, `X-Goog-Visitor-Id: <context.client.visitorData>`, `X-Youtube-Client-Version`, `X-Youtube-Client-Name` (numeric id above). When running server-side it also sends `User-Agent` and `Origin` (the request origin). (YTJS `HTTPClient.ts`, current)
- Default `context.client` built by YTJS: `hl`, `gl`, `remoteHost`, `screenDensityFloat 1`, `screenHeightPoints 1440`, `screenWidthPoints 2560`, `screenPixelDensity 1`, `visitorData`, `clientName`, `clientVersion`, `osName`, `osVersion`, `userAgent`, `platform` (`DESKTOP` by default), `clientFormFactor UNKNOWN_FORM_FACTOR`, `userInterfaceTheme USER_INTERFACE_THEME_LIGHT`, `timeZone`, `originalUrl https://www.youtube.com`, `deviceMake`, `deviceModel`, `browserName`, `browserVersion`, `utcOffsetMinutes`, `memoryTotalKbytes "8000000"`, `rolloutToken`, `deviceExperimentId`, `mainAppWebInfo{graftUrl, pwaInstallabilityStatus PWA_INSTALLABILITY_STATUS_UNKNOWN, webDisplayMode WEB_DISPLAY_MODE_BROWSER, isWebNativeShareAvailable true}`, optional `configInfo.appInstallData`; plus `user.enableSafetyMode`, `user.lockedSafetyMode false`, `request.useSsl true`, `request.internalExperimentFlags []`. For any client other than WEB, `configInfo` is dropped. (YTJS `Session.ts`, `HTTPClient.ts`, current)
- Stream (googlevideo) URLs: YTJS sets `cver=` to its own constant version for the client named in the URL's `c=` param (WEB, MWEB, WEB_REMIX, WEB_KIDS, TVHTML5, TVHTML5_SIMPLY, TVHTML5_SIMPLY_EMBEDDED_PLAYER, WEB_EMBEDDED_PLAYER). Stream request headers: `accept: */*`, `origin: https://www.youtube.com`, `referer: https://www.youtube.com`, `DNT: ?1`. (YTJS `Player.ts`, `Constants.ts`, current)
- YTJS's own examples use `YTMUSIC` for streaming (`examples/download/index.ts`, BGU `examples/*.ts`).
- BGU issue #48 (2026-08-28, LuanRT): **`MWEB` "recently transitioned to SABR-only"**, and like `WEB` it "only works with challenges from InnerTube". The source does not say which clients still get plain URLs without a PO token.
- **[obs 2026-11-23]** `/player` for `jNQXAC9IVRw` with `WEB` at the live version (`2.20261119.07.00`, read from the homepage `INNERTUBE_CLIENT_VERSION`), with visitorData from `att/get` and with `signatureTimestamp` 20774, returned `OK`. `streamingData` had only `expiresInSeconds`, `adaptiveFormats` (14 entries, **none** with `url` or `signatureCipher`), `probeUrl` and `serverAbrStreamingUrl`, plus `playerConfig.mediaCommonConfig.mediaUstreamerRequestConfig.videoPlaybackUstreamerConfig`. The same call without `signatureTimestamp` and with an invented client version returned `UNPLAYABLE` / "The page needs to be reloaded." for both WEB and MWEB.

## 2. Catalog (YouTube.js)

- **URL to channel id**: `POST /youtubei/v1/navigation/resolve_url` with body `{url}`; the response's `endpoint` is a navigation endpoint (e.g. a `browseEndpoint` with `browseId`). (YTJS `Innertube.ts`, current)
- **Channel page**: `POST /youtubei/v1/browse` with `browseId = <channel id>`. If the response carries `onResponseReceivedActions[0]` of the navigate kind, YTJS calls that endpoint instead (a redirect). (YTJS `Innertube.ts` `getChannel`, current)
- **Tabs**: YTJS hard-codes no `params` blobs for channel tabs. It reads the tab list from the browse response and picks a tab by the **last path segment of the tab endpoint's URL** (`commandMetadata.webCommandMetadata.url`). Known segments: `featured`, `videos`, `shorts`, `streams`, `releases`, `podcasts`, `courses`, `shows`, `playlists`, `posts`, `about`. It then calls the tab's own endpoint (the `browseEndpoint` with the `browseId` and `params` YouTube supplied). A selected tab is used as-is. (YTJS `src/core/mixins/TabbedFeed.ts`, `src/parser/youtube/Channel.ts`, current; `shows` tab added 18.1.0, 2026-09-22)
- **Sort and filter** ("Latest", "Popular", "Oldest", "Members only") are applied by following the chip or list-item endpoints found in the tab response, not by building params. Continuations follow the continuation token found in the feed. (YTJS `Channel.ts`, current)
- RSS, Watch Later, subscriptions and search were not studied in this pass; YTJS has search and library features, but they are outside this file's focus.

## 3. The player request (YouTube.js)

Body that YTJS sends to `/youtubei/v1/player` (YTJS `src/Innertube.ts` `getInfo` / `getBasicInfo`, current):
- `videoId`, `racyCheckOk: true`, `contentCheckOk: true` (plus `playlistId`/`playlistIndex`/`params` when coming from a watch endpoint);
- `playbackContext.contentPlaybackContext = { vis: 0, splay: false, lactMilliseconds: "-1", signatureTimestamp: <from player> }`;
- `serviceIntegrityDimensions.poToken = <per-call PO token if given, else the session PO token>`.
- `getInfo` also fires `/next` in parallel and generates a 16-character random `cpn`.
- Per-call PO tokens came in 15.0.0 (2025-07-18), described as support for **content-bound** PoTokens. The session option `po_token` is documented as a "Session bound Proof of Origin Token ... generated by BotGuard/DroidGuard". (YTJS `CHANGELOG.md`, `src/core/Session.ts`)
- YTJS parses `streamingData.serverAbrStreamingUrl` and `playerConfig.mediaCommonConfig.mediaUstreamerRequestConfig.videoPlaybackUstreamerConfig`. (YTJS `src/parser/parser.ts`, current)

## 4. Formats

Outside this file's main focus. Facts seen:
- YTJS can build a DASH manifest in which, with its `is_sabr` option (14.0.0, 2025-06-08), every representation URL is replaced by a pseudo-URL of the form `sabr://video?key=<format key>` / `sabr://audio?key=...`, which a SABR-aware player resolves. (YTJS `src/utils/StreamingInfo.ts`, current)
- xtags are parsed from protobuf to support SABR-only responses (13.2.0, 2025-03-20). (YTJS `CHANGELOG.md`)
- The format identity used by SABR is the triple (itag, last-modified `lmt`, `xtags`). (GV `protos/misc/common.proto` `FormatId` fields 1/2/3, current)

## 5. Player JavaScript and challenges (YouTube.js)

All from YTJS `src/core/Player.ts`, `src/utils/javascript/*`, `src/utils/Utils.ts`, `src/platform/jsruntime/default.ts` @77703fa unless noted.

**Finding the player**
- `GET https://www.youtube.com/iframe_api`; the player id is the text between the escaped `player\/` and the next `\/`. **[obs 2026-11-23]** id `6b3d0364`.
- Player script URL: `https://www.youtube.com/s/player/<id>/player_es6.vflset/en_US/base.js`, fetched with a random desktop UA. **[obs]** 2,703,532 bytes; begins by declaring the `_yt_player` global and opening an IIFE whose parameter is the export object, with a local `window` bound to `this`.
- A session option can force an older player id. The docs comment says this helps "when YouTube introduces breaking changes" (13.4.0, 2025-04-23).

**Extraction approach (prose)**
- The whole `base.js` is parsed into an ESTree AST with the `meriyah` parser (a dependency). This is an AST-based extractor, introduced as a breaking change in 16.0.0 (2025-10-12): "AST-based JS extraction with side-effect safe code emission". It replaces the earlier regex and global-array approaches (13.3.0 "parse global variable used by nsig/sig", 15.1.0 "fix global variable extraction"), now **historical**.
- **n and signature in one go**: YTJS no longer locates separate n and sig functions. It looks for one variable declarator whose initializer is a function expression with at least three parameters: the first a plain identifier, and the second and third with default values. Its body must (a) assign to the first parameter a `new` expression whose callee is a member expression (a player-internal URL-like class) and (b) contain a method call with exactly the two string literals `"alr"` and `"yes"`. This is the player's stream-URL builder. **[obs]** `base.js` 6b3d0364 contains `.set("alr","yes")` call sites.
- **Signature timestamp**: a variable bound to a function expression whose body contains an object literal with a property named `signatureTimestamp`. YTJS exports only its raw value. **[obs]** the literal `signatureTimestamp:20774` appears in player 6b3d0364.
- **Dependency closure**: from the match, YTJS collects every free identifier transitively and resolves each one to a declaration inside the player's IIFE. It also tracks prototype aliasing (a variable assigned some class's `.prototype`, followed by member assignments through that alias) so that class methods come along. It emits a standalone script containing only those declarations. Initializers that would run side-effecting code (calls, `new`, ...) are replaced with `undefined`. The emitted script binds `window` to the global `window` or an empty null-prototype object, `document` to the global `document` or `{}`, and `self` to the global or `window`. The source notes that some dependencies can never resolve because they hang off the real `window` (for example `yt.config_`/`ytcfg.data_` look-ups), so extraction tolerates unresolved members.
- **Running it**: YTJS appends a small driver that calls the extracted builder with a fake `https://...googlevideo.com/videoplayback?expire=...&n=<n>` URL and the `sp` and `s` values. It then calls every own method on the returned object's prototype except the constructor, `clone`, `set` and `get`, which makes the object apply its transforms. Finally it reads back the `n` param and the `sp`-named param (the signature), both URL-decoded. The deciphered signature goes into the param named by `sp`, or `signature` when `sp` is absent. An n result that starts with `enhanced_except_` is the player's own error marker: YTJS logs it, keeps the value and does not cache it.
- **JS engine requirement**: the emitted script is ordinary ES2015+ (default parameters, classes/prototypes, `Object.getPrototypeOf`, `Object.getOwnPropertyNames`, `encodeURIComponent`/`decodeURIComponent`). No DOM is needed beyond the empty stubs. YTJS ships **no evaluator** on any platform (Node, Deno, web, Cloudflare Worker): the default throws "you must provide your own JavaScript evaluator". Callers plug one in; BGU's examples use the host's `new Function`. The evaluator became async in 16.0.0. A full ES parser and a full ES engine are therefore needed: the parser handles the 2.7 MB script, the engine runs the emitted subset.
- **Caching**: the extracted script plus the signature timestamp is cached keyed by **player id**. The cache entry stores the library version and is ignored when that version differs (15.1.1). The n results are memoised per player response (6.4.1, 2023-10-02).
- **Churn**: the changelog shows extraction fixes in 10.1.0 and 10.2.0 (2024-07), 12.0.0/12.1.0/12.2.0 (2024-12), 13.3.0/13.4.0 (2025-03/04), 15.1.0 (2025-09), the rewrite in 16.0.0 (2025-10) and "Add support for new variants" in 17.0.0 (2026-03-16). No player-extraction change appears in 18.x (2026-08 to 2026-10).

## 6. PO tokens and BotGuard

### 6.1 What the tokens are and where they go
- **Token types** (BGU `README.md`, current, written 2026-07):
  - *Content-bound*: minted for each video request with the **video id** as the binding. This is what YouTube's web player does today. It reuses one minter for as long as the page is open.
  - *Cold start*: a placeholder token, also built from the same binding, used before BotGuard is ready (format in 6.5).
  - *Session-bound*: bound to the Data Sync ID when logged in, otherwise to the visitor id. "YouTube's web client does not use this token type anymore" (**historical** on web).
- **Placement**: in `/player` as `serviceIntegrityDimensions.poToken`; on non-SABR googlevideo URLs as the `pot` query param (YTJS `Player.ts`, current; BGU examples append `&pot=<token>`). For SABR the token travels **as raw bytes** inside the request protobuf (`streamerContext.poToken`, field 2), not in the URL (YTJS `Player.ts` note; GV `SabrStream`, `streamer_context.proto`).
- **Which binding for which use, a source disagreement**:
  - BGU's examples mint with the **video id** and append the result as `pot` to a `YTMUSIC` stream URL. BGU README says the web player mints per video with the video id.
  - BGU issue #31 (LuanRT): a SABR download failure "might [be] because of incorrect content binding. Try setting the identifier ... to the video id itself instead of visitor data."
  - POT's server falls back to the caller's InnerTube `context.client.visitorData` (or a freshly created one) when no binding is given. POT supports three token contexts: `gvs`, `player` and `subs` (POT `plugin/.../getpot_bgutil.py`). The caller (yt-dlp) chooses the binding per context; this file does not cover yt-dlp.
  - YTJS uses one session `po_token` both in `/player` and as `pot`, plus an optional per-call `/player` token.
- **SPS (stream protection status)** in every UMP response (BGU README; GV `stream_protection_status.proto`: `status` field 1, `max_retries` field 2):
  - `1`: the token is fine, or the account is Premium, or the stream needs none;
  - `2`: a token is required, but about 1-2 MB can still be fetched on a cold-start token; get a real one soon;
  - `3`: no more media without a valid token.
  - GV re-mints on 2 or 3 and counts each 3 as a rejection, giving up after `max_retries` (default 5) (GV `src/core/SabrStream.ts`, current).
- POT issue #285 comment (2026-11-23): tokens from a bad environment are "usable for the first 60 seconds of a video", then 403. This is consistent with the cold-start grace.

### 6.2 The three ways to get a BotGuard challenge

A challenge is: `program` (bytecode, base64-ish string), `globalName` (name of the global under which the interpreter installs the VM), `interpreterHash`, an interpreter (either a URL or the JS itself), `clientExperimentsStateBlob`, and sometimes `messageId`.

1. **Embedded in the YouTube homepage** (**current**, now the preferred source for WEB-family clients):
   - `GET https://www.youtube.com` (POT sends `accept: */*`, `accept-language: en-US,en;q=0.7`, and the BGU desktop UA).
   - The page contains `window.ytAtP = new Promise(res => window.ytAtN = res)` followed later by a call `window.ytAtN({'R': '<string>', 'T': '<string>'})`. **[obs]** The first textual occurrence is the empty `window.ytAtN();` in a cleanup line, so search for the call with an object argument.
   - `R` is a single-quoted JS string with `\xNN` escapes. Decoded, it is JSON shaped like an `/att/get` response: `responseContext`, `challenge`, and `bgChallenge{interpreterUrl.privateDoNotAccessOrElseTrustedResourceUrlWrappedValue, interpreterHash, program, globalName, clientExperimentsStateBlob}`. The object literal is loose (single quotes, trailing comma), so it must be normalised before JSON parsing. **[obs]** `R` was 42,577 characters, `program` 39,547 characters, `globalName` `trayride`, interpreter URL `//www.google.com/js/th/<interpreterHash>.js` (scheme-relative; prefix `https:`). The interpreter JS was 64,222 bytes, `text/javascript`. `T` was 95 characters; its purpose is not explained by any source.
   - **Historical form**: `window.ytAtR = '<JSON string>';`, which POT's plugin still tries as a fallback (POT commit 9612094, 2026-02-20, "support ytAtP"). BGU issue #19: YouTube itself uses the page-embedded challenge (`ytAtR` at that time) for performance.
   - The same page's `ytcfg.set({...})` JSON must be installed as `yt.config_` on the global object the VM sees, because **BotGuard reads `yt.config_.EVENT_ID`** (BGU `examples/index-innertube.ts` @3fa0402, 2026-08-04; POT 495a47f, 2026-08-21). **[obs]** the homepage contains `"EVENT_ID"` and `"VISITOR_DATA"` inside `ytcfg.set`.
2. **InnerTube `att/get`** (**historical** for WEB, kept as POT's last-resort fallback):
   - `POST https://www.youtube.com/youtubei/v1/att/get?prettyPrint=false`, JSON body `{context, engagementType: "ENGAGEMENT_TYPE_UNBOUND"}`, optionally with `ids: [...]` (YTJS `Innertube.ts` `getAttestationChallenge`, added 13.0.0, 2025-01-20).
   - POT's fallback context is `{client:{clientName:"WEB", clientVersion:"2.20260817.01.00"}}` and it sends the WAA headers plus `Content-Type: application/json`.
   - Response: `responseContext` (includes a fresh `visitorData`), `challenge` (a `&`-joined parameter string; **[obs]** keys `a, a2, b, c, d, t, c1a, c6a, c6b, mv, mt, hh`), and `bgChallenge` (same shape as above). **[obs]** HTTP 200 with the same `interpreterHash` and `globalName` as the homepage.
   - POT commit 495a47f (2026-08-21): "YouTube now binds the initial attestation challenge to the webpage session (`yt.config_.EVENT_ID`) and **rejects WebPO tokens minted from /att/get challenges** when the session is enrolled in the experiment. Symptom: player requests pass, googlevideo 403s the download." BGU's example was switched from `att/get` to the homepage on 2026-08-04 (3fa0402).
3. **WAA private API `Create`** (**current**, but only some clients accept the result):
   - `POST https://jnn-pa.googleapis.com/$rpc/google.internal.waa.v1.Waa/Create`, or the same path proxied on YouTube as `https://www.youtube.com/api/jnn/v1/Create`.
   - Headers: `content-type: application/json+protobuf`, `x-goog-api-key: AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw`, `x-user-agent: grpc-web-javascript/0.1`. Outside a browser, BGU also sends `user-agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36(KHTML, like Gecko)` (BGU `src/utils/constants.ts`, `helpers.ts`; the UA was added in 3.1.1 "Always send a valid UA when running in a server").
   - Body: a JSON array `[requestKey]`, or `[requestKey, interpreterHash]` to say which interpreter you already have.
   - YouTube's request key is `O43z0dpjhgX20SCx4KAo` (BGU examples; POT `session_manager.ts`).
   - Response: a JSON array. If element 1 is a string, it is **scrambled**: base64 (URL-safe alphabet tolerated) whose every byte, after adding 97 modulo 256, yields UTF-8 JSON. Otherwise element 0 is already the payload. The payload array holds, by position: 0 `messageId`, 1 wrapped interpreter JS (an array whose one non-empty string is the script), 2 wrapped interpreter URL (array), 3 `interpreterHash`, 4 `program`, 5 `globalName`, 6 unused, 7 `clientExperimentsStateBlob` (BGU `src/core/ChallengeFetcher.ts`).
   - **[obs]** Both hosts returned 200 with a scrambled reply of about 108 KB. The payload had 8 elements: `messageId` 4 chars, an inline interpreter of 64,222 chars (the same hash as the homepage), `program` about 9.9 KB (much shorter than the homepage program), `globalName` `trayride`.
   - Which clients accept it: BGU `examples/README.md` (2026-08-04) says the WAA challenge works for "only some clients ... e.g. YouTube Music"; the InnerTube attestation challenge "is what the official YouTube clients currently use". BGU issue #48: `WEB` and `MWEB` "only work with challenges from InnerTube".

### 6.3 Running BotGuard, step by step (facts about the VM's interface)
Sources: BGU `README.md`, `src/core/BotGuardClient.ts`, `src/core/WebPoMinter.ts`, examples, current.
1. Execute the interpreter JS in global scope (BGU uses the engine's `Function` constructor; in a browser, a `<script>` tag with id = interpreterHash). Afterwards the global object has a property named `globalName` (the VM).
2. The VM exposes an initialiser under the property `a`. Call it with nine positional arguments:
   1. the `program` string;
   2. a setup callback that receives four functions (async snapshot, shutdown, pass-event, check-camera);
   3. `true`;
   4. a user-interaction element (BGU passes undefined);
   5. a telemetry callback `(latency, flag1, flag2)`;
   6. the experiments value `[[], []]`;
   7. undefined;
   8. `false`;
   9. an array of five logger callbacks (event log, client-error count, payload size, latency, event count; Google labels them `/client_streamz/bg/el`, `/cec`, `/od/p`, `/od/n`, `/ec`). The return value's element 0 is a synchronous snapshot function. BGU waits for the setup callback before going on; that wait was a bug fix in 3.2.0.
3. Snapshot: call the async snapshot function with a result callback and a four-element array `[contentBinding, signedTimestamp, webPoSignalOutput, skipPrivacyBuffer]`. For PO tokens only `webPoSignalOutput` is needed: an empty array that the VM fills. The callback receives the **BotGuard response** string. BGU times VM calls out after 3000 ms.
4. Integrity token: `POST .../GenerateIT` (same two hosts, same headers as `Create`), JSON body `[requestKey, botguardResponse]`. The response is a JSON array `[integrityToken, estimatedTtlSecs, mintRefreshThreshold, websafeFallbackToken]`. The integrity token is base64url. POT posts to the jnn-pa host; BGU's examples post to the YouTube-hosted path. POT #285 shows `estimatedTtlSecs` 43200 (12 h).
5. Minter: element 0 of `webPoSignalOutput` is now a function. Call it with the integrity token decoded to bytes; it resolves to a mint function.
6. Mint: call the mint function with the UTF-8 bytes of the binding. It resolves to a `Uint8Array` of about 110-128 bytes. Base64url-encode it (BGU swaps `+/` for `-_` and keeps any `=` padding). That string is the PO token.
- BGU's error codes, useful as checkpoints: VM missing; VM init function missing; `PMD:Undefined` (no minter getter in the output); `APF:Failed` (the getter did not return a function); `YNJ:Undefined` / `ODM:Invalid` (mint returned nothing or not bytes). These strings come from YouTube's own minified code.

### 6.4 Lifetime, refresh and caching (POT server, current)
- The minter is cached until `now + estimatedTtlSecs`. A minted PO token is cached per binding for `TOKEN_TTL` hours (default **6**).
- The minter cache key is the proxy plus the InnerTube `remoteHost`, with a source comment that "POTs are IP-bound". POT #285: in production the integrity token is re-minted about every 12 h per egress IP.
- POT's HTTP fetch retries 3 times, 5 s apart.
- BGU's README: the web page keeps one minter for the page's lifetime and mints a new token per video. `mintRefreshThreshold` is returned but no source shows how it is used.

### 6.5 Cold-start token format (BGU `WebPoMinter.ts`, current; README says "encrypted using a simple XOR cipher")
- Bytes, in order:
  1. a tag byte `34` (0x22);
  2. a length byte (the length of everything after it);
  3. two random key bytes;
  4. a byte that is always 0;
  5. a client-state byte (default 1);
  6. the Unix time in seconds as 4 big-endian bytes;
  7. the UTF-8 binding.
- Every byte after the two key bytes is XORed with key byte (index mod 2), counting indexes from the start of the key. The whole thing is base64url-encoded.
- It works while SPS is 2, not once it becomes 3.
- BGU issue #31 (2025-10): generating a cold-start token for a visitorData that is too long produced "Content binding is too long". The length is a single byte.

### 6.6 What environment BotGuard needs (the WASM-plus-embedded-engine question)
- **BGU itself is environment-agnostic** but warns: "it does not bypass BotGuard; you still need a runtime environment that meets its checks" (BGU `README.md`).
- **Every example and POT use jsdom in Node.** They create a jsdom window for `https://www.youtube.com/` (with referrer and the BGU UA) and copy `window`, `document`, `location` and `origin` onto the global object. They also install jsdom's `navigator` on the global, but only when the global has none. For the homepage challenge they additionally set `yt = {config_: <ytcfg>}` on both the global and `window`. (BGU examples @f842f8a; POT `server/src/session_manager.ts` @26475e9)
- BGU issue #3 (LuanRT, 2024-09): "The only thing needed from jsdom is the document & window objects."
- **canvas**: POT depends on the native `canvas` npm package (jsdom's optional backend for `HTMLCanvasElement.getContext`). POT issue #208 (2026-03) proposed replacing jsdom with happy-dom to drop `canvas` and `cssstyle`. A follow-up (2026-04) reports that the closed draft PR found **happy-dom "wasn't generating tokens that work"**. BGU issue #3: happy-dom and jsdom also failed to deploy on Cloudflare Workers (module and polyfill problems, not BotGuard). No source shows whether BotGuard actually calls canvas or WebGL APIs.
- **navigator matters, and a wrong environment fails silently** (POT issue #285, opened 2026-11-22, unresolved at commit read):
  - About half of the integrity tokens minted by the Node and Deno servers were "bad": `GenerateIT` succeeded, but every GVS PO token derived from them got HTTP 403 (or worked for about 60 s and then got 403), for the whole 12 h TTL.
  - The reporters saw a perfect correlation with integrity-token length: **88 characters worked, 92 characters gave 403** (a 96-character one was also seen).
  - A commenter (2026-11-23) reports that always overwriting the global `navigator` with jsdom's makes every token usable. Node 21+ has a native global `navigator`, so the "only if absent" install never ran.
  - Implication stated by the reporters: a bad environment is not detectable at mint time; only googlevideo's response reveals it. They work around it by validating with a `Range: bytes=0-1` request and re-minting.
- **Static look at the interpreter** **[obs 2026-11-23]**: `//www.google.com/js/th/fNEcmtENESnIjy2AD1K5nOvWjmOOVLlCMb5QAaKlXs0.js` is a 64 KB obfuscated VM. Its text names `document` (8 times), `window` (4), `createElement` (2), `addEventListener` (9), `screen` (10), `performance` (3), `setTimeout` (3), `eval` (4), `atob`/`btoa`, `Symbol` and `Promise`. It does not name `navigator`, `canvas`, `getContext`, WebGL, `crypto` or `localStorage`. The browser APIs actually probed are decided by the encrypted bytecode `program`, so this list is a floor, not the full set.
- Host features BGU needs: a `Function` constructor or `eval` to load the interpreter, Promises, `setTimeout`, `TextEncoder`, `atob`/`btoa`, and `Uint8Array`. BGU 4.0.1 removed its `EventTarget` dependency.
- **No source reports running BotGuard under QuickJS, Boa, Hermes or any engine without a DOM.** Issue searches for quickjs, boa, hermes and bun in all three repos returned nothing. Deno works for POT (it has a Deno image), still with jsdom. A web search found only a project that runs BgUtils inside an iOS webview (a real browser) to avoid jsdom.

### 6.7 visitorData and VISITOR_INFO1_LIVE
- YTJS's default session bootstrap is `GET https://www.youtube.com/sw.js_data` with these headers:
  - `Referer: https://www.youtube.com/sw.js`;
  - `Cookie: PREF=tz=<Area.City>;VISITOR_INFO1_LIVE=<11-char id>`, where the id is random or decoded from a supplied visitorData.
- The reply starts with `)]}'`; strip 5 characters, then parse JSON. Element `[0][2]` is a ytcfg array: `[0][0]` is device info and `[1]` is the API key.
- Device-info indexes used by YTJS (current):

  | Index | Field |
  |---|---|
  | 0 | hl |
  | 1 | gl |
  | 3 | remoteHost |
  | 11 | deviceMake |
  | 12 | deviceModel |
  | 13 | **visitorData** |
  | 16 | WEB clientVersion |
  | 17 | osName |
  | 18 | osVersion |
  | 61 | config info (last item = appInstallData) |
  | 79 | time zone |
  | 86 | browserName |
  | 87 | browserVersion |
  | 103 | deviceExperimentId |
  | 107 | rolloutToken |

- Optional extra call: `POST /youtubei/v1/config?prettyPrint=false` with `{context}`. From the reply YTJS takes `responseContext.globalConfigGroup.{rawColdConfigGroup.configData, coldHashData, hotHashData}` into `context.client.configInfo`. (YTJS `Session.ts`)
- **Locally generated visitorData**: a protobuf with field 1 = an 11-char id (string) and field 5 = Unix seconds (int32), base64url-encoded and then URL-encoded (YTJS `protos/misc/params.proto`, `ProtoUtils.ts`). YTJS still falls back to it, but BGU 3.2.0 (2025-03-02) notes "**Locally generated Visitor IDs are no longer accepted**" (web example). YTJS's own `examples/browser` still mints against a local visitorData with BgUtils 3.x (**historical**).
- **[obs]** The `att/get` reply carries a 90-character `responseContext.visitorData`; the homepage `ytcfg` has `VISITOR_DATA`.

## 7. Login (YouTube.js)

YTJS `src/core/OAuth2.ts`, `src/utils/HTTPClient.ts`, `src/utils/Utils.ts`, `examples/auth/README.md`, current.

**TV device-code OAuth**
- Client identity:
  1. `GET https://www.youtube.com/tv` with `User-Agent: Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version`, `Referer: https://www.youtube.com/tv` and `Accept-Language: en-US`.
  2. Find the `<script id="base-js" src="...">` tag and fetch that script.
  3. In it, find `clientId:"<id>"` followed, after a comma and one more key, by a quoted secret.
- Device code: `POST https://www.youtube.com/o/oauth2/device/code`, JSON body:
  - `client_id`;
  - `scope: "http://gdata.youtube.com https://www.googleapis.com/auth/youtube-paid-content"`;
  - `device_id: <random UUIDv4>`;
  - `device_model: "ytlr::"`.

  The reply includes `device_code`, `user_code`, `verification_url`, `expires_in` and `interval`.
- Poll `POST https://www.youtube.com/o/oauth2/token` every `interval` seconds with `{client_id, client_secret, code: <device_code>, grant_type: "http://oauth.net/grant_type/device/1.0"}`. Errors handled: `authorization_pending` and `slow_down` (keep polling), `access_denied`, `expired_token`.
- Refresh: the same token URL with `grant_type: "refresh_token"`; the expiry is computed from `expires_in`.
- Revoke: `https://www.youtube.com/o/oauth2/revoke?token=<access_token>`.
- Requests then carry `Authorization: Bearer <access_token>`.
- Status: the README says "Due to recent changes by Google, OAuth2 sign-in only works when using the TV client. This method is no longer recommended - please use cookies instead." The code is still present (**current but discouraged**).

**Cookies**
- With a cookie string, YTJS reads `SAPISID` and sends:
  - `Authorization: SAPISIDHASH <ts>_<sha1hex("<ts> <SAPISID> https://www.youtube.com")>` (ts in Unix seconds);
  - `X-Goog-Authuser: <account index>`;
  - optionally `X-Goog-PageId` (acting on behalf of a brand account).

## 8. Captions
Not studied in YTJS for this file. POT lists a `subs` PO-token context, so caption (timedtext) requests can require a PO token (POT `getpot_bgutil.py`, current).

## 9. Live
- GV marks a SABR stream as live or premiere when the streaming URL's `source` param is `yt_live_broadcast` or `yt_premiere_broadcast`. The broadcast id is the part after the dot in the `id` param (GV `SabrStream.ts`, `urlUtils.ts`).
- For live streams without post-live DVR, GV starts at the live edge.
- GV runs a heartbeat every 10 s. An `OK` status continues; `LIVE_STREAM_OFFLINE` is tolerated for a 15 s grace period.
- A live stall makes GV assume the end of the stream.
- YTJS builds DASH manifests for post-live DVR (9.0.0). Nothing more on HLS live was studied here.

## 10. Failure modes and churn (dated)
- 2026-11-22/23: about half of the integrity tokens are silently bad: GVS 403 for the full 12 h TTL, linked to `navigator` (POT #285, open).
- 2026-08: tokens minted from `att/get` challenges were rejected by GVS, while `/player` still passed. Fix: use the homepage challenge plus `EVENT_ID` (POT 495a47f, BGU 3fa0402).
- 2026-07: POT #242, "occasionally not getting valid tokens", 403 after about 30 s. This is the same family as #285.
- 2026-08: MWEB went SABR-only (BGU #48).
- 2025-03: locally generated visitor ids were rejected (BGU 3.2.0).
- 2025-10: the JS evaluator was removed from YTJS (16.0.0).
- 2026-03: new n/sig variants needed support (YTJS 17.0.0).
- POT README (2026-10): a PO token "does not bypass IP-based login restrictions"; "Sign in to confirm you're not a bot" on datacenter IPs needs cookies.
- GV retries: a failed segment fetch is retried with exponential backoff from 500 ms up to 5 s, 10 attempts by default. A 30 s request timeout and 30 s stall detection apply (3 stalls maximum). On a network error GV switches to the next fallback host (below). On 403 when `expire` is past, it reloads the player response.

## 11. SABR / UMP protocol (googlevideo)

GV @0fb5b84 (`src/core/*`, `protos/video_streaming/*`), current unless noted. Field numbers are protobuf (proto2) tags.

**Inputs from `/player`**: `streamingData.serverAbrStreamingUrl` (its `n` param must be deciphered like any stream URL; YTJS does this), `playerConfig.mediaCommonConfig.mediaUstreamerRequestConfig.videoPlaybackUstreamerConfig` (base64 bytes), and `adaptiveFormats[]` (itag, lastModified, xtags, audioTrack id, isDrc ...). **[obs]** The URL carries `sabr=1` among `c, cps, ei, expire, fvip, id, ip, mn, n, pl, rqh, sig, source, sparams, xpc` and others.

**Request**: `POST <serverAbrStreamingUrl>&rn=<request number, incrementing>` with headers `content-type: application/x-protobuf`, `accept: application/vnd.yt-ump` and `accept-encoding: identity`. The body is a `VideoPlaybackAbrRequest`:
- 1 `client_abr_state` (`ClientAbrState`). GV sets at start:
  - `player_time_ms` (28);
  - `audio_track_id` (69);
  - `client_viewport_is_flexible=false` (22);
  - `drc_enabled` (46);
  - `media_capabilities` (38);
  - `enable_voice_boost` (76);
  - `sticky_resolution` = video height (21);
  - `elapsed_wall_time_ms` (36);
  - `time_since_last_action_ms` (39);
  - `time_since_last_seek` (29);
  - `visibility=2` (34);
  - `enable_sabr_host_fallback=true` (71);
  - `supports_change_type=false` (85);
  - `enabled_track_types_bitfield` (40): 0 = audio+video, 1 = audio only, 2 = video only.
- 2 `initialization_format_ids` (repeated FormatId), 3 `buffered_ranges` (repeated BufferedRange: format_id 1, start_time_ms 2, duration_ms 3, start_segment_index 4, end_segment_index 5, time_range 6), 5 `video_playback_ustreamer_config` (bytes), 16 `selected_audio_format_ids`, 17 `selected_video_format_ids`, 18 `selected_caption_format_ids`.
- 19 `streamer_context`:
  - 1 `client_info` (hl 1, device_make 12, device_model 13, client_name **as numeric id** 16, client_version 17, os_name 18, os_version 19);
  - 2 `po_token` (**raw bytes**);
  - 3 `playback_cookie` (echo of the last NextRequestPolicy cookie, which GV no longer decodes, 5.0.1);
  - 5 `sabr_contexts` (repeated {type 1, value 2});
  - 6 `unsent_sabr_contexts` (repeated int32 types).
- 21/24 SSAP (server-stitched ads) playback info.
- `FormatId` = {itag 1, last_modified 2, xtags 3}.

**Response**: must be `content-type: application/vnd.yt-ump`. The body is a stream of UMP parts, each `varint type, varint size, payload`. A part may straddle network chunks.

**UMP varint** (not protobuf's):
- The first byte's leading bits give the length: below 0x80, 1 byte; below 0xC0, 2; below 0xE0, 3; below 0xF0, 4; otherwise 5.
- For lengths 2 to 4, the value is the first byte's low 6/5/4 bits, then each next byte shifted left by 6, 5 or 4 bits plus 8 bits per byte already consumed. This is little-endian accumulation.
- For length 5, the first byte is ignored and the next 4 bytes are a little-endian uint32.

**Part ids** (GV `ump_part_id.proto`):
- `ONESIE_HEADER 10`, `ONESIE_DATA 11`, `ONESIE_ENCRYPTED_MEDIA 12`;
- **`MEDIA_HEADER 20`**, **`MEDIA 21`**, **`MEDIA_END 22`**;
- `CONFIG 30`, `LIVE_METADATA 31`, `LIVE_METADATA_PROMISE 33`, `LIVE_METADATA_PROMISE_CANCELLATION 34`;
- **`NEXT_REQUEST_POLICY 35`**;
- `USTREAMER_VIDEO_AND_FORMAT_METADATA 36`, `FORMAT_SELECTION_CONFIG 37`, `USTREAMER_SELECTED_MEDIA_STREAM 38`;
- **`FORMAT_INITIALIZATION_METADATA 42`**, **`SABR_REDIRECT 43`**, **`SABR_ERROR 44`**, `SABR_SEEK 45`, **`RELOAD_PLAYER_RESPONSE 46`**;
- `PLAYBACK_START_POLICY 47`, `ALLOWED_CACHED_FORMATS 48`, `START_BW_SAMPLING_HINT 49`, `PAUSE_BW_SAMPLING_HINT 50`, `SELECTABLE_FORMATS 51`, `REQUEST_IDENTIFIER 52`, `REQUEST_CANCELLATION_POLICY 53`, `ONESIE_PREFETCH_REJECTION 54`, `TIMELINE_CONTEXT 55`, `REQUEST_PIPELINING 56`;
- **`SABR_CONTEXT_UPDATE 57`**, **`STREAM_PROTECTION_STATUS 58`**, `SABR_CONTEXT_SENDING_POLICY 59`;
- `LAWNMOWER_POLICY 60`, `SABR_ACK 61`, `END_OF_TRACK 62`, `CACHE_LOAD_POLICY 63`, `LAWNMOWER_MESSAGING_POLICY 64`, `PREWARM_CONNECTION 65`, `PLAYBACK_DEBUG_INFO 66`, `SNACKBAR_MESSAGE 67`, `NETWORK_TIMING 68`, `CUEPOINT_LIST 69`, `STITCHED_REGIONS_OF_INTEREST 70`, `STITCHED_SEGMENTS_METADATA_LIST 71`, `PROBE_SUCCESS 72`.

**Handling of the key parts**
- `MEDIA_HEADER`:
  - fields: header_id 1, video_id 2, itag 3, lmt 4, xtags 5, segment_byte_range_start 6, compression_type 7, is_initialization_segment 8, segment_num 9, bitrate_bps 10, start_ms 11, duration_ms 12, format_id 13, segment_length_bytes 14, time_range 15, segment_lmt 16;
  - opens a segment identified by `header_id`;
  - GV ignores headers whose `video_id` is not the requested one.
- `MEDIA`: the first byte is the header_id; the rest is segment bytes to append.
- `MEDIA_END`: a single header_id byte; the segment is complete.
- `FORMAT_INITIALIZATION_METADATA`: video_id 1, format_id 2, end_time_ms 3, end_segment_num 4, mime_type 5, init_segment_range 6, index_range 7, end_time_ticks 9, end_time_timescale 10. It tells the client the format's total segments and duration.
- `NEXT_REQUEST_POLICY`: target audio/video readahead 1/2, max_time_since_last_request_ms 3, **backoff_time_ms 4** (GV waits this long before the next request), min readahead 5/6, **playback_cookie 7** (send back), video_id 8, send_initial_sabr_request 9 (added 2026-11-21).
- `SABR_REDIRECT` {url 1}: replace the streaming URL.
- `SABR_ERROR` {type 1, code 2}: fatal for GV.
- `RELOAD_PLAYER_RESPONSE`: holds reload params (field 1 with a token). The client must redo `/player` and take the new `serverAbrStreamingUrl` and ustreamer config. GV then clears its SABR contexts and SSAP state.
- `SABR_CONTEXT_UPDATE`: type 1, scope 2 (PLAYBACK 1, REQUEST 2, WATCH_ENDPOINT 3, CONTENT_ADS 4), value 3 (bytes), send_by_default 4, write_policy 5 (OVERWRITE 1, KEEP_EXISTING 2). GV stores updates by type and sends the active ones back in `streamer_context.sabr_contexts`. It lists the inactive types in `unsent_sabr_contexts`. `SABR_CONTEXT_SENDING_POLICY` (start 1, stop 2, discard 3: repeated types) toggles which types are active.
- `STREAM_PROTECTION_STATUS`: see 6.1.
- `SABR_SEEK`: a server-driven seek (including "seek to head" for live).
- After each response, unless the server seeked, GV moves its player time to the end of what is buffered and asks again with updated `buffered_ranges`. It stops when every enabled track has reached its end.

**Host fallback**: when the URL has `fvip` and `mn` (a comma-separated node list), the next host is built as `r<fvip>---<mn[k+1]>.<same domain suffix>` (`rr` prefix if the current host starts with `rr`) with `fallback_count=k+1` (GV `urlUtils.ts`).

**Practical facts for Nightglass**: SABR is a stateful POST protocol with protobuf bodies. mpv cannot speak it directly. The output is raw segments per format (fMP4/WebM), which a client must reassemble or serve. GV's own examples feed Shaka Player or a downloader.

---

## Open questions
1. Can BotGuard's interpreter and program run correctly in an embedded engine (QuickJS, Boa, ...) with hand-written `window`/`document`/`navigator`/`screen`/`performance` stubs? No source tries this. POT #285 shows that a wrong `navigator` produces integrity tokens that `GenerateIT` accepts but GVS later rejects. Correctness can therefore only be checked against googlevideo, and an "it mints" test proves nothing.
2. Which DOM, canvas or timing APIs does the bytecode program actually probe? The interpreter text names few APIs; the program is opaque. happy-dom reportedly failed where jsdom works. The difference is unknown, possibly canvas or CSS behaviour.
3. Is the 88-vs-92-character integrity-token length a reliable good/bad signal (POT #285)? If so, a plugin could re-mint at once instead of waiting for a 403.
4. What is the `T` value in `window.ytAtN({'R':..., 'T':...})`, and does it have to accompany the BotGuard response (e.g. sent to `GenerateIT`)? No source uses it.
5. Is the homepage-challenge requirement (`EVENT_ID`) universal or an experiment cohort (POT 495a47f says "when the session is enrolled in the experiment")? Does the visitorData used to fetch the homepage need to match the one later sent to `/player`?
6. Binding for GVS when logged out: video id (BGU README and #31) or visitorData (POT default)? The sources disagree, and SABR versus plain URLs may differ.
7. Which clients currently return plain (non-SABR) format URLs, and which of them need no PO token? YTJS does not document this. Observed: WEB is SABR-only; MWEB is reported SABR-only.
8. What does `mintRefreshThreshold` mean, and when should a long-lived minter refresh before `estimatedTtlSecs`?
9. Does the TV OAuth flow still yield tokens accepted by `/player` today? YTJS keeps it but calls it "limited" and "no longer recommended".
10. The plugin is single-shot (fresh instance per call, 5 min, 256 MiB). The VM, and therefore the minter, cannot persist across calls; only the integrity token (a string) can be stored. Whether a stored integrity token can be fed to a freshly re-snapshotted VM's minter is not settled by any source: BGU always uses the same VM instance's `webPoSignalOutput` that produced the snapshot.
