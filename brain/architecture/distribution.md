# Distribution

Halcyon will ship as one file, `halcyon.wasm`, a WebAssembly component for `wasm32-wasip2` built with
`opt-level = "s"`, LTO, stripped symbols and one codegen unit. CI exists; the release workflow does not yet.

## CI

`.github/workflows/ci.yml` runs on GitHub-hosted `ubuntu-24.04` runners on every pull request and every push to
`master`. The repo is public, and a self-hosted runner would run a fork's pull request on that machine. Actions are
pinned by commit. `jdx/mise-action` installs what `.mise.toml` pins: Rust with clippy, rustfmt, llvm-tools and the
`wasm32-wasip2` target, cargo-llvm-cov and ShellCheck. Two jobs: `check` (the gate in [testing.md](testing.md),
ending with the component build) and `coverage`.

When M4 brings rquickjs, the build needs a wasi-sdk given through `WASI_SDK`; without it `rquickjs-sys` downloads
one itself, which CI should avoid. Pin it in `.mise.toml` with the rest.

## Releases

As in Antenne: `.github/workflows/release.yml` on a `v*` tag checks the tag matches `version` in `Cargo.toml`, runs
the tests, builds the component, writes `halcyon.wasm.sha256`, takes the release notes from the `CHANGELOG.md`
section of that version, and publishes a GitHub release with both files. The `release` skill drives it.

## Install

The user drops `halcyon.wasm` in Nightglass's `plugins` folder and restarts it; see the README.

## Sources

- [.github/workflows/ci.yml](../../.github/workflows/ci.yml)
- [Cargo.toml](../../Cargo.toml)
- [.mise.toml](../../.mise.toml)
- [rust-toolchain.toml](../../rust-toolchain.toml)
- `.github/workflows/release.yml`: the release on a `v*` tag.
