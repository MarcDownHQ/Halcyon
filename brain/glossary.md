# Glossary

**Innertube**: YouTube's internal API, `https://www.youtube.com/youtubei/v1/<endpoint>` (`player`, `browse`,
`next`, `search`...), JSON in and out, what YouTube's own apps call.

**Innertube client**: the app identity a request claims: `clientName` and `clientVersion` in `context.client`, the
numeric `X-YouTube-Client-Name`, a User-Agent and device fields. Each client gets different formats and checks:
`visionos`, `ios_producer`, `tv_samsung` (TVHTML5 5.x), `mweb`, `web_safari`...

**visionos, ios_producer**: the two clients that played whole videos logged out with no PO token and no JavaScript on
2026-11-23; Halcyon's first and second choice.

**visitorData**: an opaque visitor id from `responseContext.visitorData` of an anonymous answer, sent back in
`context.client.visitorData` and `X-Goog-Visitor-Id`. Without it `/player` answers the bot check. Must come from
YouTube; locally made ones are rejected.

**Bot check**: `playabilityStatus` `LOGIN_REQUIRED` with "Sign in to confirm you're not a bot". It can cover a whole
IP for half an hour; Halcyon backs off rather than trying other clients.

**playabilityStatus**: the `/player` answer's verdict: `OK`, `LOGIN_REQUIRED`, `UNPLAYABLE`, `ERROR`,
`LIVE_STREAM_OFFLINE`, with a `reason` and sometimes a subreason.

**streamingData**: the part of a `/player` answer that lists `formats`, `adaptiveFormats`, `hlsManifestUrl` and
`expiresInSeconds`.

**GVS**: Google Video Server, `*.googlevideo.com`, which serves the media itself. A URL refused there is a 403 that
mpv only hits when it reads.

**itag**: the number naming a format: a container, codec and quality (137 is H.264 1080p, 251 Opus audio, 18 muxed
360p).

**Adaptive format**: a video-only or audio-only stream; playing one needs the other as a track. A **muxed** (or
progressive) format carries both, today only itag 18.

**DRC**: dynamic range compression, a "normalised" audio variant sharing its itag with the plain one, marked `isDrc`.
Dropped unless asked for.

**audioTrack**: the language and kind of an audio format (`id` like `en-US.4`, `displayName`, `audioIsDefault`,
`isAutoDubbed`): original, dubbed, auto-dubbed, descriptive.

**xtags**: `key=value` pairs in a format URL describing the audio (`lang`, `acont`), DRC, voice boost or upscaling.

**PO token**: a proof-of-origin token produced by BotGuard (web), DroidGuard (Android) or iOSGuard (iOS), bound to a
video id or visitor, required by some clients for `/player` or for GVS. Not portable between those families.

**BotGuard**: Google's obfuscated JavaScript VM that produces web PO tokens after a challenge, an attestation step
(`GenerateIT`) and a mint. Every known way to run it uses a DOM; Halcyon leaves it out of v1.

**SABR**: Server ABR, YouTube's stateful streaming protocol, POST requests with protobuf bodies, framed as **UMP**.
Formats offered only through it have no URL; mpv cannot play them and Halcyon drops them.

**Player script, base.js**: YouTube's web player, about 3 MB of JavaScript, named by an 8-hex-digit **player id**.
The code that solves the challenges lives in it.

**n challenge**: the `n` query value (or `/n/<v>/` path segment) of a media URL that must be replaced by a transform
computed by the player script. Unsolved, GVS answers 403 or throttles.

**sig challenge**: a `signatureCipher` whose `s` must be transformed by the player script and appended as `sp`
(usually `sig`). A fixed reordering that depends only on the input length.

**STS**: signature timestamp, `signatureTimestamp` in the player script, sent with `/player` by clients that use the
script; 5 digits for the main player, 8 for the TCL TV variant. A wrong one gives "The page needs to be reloaded."

**TCL variant**: the TV player used by TVHTML5 7.x, with an 8-digit STS and a different n transform interface.

**EJS**: yt-dlp's JavaScript challenge solver. Halcyon writes its own instead.

**rquickjs**: Rust bindings to QuickJS-NG, the JavaScript engine Halcyon embeds for the player script.

**hlsManifestUrl**: an HLS master playlist on `manifest.googlevideo.com`, offered for lives and, by some clients, for
VODs.

**Tail-range probe**: a small range request near the end of the chosen audio format, to check a client's URLs before
handing them to mpv.

**ytInitialData, ytcfg**: the JSON a YouTube HTML page embeds: the page's content, and its configuration
(`INNERTUBE_CONTEXT`, `VISITOR_DATA`, `STS`).

**lockupViewModel**: the current renderer of a video or playlist in listings; older ones are `videoRenderer`,
`gridVideoRenderer`, `playlistVideoRenderer`.

**Continuation**: the token at the end of a listing page that `/browse` takes to return the next page.

**UC id, handle**: a channel's id (`UC` plus 22 characters) and its `@name`. **UU playlist**: the channel's uploads as
one playlist, `UU` plus the id without `UC`.

**Device code**: the TV sign-in flow: the plugin gets a short code, the user enters it at youtube.com/activate, the
plugin polls for tokens.

**Brand account**: a channel identity under a Google account, selected with `X-Goog-Pageid`.

**Relay, filter**: Nightglass fetching a stream for the player on 127.0.0.1, optionally passing every answer through
the plugin's `filter` export.

**Track**: a stream played alongside `playback.url`: an audio language, subtitles.

**Clean room**: the split between the researchers who read third-party code and wrote facts, and the implementation,
which reads only those facts and YouTube's own answers.
