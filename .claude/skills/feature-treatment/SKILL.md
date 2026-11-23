---
name: feature-treatment
description: Ship a finished Halcyon feature branch the full way - rebase onto master, run a max-effort multi-agent code review (one lens per CLAUDE.md invariant, clean room included), fix everything confirmed, update the brain/ knowledge base, pass the gate (wasm and the 98% coverage floor included) and CI, merge keeping the layered commits, and cut a release when a milestone closes. Use when the user says "feature treatment", "treat this branch", "review and merge this feature", or names a worktree/branch to ship.
---

# Feature treatment

The pipeline a finished feature branch goes through before it lands: **rebase -> max review -> fix -> update brain
-> gate -> merge -> release -> clean up**. Nothing merges without an adversarial review and a green gate.

The branch comes from the argument (a branch or worktree name). If none is given, find it: `wt list` and
`git branch` - it is the non-`master` one; worktrees live in `~/repos/Halcyon.worktrees/<branch>`, created with
`wt switch --create <branch>`. Confirm which one if ambiguous. Branch names follow `feature/...`, `fix/...`; every
commit subject starts with `[<branch>]`.

The commits are layers: mechanical first (moves, renames, extractions), then behaviour, then its tests and docs. Each
builds and passes on its own. Review fixes go on top as new layers, never by rewriting reviewed ones.

## 1. Rebase onto master

```bash
cd ~/repos/Halcyon.worktrees/<branch>     # or the primary checkout if the branch is there
git fetch origin && git rebase origin/master
```

Resolve conflicts if any. The branch must sit directly on `master` so the review and the merge see only this
feature. Check the size: `git diff --name-only origin/master..HEAD | wc -l` must stay under 149; split along a seam
if it does not.

## 2. Max-effort review (parallel finders)

The diff: `git diff origin/master...HEAD` (exclude `Cargo.lock` from reading; a dependency change is reviewed from
`Cargo.toml`).

Spawn **independent finder subagents in parallel** (one Agent tool call with several invocations), each over the
same diff with a different lens. Give every finder the clean-room rule: it reads the diff, `src/`, `wit/`, `tests/`
and `brain/` only, and never opens third-party YouTube client code (yt-dlp, Grayjay, SmartTube, YouTube.js,
BgUtils or any clone) to check a claim. One finder per lens, scaled to what the diff touches:

- **Contract / WIT boundary** - `wit/provider.wit` is the contract. Only `src/wasm.rs` touches wit-bindgen, and it
  holds conversions, no logic (it is not covered). Everything else builds and is tested natively. The host's limits
  in `brain/architecture/host-contract.md` hold: stream addresses are network schemes, `describe` stays instant and
  touches no storage, `filter` passes media through untouched, calls fit their time limits.
- **Fresh instance and storage TTLs** - each call runs in a fresh instance, so nothing is assumed to survive in
  memory. Anything kept goes through `Storage` with a time to live no longer than what it keeps stays valid (format
  URLs about 6 h, an access token its `expires_in`), and matches the table in `brain/architecture/overview.md`. A
  failed `get` or `set` never fails a play. Keys within 256 bytes, values within 1 MiB.
- **Fully native** - no external runtime, no solver service, no token server, no yt-dlp, nothing for the user to run
  beside Nightglass. Any JavaScript YouTube makes a client run runs inside the plugin. A new dependency builds for
  `wasm32-wasip2` and does not shell out or call home.
- **Bot-check back-off** - "Sign in to confirm you're not a bot" stops and reports. No loop or fallback tries the
  next client, fresh visitorData or a retry to get past it; the client order of `brain/features/playback.md` is a
  fallback for formats and errors, never for the bot check. Any back-off state is bounded.
- **Clean room** - no code derived from a source project: no ported structure, names or constants that only a
  project's code could give. Every protocol fact the diff relies on (endpoint, JSON path, header, client constant,
  behaviour) is traceable to `brain/specs/SPEC.md`, a `brain/specs/sources/` note, or a dated entry in
  `brain/architecture/youtube.md` or `brain/specs/observations/`. A fact with no such trace is a finding; its fix is
  a dated observation, not a citation of third-party code. Nothing in code, comments or docs cites that code.
- **No DRM** - a format with `drmFamilies` is dropped, never worked around, and the error says so.
- **Correctness** - line by line: inverted conditions, off-by-one, `unwrap` or indexing on data from YouTube, retry
  loops that never end or give up too soon, JSON paths that match the fixtures, `playabilityStatus` mapped to the
  right contract error (SPEC 5.2 and 11.2), error messages that say which video and which client.
- **Reuse / simplify** - does new code re-implement the `Http` or `Storage` helpers, the Innertube request plumbing,
  the client table or the format choice that already exist? Does it belong in an existing module of
  `brain/architecture/overview.md`?
