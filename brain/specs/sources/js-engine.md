# Source: embedded JavaScript engine for the plugin

## Question

The plugin must run YouTube's player script (`base.js`, about 3 MB of minified code) inside itself to solve the signature and n-parameter challenges, and maybe the BotGuard VM for PO tokens. Which JavaScript engine can live inside a Rust `wasm32-wasip2` component built with wit-bindgen, under the host's limits: wasmtime, no threads, 256 MiB of memory, 5 minutes per call, a fresh instance per call, and storage of 1 MiB per text value and 16 MiB in total?

Each candidate below was built and run, not judged from its README.

## Recommendation

**Use `rquickjs` 0.14 (QuickJS-ng 0.16.2, MIT), built with wasi-sdk.**

- It builds for `wasm32-wasip2` inside a wit-bindgen 0.63 component with no patches. The only requirement is a wasi-sdk on the build machine, given by the `WASI_SDK` environment variable. Without that variable, `rquickjs-sys` downloads wasi-sdk 24 itself.
- It adds about 0.8 MB to the component.
- It runs the whole current `base.js` without errors in about 0.3 s, using about 32 MiB.
- Its bytecode can be cached: loading the player from cached bytecode takes about 30 ms.
- Its licence is MIT, the same as the plugin.

Evaluate scripts in sloppy mode, not strict mode. rquickjs's `Ctx::eval` defaults to strict, and then `base.js` throws "window is undefined", because the player reads `this` at the top of its wrapper function. Use `eval_with_options` with `strict = false`, or call `JS_Eval` directly.

Boa is the only pure-Rust candidate that works, but it is ruled out: it needs about 258 MiB to load the player, so it hits the 256 MiB limit. It is also 35 times slower to load and 5 to 6 times slower in loops, and it adds 4.8 MB. javy and quickjs-wasm-rs are QuickJS too, but they bring no advantage over rquickjs directly (see below).

## Setup

