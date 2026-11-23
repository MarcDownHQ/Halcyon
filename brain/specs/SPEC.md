# Spec: a native YouTube plugin for Nightglass

A clean-room specification for a YouTube source plugin, written in Rust and built for `wasm32-wasip2`, against `nightglass:provider@0.3.0`. It is merged from the per-source notes in [`sources/`](sources/) and organised by topic. The notes contain facts only, and so does this file. There is no code and no pseudo-code from any source. Protocol facts (URLs, JSON paths, header values, client constants, public API keys) are given verbatim because they are facts about YouTube.

## Provenance

Each fact carries a short tag. Unless a fact is marked **(hist.)**, it is current at the commit below. `obs 11-23` marks something the source researchers observed with live, anonymous requests on 2026-11-23. Where the sources disagree, the disagreement is shown together with the fresher side.

| Tag | Source | Licence | Commit | Date |
|---|---|---|---|---|
| `ytdlp` | yt-dlp, `yt_dlp/extractor/youtube/` | Unlicense | `1507b26` | 2026-11-22 |
| `ejs` | yt-dlp/ejs (challenge solver), vendored as 0.8.0 | Unlicense | `d48c630` | 2026-10-26 |
| `wiki` | yt-dlp wiki (PO Token Guide, EJS) | - | `ee29d16` | 2026-09-27 |
| `st` | SmartTube `a0c319a8` + MediaServiceCore `bad6cb57` | MIT | | 2026-11-22 |
| `gj` | Grayjay YouTube plugin, v373 | Source First 1.1 (facts only) | `640bd86` | 2026-11-21 |
| `ytjs` | YouTube.js 18.3.0 | MIT | `77703fa` | 2026-11-18 |
| `bgu` | BgUtils 4.0.3 | MIT | `f842f8a` | 2026-10-23 |
| `gv` | googlevideo 5.0.1 (SABR/UMP) | MIT | `0fb5b84` | 2026-11-21 |
| `pot` | bgutil-ytdlp-pot-provider 2.0.2, plus issues #208/#242/#285 | GPL-3.0 (facts only) | `26475e9` | 2026-10-07 (issues to 2026-11-23) |
| `jse` | `sources/js-engine.md`: JS engines built and measured inside wasmtime 50 | - | - | 2026-11-23 |
| `contract` | `windows/wit/provider.wit` and README "Plugins", nightglass `origin/master` | - | - | read 2026-11-23 |

Observation counts: `ytdlp` made 8 requests, `st` 9 and `ytjs`/`bgu` 11, all anonymous on 2026-11-23 from residential IPs. `gj` made none.

---

## 1. Scope and non-goals

### What v1 offers

- **Channels as series.** Each configured YouTube channel becomes a `series`. Its uploads are `episode`s with `aired`, `duration` and `thumbnail`, and no season or number. Each configured playlist becomes a `series` too.
- **Live streams as channels.** A configured channel's current live stream, or a pinned 24/7 live channel, appears as a `channel` that is resolved at play time.
- **Single videos as movies.** This is optional: video URLs listed in the config become `movie`s.
- **Playback as a URL plus tracks.** A video-only adaptive URL is the `url`. The audio streams (one per language) and the subtitles become `tracks`, which mpv joins in an EDL. For live, the URL is an HLS manifest.
- **Everything resolved at play time.** `resolves-streams = true`. Format URLs expire after about 6 h (`expiresInSeconds` 21540, obs 11-23 [ytdlp, st]), so the catalog stores only opaque ids.
- **Storage** holds what must outlive the fresh per-call instance: channel-id lookups, visitor data, the player id, STS, the challenge caches, the last client that worked, and login tokens.

### Non-goals in v1

- **Search.** The contract has no search export [contract]. The endpoints are listed in 4.9 for later.
- **SABR/UMP playback.** It is a stateful POST protocol with protobuf bodies [gv]. mpv cannot play it, and the relay neither speaks it nor rewrites DASH [contract]. Formats without a URL are dropped.
- **DASH manifests**, which the relay does not rewrite [contract]. Live uses HLS only.
- **PO-token minting (BotGuard)** stays out of v1. It is a gated research milestone (section 7).
- **Writes to YouTube**: watch history, likes, playlist edits, watch-time reporting.
- **Comments, live chat, storyboards, chapters, Shorts feed, Music.**
- **DRM content** (`drmFamilies` present, or `streamingData.licenseInfos` with nothing playable) [ytdlp].

### Contract constraints that shape the design [contract]

- **A fresh instance per call**, with 256 MiB of memory and 5 minutes; `describe` gets 10 s and `filter` 30 s. No JS VM, minter or parsed player survives between calls; only `storage` text values do.
- **Storage limits:** keys up to 256 bytes, values up to 1 MiB, and 16 MiB in total per plugin. Every source of the plugin shares the same keys. During `describe`, `get` returns nothing and `set` fails.
- **HTTP:** `http.send` only, http(s), at most 128 MiB per response. Any status counts as a response; only a failed connection is an error. A request may carry a browser TLS/HTTP2 handshake (`chrome`, `edge`, `firefox`, `safari`).
- **Relay:** it relays http(s) only and rewrites HLS, not DASH. Without `filter`, it streams responses through with `Range` passed along. With `filter`, it reads every response whole, without `Range`, so a byte-range HLS playlist cannot be filtered. It retries a 403, 502 or 503 once.
- **Stream URLs** must stay on the network (http, https, rtmp, rtsp, rtp, udp, srt, mms). No `data:` or `edl://` URL can be returned.

---

## 2. Mapping to the contract

### 2.1 `describe`

- **`id`:** `youtube`. **`name`:** `YouTube`.
- **`resolves-streams`:** `true`.
- **`presets`:** optional; one example preset with a public channel is enough.
- **No storage here:** `describe` cannot read storage [contract], so it must not depend on anything cached.

### 2.2 Source config fields

The contract offers only `text`, `secret`, `url`, `location` and `mac` field kinds, with no boolean or list kind [contract]. Lists are therefore free text, split on newlines, commas or whitespace.

| key | kind | required | Content |
|---|---|---|---|
| `channels` | text | no | `@handle`, `https://www.youtube.com/@handle`, `/channel/UC…`, `/c/<name>`, `/user/<name>`, or a bare `UC…` id. Each becomes a series of its uploads. |
| `playlists` | text | no | Playlist URLs (`…/playlist?list=<id>`) or bare ids. Each becomes a series. |
| `live` | text | no | Channels (any form above) or watch URLs to expose as live TV channels. |
| `videos` | text | no | Watch URLs or ids, exposed as movies. |
| `tabs` | text | no | Which uploads go into a channel series: any of `videos`, `streams`, `shorts`. The default is `videos streams`. |
| `max-episodes` | text | no | The number of episodes per series, default `100`. |
| `quality` | text | no | The maximum video height, for example `1080`; empty means best. |
| `sign-in` | text | no | Empty for anonymous use; `tv` starts the device-code login (section 8). |
| `account` | text | no | The brand account to act as, by name or handle, after sign-in. |
| `cookies` | secret | no | An alternative to `tv`: a browser `Cookie` header string (section 8.4). |

- **Validation:** at least one of `channels`, `playlists`, `live` and `videos` must be set; otherwise `load-catalog` returns `setting("channels")`.
- **Bad entries:** an entry that cannot be resolved (an unknown handle, a missing playlist) returns `setting("<key>")` only when *every* entry of that field fails. Otherwise it is logged and skipped.

### 2.3 What becomes what

| YouTube object | Contract record | `id` | `stream` | Notes |
|---|---|---|---|---|
| Configured channel | `series` | `ch:<UCid>` | - | `name` is the channel title, `poster` the avatar, `group` "YouTube". |
| Configured playlist | `series` | `pl:<playlistId>` | - | `name` is the playlist title, `poster` the first item's thumbnail, `group` "YouTube playlists". |
| Upload of a channel / playlist item | `episode` | `<videoId>` | `v:<videoId>` | `aired`, `duration` and `thumbnail` as in 4.8. `season` and `number` are absent: a dated broadcast [contract]. |
| Live stream now on air (from a configured `live` or `channels` entry) | `channel` | `live:<videoId>` | `v:<videoId>` | `name` is the channel title plus the stream title. `logo` is the avatar. `group` is "YouTube live". `catchup-window` is absent in v1 (section 9). |
| Pinned 24/7 live channel (`live` entry naming a channel) | `channel` | `chlive:<UCid>` | `chlive:<UCid>` | The live video is resolved at play time, because a channel's live video id changes between broadcasts. |
| Single video | `movie` | `<videoId>` | `v:<videoId>` | `duration` comes from the player or listing; `plot` is the short description. |

- **Stable ids:** keep them stable across refreshes; the app keys favourites and progress on them.
- **Opaque `stream` values:** they are allowed because `resolves-streams` is set [contract]. The plugin reads them back in `stream-url`, `movie-url` and `episode-url`.
- **`programmes`:** a live channel may carry one entry: the stream title, starting at `liveBroadcastDetails.startTimestamp` [st, ytdlp]. That gives the guide something to show. This is optional.
- **`account`:** when signed in, `catalog.account.status` carries the sign-in state and any pending device code (section 8.2).

### 2.4 What the play-time calls return

`movie-url`, `episode-url` and `stream-url` share one resolver:

1. **Parse the `stream`.**
   - `v:<id>`: use the id as given.
   - `chlive:<UC>`: GET `https://www.youtube.com/channel/<UC>/live`. The page redirects to, or embeds, the live video. If it does not, the channel is "not live" [ytdlp], and the call returns `bad-response("not live")`.
