# Halcyon brain

How Halcyon works, written so a developer or an agent can answer questions without reading the source. Navigate
through the indexes; every leaf doc ends with the source paths it describes, or with the milestone it lands with
when the part is still **(Planned)**. The code is the source of truth: if a doc disagrees with it, fix the doc.
YouTube is the source of truth for the code: what it does was observed, with a date, in
[architecture/youtube.md](architecture/youtube.md) and [specs/observations/](specs/observations/2026-11-23-clients.md).

Halcyon is a Nightglass provider plugin. Nightglass loads `halcyon.wasm`, asks it for its catalog (channels and
playlists as series, lives as channels, single videos as movies), and calls it again each time something is played;
Halcyon then asks YouTube's player API for fresh media URLs.

## Topics

| Doc | What it covers |
| --- | --- |
| [architecture/index.md](architecture/index.md) | How it is built: modules, the host contract, YouTube, the challenges, tests, releases |
| [features/index.md](features/index.md) | What it does: the catalog, playback, lives, login, the player features |
| [specs/SPEC.md](specs/SPEC.md) | The clean-room specification of YouTube's protocol, merged from the source notes |
| [glossary.md](glossary.md) | The terms the code and the docs use |
| [decisions.md](decisions.md) | Non-obvious choices and why they were made |

## Find by question

| Question | Go to |
| --- | --- |
| Which module does what, and what compiles to wasm? | [architecture/overview.md](architecture/overview.md) |
| What can a plugin do, and what are the host's limits? | [architecture/host-contract.md](architecture/host-contract.md) |
| What does YouTube answer today, and when was that observed? | [architecture/youtube.md](architecture/youtube.md) |
| Which Innertube clients work without a PO token or JavaScript? | [architecture/youtube.md](architecture/youtube.md), [specs/SPEC.md](specs/SPEC.md) section 3 |
| Where did the protocol knowledge come from, and what may I read? | [decisions.md](decisions.md), [specs/BRIEF.md](specs/BRIEF.md) |
| How are the n and signature challenges solved? | [architecture/challenges.md](architecture/challenges.md) |
| Why no BotGuard, and when would it come? | [decisions.md](decisions.md), [architecture/challenges.md](architecture/challenges.md) |
| How do channels and playlists become series? | [features/catalog.md](features/catalog.md) |
| Which client plays a video, and how are video, audio and subtitles picked? | [features/playback.md](features/playback.md) |
| What happens on "Sign in to confirm you're not a bot"? | [features/playback.md](features/playback.md), [architecture/youtube.md](architecture/youtube.md) |
| Why do lives not play yet? | [features/live.md](features/live.md) |
| How does sign-in work, and how is the code shown? | [features/login.md](features/login.md) |
| Quality, subtitle and chapter selection? | [features/player-features.md](features/player-features.md) |
| What does Halcyon keep in storage, and for how long? | [architecture/overview.md](architecture/overview.md) |
| How is it tested, and how do I probe YouTube without getting blocked? | [architecture/testing.md](architecture/testing.md) |
| How is a release made? | [architecture/distribution.md](architecture/distribution.md) |
