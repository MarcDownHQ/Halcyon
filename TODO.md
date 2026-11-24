# TODO

Small, concrete work: fixes, chores, decisions to make before a roadmap item starts. Features go in
[ROADMAP.md](ROADMAP.md). Delete an entry when it is done. The numbers (Q1, Q2...) are those of the open questions in
[brain/specs/SPEC.md](brain/specs/SPEC.md) section 13.

## Before M1 (decide the v1 design)

- [ ] **Q1, full length in mpv.** The probe of 2026-11-23 downloaded whole `visionos` files in 10 MiB chunks, and an
      open-ended read was paced to twice real time but never refused. Still to see: mpv itself playing a 10-minute
      video to the end and seeking, with no PO token. This is M1's exit criterion.
- [ ] **Q2, headers.** The probe fetched media and HLS with only the client's User-Agent and no browser handshake.
      Confirm that mpv needs nothing more (`Origin`, `Referer`), then settle what `playback.headers` carries.
- [ ] **Q3, visitorData.** The probe took it from an anonymous WEB `/player` answer. Decide between that,
      `/visitor_id` and the homepage, and measure how long one stays valid across calls (SPEC suggests about 24 h).
- [ ] **Q10, small disagreements** that touch M1: the `visionos` version (1.02 or 1.04) and User-Agent (Safari-style
      or app-style), and whether to send `X-YouTube-Client-Name` for it. The probe used 1.02, the Safari UA and the
      header, and it worked.
- [ ] **Q11, channel avatar.** No source documents where the series poster comes from; look at the channel page's
      `ytInitialData` header.
- [ ] **Bot-check back-off.** How long to back off, and whether to keep that state in storage so the next call does
      not hit YouTube again at once. The probe saw blocks of 12 to 32 minutes.

## Before M3 (live)

- [ ] **Q4, live HLS.** Every live was bot-checked on 2026-11-23. The M3 investigation must find when an anonymous
      live `/player` works, which clients return `hlsManifestUrl`, and whether live playlists carry `n` or `pot`.
- [ ] **Q16, DVR window.** Does the live HLS manifest expose more than a few segments?

## Before M4 (challenges)

- [ ] **Q6, TVHTML5 5.x Samsung.** Plain or ciphered URLs today, and does it need a TV PO token? The probe saw `n`
      on every URL and a cipher on the music video.
- [ ] **Q7.** Do the main `base.js` n and signature results apply to TV 5.x URLs?
- [ ] **Q8 and Q9, the solver.** Decided: our own solver (Rust parser plus rquickjs), not EJS. Still open: which
      MIT or Apache ES parser (`oxc` is the candidate), and whether the "alr"/"yes" anchor still finds the builder in
      the current player (no test vector past STS 20529; the live STS was 20774).
- [ ] **Q10, the rest:** the `TVHTML5_SIMPLY` client id (74 or 75), the playlists-tab `params`.
- [ ] **Q17, pre-roll ad wait.** What happens when media is fetched before it elapses.

## Before M5 (captions, quality, chapters)

- [ ] Decide how quality is offered: several video-only formats as tracks, or the HLS master. See
      [chapters-quality-subs](brain/specs/sources/chapters-quality-subs.md) and the Quality menu of Nightglass 0.4.0.

## Before M6 (login)

- [ ] **Q12.** Does a TV OAuth bearer token alone bring adaptive URLs back from TVHTML5 5.x or 7.x, without a
      living-room PO token?
- [ ] **Q13.** Does YouTube enforce `slow_down` on device-code polling?
- [ ] **Q14, cookie rotation.** Is writing `Set-Cookie` back from the plugin's own requests enough to keep
      `__Secure-1PSIDTS` alive?
- [x] Q5 (does the app show `catalog.account.status`) is settled: contract 0.4.0 has the `action-required` prompt.

## Later (M7, M8)

- [ ] **Q15.** Do YouTube `sq` live segments play as HLS segments in a synthesised catch-up playlist, and can
      `relay.filter` legitimately serve one?
- [ ] **Q18 and Q19, BotGuard.** What the program probes, how long the VM takes in an interpreter, whether the
      integrity-token length (88 against 92) tells a good token from a bad one, whether a stored integrity token
      feeds a fresh VM, the GVS binding; which token-gated clients return plain URLs with a token.
- [ ] **Q20.** Subtitle translation and Premium formats.
