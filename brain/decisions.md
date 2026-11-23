# Decisions

Each entry is a choice a reader might want to undo, with the reason not to. Numbers are stable: a reversed decision
is marked as such, not deleted.

## 1. Fully native

Everything runs inside the plugin: no yt-dlp, no Deno or Node, no solver service, no PO-token server, nothing for the
user to install or run beside Nightglass. Any JavaScript YouTube makes a client run is run by an engine compiled into
the component. The cost is doing in Rust and an embedded engine what other projects get from a browser or Node; the
gain is a single file that works anywhere Nightglass does.

## 2. Clean room: specs from sources, code from specs and YouTube

Halcyon is MIT. The protocol knowledge comes from five projects (yt-dlp, Grayjay, SmartTube, YouTube.js and BgUtils),
one of them, Grayjay, under a licence that does not allow reuse. So separate researchers read each project and wrote
facts only (endpoints, JSON paths, header values, client constants, behaviour in prose, no code) under the rules in
[specs/BRIEF.md](specs/BRIEF.md); the notes are in `specs/sources/`, merged into [specs/SPEC.md](specs/SPEC.md).
The implementation reads only those notes and YouTube's own answers, never the projects' code. New facts come from
probing YouTube, dated, in `specs/observations/` or [architecture/youtube.md](architecture/youtube.md); a question
only a source can answer goes back to a new facts-only research pass.

## 3. visionos first, ios_producer second

The probe of 2026-11-23 played whole videos through both, logged out, with no PO token and no JavaScript: signed URLs
with no `n`. `visionos` offers more (2160p60, VP9 and AV1, dubbed audio, DRC, captions); `ios_producer` covers
made-for-kids videos `visionos` refuses. yt-dlp and Grayjay put a TV client first, probably for format coverage, but
that client needs the player script; starting with the JS-free pair skips a 3 MB download and an engine run on
every play, and lets M1 ship with no engine at all.

## 4. rquickjs as the engine

The candidates were built and measured inside wasmtime with the host's limits ([specs/sources/js-engine.md](specs/sources/js-engine.md)).
rquickjs (QuickJS-NG, MIT) builds for `wasm32-wasip2` unpatched, adds about 0.8 MB, evaluates the whole player in
about 0.3 s and 36 MiB, and can cache bytecode. Boa needs about 258 MiB for the player and fails at 256 MiB;
nova_vm does not build; javy and quickjs-wasm-rs add nothing over rquickjs.

## 5. Our own solver, not EJS

yt-dlp's EJS solver is Unlicense and could run in QuickJS, but it would be vendored third-party code inside a
clean-room project, and its parse cost in QuickJS (a JavaScript parser and code generator over 3 MB) is unmeasured.
Halcyon writes its own: a Rust-side ES parser finds the player's stream-URL builder from the shape described in the
spec, and rquickjs runs the whole player with that builder exposed. See
[architecture/challenges.md](architecture/challenges.md).

## 6. No BotGuard in v1

The clients that work today need no PO token, and the clients that need one are SABR-only, so a token would not
bring mpv-playable URLs anyway. Running BotGuard without a DOM has never been done, a wrong environment produces
tokens that fail silently at GVS, and its challenge source has moved twice in 2026. It stays a gated spike (M7),
started only if the decision tree of SPEC 7.6 reaches it.

## 7. Login at M6

An anonymous plugin is useful on its own, and login brings risk to the user's account (automated traffic can get it
banned) for no gain on public videos. It comes after the catalog, lives, challenges and the player features, since
the logged-in clients are TV clients that need the M4 solver.

## 8. The sign-in prompt through the contract

The device-code flow needs to show the user a code and an address. The spec's workaround puts them in
`catalog.account.status` and a fake series; that misuses both and depends on whether the app shows the status.
Halcyon instead proposes a contract extension to Nightglass (a sign-in prompt a plugin can raise and clear), on the
0.4.0 wish list in [TODO.md](../TODO.md).

## 9. The name

Halcyon was chosen by the user: the halcyon is the kingfisher of legend, the bird of calm weather. It keeps the
YouTube trademark out of the plugin's name. The plugin id `halcyon` fits the host's lowercase rule; it replaces the
`youtube` id the spec proposed.

## Sources

- [src/wasm.rs](../src/wasm.rs)
- [src/provider.rs](../src/provider.rs)
- [specs/SPEC.md](specs/SPEC.md)
