---
name: brain
description: Use and maintain the brain/ knowledge base - the committed docs describing how Halcyon works (architecture, the host contract, YouTube as observed, the clean-room specs, features, decisions, glossary). Invoke when the user asks how/where something works ("which client plays a video", "what happens on the bot check", "how long is visitorData kept"), when onboarding, or when asked to update, audit, or fix the brain after a change. Read brain/BRAIN.md first; navigate via the indexes, not a whole-tree grep.
---

# The brain

`brain/` is the committed knowledge base: how Halcyon works, written so you can answer without reading the source,
plus the clean-room specification of YouTube's protocol that the code is written from.
Use it as the first stop for understanding, and keep it true to the code and to YouTube.

## Navigating (to answer a question)

1. Open `brain/BRAIN.md` - the entry index. It has a topic table and a **Find by question** table.
2. Drill via the sub-indexes: `brain/architecture/index.md` (the how) and `brain/features/index.md` (the what).
   Don't grep the whole tree; the indexes are the routing layer.
3. Land on the leaf doc. It is self-contained and ends with a `## Sources` list of the real code paths - follow
   those only if the doc isn't enough or you suspect drift.
4. Terms -> `brain/glossary.md`. Rationale / "why" -> `brain/decisions.md`. What YouTube answers today ->
   `brain/architecture/youtube.md` (short dated facts) and `brain/specs/observations/` (dated probe write-ups).
   The protocol as the sources describe it -> `brain/specs/SPEC.md`, merged from the facts-only notes in
   `brain/specs/sources/`.

A doc whose title says **(Planned)** describes the agreed design, not code, and names the milestone it lands with.
Say so when you answer from it.

Structure:

```
brain/BRAIN.md                 root index + find-by-question
brain/glossary.md  decisions.md
brain/architecture/index.md    overview host-contract youtube challenges testing distribution
brain/features/index.md        catalog playback live login player-features
brain/specs/                   BRIEF SPEC sources/ observations/
```

## Clean room

The rules are in `CLAUDE.md` ("Clean room") and `brain/decisions.md` (decision 2). They bind every brain edit:

- Halcyon's code is written only from `brain/specs/` and from live observation of YouTube.
- **The third-party source code is never read.** Never open, copy, search or cite the code of yt-dlp, Grayjay,
  SmartTube, YouTube.js or BgUtils, nor any clone of it, here or in a scratch directory. The notes in
  `brain/specs/sources/` were written from that code by separate researchers, as facts only; they are the only way
  that code reaches the project.
- **A new protocol fact comes only from dated observation.** Probe YouTube (anonymous, small, spaced, see
  `brain/architecture/testing.md`), then write the fact with its date in `brain/architecture/youtube.md`, or a full
  write-up in `brain/specs/observations/<date>-<topic>.md` when it took more than a request or two.
- Never edit `brain/specs/sources/*.md` from your own knowledge or observation: they record what a research pass
  found. A question only a source can answer goes to the user as a request for a new facts-only research pass.

## Maintaining (after a change)

The rule is in `CLAUDE.md` ("The brain"). The code is the source of truth for the docs, and YouTube is the source of
truth for the code - if a doc disagrees with reality, fix the doc.

- A change that makes a brain doc wrong fixes that doc on the **same branch**.
- A **(Planned)** part that lands -> drop the marker from its title and from the index rows (`BRAIN.md`, the two
  `index.md`), replace "none yet, lands with Mx" in `## Sources` with the real source paths, and move its module row
  in `brain/architecture/overview.md` from "Modules to come" to "Modules today".
- A feature's behaviour changed (catalog, playback, live, login, player features) -> its doc in `brain/features/`
  (`catalog.md`, `playback.md`, `live.md`, `login.md`, `player-features.md`), and the README's usage table when a
  field changed.
- A storage key, value or lifetime added or changed -> the "What is kept between calls" table in
  `brain/architecture/overview.md`. A lifetime is never longer than what it keeps stays valid.
- A new fact about YouTube, or a fact that changed -> dated, in `brain/architecture/youtube.md` (short fact) or a new
  `brain/specs/observations/<date>-<topic>.md` (probe write-up: date and time, network, tooling, budget, method,
  results, verdict). Mark it **Corrected** when it contradicts an earlier entry, and leave the earlier observation
  file as it was: it records what was seen that day.
- A fact that answers or changes an open question of `brain/specs/SPEC.md` section 13 -> the matching Q entry in
  `TODO.md`. `SPEC.md` itself changes only with a new research pass or to point at an observation that overrides it.
- `wit/provider.wit` re-vendored -> `brain/architecture/host-contract.md` and the commit in the WIT header.
- The n or signature solver, the player script or rquickjs changed -> `brain/architecture/challenges.md`.
- Release or CI changed -> `brain/architecture/distribution.md`. The gate, the coverage floor, fixtures or the probe
  etiquette changed -> `brain/architecture/testing.md`.
- New non-obvious decision -> append it to `brain/decisions.md` with its "why"; numbers are stable, a reversed
  decision is marked, not deleted. New term -> `brain/glossary.md`.
- Style: dense, skimmable, present tense, normal prose. No em-dashes. Cross-link siblings with relative markdown
  links. Cite source paths instead of restating code. End each doc with `## Sources`.

Before committing, check every relative link in the touched docs resolves:

```bash
python3 - <<'PY'
import glob, os, re
for f in glob.glob("brain/**/*.md", recursive=True):
    for m in re.finditer(r"\]\(([^)#]+)", open(f).read()):
        t = os.path.normpath(os.path.join(os.path.dirname(f), m.group(1)))
        if not m.group(1).startswith("http") and not os.path.exists(t):
            print("broken:", f, m.group(1))
PY
```

## Auditing (drift check on request)

When asked to verify the brain matches reality:

1. Scope it to the relevant leaf doc(s) - don't re-audit everything.
2. For each, read the `## Sources` paths and confirm the doc's claims still hold (modules, storage keys and
   lifetimes, the client order, error mapping). Quick cross-checks: `grep -rn 'const ' src` and the tests next to the
   code.
3. For `youtube.md`, re-probe only when asked and only under the etiquette in `brain/architecture/testing.md`:
   anonymous, a set budget, one request at a time, stop on the bot check, lives last. Date what you saw.
4. Where they diverge, the code (or YouTube) wins: correct the doc. Note what you changed.
5. If a feature shipped or was removed, update the indexes (`BRAIN.md`, the two `index.md`) too.

For a broad audit, fan out read-only agents (one per architecture or feature doc) that each diff their doc against
its `## Sources`, then apply the fixes. Tell each agent the clean-room rule: brain, `src/`, `wit/` and YouTube only,
never third-party YouTube client code.