- **Tests** - do the tests cover the new branches: each `playabilityStatus`, a non-2xx, a missing field, a storage
  hit, a storage refusal, the bot check? Are new fixtures real answers, trimmed, with `ip=`, visitorData, signatures
  and tokens scrubbed, and no cookie, token or account name? Does any test reach the network (it must not; see
  `brain/architecture/testing.md`)?

Each finder returns findings as JSON objects `{file, line, severity, summary, failure_scenario}`, verified (quote
the line), most-severe first. Tell them NOT to fix anything. Then optionally run one **sweep** finder that has the
merged list and hunts only for gaps.

## 3. Fix what's confirmed

Triage the findings. Re-verify a claim before fixing it - finders surface plausible-but-wrong items too. Fix every
confirmed contract, invariant, clean-room and correctness issue, and the worthwhile quality ones, as new commits on
top (`[<branch>] fix(<area>): ...`), never by rewriting the reviewed layers. A fact that needs a new observation gets
one under the probe etiquette, or goes back to the user. For anything real but out of scope, add it to `TODO.md` or
open a GitHub issue (`gh issue create -R MarcDownHQ/Halcyon`) rather than dropping it.

## 4. Update the brain

Bring `brain/` in line with what the feature changed - it ships in the SAME merge, not as a follow-up. Apply the
**brain** skill (it has the checklist); the essentials:

- A **(Planned)** doc the feature built now describes the code, with `## Sources`.
- New storage keys or lifetimes -> the table in `brain/architecture/overview.md`.
- New YouTube facts -> `brain/architecture/youtube.md` or `brain/specs/observations/`, dated.
- New non-obvious decision -> `brain/decisions.md` with its why. New term -> `brain/glossary.md`.
- `ROADMAP.md`: tick the items the branch delivered. `TODO.md`: drop the questions and decisions it settled.
- `CHANGELOG.md` `[Unreleased]` has an entry for every user-facing change; README's status and usage follow.

Commit as `[<branch>] docs: ...`, after checking the brain's relative links resolve.

## 5. Gate (must be green before merge)

```bash
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo clippy --locked --target wasm32-wasip2 --release -- -D warnings
cargo test --locked
scripts/coverage.sh --check          # 98% of lines; lowering the floor is not a fix
scripts/coverage.sh                  # look for a logic file drifting down the list
shellcheck scripts/*.sh
cargo build --locked --release --target wasm32-wasip2
```

When the branch changes how Halcyon talks to YouTube, check it live with a small anonymous probe under the etiquette
in `brain/architecture/testing.md`: a request budget set first, one request at a time, VODs before lives, stop on the
bot check. Date anything new you learn in the brain.

Every commit must pass on its own, not just the tip; check from clean exports (a stale `target/` hides a broken
layer):

```bash
w=$(mktemp -d); export CARGO_TARGET_DIR="$w-target"
for c in $(git rev-list --reverse origin/master..HEAD); do
  rm -rf "$w"/* && git archive "$c" | tar -x -m -C "$w"
  (cd "$w" && cargo clippy -q --all-targets --locked -- -D warnings && cargo test -q --locked) \
    >/dev/null 2>&1 && echo "ok   $(git log -1 --format=%s "$c")" || echo "FAIL $(git log -1 --format=%s "$c")"
done
rm -rf "$w" "$w-target"; unset CARGO_TARGET_DIR
```

## 6. Push and let CI run

`origin` is GitHub (`MarcDownHQ/Halcyon`).

```bash
git push --force-with-lease origin <branch>   # the rebase rewrote it
gh pr create -R MarcDownHQ/Halcyon --head <branch> ... || gh pr edit ...
gh pr checks --watch -R MarcDownHQ/Halcyon
```

CI runs on GitHub-hosted runners: the `check` job (fmt, clippy for the host and for wasm, tests, the component
build) and the `coverage` job (ShellCheck, then the 98% floor). Both must be green.

## 7. Merge (keep the layers)

Fast-forward `master` onto the branch rather than squashing, so the layers survive:

```bash
cd ~/repos/Halcyon
git switch master && git pull --ff-only
git merge --ff-only <branch>
git push origin master                         # GitHub marks the PR merged
```

## 8. Release

When the branch closes a roadmap milestone, run the **release** skill: minor for new features or behaviour, patch
for fix-only. Otherwise leave the changes in `[Unreleased]`.

## 9. Clean up

```bash
wt remove <branch>                            # worktree and branch
git push origin --delete <branch>
```

## Done when

The feature is on `master`, the brain reflects it, every layer and CI were green, the branch and worktree are gone,
and a release is cut if a milestone closed. Report a one-line summary of what the review fixed.
