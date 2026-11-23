# Halcyon

A [Nightglass](https://github.com/MarcDownHQ/nightglass) plugin, built as a WebAssembly component
(`wasm32-wasip2`), that brings YouTube channels, playlists, videos and lives into Nightglass, fully native: no
yt-dlp, no external runtime, no token server. Start with `brain/BRAIN.md` to find how anything works; `ROADMAP.md`
is the agreed backlog, `TODO.md` the small work and open decisions.

## The brain

`brain/` is the committed knowledge base: architecture, features, decisions, glossary, and the clean-room specs in
`brain/specs/`. Navigate it through `brain/BRAIN.md` and the two `index.md` files rather than grepping the tree.

- Keep it true to the code. A change that makes a brain doc wrong fixes that doc in the same branch.
- The code wins: if the brain disagrees with reality, correct the brain. YouTube wins over both: it changes without
  notice, and `brain/architecture/youtube.md` and `brain/specs/observations/` record what was observed and when.
- A doc for a part that is not built yet says **(Planned)** in its title and names the milestone it lands with. When
  the part lands, drop the marker and list the real source paths.

## Clean room

Halcyon's code is written only from `brain/specs/` and from live observation of YouTube.

- The notes in `brain/specs/sources/` were written by separate researchers from the code of yt-dlp, Grayjay,
  SmartTube, YouTube.js and BgUtils, as facts only. Grayjay is under a non-MIT licence, so this separation is
  required, not a courtesy.
- Never read, copy, search or cite those projects' code, nor any clone of it, here or in a scratch directory.
- A new protocol fact comes from observing YouTube: date it and write it in `brain/specs/observations/` (a probe
  write-up) or `brain/architecture/youtube.md` (a short fact).
- To learn something only a source knows, ask the user for a new facts-only research pass. Do not go and look.

## Invariants

- **The contract is `wit/provider.wit`** (`nightglass:provider@0.3.0`), vendored from Nightglass; its header names the
  commit. Only `src/wasm.rs` touches wit-bindgen; everything else is plain Rust that builds and is tested natively.
- **Each call runs in a fresh instance.** Anything kept between calls goes through storage, with a time to live no
  longer than what it keeps stays valid (format URLs about 6 h, an access token its `expires_in`). A storage
  failure never fails playback. During `describe`, storage is unavailable.
- **Fully native.** No external runtime, no solver service, no token server. Whatever JavaScript YouTube makes a
  client run, runs inside the plugin.
- **Back off on the bot check.** "Sign in to confirm you're not a bot" means stop and report, not try every client in
  turn: the probe showed it can cover the whole IP for up to half an hour. Never cycle through clients to get past
  it.
- **Never log in during probes.** Live checks against YouTube are anonymous, small and spaced out (see
  `brain/architecture/testing.md`).
- **No DRM.** A format with `drmFamilies` is dropped, never worked around.

## Conventions

- `CHANGELOG.md` `[Unreleased]` gets an entry with every user-facing change; it becomes the release notes.
- The Rust toolchain is pinned in `rust-toolchain.toml` and `.mise.toml` (with the `wasm32-wasip2` target); bump both
  together.
- Before finishing: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo clippy --target wasm32-wasip2 --release -- -D warnings`, `cargo test`. CI runs the same and builds the
  component.
- Line coverage stays at or above 98% (`scripts/coverage.sh --check`, a CI job). New code comes with tests that keep
  it there; `scripts/coverage.sh` ranks the files with the most uncovered lines.
- Comments only for a constraint the code cannot show. No em-dash anywhere.
- Releases go through the `release` skill.
