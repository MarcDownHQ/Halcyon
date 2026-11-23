---
name: release
description: Cut a Halcyon release, the whole shebang - docs sweep (CHANGELOG, README, brain), a careful live check, then the version bump, gate, commit, annotated tag and push, then watch the Release workflow until the GitHub release carries halcyon.wasm and its SHA-256. Use when the user says "cut a release", "release", "tag vX.Y.Z", or "the whole release shebang".
---

# Release shebang

Everything between "the code is on master" and "users can download it". Pushing the tag is what builds and
publishes the release, so there is no undo once step 4 runs: a bad tag means a new patch release.

## 0. What a release publishes

A GitHub release on `MarcDownHQ/Halcyon` with `halcyon.wasm` and `halcyon.wasm.sha256`, its notes taken from the
version's `CHANGELOG.md` section. The workflow is `.github/workflows/release.yml`; the pipeline is described in
`brain/architecture/distribution.md`.

## 1. Pre-flight

- On `master`, working tree clean, level with `origin/master` (GitHub). Stash unrelated dirty files and restore
  them afterwards.
- Anything still on a feature branch is not in this release: finish it first (the **feature-treatment** skill) or
  leave it out on purpose.
- Last version: `git describe --tags --abbrev=0` (tags are `vX.Y.Z`; before the first release there is none, and
  `Cargo.toml` already says `0.1.0`). Pick the next semver: minor for a milestone, new fields or new behaviour, patch
  for fix-only. A change to the vendored WIT that needs a newer Nightglass is a minor bump that says so in the
  changelog.

## 2. Docs sweep (review against `git diff v<last>..HEAD`)

- **CHANGELOG.md `[Unreleased]`** covers every user-visible change since the last tag: a completeness check against
  `git log v<last>..HEAD --oneline`. That section becomes the release notes verbatim.
- **README.md**: the status line, the usage table (fields and defaults), the install folders.
- **brain/**: every doc a change made wrong is fixed, and no part that shipped still says **(Planned)** (the
  **brain** skill has the checklist).
- **ROADMAP.md**: the milestone this release closes is fully ticked.
- **Dependencies**: `git diff v<last>..HEAD -- Cargo.toml`. A new dependency must build for `wasm32-wasip2` and
  must not need anything outside the plugin (fully native).
- Commit the sweep as `[master] docs(release): ...`.

## 3. Live check

YouTube changes without notice, so check it before shipping, once there is something that plays (from M1 on;
before that, skip this step). Follow the probe etiquette in `brain/architecture/testing.md`:

- anonymous only, no cookies, no account; a request budget set first; one request at a time, spaced;
- one ordinary public VOD through the first client of `brain/features/playback.md`, then a small range of its media;
  lives last, and only if the release touches lives;
- on "Sign in to confirm you're not a bot", stop and wait; never try other clients to get past it.

A failure caused by YouTube having changed is fixed first (with the new fact dated in `brain/architecture/youtube.md`
or `brain/specs/observations/`), or the release says what no longer works in the changelog. Never open third-party
YouTube client code to understand a failure.

## 4. Cut it

```bash
v=<x.y.z>
sed -i "0,/^version = \".*\"/s//version = \"$v\"/" Cargo.toml     # the crate version
cargo update -p halcyon --offline -q                                # Cargo.lock follows
python3 - "$v" <<'PY'
import sys, datetime
v = sys.argv[1]; p = "CHANGELOG.md"; s = open(p).read()
today = datetime.datetime.now(datetime.UTC).date().isoformat()
open(p, "w").write(s.replace("## [Unreleased]\n", f"## [Unreleased]\n\n## [{v}] - {today}\n", 1))
PY
cargo fmt --all --check && cargo clippy --all-targets --locked -- -D warnings \
  && cargo clippy --locked --target wasm32-wasip2 --release -- -D warnings && cargo test --locked \
  && scripts/coverage.sh --check && cargo build --locked --release --target wasm32-wasip2
git commit -am "[master] chore(release): $v"
git tag -a "v$v" -m "Halcyon $v"
git push origin master "v$v"
```

Check before committing that the new `## [x.y.z]` section is not empty and that the version in `Cargo.toml` is the
one tagged: the Release workflow refuses a tag that does not match it, and a version with no changelog section.
For the first release with `Cargo.toml` already at the chosen version, the `sed` and `cargo update` change nothing;
the commit then carries only the changelog.

## 5. Watch the Release workflow

```bash
run=$(gh run list -R MarcDownHQ/Halcyon --workflow Release --event push --branch v<x.y.z> --limit 1 --json databaseId --jq '.[0].databaseId')
gh run watch "$run" -R MarcDownHQ/Halcyon --exit-status
gh release view v<x.y.z> -R MarcDownHQ/Halcyon --json assets --jq '.assets[].name'
```

Read a failing job with `gh run view "$run" --log-failed`. A re-run executes the workflow files and the code at the
tag, so it only helps with a transient failure (the runner, the network): `gh run rerun "$run" --failed`. Anything
else is fixed on `master` and shipped as the next patch release; never move or reuse a tag. The workflow can also be
started by hand on an existing tag (`gh workflow run Release -R MarcDownHQ/Halcyon -f tag=v<x.y.z>`).

## 6. Verify the download

```bash
d=$(mktemp -d) && gh release download v<x.y.z> -R MarcDownHQ/Halcyon -D "$d" \
  && (cd "$d" && sha256sum -c halcyon.wasm.sha256); rm -rf "${d:?}"
```

## Done when

The tag is pushed, the GitHub release has `halcyon.wasm` and its checksum, the checksum verifies, and
`CHANGELOG.md` has a dated section for it. Report the version and the release URL.
