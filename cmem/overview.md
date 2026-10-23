# overview.md — what rsxtk is

Read from `README.md`, `Cargo.toml` and `src/main.rs` on 2026-09-20; re-measured 2026-10-23.

## Purpose

`rsxtk` (Rust Scripting Toolkit) runs single-file Rust scripts as standalone programs. A script is a
TOML manifest between `---` fences followed by Rust source. rsxtk mirrors it into a hidden Cargo
project under `.tk/`, compiles to `wasm32-wasip1`, pre-compiles to a `.cwasm` for the host CPU,
and runs it in a WASI preview-1 sandbox pre-opened on the script's parent directory. It also runs
`.wasm`, `.wat` and `.cwasm` directly, and can run a script natively via `cargo -Z script`.

## The CLI as `src/main.rs` defines it (0.5.0)

`clap` derive, `rename_all = "lowercase"`. Matches `rsxtk --help` (0.5.0 binary, 2026-10-23; the same 16
commands as 0.4.4);
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
| `resource` | `[dir]`                           | create `<dir>/rsc/` (default `.`); a FILE path fails (os error 183). Read at run time via the preopen; mirrored to `.tk/<stem>/src/rsc/` for `include_str!` (2026-10-23) |
| `bench`    | `<path> [-i N=5]`                 | time N executions                                                 |
| `add`      | `<path> <crate> [version]`        | add a dependency; missing version fetched from crates.io (`reqwest`, `USER_AGENT`) |
| `remove`   | `<path> <crate>`                  | remove a dependency (exact name match since 2026-10-23)           |
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
- `Cargo.toml` `repository` / `documentation` name the org URL (fixed 2026-10-20; the crates.io
  page keeps the old URL until the next publish). `homepage` was removed 2026-10-23 (Cargo's
  `redundant_homepage` lint: it repeated `repository`). `authors` carries the maintainer
  contact email — the one allowed email (owner, 2026-10-23; `INDEX.md` § Policy).
- **Publishing**: crates.io crate `rsxtk`, 34 versions 0.1.0 → 0.4.4 (2026-01-06 → 2026-04-20), 10
  not yanked: 0.3.3 and 0.3.6 → 0.4.4 (crates.io API, re-counted 2026-10-23; "9" was a miscount).
  **0.5.0** bumped and committed 2026-10-23; NOT yet on crates.io — this machine has no crates.io
  token (no `~/.cargo/credentials.toml`, no `CARGO_REGISTRY_TOKEN`); the owner runs `cargo login`,
  then `cargo publish` from the clean, pushed tree. Its page will then show the org repository URL. Published by hand with `cargo publish`; the repo has no
  CI and no `.github/`, so no trusted-publishing (OIDC) policy is involved (workspace letter H20
  does not apply). The crates.io API exposes no author/email field — only the publisher's GitHub
  login.

## History — purged 2026-10-23 (workspace letter H19; owner: "it is ok to change the files")

- **What was removed, from every commit of `main`, `0.4.0` and tag `rsxtk_v0.4.0`:** the owner's
  personal address as author/committer (53 fields over 36 commits → the GitHub noreply address,
  also the local `user.email`), and `target/` — build output committed before `/target` was ignored
  (2026-04-16), the only place absolute drive paths ever occurred (8 commits). `Cargo.toml`
  `authors` was left as it is (allowed maintainer contact).
- **How:** a private bundle of the pre-purge state first (the workspace's machine-local
  `cmem/local/backups/`, never committed); `git filter-branch` with an env filter (that one address
  only) and an index filter (`git rm -r --cached target`), `--tag-name-filter cat`, empty commits
  kept so the map is 1:1; force-push with a lease per ref; `refs/original` dropped, reflogs
  expired, `gc --prune=now`.
- **Verified** against the old refs as positive control: 36 → 36 commits; personal-address fields
  53 → 0; commits with `target/` or drive paths 8 → 0; messages 0 hits before and after; names,
  dates and messages identical on all 36; trees of `main` and `0.4.0` byte-identical; the TAG's
  tree changed (owner-approved exception to H19's byte-identical rule): 2587 files → the same 8
  without `target/`. `.git`: 416.55 MiB → 2.93 MiB, one pack, `fsck` clean.
- **Not in our control:** `refs/pull/1/head` on GitHub (read-only) still points at the old
  `3f0c695`, so the old history stays reachable there, and GitHub serves old commits by hash until
  its own GC — a full purge is a GitHub Support request (owner). crates.io 0.4.x
  `.cargo_vcs_info.json` keep the old hashes forever.
- **Map (old → new)**, oldest first: `6415fbe`→`55f0535`, `cac9cdf`→`875ce7d`, `ddb3178`→`bd53e30`,
  `a966f1a`→`52d0955`, `adaa0f1`→`1b49927`, `3fa1918`→`07dc287`, `74cee0f`→`4a875ca`,
  `fcf91b8`→`aae92c8`, `2980b6b`→`62acd0a`, `fa2978f`→`eb32878`, `6a0ea17`→`552540c`,
  `33997a4`→`2c7f2a0`, `47268c6`→`3bb450f`, `ebb769f`→`dfa181c`, `56c71ea`→`8263a86` (tag
  `rsxtk_v0.4.0`), `a6cfe10`→`21f517d`, `3f0c695`→`704dcb0` (branch `0.4.0`), `a2d98fd`→`4c28557`,
  `9c565a9`→`c4d6f89`, `591525f`→`f530975`, `35c06a3`→`aa94741`, `1f55c0a`→`77fe05b`,
  `d32b900`→`fdd532e`, `df34276`→`710cd59`, `b120149`→`878c53f`, `d95976a`→`fefb330`,
  `5fb29db`→`e587396`, `8a74017`→`82e2c51`, `87ad0e0`→`de62a6d`, `1211acf`→`d0866d9`,
  `02d4827`→`7b2a5bc`, `8e916a3`→`c3a3751`, `6c5a31d`→`e7c5fc8`, `80769c2`→`e7e29de`,
  `f96efed`→`aa0029c`, `e6cbf6c`→`91f30b7` (`main`).

## Repo layout

```
rsxtk/
  Cargo.toml  Cargo.lock        version line = the release arm (0.5.0)
  src/main.rs                   the whole program (534 lines, 2026-10-23)
  tests/cli.rs                  end-to-end `cargo test` suite (41 tests; testing.md)
  README.md  rsxtk_logo.png     public doc
  LICENSE-MIT  LICENSE-APACHE
  CLAUDE.md                     gitignored pointer (untracked 2026-10-20; was committed empty before)
  .claude/settings.local.json   gitignored (machine-local tool permissions)
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
