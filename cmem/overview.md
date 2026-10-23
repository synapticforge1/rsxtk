# overview.md — what rsxtk is

Read from `README.md`, `Cargo.toml` and `src/main.rs` on 2026-09-20.

## Purpose

`rsxtk` (Rust Scripting Toolkit) runs single-file Rust scripts as standalone programs. A script is a
TOML manifest between `---` fences followed by Rust source. rsxtk mirrors it into a hidden Cargo
project under `.tk/`, compiles to `wasm32-wasip1`, pre-compiles to a `.cwasm` for the host CPU,
and runs it in a WASI preview-1 sandbox pre-opened on the script's parent directory. It also runs
`.wasm`, `.wat` and `.cwasm` directly, and can run a script natively via `cargo -Z script`.

## The CLI as `src/main.rs` defines it (0.4.4)

`clap` derive, `rename_all = "lowercase"`. Matches `rsxtk --help` (0.4.4 binary, 2026-10-23);
`README.md` lists all sixteen since 2026-10-23.

| command    | args                              | does                                                             |
| ---------- | --------------------------------- | ---------------------------------------------------------------- |
| `run`      | `<path>`                          | WASI run of `.rs` (built), `.wasm`, `.wat`, `.cwasm`; tries `_start`, else lists callable exports |
| `mod`      | `<path> [args…]`                  | call an export of a library module with CLI arguments (`allow_hyphen_values`) |
| `csrun`    | `<path>`                          | native run via `cargo -Z script` (nightly)                       |
| `build`    | `<path> <wasi|wasm|wat>`          | build through the virtual Cargo project; `wat` converts the result |
| `convert`  | `<input> [-o output]`             | `.wasm` ⇄ `.wat` (via `wat` / `wasmprinter`)                      |
| `init`     | `<name>`                          | new script template with manifest header                          |
| `initmod`  | `<name>`                          | new library-module template (no `fn main` → built as a `cdylib`; `mod <name>.rs add 2 3` → `5`) |
| `resource` | `[dir]`                           | create `<dir>/rsc/` (default `.`); a FILE path fails (os error 183). NOT mirrored into `.tk/` — a script reads `rsc/…` at run time via the preopen |
| `bench`    | `<path> [-i N=5]`                 | time N executions                                                 |
| `add`      | `<path> <crate> [version]`        | add a dependency; missing version fetched from crates.io (`reqwest`, `USER_AGENT`) |
| `remove`   | `<path> <crate>`                  | remove a dependency                                               |
| `list`     | `<path>`                          | list dependencies                                                 |
| `fmt`      | `<path>`                          | rustfmt the code part, preserving the manifest                    |
| `optimize` | `-i <input> [-o output]`          | optimize a `.wasm` via `walrus`                                   |
| `clean`    |                                   | wipe `.tk/`                                                       |
| `info`     | `<path>`                          | export signatures (params / results), "wasmtk style"              |

Engine config (`create_engine`): Cranelift `OptLevel::Speed`, SIMD, bulk memory, multi-memory on.
⚠️ No wasmtime disk cache is configured (corrected 2026-10-23 — the README and this file used to
claim one): only `.rs` scripts are cached, as `.tk/<stem>/<stem>.cwasm`, reused while the
machine id (OS + arch + hostname hash) matches and the `.cwasm` is newer than the script.
`.wasm` / `.wat` are recompiled by Cranelift on every run.

## Dependencies (`Cargo.toml`)

| crate                        | version  | why / constraint                                                              |
| ---------------------------- | -------- | ----------------------------------------------------------------------------- |
| `wasmtime`                   | ^40.0.1 (lock 40.0.4) | `default-features = false`, features `cranelift`, `runtime`; **42+ reintroduces `aws-lc-sys` (NASM + cmake)**. `component-model` is still ON — `wasmtime-wasi` (`p2`, implied by `p1`) and `wasmtime-wasi-io` enable it |
| `wasmtime-wasi`              | ^40.0.1 (lock 40.0.4) | `p1` requested; it implies `p2`                                |
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
  crates.io page keeps the old URL until the next publish). `authors` carries the maintainer
  contact email — the one allowed email (owner, 2026-10-23; `INDEX.md` § Policy).
- **Publishing**: crates.io crate `rsxtk`, 34 versions 0.1.0 → 0.4.4 (2026-01-06 → 2026-04-20), 9
  not yanked (crates.io API, 2026-10-23). Published by hand with `cargo publish`; the repo has no
  CI and no `.github/`, so no trusted-publishing (OIDC) policy is involved (workspace letter H20
  does not apply). The crates.io API exposes no author/email field — only the publisher's GitHub
  login.

## Repo layout

```
rsxtk/
  Cargo.toml  Cargo.lock        version line = the release arm (0.4.4)
  src/main.rs                   the whole program (480 lines, 2026-10-23)
  README.md  rsxtk_logo.png     public doc
  LICENSE-MIT  LICENSE-APACHE
  CLAUDE.md                     gitignored pointer (untracked 2026-10-20; was committed empty before)
  cmem/                         this memory (first committed 2026-10-20); cmem/local/ gitignored
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