- **Player script:** `https://www.youtube.com/s/player/6b3d0364/player_ias.vflset/en_US/base.js`. The player id comes from `https://www.youtube.com/iframe_api`, fetched on 2026-11-23. The file is 3,060,278 bytes. It is mostly ES5 output, but it relies on ES2015+ built-ins (Symbol, Promise, Proxy, Reflect, BigInt) and contains a few `?.` and `yield`.
- **Environment stubs:** the script expects a browser. About 20 lines of prelude are enough for it to finish: `window`, `self`, `document` with stub elements, `location`, `navigator`, `screen`, timers, `localStorage`, `performance`, and a few constructors. The success check is `Object.keys(_yt_player).length === 558`, the same value Node 26 returns in a bare `vm` context with the same prelude (Node takes 126 ms there).
- **Loop benchmark:** 1e6 iterations of `split`, `reverse`, `push`/`shift`, `splice`, `join`, `charCodeAt`, `indexOf`, and array push/slice. The expected result is `110266684:2624`. Node reference: 236 ms with the JIT, 329 ms with `--jitless`.
- **Guests:** one cdylib per engine, built with Rust 1.100.0 (Antenne's toolchain), wit-bindgen 0.63.0, and Antenne's release profile (`opt-level = "s"`, LTO, strip, `codegen-units = 1`). Each exports `run(mode, script, prelude, blob) -> tuple<string, list<u8>>` and times parsing and evaluation itself with `std::time::Instant`, which reads WASI clocks.
- **Host:** wasmtime 50.0.2, the version and feature set nightglass's `plugin-host` uses: async, Cranelift, epoch interruption with a 10 ms tick, a 300 s timeout, and the WASI p2 linker. It adds a `ResourceLimiter` that refuses linear memory above 256 MiB and records the peak. A fresh store is created for every run, as in the app.
- **Baseline:** a guest with no engine is 48 KB, and its peak memory with the 3 MB script passed in is 3.9 MiB. The "added size" column subtracts the 48 KB.
- **Build tools:** wasi-sdk 34 for the C engines, cmake from mise.
- **Scratch files:** the benchmark host, the guests, the prelude and loop scripts and the bench WIT lived in a temporary scratch directory and were not kept.

## Results

Times are wall-clock inside wasmtime on this machine. Where a run was repeated, the table shows the typical value of 2 or 3 runs. Peak memory is the guest's linear memory, including the roughly 4 MiB of inputs.

| Engine | Builds for wasip2 component | Added size | Load `base.js` (parse + eval) | Peak memory, load | 1e6 loop | Licence |
|---|---|---|---|---|---|---|
| **rquickjs 0.14** (QuickJS-ng 0.16.2), `opt-level=s` | yes | +0.80 MB | **293 ms** (273 + 9) | 35.6 MiB | 3.7 s | MIT |
| rquickjs 0.14, `opt-level=3` | yes | +1.18 MB | 281 ms (264 + 7) | 35.6 MiB | 3.1 s | MIT |
| rquickjs, load from cached bytecode (stripped) | yes | - | **33 ms** (read 13 + eval 8) | 15.7 MiB | - | - |
| javy 8.1.0 (wraps rquickjs 0.12.2) | yes | +0.90 MB | 290 ms | 32.3 MiB | 4.0 to 4.4 s | Apache-2.0 WITH LLVM-exception |
| quickjs-wasm-rs 3.1.0 (older Bellard QuickJS) | yes | +0.85 MB | 690 ms | 31.1 MiB | 3.2 s | Apache-2.0 WITH LLVM-exception |
| Boa 0.22.0 (pure Rust) | yes | +4.78 MB | 10.7 s (parse 5.1 + eval 5.6) | **257.9 MiB: fails at 256 MiB** (allocation failure, then trap after 11 s) | 19.0 s | MIT OR Unlicense |
| nova_vm 1.0.0 (pure Rust) | **no** | - | - | - | - | MPL-2.0 |

Wasmtime's Cranelift compile time, paid once per component load and cacheable by the host: 110 to 350 ms for the QuickJS guests, 460 to 650 ms for Boa.

## Per-engine notes

### rquickjs (QuickJS-ng)

- `rquickjs-sys` 0.14 ships pre-generated bindings for `wasm32-wasip2`, so no bindgen run is needed. It compiles the C sources with the wasi-sdk named by `WASI_SDK`. Without that variable it downloads wasi-sdk 24 into `OUT_DIR`, which CI should avoid by providing the SDK.
- Loading `base.js` from source costs about 270 ms of parsing and bytecode generation, then 9 ms of evaluation. Peak memory is 36 MiB, and QuickJS's own heap is 11.9 MiB after the load. That leaves more than 200 MiB of headroom.
- The engine runs on one thread. Epoch interruption reaches it because it is plain wasm. `Runtime::set_memory_limit` and `set_max_stack_size` give in-engine limits below the host's limits, so a runaway script throws a JS exception instead of trapping the whole call. Wasm's own stack is the Rust default of 1 MiB; deep recursion, for example in BotGuard, may need the `-z stack-size` link argument.
- The interpreter runs about 15 times slower than Node with its JIT on the loop benchmark. That does not matter for the signature and n-parameter functions: each is a single short call. **The BotGuard VM was not measured.** It is the case where interpreter speed matters. The 5-minute call limit leaves plenty of room unless it needs several minutes of computation in an interpreter.

### javy

- The `javy` crate is the Bytecode Alliance's wrapper around rquickjs. Version 8.1.0 still uses rquickjs 0.12.2 and an older QuickJS-ng. It builds for wasip2 as a library and gives the same results as rquickjs, a little slower in loops. It adds javy's own APIs (JSON, console, and others), which we do not need.
- Javy's real approach is ahead-of-time work. The `javy` CLI compiles JS to QuickJS bytecode at build time and uses Wizer to snapshot an initialised heap into the wasm. That does not fit our case: `base.js` changes often and arrives at run time, and the plugin cannot rewrite its own wasm. What carries over is the runtime equivalent, caching QuickJS bytecode in storage (see Caching). That needs only rquickjs.
- Its licence, Apache-2.0 with the LLVM exception, is compatible with an MIT project, but it adds a NOTICE obligation for nothing we need.

### quickjs-wasm-rs

- Deprecated. It is the crate javy used before switching to rquickjs; the last release was in May 2024. It builds for wasip2 and runs the player correctly. It bundles the older Bellard QuickJS, which loads the player 2.4 times slower (690 ms). Not recommended, because it is unmaintained.

### Boa

- Pure Rust, so it builds with no C toolchain. It is conformant enough: it runs the whole player without errors and gives the same result, 558 keys.
- It does not fit the budget. Loading the player needs about 258 MiB of linear memory. With the 256 MiB limit, the run fails after 11 s with `memory allocation of 2621440 bytes failed`, then a trap. With a 4 GiB limit, the load takes 10.7 s: 5.1 s to parse and 5.6 s to evaluate. Its loop takes 19 s, 5 to 6 times QuickJS. The component is 4.8 MB. It has no bytecode serialisation, so the 10 s parse would be paid on every call.
- It could still be used if we only ran extracted functions (see Caching). The loop speed would then still be a risk for BotGuard.

### nova_vm (not on the original list)

- Pure Rust, MPL-2.0. Version 1.0.0 does not build for `wasm32-wasip2`. It has a hard (non-optional) dependency on `usdt` (DTrace probes). That dependency pulls in `thread-id`, which has no wasi code; once `thread-id` is patched, `usdt-impl` refuses to build with "USDT only supports x86_64 and ARM64 architectures". Using it would mean forking Nova. Not pursued.

### Others considered, not built

- **SpiderMonkey through StarlingMonkey or ComponentizeJS:** this is a JS runtime packaged as its own component, not a library a Rust component can embed. It would mean composing two components, so the plugin would no longer be one Rust component. Its size is around 8 to 10 MB.
- **Duktape:** ES5 only. It has no Symbol, Promise, Proxy or Reflect, all of which the player uses.
- **Micro QuickJS (mquickjs):** a reduced subset aimed at microcontrollers; not a match for a 3 MB browser bundle.
- **V8 or Hermes:** no practical `wasm32-wasip2` build without threads and JIT; out of scope.

## Caching

Each call runs in a fresh instance, so nothing stays in memory between calls. What can stay is a value in storage.

- **QuickJS bytecode** (`JS_Eval` with `JS_EVAL_FLAG_COMPILE_ONLY`, then `JS_WriteObject`; reading back with `JS_ReadObject` and `JS_EvalFunction`, all reachable through `rquickjs::qjs` and `Ctx::as_raw`):

  | Form | Size |
  |---|---|
  | Full (with source text and debug info) | 9.8 MB |
  | `JS_WRITE_OBJ_STRIP_SOURCE \| JS_WRITE_OBJ_STRIP_DEBUG` | 3.07 MB |
  | Stripped + gzip -9 | 1.28 MB |
  | Stripped + gzip + base64 (storage holds text) | about 1.70 MB, so 2 values of up to 1 MiB |

  Loading from stripped bytecode takes 33 ms instead of 293 ms, and peak memory drops from 36 to 16 MiB. Writing the bytecode takes 10 ms.

  Costs and risks:
  - The cache uses about 1.7 of the 16 MiB.
  - The bytecode format is tied to the exact QuickJS-ng build, so the cache key must include the engine version as well as the player id.
  - `JS_ReadObject` trusts its input. A truncated or corrupt value can crash the instance, so check a length or hash before reading it.
  - Stripping the debug information removes line numbers from error stack traces.
  - For comparison, the gzipped source is 0.96 MB. Caching the source saves the download but not the 270 ms parse.
- **Extracted function source text:** keep only the signature and n-parameter functions and what they depend on, a few KB to a few tens of KB. This is the cheapest cache: it fits easily in one value, it is independent of the engine version, and it parses in milliseconds. It depends on an extraction step that finds those functions in the player and must keep up with changes to it. The other option needs no extraction: run the whole player once, about 0.3 s, then cache the bytecode or the challenge results.
- **Recommended order:** cache the **results** of the challenges per player id and input when possible. Cache the stripped and compressed bytecode per player id and QuickJS version so that a miss costs 30 ms instead of 300 ms. Treat extraction of function text as an optimisation, not a requirement: the measurements show that loading the whole player each time already fits easily within the limits.

## Licences

| Component | Licence | MIT-compatible |
|---|---|---|
| rquickjs, rquickjs-sys | MIT | yes |
| QuickJS-ng (C, vendored in rquickjs-sys) | MIT (Bellard, Gordon, Noordhuis) | yes |
| wasi-libc (linked through wasi-sdk) | Apache-2.0 WITH LLVM-exception, MIT, and others | yes, permissive; keep the notices |
| Boa | MIT OR Unlicense | yes |
| javy, quickjs-wasm-rs, quickjs-wasm-sys | Apache-2.0 WITH LLVM-exception | yes, with NOTICE and attribution obligations |
| nova_vm | MPL-2.0 | usable as a dependency, but the file-level copyleft applies to Nova's own files |