2. **Run the client strategy** of section 3.3 to get a player response with usable URLs.
3. **Build the `playback`.**
   - **VOD:**
     - `url` is the chosen video-only adaptive format URL.
     - `tracks` holds, in order: the preferred audio, then other audio languages, then subtitles. Each track has `language` (ISO 639) and `title`.
     - `headers` holds `User-Agent` of the client that produced the URLs (matching the URL's `c=` parameter) [gj]. For the web-family `c=` values it also holds `Origin`/`Referer: https://www.youtube.com` [gj, ytjs].
     - If no video-only format is usable, the fallback is a muxed progressive format (itag 18), then the client's `hlsManifestUrl` as a single URL.
   - **Live:** `url` is the HLS `hlsManifestUrl`, with any `/n/<v>/` path segment solved (section 6), and `tracks` is empty.
   - **`relay`:** `none` by default. Section 10 says when to switch it on: `relay{browser: none, filter: false}` to have the app retry random 403s once, and `filter: true` only for the live n-rewrite case in 9.4.
4. **`catchup-url`:** returns `http(501)` in v1, a "not supported" status. Section 9.3 covers what a later version can do.

---

## 3. Innertube clients

### 3.1 Request plumbing common to all clients

- **Endpoint:** `POST https://www.youtube.com/youtubei/v1/<endpoint>?prettyPrint=false` [ytdlp, st, ytjs].
  - **API key:** `ytdlp` and `ytjs` send **no** `key=` parameter. `st` sends `key=AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8` on anonymous calls only, and `gj` sends it to `https://youtubei.googleapis.com/youtubei/v1/player` for non-WEB clients. All of these are current, so the key is optional. Prefer no key, and keep the public key as a fallback.
  - **`t=`:** `st` and `gj` also add `t=<12 random chars>` on some endpoints.
- **Headers** [ytdlp, st, ytjs]:
  - `Content-Type: application/json`;
  - `X-YouTube-Client-Name: <numeric id>` and `X-YouTube-Client-Version: <clientVersion>`;
  - `User-Agent: <client UA>`;
  - `Origin: https://www.youtube.com`;
  - `X-Goog-Visitor-Id: <visitorData>`;
  - `Accept-Language: en-US` (or `en-US, en;q=0.9` [gj]).
  - Avoid `br` in `Accept-Encoding` if the HTTP layer cannot decode it. `st` asks for `gzip, deflate`.
- **`context.client`:** `clientName`, `clientVersion`, the client's device fields from the table, `hl:"en"`, `gl:"US"`, `timeZone:"UTC"`, `utcOffsetMinutes:0` and `visitorData` [ytdlp, gj].
  - Forcing `en` keeps the relative-date and duration text parseable [gj, ytdlp]. `st` sends `acceptLanguage`/`acceptRegion` instead of `hl`/`gl`, which also works.
- **`context.user`:** `{"lockedSafetyMode": false}` [gj, st].
- **Body flags:** `contentCheckOk: true` and `racyCheckOk: true` at the top level [ytdlp, st, ytjs]. `gj` sends `false` first and `true` only on a logged-in retry. It also has a misspelled `racyCheckOn`, probably ignored.
- **`cpn`:** a 16-character random client playback nonce from a 64-character URL-safe alphabet, **new for every `/player` call** [st, gj, ytjs]. `st` calls it a PO-token/attestation fix (2026-11-14).

**visitorData is mandatory.**
- **Without it:** obs 11-23 [ytdlp], `VISIONOS` and `TVHTML5` 5.x without visitor data both returned `LOGIN_REQUIRED` "Sign in to confirm you're not a bot".
- **With it:** the same `VISIONOS` call with `X-Goog-Visitor-Id` and `context.client.visitorData` taken from a prior anonymous `/player` response's `responseContext.visitorData` returned `OK`.
- **Ways to get one,** all current:
  - `POST /youtubei/v1/visitor_id` with a WEB context, reading `responseContext.visitorData` [st, gj]. `st` sends `Cookie: SOCS=CAE=`, a Firefox UA and client version `2.20250213.05.00`.
  - Any anonymous Innertube response's `responseContext.visitorData` [ytdlp].
  - The homepage or `/tv` page `ytcfg` `VISITOR_DATA` [st, gj, ytdlp].
  - `/att/get` `responseContext.visitorData` (90 chars, obs 11-23 [bgu]).
- **Locally generated visitor ids are rejected** since 2025-03 [bgu]. Do not synthesise one.
- **Caching:** store visitorData persistently with a TTL of about 24 h. Refresh it on a bot-check reply. `st` says visitorData is tied to player versions and can be invalidated.

### 3.2 The merged client table

Abbreviations:
- **JS** = needs the player script for `n` and/or `sig`.
- **PO** = whether a PO token is needed for GVS (media) / `/player`.
- **Login** = cookies (C) or TV OAuth (O) accepted.
- **HLS** = returns `hlsManifestUrl`.
- **id** = the `X-YouTube-Client-Name` value.

| Client | `clientName` / `clientVersion` (source, date) | id | User agent | Context extras | JS | PO (GVS / player) | Login | HLS | Status (date) |
|---|---|---|---|---|---|---|---|---|---|
| **VISIONOS** | `VISIONOS` / `1.02` [ytdlp, st, ytjs]; `1.04` [gj 11-21] | 101 (`gj` does not send the header) | `Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15` [ytdlp]; `gj`: `com.google.visionos.youtube/1.04 (RealityDevice17,1; U; CPU visionOS 26_6_0 like Mac OS X; US) gzip` | `deviceMake:"Apple"`, `deviceModel:"RealityDevice17,1"`, `osName:"visionOS"`, `osVersion:"26.5.23O471"` [ytdlp] or `"26.6.0"` [gj] | **No.** obs 11-23 [ytdlp]: 16 adaptive formats with plain `url`, `sig` present, **no `n`** | none / none [ytdlp policy, gj] | No | **Yes** (obs 11-23) | **Current, working.** In `ytdlp` defaults since 2026-07-20 and the only client used without a JS runtime; `gj` #2. "Made for kids" is unavailable. `st` says it "returns no URL formats" (older note); the `ytdlp` obs of 11-23 is fresher. No itag 18. |
| **IOS_PRODUCER** | `IOS_PRODUCER` / `0.147.945` [ytdlp, gj] | 92 | `com.google.ios.youtubeproducer/0.147.945 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)` | `deviceMake:"Apple"`, `deviceModel:"iPhone16,2"`, `osName:"iPhone"`, `osVersion:"18.3.2.22D82"` | No | none / none [ytdlp, gj] | No (`gj` keeps it anonymous on purpose) | Yes (live, `gj` reads it) | **Current, new.** `ytdlp` fallback since 2026-11-20 (made-for-kids); `gj` #3 since 2026-11-21. **H.264 and mp4a only** [ytdlp]. |
| **TVHTML5 5.x "Samsung"** | `TVHTML5` / `5.20260707` [ytdlp, st, gj] | 7 | `Mozilla/5.0 (SMART-TV; Linux; Tizen 2.4.0) AppleWebKit/538.1 (KHTML, like Gecko) Version/2.4.0 TV Safari/538.1` | `deviceMake:"Samsung"`, `deviceModel:"UKS9800"`, `osName:"Tizen"`, `osVersion:"2.4.0"`; `gj` also sends `userAgent` in the context | **Yes**: `n`; `sig` when `signatureCipher` appears. Sends a **5-digit** STS (non-TCL player) | none [ytdlp]; `st` flags it for a **TV token** (disagreement; both read 2026-11-22, unresolved) | C [ytdlp], O [st] | Yes (live, `gj`) | **Current.** `ytdlp` default #1 since 2026-10-10 (logged-out and logged-in); `gj` #1 for VOD since 2026-11-13; `st` #2. Never independently observed. |
| **TVHTML5 5.x "downgraded"** | `TVHTML5` / `5.20260901` [st] or `5.20260707` [ytdlp `tv_downgraded`] | 7 | `Mozilla/5.0 (DirectFB; Linux x86_64) Cobalt/4.13031-qa (unlike Gecko) Starboard/1` [st]; `Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version` [ytdlp] | none | **Yes**: obs 11-23 [st]: every adaptive format had a plain `url` with `sig` present, but `n` unsolved gave **HTTP 403** on a `range=0-9999` GET | none / none [st, ytdlp] | C, O | not reported | **Current** in `st` (#5); replaced in `ytdlp` defaults by `tv_samsung` on 2026-10-10, with no reason given. obs 11-23: `OK`, with no `serverAbrStreamingUrl`. |
| **TVHTML5 7.x** | `TVHTML5` / `7.20260901.15.00` [st, Fire TV UA]; `7.20260707.07.00` [ytdlp]; `7.20260311.12.00` [ytjs] | 7 | Fire TV Cobalt UA [st]: `Mozilla/5.0 (Linux armeabi-v7a; Android 7.1.2; Fire OS 6.0) Cobalt/22.lts.3.306369-gold (unlike Gecko) v8/8.8.278.8-jit gles Starboard/13, Amazon_ATV_mediatek8695_2019/NS6294 (Amazon, AFTMM, Wireless) com.amazon.firetv.youtube/22.3.r2.v66.0`; `ytdlp`: `Mozilla/5.0 (ChromiumStylePlatform) Cobalt/25.lts.30.1034943-gold (unlike Gecko), Unknown_TV_Unknown_0/Unknown (Unknown, Unknown)` | `st` on browse: `tvAppInfo{appQuality:"TV_APP_QUALITY_FULL_ANIMATION", zylonLeftNav:true}`, `webpSupport:false`, `animatedWebpSupport:true`; for `/player` `tvAppInfo.livingRoomPoTokenId` when there is a token | **Yes**, through the **TCL** player variant with an **8-digit** STS [st] | `ytdlp`: none. `st`: TV (living-room) session token | **O** (the only client family that takes the OAuth token [st, ytjs]), C | Yes (live) | **Partly usable.** obs 11-23 [st], anonymous, no token: adaptive formats had **no URL** (SABR-only); only itag 18 had a `url` (with `sig` and `n`). The `ytdlp` wiki adds that all formats are DRM'd without cookies for some sessions; deleting `configInfo.appInstallData` helps when logged out. The `st` default for login. |
| **IOS** | `iOS` / `21.26.4` [ytdlp, st]; `20.11.6` [ytjs]; `19.14.3` [gj, hist.] | 5 | `com.google.ios.youtube/21.26.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)` | as IOS_PRODUCER | No | https: **required** unless a player token was sent; HLS: **required "30 seconds in"** [ytdlp] | No | Yes (live and VOD) | **Unusable without a token** (iOSGuard tokens are not web tokens [wiki]). `gj` disabled it. |
| **WEB** | `WEB` / live from the homepage `INNERTUBE_CLIENT_VERSION` (obs 11-23: `2.20261119.07.00` [ytjs]); constants `2.20260708.00.00` [ytdlp], `2.20260907.06.00` [st], `2.20260623.01.00` [ytjs] | 1 | desktop Chrome | the page's `INNERTUBE_CONTEXT` | Yes | GVS **required** [ytdlp] | C | No (`web_safari` HLS only for "logged-in or trusted" sessions since 2026-07 [ytdlp]) | **SABR-only for media**: obs 11-23 [ytdlp, ytjs], 14 adaptive formats without `url`/`signatureCipher`. Useful only for metadata, captions and the player id. A wrong STS or version gives `UNPLAYABLE` "The page needs to be reloaded." [ytjs obs]. |
| **MWEB** | `MWEB` / `2.20260708.05.00` [ytdlp]; `2.20260907.05.00` [st] | 2 | iPad Safari 16.6 UA ending `,gzip(gfe)` | none | Yes | GVS required | C | No | **SABR-only since 2026-08** [bgu #48]. |
| **WEB_EMBEDDED_PLAYER** | `2.20260708.00.00` [ytdlp]; `2.20260908.01.00` [st]; `1.20260206.01.00` [ytjs] (disagreement in major version; `ytdlp`/`st` fresher) | 56 | Safari 15.5 Mac UA | `clientScreen:"EMBED"`, `context.thirdParty.embedUrl` = any non-YouTube URL; `encryptedHostFlags` from the embed page | Yes | web policy since 2026-11-14 [ytdlp] | C | Yes (lower-quality audio) | Age-gate workaround for embeddable videos; `st` calls it "not working". Low value. |
| **WEB_CREATOR** | `1.20260708.06.00` [ytdlp]; `1.20241203.01.00` [st, ytjs] | 62 | - | - | Yes | requires a token | **login required for every video** | - | Logged-in age-gate workaround only. |
| **TVHTML5_SIMPLY** | `1.0` | **74** [st, ytjs] vs **75** [ytdlp] (disagreement) | - | - | Yes | GVS required [ytdlp] | No | - | `st` disabled it ("hangs?"). Skip. |
| **TVHTML5_SIMPLY_EMBEDDED_PLAYER** | `2.0` | 85 [st, ytjs] | Chromecast UA [gj] | `clientScreen:"EMBED"`, `thirdParty.embedUrl` | Yes | web token family | - | - | **Historical** age-gate bypass [gj, st]. |
| **ANDROID** | `21.26.364` [ytdlp, st]; `21.03.36` [gj, ytjs] | 3 | `com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip` | `androidSdkVersion:30`, `osName:"Android"`, `osVersion:"11"` | No | GVS required unless a player token (DroidGuard) was sent | No | - | **Unusable** (DroidGuard). `gj` disabled it. |
| **ANDROID_VR** | `1.65.10` | 28 | `com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip` | Oculus Quest 3 fields | No | - | No | - | **Dead**: "since 2026.08.17, ALL formats (including live HLS and itag 18) are 403'd" [ytdlp]; removed from the defaults 2026-08-18 [ytdlp], disabled 2026-08-22 [gj]. |
| WEB_REMIX | `1.20260707.12.00` [ytdlp, st] | 67 | - | host `music.youtube.com` | Yes | GVS required | C | - | Out of scope (Music). The only client that accepts WAA `Create` challenges [bgu]. |

Other client facts:
- **Geo fix:** a WEB `/player` body with `params:"CgIQBg%3D%3D"` bypasses geo-blocking [st]. It is useless for media because WEB is SABR-only.
- **HTML pages and client versions:** fetch pages with the UA of the client whose context you want [ytdlp]. A page's `ytcfg` gives the live `INNERTUBE_CONTEXT` and version for WEB.
- **TV DRM workaround:** for `tv` when logged out, delete `context.client.configInfo.appInstallData` [ytdlp].
- **Rental content:** when a response is unplayable but has `playabilityStatus.errorScreen.playerLegacyDesktopYpcTrailerRenderer.trailerVideoId`, play the trailer instead [st, ytdlp].
- **Mismatched responses:** a player response whose `videoDetails.videoId` differs from the request must be discarded. If every response is like that, the IP is likely blocked [ytdlp].

### 3.3 Recommended client strategy

The principle: get usable URLs with no PO token and as little JS as possible, and confirm them with one cheap probe before handing them to mpv. A 403 from googlevideo is silent until mpv hits it, which is why both `gj` and `st` probe or rotate.

**VOD, anonymous.** Try in order and stop at the first client whose formats pass the probe:
1. **VISIONOS** (no JS, no token, plain signed URLs, HLS available). It is the only path that works without a JS engine, so it is milestone 1.
2. **IOS_PRODUCER** (no JS, no token, H.264/AAC only). It is the fallback for made-for-kids videos, which VISIONOS refuses [ytdlp].
3. **TVHTML5 5.x Samsung**, which needs `n`, and `sig` if ciphered: the `ytdlp` and `gj` first choice, used once the JS engine exists.
4. **TVHTML5 5.x downgraded**, which needs `n`; obs 11-23 gave plain adaptive URLs.
5. **TVHTML5 7.x itag 18 only** (360p muxed, needs `n`). This is a last resort; `st` uses it as its "480p fallback" pass.
6. Otherwise return the most informative `playabilityStatus` as an error (section 11).

Rationale and notes:
- `ytdlp` (with JS) and `gj` both put TV Samsung first. The likely reason is format coverage, since VISIONOS lacks itag 18 and made-for-kids videos, but neither source says why. We put VISIONOS first anyway: it skips the 3 MB player download and the JS run. **The live probe (section 12) should confirm this order.**
- **The probe** (adapted from `gj`): take the chosen audio format, compute its length from `contentLength` or the `clen` URL parameter, and request a small range near the end (`gj` uses length-150 to length-50). A 2xx means the client's URLs are good. On 403, try the next client. Probe the audio URL, because audio and video URLs share the same signature and `n`.
- **Persist the last client that worked** and start from it next time [st]. Re-test from the top after 24 h so a recovered client gets its place back.
- **Batching:** clients can be queried one at a time, which saves requests (the guest budget is about 300 videos/h [wiki]), or several at once. `http.send` is synchronous, so one at a time is the natural fit.

**Live, anonymous.** Take `hlsManifestUrl` from VISIONOS, then IOS_PRODUCER, then TVHTML5 5.x Samsung (the `gj` order is TV Samsung, VISIONOS, IOS_PRODUCER). Solve any `/n/<v>/` path segment. Never use IOS for live: it needs a token 30 s in [ytdlp].

**Logged in.** OAuth is accepted only by TV clients [st, ytjs]; cookies by web and TV clients, not by VISIONOS, IOS* or ANDROID* [ytdlp].
- **Public videos:** keep using the anonymous VOD order. Login does not help there and only adds risk to the account.
- **Videos that need login** (`LOGIN_REQUIRED` for age, members-only or private; `ERROR`/`UNPLAYABLE` for made-for-kids when the account allows it), and the logged-in library: use TVHTML5 5.x Samsung with OAuth or cookies, then TVHTML5 5.x downgraded, then TVHTML5 7.x (itag 18 unless URLs come back).
- **Premium formats** (itags `616`, `235` [ytdlp]; `paygatedQualitiesMetadata.restrictedAdaptiveFormats` [st]) need TV with login. `ytdlp` uses `web_creator` first for Premium, which needs a PO token, so we cannot.

---

## 4. Catalog

All catalog calls are anonymous unless the source is signed in. Force `hl=en`, `gl=US` so the text fields parse. Send `Cookie: PREF=hl=en&gl=US` on HTML fetches [gj, ytdlp]. `ytdlp` also sends `SOCS=CAI` to accept consent.

### 4.1 Resolving a handle or URL to a channel id

Accept these forms [ytdlp]:
- a handle: `@` plus 3 to 30 of `[A-Za-z0-9_.-]`;
- a channel id: `UC` plus 22 of `[A-Za-z0-9_-]`;
- URLs `/channel/<id>`, `/c/<name>`, `/user/<name>`, `/@handle`, or a bare `/<name>`, unless the name is reserved (`watch`, `shorts`, `playlist`, `feed`, `results`, `live`, `embed`, `hashtag`, …).

Resolution order:
1. **The id is in the input** (`/channel/UC…` or a bare `UC…`): use it.
2. **Otherwise, the HTML page.** GET `https://www.youtube.com/<form>/videos`. This page also serves as the first uploads page in 4.3. Parse `ytInitialData` and read the id:
   - `metadata.channelMetadataRenderer.externalId` [ytdlp, gj];
   - fallback: the id inside `metadata.channelMetadataRenderer.channelUrl` [ytdlp];
   - old layout: `header.c4TabbedHeaderRenderer.channelId` [gj];
   - simplest fallback: the `<link rel="canonical" href="https://www.youtube.com/channel/UC…">` tag [st].

   Name: `metadata.channelMetadataRenderer.title`. Handle: `vanityChannelUrl` [gj, ytdlp]. Avatar: not documented by the sources (see 13).
3. **API fallback.** `POST /youtubei/v1/navigation/resolve_url` with `{"context":…, "url":"<full youtube URL>"}`, reading `endpoint.browseEndpoint.browseId` (and `params`) [ytdlp, ytjs]. `gj` has this path complete but switched off since 2026-09-04, with no reason given.
4. **Failure signs.**
   - A selected tab with `tabIdentifier == "FEwhat_to_watch"` means the channel does not exist [ytdlp].
   - `alerts[].alertRenderer.type == "ERROR"` means the channel is unavailable, with that alert's text [gj].
   - A `navigateAction` in `onResponseReceivedActions` is a regional redirect to follow [ytdlp, st, ytjs].
5. **Cache** `handle → UCid` persistently with no TTL. Handles rarely move.

**Extracting `ytInitialData` from HTML** [gj, ytdlp]:
- It is the object after `var ytInitialData = ` (or `ytInitialData = `) in a script tag, or the content of `<script id="yt-initial-data">`.
- It is sometimes a single-quoted JS string with `\xNN` escapes, which must be decoded before JSON parsing.
- `ytcfg` is the `ytcfg.set({…})` call whose JSON contains `INNERTUBE_CONTEXT`.
- **EU consent:** if the HTML holds a form posting to `https://consent.youtube.com/save`, either send `SOCS=CAI` up front [ytdlp] or POST the refusal form [gj].

### 4.2 Channel tabs and `params`

Tab `params` for `POST /youtubei/v1/browse` with `{"context":…, "browseId":"UC…", "params":"<p>"}`:

| Tab | `params` | Source |
|---|---|---|
| Videos | `EgZ2aWRlb3PyBgQKAjoA` | gj, st (they agree) |
| Shorts | `EgZzaG9ydHPyBgUKA5oBAA%3D%3D` | gj, st |
| Live (streams) | `EgdzdHJlYW1z8gYECgJ6AA%3D%3D` | gj, st |
| Playlists | `EglwbGF5bGlzdHPyBgoKCEIGCgIQaCIA` [gj] vs `EglwbGF5bGlzdHPyBgQKAkIA` [st] | disagreement; both current |
| Home | `EghmZWF0dXJlZPIGBAoCMgA%3D` [st]; none [gj] | |

`ytdlp` and `ytjs` hard-code **no** params. They read each tab's own `tabRenderer.endpoint.browseEndpoint.params` from the response and pick the tab by the last path segment of `endpoint.commandMetadata.webCommandMetadata.url` (`videos`, `streams`, `shorts`, `playlists`, …). That is the self-healing approach.

**Recommendation:**
- **First page:** use the HTML tab page (`/videos`, `/streams`, `/shorts`). It costs one request and carries the id and title too, which is the current `gj` and `ytdlp` path.
- **Following pages:** use `/browse` continuations.
- **When the hard-coded params go stale:** if a browse with them returns a tab that is not `selected`, browse the channel without params, take that tab's own `params`, cache them persistently, and retry [gj].

**Response layout of a channel tab** (WEB):
- **Tabs** are at `contents.twoColumnBrowseResultsRenderer.tabs[].tabRenderer` (or `expandableTabRenderer`); the one with `selected: true` is the current tab.
- **Content** is under `tab.content.richGridRenderer.contents[]`, or under `tab.content.sectionListRenderer.contents[].itemSectionRenderer.contents[]` [ytdlp, gj].
- **Grid entries:**
  - `richItemRenderer.content` holds one item;
  - `richSectionRenderer.content.richShelfRenderer` holds a shelf;
  - `continuationItemRenderer` holds the next-page token.

### 4.3 Item renderers (current first, then fallbacks)

**`lockupViewModel`** (the current main layout) [ytdlp, gj, st]:
- **Video id:** `contentId` when `contentType == "LOCKUP_CONTENT_TYPE_VIDEO"` [ytdlp], or `rendererContext.commandContext.onTap.innertubeCommand.watchEndpoint.videoId` [gj].
- **Type:** `LOCKUP_CONTENT_TYPE_PLAYLIST` and `LOCKUP_CONTENT_TYPE_PODCAST` are also possible.
- **Title:** `metadata.lockupMetadataViewModel.title.content`.
- **Thumbnails:** `contentImage.thumbnailViewModel.image.sources[]` (`url`, `width`, `height`). For playlists: `contentImage.collectionThumbnailViewModel.primaryThumbnail.thumbnailViewModel.image.sources[]`.
- **Duration text** ("h:mm:ss"): `contentImage.thumbnailViewModel.overlays[].thumbnailBottomOverlayViewModel.badges[].thumbnailBadgeViewModel.text`, or `…thumbnailOverlayBadgeViewModel.thumbnailBadges[].thumbnailBadgeViewModel.text`. The text "upcoming" means an upcoming stream.
- **Live:** a badge text `LIVE`, a badge style `THUMBNAIL_OVERLAY_BADGE_STYLE_LIVE`, or a row badge `badgeViewModel.badgeStyle == "BADGE_STYLE_TYPE_LIVE_NOW"`.
- **Views and date:** `metadata.lockupMetadataViewModel.metadata.contentMetadataViewModel.metadataRows[].metadataParts[].text.content`, as free text ("N views", "N watching", "3 days ago", "Streamed 2 weeks ago", "Scheduled for …", "Premieres …"). `ytdlp` uses the last row whose last part has an `accessibilityLabel`: part 0 is the views, the last part the relative time.
- **Members-only:** `metadataRows[].badges[].badgeViewModel.badgeStyle == "BADGE_MEMBERS_ONLY"`. Skip these unless signed in.

**`shortsLockupViewModel`** [ytdlp, gj]:
- **Id:** `onTap.innertubeCommand.reelWatchEndpoint.videoId`.
- **Title:** `overlayMetadata.primaryText.content`, or parsed from `accessibilityText`.
- **Thumbnail:** `thumbnailViewModel.thumbnailViewModel.image.sources[]`.
- No date and no duration.

**Older renderers** (`videoRenderer`, `gridVideoRenderer`, `playlistVideoRenderer`, `compactVideoRenderer`, `reelItemRenderer`) [ytdlp, gj, st]:
- **Id and title:** `videoId`; `title` (`runs[]` or `simpleText`).
- **Duration:** `lengthSeconds` (playlist items), else `lengthText.simpleText`, else `thumbnailOverlays[].thumbnailOverlayTimeStatusRenderer.text`.
- **Date:** `publishedTimeText.simpleText`.
- **Thumbnails:** `thumbnail.thumbnails[]`.
- **Live:** `badges[].metadataBadgeRenderer.style == "BADGE_STYLE_TYPE_LIVE_NOW"`, or the overlay `style == "LIVE"`.
- **Upcoming:** `upcomingEventData.startTime` (Unix seconds).
- **Short:** the overlay `style == "SHORTS"`, or a `/shorts/` navigation URL.
- **Skip:** `adSlotRenderer`.

**TV `tileRenderer`** (only if the catalog uses the TV client, as `st` does) [st]:
- **Title:** `metadata.tileMetadataRenderer.title`.
- **Id:** `onSelectCommand.watchEndpoint.videoId`.
- **Thumbnail:** `header.tileHeaderRenderer.thumbnail.thumbnails[]`.
- **Duration or badge:** `header.tileHeaderRenderer.thumbnailOverlays[].thumbnailOverlayTimeStatusRenderer.text`, with `style` `LIVE`, `UPCOMING` or `SHORTS`.
- There is no structured date. Not needed for v1.

### 4.4 Continuations

**Where the token is** [ytdlp, gj, st]. Try these in order:
1. the last item's `continuationItemRenderer.continuationEndpoint.continuationCommand.token`;
2. `continuationItemViewModel.continuationCommand.innertubeCommand.continuationCommand.token` (newer);
3. a token nested under `commandExecutorCommand.commands[]`;
4. legacy `continuations[0].nextContinuationData.continuation` or `reloadContinuationData.continuation`.

**Request body:**

```json
{"context": {"client": {"clientName": "WEB", "clientVersion": "<from ytcfg>", "hl": "en", "gl": "US", "visitorData": "<vd>"}},
 "continuation": "<token>",
 "clickTracking": {"clickTrackingParams": "<ctp, only if present>"}}
```

POST it to `/youtubei/v1/browse`. Send `X-Goog-Visitor-Id` with the **latest** `responseContext.visitorData` from the previous page; `ytdlp` says this prevents an infinite loop. `gj` copies `responseContext.visitorData` into the context and retries once when a page comes back empty.

**Where the items are in the response:** `onResponseReceivedActions[].appendContinuationItemsAction.continuationItems` (or `reloadContinuationItemsCommand`, or `onResponseReceivedCommands`). The legacy form is `continuationContents.{gridContinuation, playlistVideoListContinuation, sectionListContinuation, itemSectionContinuation}`.

**Stopping:** stop on a repeated token [ytdlp], or once `max-episodes` items have been collected. A page that holds only a continuation item is followed [ytdlp, fixed 2026-06-13].

**Retries:** retry once on 408, 5xx or 404 [gj, ytdlp]. Never retry 403 or 429 [ytdlp]. Retry a 200 whose `alerts` say "unknown error" [ytdlp].

### 4.5 RSS

- **URL:** `GET https://www.youtube.com/feeds/videos.xml?channel_id=UC…`, an Atom feed with `yt:` and `media:` namespaces [gj, st; obs 11-23 ytdlp: 200].
- **Fields per `entry`:** `yt:videoId`, `title` / `media:group/media:title`, **`published`** (an exact timestamp), `updated`, `media:group/media:thumbnail@url`, `media:group/media:description` and `media:community/media:statistics@views`. At feed level: `author/name` and `author/uri` [gj, st].
- **Id format:** `<yt:channelId>` holds the id **without** its `UC` prefix (obs 11-23 [ytdlp]).
- **Missing:** duration, live flag and Shorts flag [gj, st].
- **Use:** RSS is cheap and has exact dates. Fetch it alongside the first uploads page and copy `published` onto matching video ids. `st` does the reverse: it lists from RSS and cross-checks against the grid to drop Shorts.

### 4.6 Uploads as one playlist

- **The `UU` playlist:** `UU` + the channel id without `UC` is the channel's uploads playlist [ytdlp]. Browse it as `VLUU…` or open `https://www.youtube.com/playlist?list=UU…`.
- **Why use it:** its `playlistVideoRenderer` items carry **`lengthSeconds`**, a structured duration, and it merges videos, streams and Shorts. `UUMO…` is a members-only variant [ytdlp test].
- **Recommendation:**
  - For `tabs = videos streams` (the default), list the `/videos` and `/streams` tabs and merge them by date, which excludes Shorts.
  - When the tab layout breaks, fall back to `UU…`.

### 4.7 Playlists

- **First page:**
  - HTML `https://www.youtube.com/playlist?list=<id>` with `ytInitialData` [gj, ytdlp], or
  - `POST /browse` with `{"browseId":"VL<playlistId>"}` [ytdlp, st, ytjs].
- **Header:**
  - old: `header.playlistHeaderRenderer` (`title`, `ownerText.runs`, `numVideosText`);
  - new: `header.pageHeaderRenderer` (`pageTitle`; the count from a "N videos" text part) [gj].
- **Items:** `…playlistVideoListRenderer.contents[].playlistVideoRenderer`, with `videoId`, `title`, `lengthSeconds`, `thumbnail.thumbnails[]` and `index`. Some playlists render as `richGridRenderer` or `sectionListRenderer` instead [gj]. Continuations work as in 4.4.
- **Including unavailable items:** re-browse with `{"browseId":"VL<id>", "params":"wgYCCAA="}`. Ignore the warnings "Unavailable videos will be hidden during playback" and "Unavailable videos are hidden" [ytdlp].
- **Id shape:** a prefix `PL|LL|EC|UU|FL|RD|UL|TL|PU|OLAK5uy_` followed by 10 or more of `[0-9A-Za-z-_]`, or one of the specials `RDMM|WL|LL|LM` [ytdlp].

### 4.8 Field mapping to `episode`

- **`id`:** the video id. **`stream`:** `v:<id>`. **`title`:** the item title.
- **`aired`:**
  1. the RSS `published` date when the video is in the feed;
  2. otherwise, an approximation from the relative text ("3 days ago", "Streamed 2 weeks ago"). Parse it in English; `ytdlp` understands `s/sec/min/h/hr/d/w/wk/mo/y/yr`;
  3. for upcoming items, `upcomingEventData.startTime`.

  Exact dates are not in tab listings [ytdlp]. The only per-video exact source is `/player` `microformat.playerMicroformatRenderer.uploadDate`, one request per video, which is too expensive for a catalog. Mark approximations by leaving them coarse to the day.
- **`duration`:** `lengthSeconds` when present, else the parsed "h:mm:ss" badge.
- **`thumbnail`:** the largest `sources[]`/`thumbnails[]` entry, or `https://i.ytimg.com/vi/<id>/hqdefault.jpg` [ytdlp names `maxresdefault`, `hq720`, `sddefault`, `hqdefault`, …; strip the query from `maxresdefault` URLs].
- **`plot`:** the RSS `media:description` when present.
- **Excluded:** members-only items, ads, mixes, and upcoming items whose start is more than a day away.
- **Live items:** a live-now item in an uploads listing still becomes an episode, because it is playable through HLS. It is also surfaced as a `channel` (2.3).

### 4.9 Logged-in library (milestone 6)

| Item | Request | Source |
|---|---|---|
| Watch Later | `browseId:"VLWL"` (list `WL`) | st, ytdlp |
| Liked | `VLLL` (list `LL`) | st, ytdlp |
| Library playlists | TV `browseId:"FEplaylist_aggregation"`; Watch Later is the first item, Liked the one whose browseId starts `VLLL` | st |
| Subscriptions (channel list) | TV `FEsubscriptions`. Path: `contents.tvBrowseRenderer.content.tvSecondaryNavRenderer.sections[0].tvSecondaryNavSectionRenderer.tabs[]`; tabs other than the first are channels. WEB alternative: HTML `https://m.youtube.com/feed/channels`, items `channelListItemRenderer` | st, gj |
| History | `FEhistory` | st |

Map Watch Later and Liked to series, and subscriptions to extra channel series.

**Search (not in the contract, for later):** `POST /youtubei/v1/search` with `{"query":"…","params":"EgIQAfABAQ=="}` (videos only) [ytdlp]. Filters [st]: channel `EgIQAg%3D%3D`, playlist `EgIQAw%3D%3D`, live `EgJAAQ%3D%3D`.

### 4.10 Request budget

`wiki`: guests get about 300 videos/h (about 1000 page and player requests); accounts about 2000 videos/h. It recommends 5 to 10 s between videos.

A catalog refresh costs one request per channel (HTML first page) plus one RSS fetch. Cache each channel's first page in storage with a ~15 min TTL, so that `episodes` right after `load-catalog` does not fetch it again.

---

## 5. The player request and formats

### 5.1 Body

`POST https://www.youtube.com/youtubei/v1/player?prettyPrint=false`, with the headers of 3.1. Example for VISIONOS:

```json
{
  "context": {
    "client": {
      "clientName": "VISIONOS", "clientVersion": "1.02",
      "deviceMake": "Apple", "deviceModel": "RealityDevice17,1",
      "osName": "visionOS", "osVersion": "26.5.23O471",
      "hl": "en", "gl": "US", "timeZone": "UTC", "utcOffsetMinutes": 0,
      "visitorData": "<visitorData>"
    },
    "user": {"lockedSafetyMode": false}
  },
  "videoId": "<id>",
  "cpn": "<16 random chars>",
  "playbackContext": {"contentPlaybackContext": {"html5Preference": "HTML5_PREF_WANTS"}},
  "contentCheckOk": true,
  "racyCheckOk": true
}
```

The other clients use the same body with these changes:
- **JS-player clients** (TVHTML5 5.x) add `playbackContext.contentPlaybackContext.signatureTimestamp: <STS integer>` [ytdlp, st, gj]. It is required: without it, or with the wrong variant, WEB returns `UNPLAYABLE` "The page needs to be reloaded." [ytjs obs 11-23, st].
- **TV clients:** `st` also sends `playbackContext.devicePlaybackCapabilities{supportsVp9Encoding:true, supportXhr:false}`, citing a report that `supportXhr:false` brings URLs back for TV. It also sends `lactMilliseconds:60000` and `isInlinePlaybackNoAd:true`. These are optional and worth probing on TVHTML5 7.x.
- **Embedded clients** add `context.thirdParty.embedUrl` and `contentPlaybackContext.encryptedHostFlags` [ytdlp, st].
- **With a token:** `serviceIntegrityDimensions.poToken: "<token>"` (not in v1).
- **With login:** the auth headers of section 8.

### 5.2 `playabilityStatus`

| `status` | Meaning | Plugin action |
|---|---|---|
| `OK` | playable | continue |
| `LOGIN_REQUIRED` + reason "Sign in to confirm you're not a bot" | bot check | refresh visitorData once and retry the same client; then next client; finally `bad-response("YouTube bot check: …")` |
| `LOGIN_REQUIRED` / `AGE_CHECK_REQUIRED` / `CONTENT_CHECK_REQUIRED` / `AGE_VERIFICATION_REQUIRED`, or `reason` containing "confirm your age", "age-restricted", "inappropriate", or `desktopLegacyAgeGateReason` present | age or content gate [ytdlp, st, gj] | if signed in, retry on a logged-in TV client; else `auth-failed` |
| `UNPLAYABLE` | various: members-only ("member-only content" in reason [gj]), made-for-kids on VISIONOS [ytdlp], embedding disabled, "page needs to be reloaded" (bad STS), possibly a temporary ban [st] | next client; members-only → logged-in TV if signed in |
| `ERROR` | removed or private [st] | `bad-response(reason)` without trying further clients, unless reason says "made for kids" (then IOS_PRODUCER) |
| `LIVE_STREAM_OFFLINE` | upcoming or ended live; `liveStreamability.liveStreamabilityRenderer.offlineSlate.liveStreamOfflineSlateRenderer.scheduledStartTime` gives the start [gj] | `bad-response("Live starts at …")` |
| `errorScreen.playerCaptchaViewModel` present | captcha [ytdlp] | `http(429)` |
| reason "This content isn't available, try again later" | session or IP **rate limit**, up to an hour [ytdlp] | `http(429)`; do not rotate clients |

Error text to surface:
- the reason: `errorScreen.playerErrorMessageRenderer.reason` (`runs`/`simpleText`), else `playabilityStatus.reason`, else `errorScreen.playerInterstitialRenderer.content.interstitialViewModel.title.content`;
- followed by the subreason: `errorScreen.playerErrorMessageRenderer.subreason`.

Strip the trailing "This helps protect our community. Learn more" [ytdlp].

Geo-blocking: a subreason starting with "The uploader has not made this video available in your country" [ytdlp].

### 5.3 `streamingData` and metadata

**Read** [ytdlp, st, gj]:
- `formats[]`, `adaptiveFormats[]`, `hlsManifestUrl`, `dashManifestUrl` (ignored), `serverAbrStreamingUrl` (ignored), `expiresInSeconds` and `licenseInfos` (DRM).
- `videoDetails`: `videoId`, `title`, `author`, `channelId`, `lengthSeconds`, `shortDescription`, `isLive`, `isLiveContent`, `isUpcoming`, `isPostLiveDvr`, `isLowLatencyLiveStream`, `thumbnail.thumbnails[]`.
- `microformat.playerMicroformatRenderer`: `uploadDate`, `lengthSeconds`, `liveBroadcastDetails{isLiveNow, startTimestamp, endTimestamp}`, `availableCountries`.
- `captions.playerCaptionsTracklistRenderer`.

**Format fields** [ytdlp, st, gj]:
- `itag`, `url` / `signatureCipher` (legacy `cipher`), `mimeType` (`type/subtype; codecs="…"`), `bitrate`/`averageBitrate`, `contentLength`, `approxDurationMs`;
- video: `width`, `height`, `fps`, `quality`, `qualityLabel`;
- audio: `audioQuality`, `audioSampleRate`, `audioChannels`;
- `initRange`, `indexRange`, `type` (`FORMAT_STREAM_TYPE_OTF`), `targetDurationSec`, `maxDvrDurationSec`, `drmFamilies`, `projectionType`, `spatialAudioType`;
- `audioTrack{id, displayName, audioIsDefault, isAutoDubbed}`, `isDrc`, `isVb`, `xtags`.

**Discard a format when** [ytdlp, gj]:
- it has neither `url` nor a `signatureCipher` that holds both `url` and `s`. That is SABR-only: on web clients YouTube "is forcing SABR"; on others a session SABR experiment is running.
- `drmFamilies` is non-empty;
- `type == "FORMAT_STREAM_TYPE_OTF"`. Such a format needs `&sq=` segment fetching [ytdlp, st];
- `targetDurationSec` is present, which marks a live adaptive format;
- `n` is present but could not be solved [ytdlp];
- `approxDurationMs` is under half the video length, which means "possibly damaged" [ytdlp];
- it is itag 22, "likely damaged" [ytdlp], or itag 17 (3gp).

### 5.4 Building a usable URL [ytdlp, gj, st]

1. **Take the base URL.** If `url` is present, use it. Otherwise parse `signatureCipher` as a query string with `url`, `s` and `sp`; `sp` defaults to `signature`. Solve `s`, then append `&<sp>=<solved>`.
   - Disagreement: `st` always writes the result as `sig` and ignores `sp`; `ytdlp` and `ytjs` use `sp`. Use `sp`.
2. **Solve `n`.** If the URL has `n=` (or a `/n/<v>/` path segment, in manifests), replace it with the solved value. The same `n` value is shared by every format of one response, so solve it once [st].
3. **Add nothing else** for direct playback. `ytdlp` adds no `cpn`, `rn`, `alr` or `ump` parameters and works. `gj` adds `cpn` if missing, plus `rn` for each request; that is optional.
4. **Keep the parameters YouTube put there:** `expire`, `ei`, `ip`, `id`, `itag`, `source`, `requiressl`, `xpc`, `mime`, `clen`, `dur`, `lmt`, `sig`/`lsig`, `sparams`, `fvip`, `mn`. Here `lsig` marks a pre-signed URL [st], and `clen`/`dur` give the content length and duration.

### 5.5 Choosing the video

- **Candidates:** the video-only adaptive formats left after the filtering in 5.3.
- **Height cap:** the `quality` field caps the height.
- **Ranking:** prefer the highest `height`, then `fps`, then a codec order.
  - **Codec order:** VP9 (`vp9`/`vp09`), then AVC (`avc1`), then AV1 (`av01`). `gj` drops AV1 from its direct path; AV1 is heavy for software decoding.
  - **Opt-in only:** formats with `xtags` `sr=1` (AI-upscaled [ytdlp]) and HDR (`projectionType`/HDR codecs).
- **IOS_PRODUCER** gives H.264 only [ytdlp].
- **Muxed fallback:** if no video-only format exists, use the best muxed `formats[]` entry (itag 18 for 360p) as `url`, with no audio track.

### 5.6 Choosing audio tracks (multi-language, DRC)

- **Grouping:** group the audio formats by language.
  - **Language:** the part of `audioTrack.id` before the first `.` [ytdlp, gj]. When `audioTrack` is absent, use the `lang` key of the format URL's `xtags` [st].
  - **`xtags` syntax:** `key=value` pairs separated by `:`. `acont` is one of `original`, `descriptive`, `dubbed`, `dubbed-auto` or `secondary` [st].
- **Picking one format per language:**
  - Drop `isDrc` (the DRC/"normalised" variant shares its itag [ytdlp, st]) unless the user wants DRC.
  - Drop `isVb` (voice boost) [gj].
  - Drop `descriptive` audio unless it is the only one.
  - Pick the highest `bitrate`. Prefer Opus (itag 251) or AAC (itag 140) to suit mpv; both work.
- **Track order:**
  1. the **original** language: a `displayName` containing "original" [ytdlp, gj], or `acont=original`, else `audioIsDefault`;
  2. the other human-dubbed languages;
  3. auto-dubbed languages (`isAutoDubbed` or `acont=dubbed-auto`) last.

  mpv's language preference picks among them and falls back to the first [contract].
- **Track fields:** `language` is an ISO 639 code (the language part above, for example `en`, `fr`, `pt-BR` → `pt`). `title` is `audioTrack.displayName`, or "<lang> (<acont>)".
- **Single-audio videos:** they have no `audioTrack`. Use one track with no language.

### 5.7 Captions as tracks

- **Track list:** `captions.playerCaptionsTracklistRenderer.captionTracks[]`, with `baseUrl`, `languageCode`, `vssId`, `name` (`simpleText` or `runs`), `kind` (`asr` = auto-generated) and `isTranslatable` [ytdlp, st, gj].
- **URL:** make a relative `baseUrl` absolute against `https://www.youtube.com` [st, gj]. Set `fmt=vtt` [st, ytdlp] and remove `xosf`, which adds unwanted positioning [ytdlp].
- **Track fields:** `language` is `languageCode`. `title` is `name`, plus " (auto)" for `asr`.
- **Order:** manual tracks first, then ASR. Cap the list at about 10 tracks.
- **Translated tracks:** the `translationLanguages[]` list allows `&tlang=<code>` on one origin track. Never set `tlang` to the track's own language, which "returns damaged subtitles" [ytdlp]. `st` picks the origin track as the first ASR track that is neither a dub transcript (`variant=timing-optimized`) nor auto-dubbed. Leave translation out of v1.
- **PO token:** a caption `baseUrl` whose `exp` values include `xpe` or `xpv` needs a token [ytdlp]. Skip those tracks in v1. `gj` and `st` append `pot` when they have one, but neither stops when it is missing.

---

## 6. Challenges: player script, signature and n

v1 milestone 1 needs none of this (VISIONOS). It becomes necessary for TVHTML5 5.x, for the `/n/` segment in live HLS manifests from JS clients, and for any signed-in playback.

### 6.1 Finding the player and its version

- **Player id:**
  - `GET https://www.youtube.com/iframe_api` and find the 8-hex-digit id after `player\/` (slashes are backslash-escaped). Obs 11-23: `6b3d0364` [ytdlp, ytjs, jse].
  - Or take `PLAYER_JS_URL` (or `WEB_PLAYER_CONTEXT_CONFIGS.*.jsUrl`) from a page's `ytcfg` [ytdlp, gj].
- **Script URL** for the web ("main") variant: `https://www.youtube.com/s/player/<id>/player_ias.vflset/en_US/base.js`. Obs 11-23: 3,060,278 bytes [ytdlp, jse]. `ytdlp` rewrites every player URL to this main variant, even for `tv_samsung` [ytdlp]. `ytjs` uses `player_es6.vflset/en_US/base.js` (2,703,532 bytes, obs 11-23).
- **The TV variant:** `/tv` gives `tv-player-es6`/`tv-player-ias`. The TCL TV variant is `/s/player/<id>/tv-player-ias-tcl.vflset/tv-player-ias-tcl.js` [st]. `tv_config?action_get_config=true&client=lb4&theme=cl` gave `jsUrl` `…/tv-player-es6-tcl.vflset/…` (obs 11-23 [st]).
- **The id rotates.** `/tv` returned `6b3d0364` and later `7f755914` on the same day [st obs 11-23], so it is A/B-assigned per request. Cache the id with a short TTL (~1 h). Accept that two players can be live at once: key every cache by player id, never assume a single current one.
- **Downloads:** retry 3 times, 1 s apart. Never cache a failed or partial download [st 11-22].

### 6.2 Signature timestamp (STS): 5 versus 8 digits

- **Where:** the player text holds `signatureTimestamp` (or `sts`), optional whitespace, `:` or `=`, then digits [ytdlp, gj, st]. A page's `ytcfg` `STS` is used first when present [ytdlp].
- **The two formats:**
  - **Web and non-TCL TV players** (`base.js`, `tv-player-es6.js`): **5 digits**. Obs 11-23: `20774` for `6b3d0364` [ytdlp, st, ytjs].
  - **TCL TV players:** **8 digits**, the 5-digit value with `001` appended. Obs 11-23: `20774001` [st]. They are used only by TVHTML5 7.x.
- **Matching the client:** the STS must come from the variant matching the client. A mismatch on TV gives "page should be reloaded" [st]. For TVHTML5 5.x (Samsung, downgraded) use the 5-digit value; `st` downgrades the UA precisely to get the non-TCL player.
- **Parsing:** `ytdlp`'s pattern is fixed at 5 digits and will break at 100000. Parse any run of digits.
- **When it is missing:** treat it as a hard error for JS clients [gj].
- **Caching:** store it per player id persistently (it is tiny).

### 6.3 What sig and n are

- **sig:** `signatureCipher.s` must be transformed by player code. The transform is a fixed **reordering and deletion of positions** that depends only on the input length [ytdlp, ejs tests]. Example: player `631d3938` maps a 103-character input to 100 characters.
  - **Index map:** solve once per (player, length) with a probe input made of distinct characters, read the output as a list of source positions, cache that map, and apply it natively from then on [ytdlp].
  - **When it is needed:** VISIONOS and TV URLs observed so far arrive pre-signed (`sig` present, no cipher) [ytdlp obs, st obs]. `gj` made sig optional in v336. Sig solving is a fallback, not a v1 requirement.
- **n:** the `n` query value (or the `/n/<v>/` manifest path segment) must be replaced by a transform of itself. With n unsolved, googlevideo throttles or 403s: obs 11-23 [st], a downgraded-TV URL with raw `n` gave 403. n is the part that breaks most often [gj fix 2026-11-13, st, ejs].
  - **Error marker:** a result starting with `enhanced_except_` is the player's own error marker [ytjs]. Treat it as a failure and do not cache it.
  - **Test vectors** [ejs; useful only if those players are fetched]: `74edf1a3` (STS 20522) maps `IlLiA21ny7gqA2m4p37` to `9nRTxrbM1f0yHg`; `631d3938` (20529) maps `KBx1qz7jMhxELa8c` to `ttPvh7WIptsgSw`; `5ec65609` maps `IMKPt9L6DXEYGGRzJ` to `zL66zSisCR_I4g` [gj].
  - **No current vector:** none exists for the current STS 20774, so self-test as in 6.5.

### 6.4 Locating and running the transforms (approach, in prose)

Both EJS [ejs, 8 months without a logic change until 2026-10-26] and YouTube.js [ytjs, since 16.0.0, 2025-10] stopped hunting for separate "sig" and "n" functions. Instead they drive the player's own stream-URL builder:

- **The anchor:** in the player there is a routine that builds a stream URL object and, among other things, sets the parameter pair `"alr"` / `"yes"` on it. The literal pair occurred 3 times in `6b3d0364` (obs 11-23 [ytdlp]).
  - **EJS** takes every top-level function whose own statements contain a method call with exactly those two string arguments.
  - **YouTube.js** looks for a function expression with at least three parameters, the last two with defaults, which assigns a `new` of a player-internal class to its first parameter and makes that call.
- **Driving it:**
  1. Call the candidate with a dummy `https://…googlevideo.com/videoplayback?…&n=<challenge>` URL, the signature parameter name, and the URL-encoded `s`.
  2. Take the returned object and call the first method on its prototype other than `constructor`, `set`, `get` and `clone`. That makes the object apply the player's own deciphering.
  3. Read back the `n` parameter and the signature parameter, URL-decoded.
- **Agreement:** `ejs` tries every candidate and requires them all to agree. Zero results is a failure, and differing results is "invalid solutions".
- **Making the routine reachable:** the player is one wrapper function (an IIFE called on `this`, or a second statement that is an IIFE), so the routine is not global. Two documented ways:
  - **(a) EJS:** parse the player, keep only the wrapper body's declarations, assignments and directives, drop the side-effecting expression statements, prepend stubs, run that, and expose the candidates.
  - **(b) YouTube.js:** collect only the matched function's transitive dependencies (including prototype methods assigned through aliases) into a small standalone script. Initialisers with side effects become `undefined`. Unresolvable `window`-hung look-ups are tolerated.
- **Our approach in Rust:**
  1. **Parse with an MIT/Apache Rust ES parser** (for example `oxc`; it must handle ES2015+, `?.` and `yield` [jse]). Find the wrapper body and the candidate functions by the anchor shape above.
  2. **Expose the candidates.** Either do (a) by emitting the filtered body plus an export of the candidates, or, simpler, insert one statement at the end of the wrapper body that assigns the candidate functions to a global result object, and evaluate the **whole** player.
  3. **Evaluate in rquickjs.** `jse` measured that the whole current player evaluates in rquickjs in about 0.3 s (36 MiB peak) with about 20 lines of stubs: `window`, `self`, `document` with stub elements, `location`, `navigator`, `screen`, timers, `localStorage`, `performance` and a few constructors. The check was `Object.keys(_yt_player).length === 558`, the same as Node.
  4. **Drive** as described above.

  Keep (a) or (b) as the fallback if a future player's top-level code throws under the stubs.
- **Stubs EJS needs** [ejs]: define only what is missing. `XMLHttpRequest` is an object with an empty `prototype`. `location` is a `URL` for a watch page, or a plain object with the usual fields. `document` and `navigator` are empty null-prototype objects. `self` and `window` are the global object. Players 20528/20529 needed `self.location.origin` [ejs].
- **Engine requirements:** a full ES2015+ engine, `Function`, Symbol, Promise, Proxy, Reflect, BigInt; no DOM [ejs, jse]. EJS supports QuickJS from 2023-12-09 and any QuickJS-NG. Older than QuickJS 2025-04-26 or QuickJS-NG 0.12.0 can take "several minutes" [wiki, ejs]. rquickjs 0.14 bundles QuickJS-NG 0.16.2 [jse].
- **The TCL variant** (TVHTML5 7.x only) is different. Its n transform takes a URL-parameters object, not a string. Wrap the challenge as `https://www.youtube.com/videoplayback?n=<v>` [st]. We avoid TCL in v1 except for itag 18, and the main variant's solution applies to a 5.x client's URLs. `ytdlp` solves `tv_samsung` URLs with the main player.
- **Batching:** collect every distinct n value and every distinct signature length across the responses and solve them in one engine run per player [ytdlp].

### 6.5 Self-test

- **For each new player id:** solve a fixed sample n, and a sample signature of a typical length (for example 103). The outputs must differ from the inputs and be non-empty [st]. Require that n passes and the STS is found; sig may be optional [gj, st].
- **On failure:** mark that player id as bad for 1 h and fall back to the JS-free clients.

### 6.6 Engine: rquickjs (merged from `jse`)

- **Choice:** `rquickjs` 0.14 (QuickJS-NG 0.16.2, MIT). It builds for `wasm32-wasip2` inside a wit-bindgen 0.63 component with no patches, given a wasi-sdk through the `WASI_SDK` environment variable. Without it, `rquickjs-sys` downloads wasi-sdk 24, which CI should avoid. It adds about 0.8 MB to the component.
- **Sloppy mode is required.** Evaluate with `strict = false` (`eval_with_options`), because the player reads `this` at the top of its wrapper. Under the strict default it throws "window is undefined".
- **Measured inside wasmtime 50** with the app's settings (Cranelift, epoch interruption, a 256 MiB limiter, a fresh store per run):

  | Operation | Time | Peak memory |
  |---|---|---|
  | Parse + eval of the whole 3 MB `base.js` | 293 ms (273 + 9) | 35.6 MiB |
  | Load from cached stripped bytecode | 33 ms | 15.7 MiB |
  | Write the bytecode | 10 ms | - |
  | 1e6-iteration loop | 3.7 s (about 15× Node with JIT) | - |

- **In-engine limits:** use `set_memory_limit` and `set_max_stack_size` so a runaway script throws instead of trapping the call. The wasm stack is 1 MiB by default; deep recursion may need `-z stack-size`.
- **Rejected:** Boa needs about 258 MiB and fails at 256 MiB; nova_vm does not build; javy and quickjs-wasm-rs bring no advantage [jse].

### 6.7 Caching per player id in storage

Each call is a fresh instance, so the caches live in storage. Order of preference [jse, ytdlp, st, ytjs]:

| Key | Value | TTL / persist | Size |
|---|---|---|---|
| `player:id` | current player id | 1 h, not persisted | tiny |
| `sts:<pid>` | STS (5-digit; 8-digit for `tcl`) | 30 d, persisted | tiny |
| `n:<pid>:<n>` | solved n | 6 h (URL lifetime), not persisted | tiny |
| `sig:<pid>:<len>` | position map for that length | 30 d, persisted | < 1 KB |
| `js:<pid>:<qjsver>:<k>` | stripped bytecode of the transformed player, gzip + base64, split into ≤ 1 MiB chunks `k=0,1`, with a header holding its length and hash | 7 d, persisted | about 1.7 MB in 2 values [jse] |
| `jsx:<pid>` (optional) | extracted standalone script (approach b) | 7 d, persisted | a few to tens of KB |

- **Hits:** a hit on `n:`/`sig:` needs no engine at all. A bytecode hit costs about 30 ms, a miss about 300 ms plus the 3 MB download.
- **Bytecode safety:**
  - The format is tied to the exact QuickJS-NG build, so the key must include the engine version.
  - `JS_ReadObject` trusts its input, so verify the length and hash before loading; a corrupt value could crash the instance.
  - Stripping the debug information removes line numbers from errors [jse].
- **Budget:** keep only the bytecode of the most recent 1 or 2 player ids, deleting older keys, which stays within the 16 MiB total. `st` keeps one per variant; `ytdlp` does not cache the preprocessed player on disk at all.
- **Invalidation:** wipe a player's entries on a solver exception or a failed self-test [st].
- **Alternative to the Rust parser:** run EJS itself (Unlicense) inside QuickJS, as `st` does in V8 and `ytdlp` supports with QuickJS. It is legally compatible with MIT, but it is a vendored dependency rather than clean-room code. Its parse cost in QuickJS (meriyah on a 3 MB input, then astring regeneration, then re-parsing) is unmeasured. This is a user decision (section 13).

---

## 7. PO tokens

### 7.1 What they are and where they go

- **Origin:** a PO (proof of origin) token is produced by BotGuard on web, DroidGuard on Android and iOSGuard on iOS. Tokens are **not portable** across these [wiki].
- **Contexts:**
  - **GVS** (googlevideo media): `pot=` on https URLs; `/pot/<token>` in HLS/DASH manifest paths, before a trailing `/file/index.m3u8` [ytdlp]; raw bytes in SABR `streamerContext.poToken` [gv, ytjs].
  - **Player:** `/player` `serviceIntegrityDimensions.poToken`.
  - **Subs:** `pot=<t>&potc=1&c=<clientName>` on caption URLs [ytdlp, gj].
- **Binding** (what the token is minted for):
  - Player and Subs: the **video id** [wiki].
  - GVS logged out: visitor data or visitor id, or the video id when `serializedExperimentFlags` has `html5_generate_content_po_token=true` [ytdlp, gj].
  - GVS logged in: `DATASYNC_ID` [ytdlp, gj].
  - The `bgu` README says the web player today mints **per video with the video id**, and session-bound tokens are "not used anymore" on web. `pot` defaults to visitorData. **Disagreement, unresolved**; `bgu` is fresher on intent.
  - **TV:** a session token bound to `LIVING_ROOM_PO_TOKEN_ID`, scraped from `/tv` together with that page's `visitorData`, and sent with `tvAppInfo.livingRoomPoTokenId` [st, added 2026-10-04/05].
- **Who needs a token:** see the client table. In short, every client that still returns URLs without SABR (VISIONOS, IOS_PRODUCER, TVHTML5 5.x) needs **no** token today [ytdlp, gj]. Web-family clients need one but are SABR-only.

### 7.2 The BotGuard flow, end to end

Facts from `bgu`, `st`, `gj` and `pot`:

1. **Get a challenge.** There are three sources:
   - **Homepage (current for WEB/MWEB):**
     - `GET https://www.youtube.com` with a desktop UA. In the HTML, find the call `window.ytAtN({...})` that has an object argument. Skip the empty `window.ytAtN();` that appears first (obs 11-23 [bgu]).
     - Its `R` value is a single-quoted string with `\xNN` escapes. Decoded, it is JSON holding `bgChallenge{interpreterUrl.privateDoNotAccessOrElseTrustedResourceUrlWrappedValue, interpreterHash, program, globalName, clientExperimentsStateBlob}`. The object literal is loose (single quotes, trailing comma) and must be normalised.
     - Obs 11-23: `globalName` `trayride`; `program` 39,547 chars; interpreter `//www.google.com/js/th/<hash>.js` (prefix `https:`), 64,222 bytes.
     - `T` (95 chars) has an unknown purpose.
     - The same page's `ytcfg.set({...})` must be installed as `yt.config_` on the VM's global, because **BotGuard reads `yt.config_.EVENT_ID`** [bgu 2026-08-04, pot 2026-08-21, st].
     - Historical form: `window.ytAtR = '<json>'`.
   - **`att/get`:** `POST /youtubei/v1/att/get?prettyPrint=false` with `{context, engagementType:"ENGAGEMENT_TYPE_UNBOUND"}`. Since 2026-08, tokens minted from it are **rejected at GVS** for WEB when the session is in the EVENT_ID experiment: `/player` passes and googlevideo 403s [pot 495a47f]. **Historical** for WEB. `st` still uses it for the TV challenge, with `clientName:"TVHTML5"`.
   - **WAA `Create`:**
     - `POST https://jnn-pa.googleapis.com/$rpc/google.internal.waa.v1.Waa/Create` (or `https://www.youtube.com/api/jnn/v1/Create`).
     - Body `["O43z0dpjhgX20SCx4KAo"]`.
     - Headers `content-type: application/json+protobuf`, `x-goog-api-key: AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw`, `x-user-agent: grpc-web-javascript/0.1`, plus a desktop UA.
     - The reply's element 1, when a string, is base64 whose every byte plus 97 (mod 256) is JSON: `[messageId, wrapped interpreter JS, wrapped URL, interpreterHash, program, globalName, -, clientExperimentsStateBlob]`.
     - Only some clients (YouTube Music) accept tokens from it [bgu]. `gj` uses it as its engine fallback.
2. **Run the interpreter** in global scope, which defines `globalThis[globalName]`.
3. **Initialise the VM:** call its `a` function.
   - `bgu` passes 9 arguments: the program; a setup callback receiving four functions (async snapshot, shutdown, pass-event, check-camera); `true`; undefined; a telemetry callback; `[[],[]]`; undefined; `false`; and an array of five logger callbacks.
   - `gj` passes 6 (the program, a callback, `true`, `null`, a no-op, `[[],[]]`). `gj` is fresher (11-21) but uses the Create path.
   - Wait for the setup callback, which `bgu` 3.2.0 fixed. `st` polls for up to 10 s.
4. **Snapshot:** call the async snapshot with `[contentBinding?, signedTimestamp?, webPoSignalOutput, skipPrivacyBuffer?]`. Only `webPoSignalOutput`, an empty array that the VM fills, is needed. The result callback gets the **botguardResponse** string. `bgu` times out after 3 s.
5. **GenerateIT:**
   - `POST https://www.youtube.com/api/jnn/v1/GenerateIT` (or the jnn-pa `…/Waa/GenerateIT`, which gives "fewer 403s but can hang" [st]), with the same headers as `Create`.
   - Body `[requestKey, botguardResponse]`. The web key is `O43z0dpjhgX20SCx4KAo`; the TV key comes from `tv_config` `challengeRequestKey` [st].
   - Reply: `[integrityToken (base64url, '.' as padding), estimatedTtlSecs (43200), mintRefreshThreshold, websafeFallbackToken]`.
6. **Minter:** `webPoSignalOutput[0]` is now a function. Call it with the integrity token's bytes; it resolves to the mint function.
7. **Mint:** call it with the UTF-8 bytes of the binding. It returns about 110 to 128 bytes. Base64url-encode them, keeping `=` padding [bgu, gj].
8. **Lifetime:**
   - The integrity token lasts about 12 h. `st` expires it 600 s early; `pot` caches minted tokens 6 h.
   - Tokens are IP-bound: `pot` keys its cache by egress IP and `remoteHost`.
   - `st` retries a failed mint once with a new generator. `gj` assumes a 5-minute minter lifetime in the browser.

**Error checkpoints** [bgu]: `PMD:Undefined` (no minter getter), `APF:Failed` (the getter is not a function), `YNJ:Undefined` / `ODM:Invalid` (bad mint output).

### 7.3 Cold-start tokens

- **What it is:** a placeholder used before BotGuard is ready.
- **Layout:** tag byte `0x22`, a length byte, 2 random key bytes, a 0 byte, a client-state byte (1), a 4-byte big-endian Unix time, and then the UTF-8 binding (at most about 118 bytes; the length is one byte). Every byte after the key is XORed with the key byte at (index mod 2), then the whole is base64url-encoded [bgu, st (hist.)].
- **Disagreement:** `gj` describes the byte after the key as 1, with no binding and standard base64.
- **Use:** it works only while SABR's stream-protection status is 2, which allows about 1 to 2 MB of media before status 3 [bgu, gv]. **Useless for us:** it is a SABR mechanism, and there is no evidence that it helps `pot=` on plain URLs.

### 7.4 Running BotGuard without a DOM: what is known

- **Nobody has done it outside a DOM.** Every working implementation uses jsdom (Node or Deno) or a real browser or WebView [bgu, pot, st, gj]. No source reports QuickJS, Boa or any engine without a DOM [bgu issue search]. `st` tried bare V8 with polyfills and abandoned it, saying a browser-like environment cannot be built on bare V8.
- **What the interpreter names** (64 KB, obs 11-23 [bgu]): `document` (8 times), `window` (4), `createElement` (2), `addEventListener` (9), `screen` (10), `performance` (3), `setTimeout` (3), `eval` (4), `atob`/`btoa`, `Symbol`, `Promise`. It does **not** name `navigator`, `canvas`, `getContext`, WebGL, `crypto` or `localStorage`. The encrypted `program` decides what is actually probed, so this list is a floor.
- **Host needs** [bgu]: `Function`/`eval`, Promises, `setTimeout`, `TextEncoder`, `atob`/`btoa` and `Uint8Array`.
- **The silent bad-token failure** [pot #285, opened 2026-11-22, open]:
  - About half the integrity tokens minted under Node or Deno were bad. `GenerateIT` succeeded, but every token derived from them got GVS 403, or worked about 60 s and then got 403, for the full 12 h.
  - The integrity-token length correlated perfectly: 88 characters were good, 92 (and 96) were bad.
  - Cause: Node 21+ has its own global `navigator`, so jsdom's was not installed. Always installing jsdom's `navigator` fixed it.
  - **A wrong environment cannot be detected at mint time.** Only googlevideo tells you.
- **happy-dom** "wasn't generating tokens that work" where jsdom does [pot #208, 2026-04]. The missing piece is unknown, possibly canvas or CSS.
- **Statelessness:** a fresh instance per call means the VM and minter die with the call. Only the integrity token (a string) can be stored. No source shows a stored integrity token fed to a *different* VM's minter [bgu open question], so each play call would redo steps 1 to 7: the homepage fetch, the 64 KB interpreter, the VM run, `GenerateIT` and the mint.

### 7.5 Risk assessment

**Can it run natively in QuickJS?** Possibly, but it is **high risk and unproven**.

Points in favour:
- The engine exists, is fast enough for the player, and has more than 200 MiB of headroom [jse].
- The VM's API surface is small.
- Within one call, all of steps 1 to 7 fit the 5-minute budget if the VM runs in seconds. The VM's interpreted run time was **not** measured; it is the one workload where QuickJS's speed (about 15× slower than JIT) matters [jse].

What must be stubbed or added:
- `window`, `self`, `document` (`createElement` returning stub elements with `addEventListener` and style objects), `screen` (width, height, availWidth, colorDepth…), `navigator` (a **plausible, consistent** desktop UA, platform, languages, hardwareConcurrency, webdriver=false; this is the one #285 shows matters), `location` and `origin` for `https://www.youtube.com/`, and `performance.now`.
- Host-driven `setTimeout`/`setInterval`, run from a job queue the Rust side pumps. rquickjs has no timer loop of its own.
- `atob`/`btoa` and `TextEncoder`, which QuickJS-NG lacks.
- `yt.config_` = the homepage ytcfg.
- Possibly a canvas 2D context stub, a WebGL stub and `crypto.getRandomValues`. Unknown, because the program is opaque.

Why it could fail:
- **Silent failure:** tokens can look fine and 403 later. Validation needs a real GVS probe, either a small range on a token-gated format or the 88-character heuristic, which is unproven.
- **The payoff is low today.** The token-free clients work. The token-gated web clients are SABR-only, so a token would not give mpv-playable URLs anyway; only TVHTML5 7.x with a living-room token might bring URLs back, and that is unconfirmed.
- **Churn:** the challenge source moved in 2026-08 (`att/get` to the homepage plus EVENT_ID), and the bad-token class appeared in 2026-07 and 2026-11 [pot].

**Verdict:** do not put BotGuard in the main path. Keep it as a research spike, milestone 7, to start only when the decision tree below reaches it.

### 7.6 Decision tree

1. **A token-free client** (VISIONOS, IOS_PRODUCER, TVHTML5 5.x) returns URLs that pass the probe. Use it. No BotGuard. This is the state on 2026-11-23 [ytdlp obs, st obs].
2. **Every token-free client fails for VOD** (SABR-only, 403, or the bot check after refreshing visitorData):
   1. **If signed in,** try TVHTML5 5.x and 7.x with OAuth or cookies. If URLs come back and pass the probe, use them.
   2. **Otherwise, if live:** HLS from any client that still returns it and needs no token (web clients' live HLS needs no token [wiki]). Use it.
   3. **Otherwise,** decide whether to start milestone 7:
      - If a token-gated client is shown to return **plain URLs** with a token (candidates: TVHTML5 7.x with a living-room token [st]; `tv_simply`; `web_embedded`), build BotGuard-in-QuickJS for that binding, with a GVS probe after each mint and a re-mint on failure.
      - If every client is SABR-only even with a token, a native plugin cannot play through mpv under the current contract. It would need an app-side SABR bridge (a contract change). Report `bad-response("YouTube requires SABR streaming; not supported")`.
3. **Captions only gated** (`exp` has `xpe`/`xpv`): drop those tracks. Never mint just for subtitles.

---

## 8. Login

Login is optional: a v1 that is anonymous only is useful. Login unlocks age-restricted, members-only and private videos, Premium formats, and the library (4.9).

### 8.1 TV device-code OAuth: endpoints [st, ytjs]

- **Status, by source:**
  - **`st`: current and working.** The client-id extraction was observed working on 2026-11-23.
  - **`ytjs`: present but discouraged.** "OAuth2 sign-in only works when using the TV client … no longer recommended".
  - **`ytdlp`: removed.** "Login with OAuth is no longer supported"; the wiki says "due to new restrictions … no longer works with yt-dlp".
  - **The disagreement** reduces to this: the token works for TV clients only, and `ytdlp` no longer has a usable TV path for it.
- **Client id and secret,** scraped at runtime:
  1. `GET https://www.youtube.com/tv` with a TV UA (`st` uses Fire TV Cobalt; `ytjs` uses `Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version` with `Referer: https://www.youtube.com/tv`).
  2. Find the "base" script URL: `<script id="base-js" src="…">`, or on Cobalt pages an inline assignment of a `.src` ending in `m=base`.
  3. Fetch it (about 1 MB) and take the **first** `clientId:"<…>.apps.googleusercontent.com"` property that is followed directly by another short string property. That second string is the secret. Later occurrences carry other client ids that fail with 401 [st].
  - Obs 11-23 [st]: client id `861556708454-d6dlm3lh05idd8npek18k6be8ba3oc68.apps.googleusercontent.com`, secret starting `SboV…`.
  - Cache the pair persistently and re-scrape on a 401.
- **Device code:** `POST https://www.youtube.com/o/oauth2/device/code`, JSON body `{"client_id", "device_id": <UUIDv4, persisted per install>, "device_model" (ytjs) / "model_name" (st): "ytlr::", "scope": "http://gdata.youtube.com https://www.googleapis.com/auth/youtube-paid-content"}`. The field name differs between `st` and `ytjs`; sending both is harmless. The reply holds `device_code`, `user_code`, `verification_url`, `expires_in` and `interval`.
- **Poll:** `POST https://www.youtube.com/o/oauth2/token` with `{"client_id","client_secret","code":<device_code>,"grant_type":"http://oauth.net/grant_type/device/1.0"}`.
  - On `authorization_pending` or `slow_down`, keep polling.
  - On `access_denied` or `expired_token`, stop [ytjs].
  - On success the reply holds `access_token`, `expires_in`, `refresh_token`, `token_type` and `scope`.
  - Interval: `ytjs` honours `interval`. `st` uses a fixed 3 s for up to 200 tries and ignores it. Honour `interval`.
- **Refresh:** the same URL with `{"refresh_token","client_id","client_secret","grant_type":"refresh_token"}`. The access token lives at most about 60 min [st].
- **Revoke:** `https://www.youtube.com/o/oauth2/revoke?token=<access_token>` [ytjs].
- **Using the token:** send `Authorization: Bearer <access_token>` (`st`: `<token_type> <access_token>`) on Innertube calls with a **TV client**, and drop `key=`.

### 8.2 Surfacing the code within the contract

The contract has no UI hook, no notification and no background task. The plugin can only speak through return values and `log`. Proposal:

1. **Turning it on:** the user sets `sign-in = tv`.
2. **First `load-catalog`:**
   - With no refresh token stored, request a device code. Store `device_code`, `user_code` and the expiry under `auth:pending` with `ttl = expires_in`, not persisted.
   - Return the normal anonymous catalog, plus:
     - **`catalog.account`** = `{status: "Sign in: visit youtube.com/activate and enter ABCD-EFGH", expires: <now + expires_in>, trial: false}`;
     - **a placeholder `series`** named "Sign in: youtube.com/activate, code ABCD-EFGH", with that text as its `plot`, so the code is visible even if the app does not show `account.status`;
     - a `log(info)` line with the same text.
3. **Next `load-catalog` (or any call) while `auth:pending` exists:** poll the token endpoint once.
   - On `authorization_pending`, keep the placeholder.
   - On success, store the tokens, delete `auth:pending`, and build the signed-in catalog.
   - `load-catalog` may also poll several times within its own call, up to about 2 minutes at `interval`, so a user who enters the code while the source refreshes gets signed in in one pass.
4. **Expired code:** request a new one on the next load.
5. **Revoked refresh token** (`invalid_grant`): delete it and return `auth-failed`. Do not fall back silently, because the user asked for a signed-in source.

**Decision for the user:** whether the app shows `catalog.account.status` on the Sources screen. If not, the placeholder series is the only surface. A small contract addition (for example a "message" on the catalog, or a field kind for an action) would be cleaner.

### 8.3 What is stored and refreshed

| Key | Value | TTL / persist |
|---|---|---|
| `auth:client` | `{client_id, client_secret}` | persisted, no TTL; re-scraped on a 401 |
| `auth:device-id` | UUIDv4 | persisted |
| `auth:refresh` | refresh token | persisted |
| `auth:access` | access token | `expires_in - 60` s, not persisted |
| `auth:pending` | `{device_code, user_code, interval}` | `expires_in`, not persisted |
| `auth:accounts` | the `accounts_list` result (name, handle, pageId) | 1 d, persisted |

- **Sharing:** keys are shared by every source of the plugin [contract]. To support several accounts, suffix every key with a hash of the source's `sign-in`/`account` values.
- **Refresh:** refresh the access token on demand at the start of any call that needs it. At most one refresh per call.

### 8.4 Brand accounts

- **Listing:** `POST https://www.youtube.com/youtubei/v1/account/accounts_list`, with a TV context, the bearer token, and `accountReadMask:{returnOwner:true, returnBrandAccounts:true, returnPersonaAccounts:false}` [st].
- **Where the accounts are:** `contents[0].accountSectionListRenderer.contents[0].accountItemSectionRenderer.contents[*].accountItem`, with fields `accountName`, `channelHandle`, `isSelected`, `hasChannel` and `serviceEndpoint.selectActiveIdentityEndpoint.supportedTokens[*].pageIdToken.pageId`.
- **Acting as one:** every account shares the same refresh token. To act as a brand account, add **`X-Goog-Pageid: <pageId>`** to every request [st]. The `account` config field selects it by name or handle.

### 8.5 Cookies and SAPISIDHASH (alternative)

The user pastes a `Cookie` header string into the `cookies` secret field.

- **Logged-in check:** the cookies count as logged in when they include `LOGIN_INFO` plus one of `SAPISID`, `__Secure-1PAPISID` or `__Secure-3PAPISID` [ytdlp].
- **The `Authorization` header** [ytdlp, ytjs]:
  - It holds up to three space-separated values: `SAPISIDHASH <v>` (from `SAPISID`, else `__Secure-3PAPISID`), `SAPISID1PHASH <v>` (from `__Secure-1PAPISID`) and `SAPISID3PHASH <v>` (from `__Secure-3PAPISID`).
  - Each `<v>` is `<unix seconds>_<sha1 hex>`. The SHA-1 input is the timestamp, the cookie value and the origin `https://www.youtube.com`, joined by single spaces.
  - With a user session id, that id is prepended to the hashed input and `_u` is appended to `<v>`.
- **Also send:** `X-Origin: https://www.youtube.com`, `X-Goog-AuthUser: <SESSION_INDEX or 0>`, `X-Goog-PageId` for brand channels, and `X-Youtube-Bootstrap-Logged-In: true` [ytdlp].
- **Which clients:** cookies work with web and TV clients only. VISIONOS, IOS*, ANDROID* and TV Simply do not take them [ytdlp].
- **Rotation:**
  - YouTube rotates `__Secure-1PSIDTS` (and others) through `Set-Cookie`.
  - `gj`'s host writes back any `Set-Cookie` whose name is already stored and ignores new cookies and deletions. The plugin sees every response of its own `http.send`, so it can do the same and store the updated jar under `auth:cookies` (persisted). It must not overwrite the user's field, which it cannot do anyway.
  - `ytdlp` warns when `LOGIN_INFO` disappears ("cookies … likely rotated"). Export advice: take cookies from a private window that is then closed [wiki].
- **Expiry signals:** a short page (under 100 KB) containing `accounts/answer/61416`, or a browse 500 while logged in [gj]. Report it as `auth-failed`.
- **Risk:** accounts that send automated traffic can be banned [wiki]. Say so in the field's placeholder or the docs.

**Recommendation:** offer TV OAuth first. It needs no browser export and gives a refresh token that does not rotate. Keep cookies as the fallback if OAuth stops being accepted, since `ytdlp` reports it as dead for its use.

---

## 9. Live

### 9.1 Detecting live

**Status derivation** [ytdlp]:
- `isPostLiveDvr` means post-live;
- otherwise `isLive` or `liveBroadcastDetails.isLiveNow` means live;
- otherwise `isUpcoming` means upcoming;
- otherwise `isLiveContent` means a past broadcast (VOD);
- otherwise it is not live.

**Listings:** live-now badges (4.3), and `upcomingEventData.startTime` for upcoming items.

**Pinned channels:** for a pinned `chlive:<UC>`, resolve the live video by fetching `/channel/<UC>/live` at play time. It is a redirect to the live watch page; read the video id from the final URL or from `ytInitialPlayerResponse.videoDetails.videoId`.

### 9.2 HLS per client

- **Which clients return `hlsManifestUrl` for live:** VISIONOS (obs 11-23 for VOD; `gj` uses it for live), IOS_PRODUCER [gj], TVHTML5 5.x Samsung [gj], IOS (needs a token 30 s in) [ytdlp], TV 7.x [st], and web clients (no token needed for live HLS [wiki]; but SABR elsewhere).
- **Order:** VISIONOS, IOS_PRODUCER, TV Samsung (3.3).
- **URL rewriting:** rewrite any `/n/<challenge>/` path segment with the solved n [ytdlp, gj]. `gj` appends `?pot=<token>` to live HLS by default, but only because it has tokens. The wiki says live HLS needs none outside `ios`. v1 sends no `pot`.
- **Return value:** `playback.url` is the manifest URL, with the client UA in `headers`. mpv plays the master playlist and its variants directly.
- **Not wanted:** the DASH manifest (`dashManifestUrl`), because DASH is not relayed [contract]. Live adaptive formats need `sq=` segment logic.

### 9.3 DVR window and catch-up (`catchup-window`)

- **What the sources know:**
  - Live adaptive formats carry `maxDvrDurationSec` and `targetDurationSec` [st].
  - YouTube keeps **at most 120 hours** (432000 s) of live segments, addressed as `<format url>&sq=<n>`. The newest `n` is in the `X-Head-Seqnum` header of a HEAD on the format URL; on `tv_samsung` that HEAD may return 500 but still carry the header [ytdlp].
  - `st` reads `X-Sequence-Num`/`X-Head-Seqnum`, `X-Head-Time-Millis` and `X-Walltime-Ms`, or text lines `Sequence-Number:`, `Ingestion-Walltime-Us:` and `Stream-Duration-Us:` in the segment body. It builds a 48 h window.
  - Whether the **HLS** manifest itself exposes a DVR window (a sliding playlist longer than a few segments) is not documented by any source.
- **v1:** leave `catchup-window` absent. `catchup-url` returns `http(501)`.
- **Later (milestone 8):** `catchup-window = min(maxDvrDurationSec, 432000)`.
  - The natural route is a synthesised HLS media playlist whose segments are `&sq=<n>` URLs, computed from `X-Head-Seqnum`, the segment duration, and the requested `start`/`end`. It needs one audio and one video playlist, or a master with an audio group.
  - The problem: the playlist must be served from a network URL. The only documented way is `relay.filter`, which replaces the body of some real http(s) response with the synthesised playlist (for example the live HLS master URL itself, or `hlsManifestUrl` with a marker parameter). It is then rewritten by the relay like any HLS. Whether YouTube's `sq` segments play as HLS fMP4/WebM segments without an `EXT-X-MAP` init is unverified. This is an open question that needs a probe.

### 9.4 n inside live HLS (when a JS client is the source)

- **The problem:** variant playlists are refreshed every few seconds. If their segment URLs also carry an `n` challenge (no source says they do), mpv cannot fix them.
- **The workaround:** `relay{filter: true}`. The `filter` export receives each playlist and rewrites the `/n/<v>/` segments from the `n:` cache (no engine needed on a hit; `filter` has 30 s). Segments pass through the filter whole, which is acceptable for small live segments.
- **When:** only if the probe shows segment URLs carrying `n`. VISIONOS and IOS_PRODUCER have no `n` at all.

### 9.5 Post-live and upcoming

- **Post-live** (`isPostLiveDvr`): the video is still processing. Its HLS and DASH manifests hold only "the last ~2 hours" [ytdlp]. If `lengthSeconds` (or end minus start) is under 2 h, play the HLS. Otherwise return `bad-response("Stream ended; replay is still processing")`. After processing, the video is an ordinary VOD with `isLiveContent`.
- **Upcoming / `LIVE_STREAM_OFFLINE`:** return `bad-response("Live starts at <scheduledStartTime>")`.
- **Premieres:** they follow the same rules [gj fixed premieres 2026-08].
- **Low latency** (`isLowLatencyLiveStream`, `latencyClass`): not handled by any source. Ignore it.

---

## 10. Downloading through mpv

### 10.1 What mpv does and what googlevideo tolerates

- **Ranges:** mpv (through ffmpeg's http) opens a progressive or range-requested URL with an open-ended `Range: bytes=N-` header, and issues new ranges on seeks.
- **What the sources say about limits:**
  - **No hard per-request limit appears in any source.** `ytdlp` downloads https formats in **10 MiB chunks** through the `Range` header, with `Accept-Encoding: identity`. Its optional "dashy" mode uses the `range=<a>-<b>` **query parameter** in 10 MiB fragments.
  - `gj` turns every `Range` header into a `range=` query parameter, adds `rn=<incrementing>` and `TE: trailers`, and sets no chunk cap.
  - `st` leaves ranges to ExoPlayer's index-driven requests.
  - Obs 11-23: a 64 KiB `range=0-65535` query on VISIONOS returned exactly 65536 bytes [ytdlp]. `range=0-9999` with an unsolved `n` returned 403 [st].
- **What is untested:** whether googlevideo serves a long **open-ended** `Range` at full speed for VISIONOS/TV URLs, or throttles or cuts it. With `n` solved (or absent), the classic throttling is gone in principle. This is the most important probe (section 12).
- **Expiry:** URLs expire after about 6 h (`expiresInSeconds` 21540 [ytdlp, st obs]). A pause longer than that breaks seeking, and the app must call `episode-url` again. `gv` reloads the player response on a 403 when `expire` is past.

### 10.2 Is the relay needed?

The relay without a filter passes `Range` through and can add a browser handshake. It cannot split one range into several [contract]. With a filter, it reads whole responses and drops `Range`, which is unusable for files of hundreds of MB.

| Need | Without relay | With relay |
|---|---|---|
| Custom headers (`User-Agent`, `Origin`) | `playback.headers`, which mpv sends on every request | not needed |
| Browser TLS handshake | not reported as needed for googlevideo (`ytdlp` impersonates only for captions) | `relay{browser: chrome}` if a probe shows 403s that a handshake fixes |
| Random 403s | mpv fails | the relay retries a 403/502/503 **once** [contract], which is cheap insurance |
| Chunking open-ended ranges | not possible | **not possible** either |
| n rewriting inside live HLS | not possible | `filter` (9.4) |

**Recommendation:**
- **v1:** no relay for VOD and live by default.
- **If the probe shows that open-ended ranges are throttled or cut:**
  1. switch VOD to the client's `hlsManifestUrl` (VISIONOS returns one), which mpv fetches in small segments. The cost is quality: muxed HLS may have lower-quality audio [ytdlp];
  2. or ask for a contract change: a relay option that caps each upstream request at N bytes and splits mpv's ranges.

  **Decision for the user.**
- **If random 403s show up mid-stream:** enable `relay{browser: none, filter: false}` for its single retry.

---

## 11. Failure modes and churn

### 11.1 What breaks, how often, and how it shows

| Area | How often (evidence) | Symptom | Detection |
|---|---|---|---|
| Client versions / UAs / token or SABR enforcement waves | every few weeks: ANDROID_VR added 06-02, dead 08-17; VISIONOS added 07-09; TV Samsung 10-10; IOS_PRODUCER 11-20; WEB_EMBEDDED policy 11-14 [ytdlp, gj] | adaptive formats without URLs; 403 on media; `LOGIN_REQUIRED` bot check | no URL in formats; probe 403 |
| n transform | the most frequent break: `gj` fix 11-13; EJS 7 fixes 2025-10 to 2026-03, then stable to 2026-10-26; `st` TCL churn 2026-10 | 403 or throttled media; solver exception | self-test fails; probe 403 |
| Player variant / STS format | 2026-10 (TCL, 8-digit) [st]; `ytdlp` pinned a player in 2026-03, then reverted | `UNPLAYABLE` "The page needs to be reloaded." | that reason text |
| visitorData / bot check | ongoing [ytdlp obs] | `LOGIN_REQUIRED` "Sign in to confirm you're not a bot" | reason text |
| Rate limit | per IP/session, up to 1 h [ytdlp, wiki] | "This content isn't available, try again later"; HTTP 429; captcha | reason text; status 429; `playerCaptchaViewModel` |
| IP block | rare | `videoDetails.videoId` differs from the request in every client [ytdlp] | id mismatch |
| Catalog renderer migration | `lockupViewModel` rollout since 2024-04; `ytdlp` tab fixes 2026-06/08; `st` Shorts parsing 2026-04 and 2026-11 | empty series; missing durations or dates | zero items parsed from a non-empty page |
| Browse params staleness | `gj` disabled its browse API 2026-09-04 (reason unknown) | a tab that is not `selected` | the `selected` flag |
| PO-token acceptance (only with milestone 7) | 2026-07, 2026-08, 2026-11 [pot] | GVS 403 after about 60 s, or at once | GVS probe |
| OAuth | `ytdlp` removed it; `st` working 11-23 | 401 on token/refresh; `LOGIN_REQUIRED` while signed in | status codes |
| TV DRM experiment | ongoing for `tv` without cookies [wiki] | every format has `drmFamilies` | the field |
| Pre-roll ad wait | `ytdlp` enforces waiting for the summed pre-roll ad duration before download since 2026-06 (failure mode unknown) | unknown, possibly a 403 at start | not observed; see 13 |

### 11.2 Mapping to the contract's `error` variants

| Situation | Return |
|---|---|
| A `channels`/`playlists`/`live`/`videos` entry that cannot be resolved (all entries of the field fail), a malformed `quality`/`max-episodes` | `setting("<key>")` |
| Expired or revoked OAuth/cookies; age, members-only or private content when not signed in, or signed in without the rights | `auth-failed` |
| A final HTTP error from YouTube that is not otherwise classified (`403` from all clients' probes, `404` for a missing playlist page) | `http(<status>)` |
| Rate limit, captcha | `http(429)` |
| `catchup-url` in v1 | `http(501)` |
| A failed connection from `http.send` | `network(<message>)` (pass it through) |
| Unplayable with a reason (removed, geo-blocked, not live yet, SABR-only everywhere, the solver failed for every client), or unparseable JSON | `bad-response("<YouTube reason + subreason>")`, in English, with the reason first |

**Log** every client attempt at `debug`, with the client name, status and reason, and each fallback at `info`. The user can then report breakage with context.

### 11.3 Retry and fallback order (one `*-url` call)

1. **Start:** load visitorData from storage, or create it (1 request).
2. **For each client** in the 3.3 order, starting with the last good one:
   1. **`/player`.**
      - On a network error or 5xx: retry once.
      - On 403 or 429: no retry (`ytdlp`); go to the next client, or stop on 429.
   2. **Bot check:** refresh visitorData once (at most once per call), then retry this client.
   3. **`UNPLAYABLE`/age/login:** classify as in 5.2 and go to the next client, or to a logged-in TV client.
   4. **Solve the challenges** if needed (6.4).
      - On a solver failure, mark the player bad and go on to the JS-free clients.
   5. **Probe the audio URL** (10.1).
      - On 403, go to the next client. When signed in, retry the same client once without the auth headers before moving on (the 60 s token-cache reset in `st` is not relevant without tokens).
3. **All clients failed:** return the most specific error seen. Priority: auth > rate limit > geo/removed > SABR-only > the last HTTP status.
4. **Success:** persist the client as last good.

**Budget:** typically 1 to 3 requests and under 1 s on the JS-free path. With the JS path: one 3 MB download plus 0.3 s on a cache miss.

---

## 12. Live probe results

The full write-up is in [observations/2026-11-23-clients.md](observations/2026-11-23-clients.md): 133 anonymous requests on 2026-11-23, 09:51-10:52 UTC, with no PO token.

| Client | Playable logged out with no token | Notes |
|---|---|---|
| `visionos` | **Yes, end to end (VOD)** | URLs are already signed, with no `n`. Up to 2160p60 VP9/AV1, Opus/AAC, DRC and dubbed audio tracks, HLS with subtitles, `fmt=vtt` captions. An 11 min video downloaded completely in 10 MiB chunks; the rerun on a second video behaved the same. |
| `ios_producer` | **Yes, end to end (VOD)** | H.264 up to 1080p and AAC only, the original audio only, no captions. Full download and HLS both OK. |
| `tv_samsung` | No, without JS | Every URL carries `n`, and an unsolved `n` gets 403. The music video comes back with `signatureCipher`. |
| `mweb`, `web_safari` | No | Their adaptive formats are SABR-only. `web_safari` gives HLS on the music video only, and it carries `n` (403). |
| TVHTML5 5.x (downgraded), `web_embedded` | No | "The page needs to be reloaded" / "This video is unavailable". |
| Live, every client | **Blocked** | "Sign in to confirm you're not a bot" on three 24/7 lives, so live HLS is untested. |

What this changes in the design:

- **Ranges.** `&range=` in the query and a `Range:` header are equally fast; the query form answers 200 and the header form a proper 206.
  - A request larger than about 11-16 MiB, including a whole file without a range, is never refused, but it is throttled to exactly twice the real-time rate.
  - So mpv can read the URLs directly, with no relay. 10 MiB chunks are faster, but a relay that splits ranges is optional.
- **IP-wide bot check.** Twice, right after live `/player` calls, the whole IP got the bot check on ordinary videos too.
  - It lasted 12-32 minutes the first time, and fresh visitor data did not clear it.
  - URLs and captions already handed out kept working throughout.
  - The plugin must treat that error as "back off" (section 11.3) rather than cycle through clients. It should also avoid probing live channels at catalog time.
- **Milestone 1 holds.** `visionos` first, `ios_producer` as the fallback, no JS engine and no PO token. Live needs a separate investigation; it is its own milestone.

---

## 13. Open questions (prioritised)

**P0: they decide the v1 design.**
1. **Do VISIONOS adaptive URLs play full length in mpv without a PO token**, with an open-ended `Range` and no throttling or cut-off? Only a 64 KiB range was tested [ytdlp]. `android_vr` had "intermittent POT enforcement" before it died [ytdlp].
2. **Does mpv accept these URLs with only `User-Agent` in `headers`,** and is `Origin`/`Referer` needed? Is a browser TLS handshake ever needed for googlevideo?
3. **Which visitorData source is cheapest and stable** (`/visitor_id`, a `/player` response, or the homepage), and for how long does one stay valid across calls?
4. **Does VISIONOS return a live `hlsManifestUrl`,** and does IOS_PRODUCER? Do live HLS variant or segment URLs carry `n` or need `pot`?
5. **Does the app show `catalog.account.status`?** If not, is a placeholder series an acceptable way to show the device code, or should the contract grow a message surface (8.2)?

**P1: needed for robustness (milestones 3 to 4).**

6. **Does TVHTML5 5.x Samsung return plain or ciphered URLs today, and does it need a TV PO token?** `ytdlp` says none; `st` flags one. Why did `ytdlp` switch from `tv_downgraded` to `tv_samsung`?
7. **Do the main `base.js` n/sig results apply to TV 5.x URLs?** `ytdlp` assumes so.
8. **Rust-side AST (oxc) plus whole-player eval, or EJS vendored in QuickJS?** EJS's parse time in QuickJS is unmeasured.
9. **Does the "alr"/"yes" anchor survive?** No EJS test vector goes past STS 20529; the live STS is 20774. It worked on 2026-11-23 only in the sense that the literal is present.
10. **Disagreements to settle:** the `TVHTML5_SIMPLY` id (74 or 75); the VISIONOS version (1.02 or 1.04) and UA (Safari-style or app-style); the playlists-tab `params`; the client id and `X-YouTube-Client-Name` header on VISIONOS (`gj` omits it).
11. **The channel avatar path** for the series poster. Not documented by the sources; candidates are the header avatar in `ytInitialData` or the RSS feed (which has none).

**P2: login (milestone 6).**

12. **Does a TV OAuth bearer token alone get adaptive URLs** from TVHTML5 5.x/7.x, without a living-room PO token [st open question]?
13. **Does YouTube enforce `slow_down` on device-code polling?**
14. **Cookie rotation in a plugin with only its own requests:** is writing `Set-Cookie` back enough to keep `__Secure-1PSIDTS` alive?

**P3: later features.**

15. **Do YouTube `sq` live segments play as HLS segments** (fMP4/WebM, init handling) for a synthesised catch-up playlist (9.3)? Can `relay.filter` legitimately serve a synthesised playlist?
16. **Does the HLS live manifest expose a DVR window?**
17. **What happens when media is fetched before the pre-roll ad wait elapses** (`ytdlp` enforces the wait; the failure mode is unknown)?
18. **BotGuard in QuickJS:**
    - Which APIs does the `program` probe?
    - How long does the VM take in an interpreter?
    - Is the 88-versus-92-character integrity-token length a reliable good/bad signal?
    - Can a stored integrity token feed a fresh VM's minter?
    - What is `T` in `ytAtN`?
    - Is the GVS binding the video id or the visitorData?
19. **Which token-gated clients return plain URLs once given a token** (TV 7.x with a living-room token, `tv_simply`, `web_embedded`)?
20. **Translation and Premium:** subtitle translation (`tlang`) as extra tracks; Premium itags (`616`) through a logged-in TV client.

---

## 14. Proposed milestones

Each milestone ends with a plugin that loads and plays something more than the last.

**M1: VOD through VISIONOS, RSS catalog (smallest useful slice). No JS engine.**
- **Fields:** `channels` and `videos`.
- **Catalog:** resolve `@handle`/URL to a UC id (HTML canonical or `externalId`, cached). Each channel becomes a series built from its RSS feed: exact `aired`, thumbnail, title, plot, no duration. Videos become movies.
- **visitorData** bootstrap, cached.
- **`episode-url`/`movie-url`:** VISIONOS `/player`, video-only and audio tracks (5.5, 5.6), UA header, tail-range probe. Fallbacks: IOS_PRODUCER, then the VISIONOS `hlsManifestUrl`.
- **Errors:** the 5.2 classification, mapped as in 11.2.
- **Exit criterion:** a 10-minute video plays to the end and seeks in mpv.

**M2: full catalog.**
- Uploads through the `/videos` (and `/streams`) HTML tab plus `/browse` continuations, the `lockupViewModel` parser with the older renderers as fallbacks, duration badges, and relative dates merged with RSS.
- `max-episodes` and `tabs`.
- Playlists (`playlists` field).
- The `UU` fallback.
- Self-healing tab params.
- First-page caching.

**M3: live.**
- The `live` field.
- `chlive:` channels and live-now items from the streams tab.
- HLS from VISIONOS, then IOS_PRODUCER.
- Offline and upcoming messages.
- An optional `programmes` entry.

**M4: player script and challenges.**
- rquickjs in sloppy mode.
- Player id from `iframe_api`, the STS.
- The candidate search with a Rust parser and whole-player evaluation.
- n and sig solving with a self-test.
- The storage caches (6.7).
- This enables TVHTML5 5.x Samsung and downgraded in the client order, and `/n/` rewriting in HLS manifests.

**M5: captions.**
- Caption tracks as `tracks` (VTT, `xosf` removed, `exp`-gated tracks skipped).
- Audio-track polish (auto-dubbed ordering, DRC option).

**M6: login.**
- TV device code with the 8.2 surfacing and token refresh.
- Brand accounts through `accounts_list` and `X-Goog-Pageid`.
- Logged-in TV client for gated videos.
- Watch Later, Liked, library playlists and subscriptions as series.
- Cookies/SAPISIDHASH as an alternative.

**M7 (spike, gated by 7.6): BotGuard in QuickJS.**
- Homepage challenge, DOM/navigator stubs, host timers, `GenerateIT`, mint, a GVS probe, and the bad-token re-mint loop.
- Ship only if a token unlocks a plain-URL client that the token-free ones no longer cover.

**M8: catch-up and DVR.**
- `catchup-window` from `maxDvrDurationSec`.
- A synthesised `sq` playlist through `relay.filter`, if the M8 probe shows the segments play.
