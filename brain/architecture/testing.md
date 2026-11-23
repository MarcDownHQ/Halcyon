# Testing

The gate, run before every commit and in CI:

```bash
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo clippy --locked --target wasm32-wasip2 --release -- -D warnings
cargo test --locked
cargo build --locked --release --target wasm32-wasip2
```

## Coverage

```bash
scripts/coverage.sh            # summary, then files ranked by uncovered lines
scripts/coverage.sh --check    # fails below 98% of lines (HALCYON_COVERAGE_FLOOR)
scripts/coverage.sh --html     # browsable report
scripts/coverage.sh --lcov     # target/llvm-cov/lcov.info
```

`cargo-llvm-cov` over the `halcyon` crate, measured natively. `src/wasm.rs` only builds for wasm and is not counted,
which is why it holds conversions and no logic. CI runs `shellcheck scripts/*.sh` then the check. New code comes
with the tests that keep the floor; lowering the floor is not a fix.

## No network in tests (Planned, M1)

Every resolver will take the `Http` and `Storage` traits, so the tests run on fakes: a `FakeHttp` answering by URL
or prefix, in turn, and recording what was sent, and a `FakeStorage` recording each value's lifetime and able to
refuse writes. Fixtures in `tests/fixtures/` are real YouTube answers trimmed to what the tests read:

- `/player` answers per client and per `playabilityStatus` (OK, bot check, age gate, unplayable, offline);
- RSS feeds, channel and playlist pages, `/browse` continuations;
- later the player script's relevant shape, caption lists, device-code answers.

Scrub every fixture: media URLs carry the requester's IP (`ip=`), and `visitorData`, signatures and tokens must be
shortened or replaced. Never commit a cookie, an OAuth token or a real account name.

## Live-probe etiquette

A probe is how a new protocol fact enters the project (see the clean-room rule in `CLAUDE.md`). YouTube punishes
careless probing with an IP-wide bot check, so:

- **Anonymous only.** Never log in, never send cookies, never use an account during a probe.
- **Small and spaced.** Set a request budget first (the 2026-11-23 probe used 133), send one request at a time, with
  a gap of a second or so, and stop when the question is answered.
- **Stop on the bot check.** On "Sign in to confirm you're not a bot", stop calling `/player` and wait; do not try
  other clients or fresh visitorData to get past it. Note when it started and when it cleared.
- **Lives last.** Live `/player` calls came right before both IP-wide blocks on 2026-11-23. Probe VODs first, lives
  at the end, and never live channels at catalog time.
- **Keep the downloads small.** Fetch a few chunks or segments, not whole files, unless the question is about whole
  files.
- **Write it up**, dated, in `brain/specs/observations/<date>-<topic>.md`: date and time, network, tooling, budget,
  method, results, verdict. Raw answers stay out of the repo, except trimmed and scrubbed fixtures.
- **Never open third-party code** to interpret what you see.

## Sources

- [scripts/coverage.sh](../../scripts/coverage.sh)
- [.github/workflows/ci.yml](../../.github/workflows/ci.yml)
- `src/testing.rs` and `tests/fixtures/`: none yet, land with M1.
