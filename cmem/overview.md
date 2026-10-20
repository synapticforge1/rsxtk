# overview.md — what rsxtk is

Read from `README.md`, `Cargo.toml` and `src/main.rs` on 2026-09-20.

## Purpose

`rsxtk` (Rust Scripting Toolkit) runs single-file Rust scripts as standalone programs. A script is a
TOML manifest between `---` fences followed by Rust source. rsxtk mirrors it into a hidden Cargo
project under `.tk/`, compiles to `wasm32-wasip1`, pre-compiles to a `.cwasm` for the host CPU,
and runs it in a WASI preview-1 sandbox pre-opened on the script's parent directory. It also runs
`.wasm`, `.wat` and `.cwasm` directly, and can run a script natively via `cargo -Z script`.

## The CLI as `src/main.rs` defines it (0.4.4)

`clap` derive, `rename_all = "lowercase"`. ⚠️ `README.md` documents only ten of these and still
heads its feature list "New Features in v0.4.2".

| command    | args                              | does                                                             |
| ---------- | --------------------------------- | ---------------------------------------------------------------- |
| `run`      | `<path>`                          | WASI run of `.rs` (built), `.wasm`, `.wat`, `.cwasm`; tries `_start`, else lists callable exports |
| `mod`      | `<path> [args…]`                  | call an export of a library module with CLI arguments (`allow_hyphen_values`) |
| `csrun`    | `<path>`                          | native run via `cargo -Z script` (nightly)                       |
| `build`    | `<path> <wasi|wasm|wat>`          | build through the virtual Cargo project; `wat` converts the result |
| `convert`  | `<input> [-o output]`             | `.wasm` ⇄ `.wat` (via `wat` / `wasmprinter`)                      |
| `init`     | `<name>`                          | new script template with manifest header                          |
| `initmod`  | `<name>`                          | new library-module template                                       |
| `resource` | `[path]`                          | create `rsc/` for local assets, mirrored into the build            |
| `bench`    | `<path> [-i N=5]`                 | time N executions                                                 |
| `add`      | `<path> <crate> [version]`        | add a dependency; missing version fetched from crates.io (`reqwest`, `USER_AGENT`) |
| `remove`   | `<path> <crate>`                  | remove a dependency                                               |
| `list`     | `<path>`                          | list dependencies                                                 |
| `fmt`      | `<path>`                          | rustfmt the code part, preserving the manifest                    |
| `optimize` | `-i <input> [-o output]`          | optimize a `.wasm` via `walrus`                                   |
| `clean`    |                                   | wipe `.tk/`                                                       |
| `info`     | `<path>`                          | export signatures (params / results), "wasmtk style"              |

Engine config (`create_engine`): Cranelift `OptLevel::Speed`, SIMD, bulk memory, multi-memory on;
the default wasmtime disk cache is loaded so repeated runs skip Cranelift. A machine id
(OS + arch + hostname hash) keys cached artifacts.

## Dependencies (`Cargo.toml`)

| crate                        | version  | why / constraint                                                              |
| ---------------------------- | -------- | ----------------------------------------------------------------------------- |
| `wasmtime`                   | 40.0.1   | `default-features = false`, features `cranelift`, `runtime`, `component-model`; **42+ reintroduces `aws-lc-sys` (NASM + cmake)** |
| `wasmtime-wasi`              | 40.0.1   | `p1` only                                                                     |
| `wat`, `wasmprinter`, `walrus` | 1.244 / 0.244 / 0.24.4 | text ⇄ binary and optimization; **toolkit scope, stay when the engine changes** |
| `reqwest`                    | 0.13.1   | `blocking`, `json`, `http2`, **`native-tls`** (Windows SChannel — no cmake/NASM) |
| `clap` 4.5, `anyhow`, `serde`, `serde_json` | — | CLI, errors, manifest                                          |

Release profile: `opt-level = 3`, LTO, `codegen-units = 1`, `panic = "abort"`, `strip`. Edition 2024.
License `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`). Prerequisites: nightly Rust,
`wasm32-wasip1` target.

## Hosting (2026-10-20)

- Canonical home: **`https://github.com/synapticforge1/rsxtk`**, owned by the `synapticforge1`
  GitHub organization (owner's). Moved there from the personal account `jrmarcum/rsxtk`; the GitHub
  API reports the new `full_name` and the old URL redirects. Use the org URL everywhere new.
- `Cargo.toml` `repository` / `homepage` / `documentation` name the org URL (fixed 2026-10-20; the
  crates.io page keeps the old URL until the next publish). `authors` is a person, not the host,
  so it stays.

## Repo layout

```
rsxtk/
  Cargo.toml  Cargo.lock        version line = the release arm (0.4.4)
  src/main.rs                   the whole program (463 lines)
  README.md  rsxtk_logo.png     public doc
  LICENSE-MIT  LICENSE-APACHE
  CLAUDE.md                     gitignored pointer
  cmem/                         this memory
  target/                       build output (gitignored)
```

## Where it sits in the workspace

- **wasmtk → rsxtk**: wasmtk's Rust producer (`rustwasic.ts`) delegates fully to rsxtk — "do NOT
  reimplement via cargo" (wasmtk `cmem/polyglot-producers.md`, shipped 2026-06-07). rsxtk is a
  build/run CLI, distinct from the `universalWasmLoader-rs` library loader; "adjacent, may
  converge later" (wasmtk `cmem/vision.md`).
- **wasmrt → rsxtk**: 🔒 owner: rsxtk takes wasmrt by default through the native `wasmrt-core`
  rlib (wasmrt `cmem/loaders.md`); only `wasmtime` + `wasmtime-wasi` are replaceable; `.cwasm`
  would not be the default on an interpreter. 🔒 2026-09-20: rsxtk and wasmtk receive the runtime
  slot first, for execution speed during development; the loaders follow. The rlib rsxtk would
  link has never been measured (wasmrt T11).
- Workspace rules, contracts and the cross-repo map: `../../cmem/INDEX.md`.
