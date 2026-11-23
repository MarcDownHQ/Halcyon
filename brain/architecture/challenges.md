# Challenges: the player script, n and signature (Planned)

What M4 builds, from [SPEC](../specs/SPEC.md) section 6 and the engine measurements in
[sources/js-engine.md](../specs/sources/js-engine.md). M1 to M3 need none of it: `visionos` and `ios_producer` hand
out URLs with no `n` and already signed.

## When it is needed

- TVHTML5 5.x clients (Samsung, downgraded), whose URLs carry `n` and sometimes a `signatureCipher`.
- `/n/<v>/` path segments in HLS manifests from JS clients.
- Signed-in playback (M6), which goes through TV clients.

## The plan

1. **Player id** from `https://www.youtube.com/iframe_api` (the 8-hex-digit id after `player\/`), kept an hour. The
   id is assigned per request and can differ between two calls on the same day, so every cache is keyed by player
   id.
2. **The script**: `https://www.youtube.com/s/player/<id>/player_ias.vflset/en_US/base.js`, about 3 MB. Retried 3
   times; a failed or partial download is never cached.
3. **STS**: the digits after `signatureTimestamp`; 5 digits for the main and non-TCL TV players, the same plus `001`
   for the TCL variant. Parse any run of digits. Sent as
   `playbackContext.contentPlaybackContext.signatureTimestamp`; a wrong one gives "The page needs to be reloaded."
4. **Finding the builder.** A Rust-side ES parser (MIT or Apache, `oxc` the candidate) finds the player's wrapper and
   the functions that build a stream URL object and set the pair `"alr"`/`"yes"` on it. One statement appended to
   the wrapper exposes them on a global.
5. **Running it** in rquickjs, sloppy mode, with about 20 lines of browser stubs, under memory and stack limits. The
   whole player evaluates in about 0.3 s and 36 MiB inside wasmtime.
6. **Driving it**: call each candidate with a dummy googlevideo URL carrying the `n` value and the signature, call
   the returned object's first own method, read back `n` and the signature. All candidates must agree. A result
   starting with `enhanced_except_` is a failure.
7. **Batching**: every distinct `n` and signature length of one answer, solved in one engine run.
8. **Self-test** on each new player id: a sample `n` and a sample signature must come back changed and non-empty. A
   failure sets the player aside for an hour; the JS-free clients carry on.

## Caches

Solved `n` values (6 h), signature position maps per length (30 days), the STS (30 days), and the prepared player as
QuickJS bytecode split into chunks under 1 MiB (7 days, the engine version in the key, length and hash checked
before loading). Only the newest one or two players are kept, to stay under 16 MiB. See
[overview.md](overview.md).

## Why our own solver

yt-dlp's EJS could run in QuickJS and its licence allows it, but it would be vendored code rather than clean-room
work, and its parse cost in QuickJS is unmeasured. See [decisions.md](../decisions.md).

## Sources

None yet; lands with M4.
