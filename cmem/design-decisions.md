# design-decisions.md — invariants that must not be silently reverted

Dated; the reason travels with the rule. Created 2026-09-20 from `Cargo.toml` comments, `README.md`
§ Troubleshooting, and the sibling memories that record decisions about rsxtk.

## Engine and dependencies

- 🔒 **`wasmtime = "40.0.1"`, `default-features = false`.** Versions 42.x+ reintroduce `aws-lc-sys`
  in the dependency chain, which needs NASM and cmake to build; this machine has neither, and the
  build fails with `Missing dependency: cmake`. Do not upgrade until NASM + cmake are available or
  a pure-Rust crypto path exists in that version. The specifier is a caret (`>=40.0.1, <41`), which
  is enough: it admits 40.x patches (`Cargo.lock` holds 40.0.4, 2026-10-23) and never 41/42. It is
  not an exact pin; do not "fix" it to `=40.0.1`.
- 🔒 **`reqwest` on `native-tls`** (Windows SChannel) for the same reason — no cmake/NASM.
- 🔒 **`wat` / `wasmprinter` / `walrus` are toolkit scope**, not engine scope; they stay when the
  engine is swapped (wasmrt `cmem/loaders.md`).
- `component-model` dropped from rsxtk's own wasmtime features (2026-10-23): rsxtk never used it,
  and the measurement showed it costs nothing to declare or drop — `wasmtime-wasi` (`p2`, implied
  by `p1`) and `wasmtime-wasi-io` turn it on regardless. `cargo tree -e features -i wasmtime`
  lists the same 8 features with and without the line, so the build is identical. It goes away
  only with the wasmtime → wasmrt switch.

## Execution model

- **Nightly toolchain is a prerequisite**: `csrun` uses `cargo -Z script`.
- **Zero-config WASI sandbox**: the script's parent directory is the only preopen. Widening it is
  a security decision, not a convenience.
- **`.cwasm` pre-compilation** makes repeated runs of a `.rs` script skip `cargo` and Cranelift;
  the cache key includes a machine id (OS, arch, hostname). No wasmtime disk cache is configured
  (corrected 2026-10-23). ⚠️ Under an interpreter engine (wasmrt) `.cwasm`
  would not be the default — re-decide when the engine changes.
- **Smart entry**: `_start` if exported, else list callable exports; `mod` calls one by name.
- 🔒 **`build.rs` and `rsc/` are mirrored** (owner chose option A, 2026-10-23; the README had
  promised it since 0.4.2, but no version ever did it). From the SCRIPT's folder: `build.rs` →
  `.tk/<stem>/build.rs` (Cargo runs it from the package root); `rsc/` → `.tk/<stem>/src/rsc/`, so
  `include_str!("rsc/…")` resolves as beside the script and under `csrun`. Stale copies are
  removed. The cache check takes the newest mtime of script + `build.rs` + all of `rsc/` (its
  folders count, so a deleted file registers), and rebuilds if either appeared or vanished. A
  script named `build.rs` is not its own build script. All scripts in one folder share that
  folder's `build.rs` and `rsc/`. `build.rs` runs on the HOST, unsandboxed, like any Cargo build
  script; the README says so. Run-time reads of `rsc/` never needed this: the preopen covers them.
- **Bin or lib by `fn main`** (2026-10-23): a `.rs` script whose code has `fn main` (then optional
  whitespace, then `(`) is mirrored as a binary (`src/main.rs`); otherwise as a library
  (`src/lib.rs`, `crate-type = ["cdylib"]`), so `initmod` templates build and `mod` can call them.
  The other kind's source file is deleted on each build (Cargo would build both). A library's
  output is named with `-` → `_`; `build_and_cache_script` returns the real `.wasm` path so callers
  never rebuild the name. Before this, every script was a binary and `initmod` failed with E0601.
- **The manifest is the `---`-fenced TOML header**; `fmt` must preserve it (a formatter that eats
  the header eats the dependencies).
- **`add`/`remove` match the crate name EXACTLY** (2026-10-23, `dep_key`): a line's name is the
  text before its first `=` or `.`, unquoted — `serde = …`, `serde.workspace = …`, `"serde" = …`.
  It was a prefix match before, so `remove serde` also deleted `serde_json`. Limitation kept: a
  `[dependencies.<name>]` table spanning several lines is not recognised (no `=` on its header).

## The wasmrt switch (planned, owner decisions)

- 🔒 rsxtk takes **wasmrt** by default through the native `wasmrt-core` rlib — no C ABI (owner;
  wasmrt `cmem/loaders.md`).
- 🔒 2026-09-20: rsxtk and wasmtk receive the runtime slot **first**; the universalWasmLoader
  ports follow once issues surfaced here are identified.
- Before switching: measure the rlib's size and rsxtk's end-to-end time on wasmtime vs on wasmrt,
  same machine, interleaved, `.cwasm` off on both (wasmrt T11: "do not put it in the README until
  it has been run").

## Versioning and release (workspace rule, owner 2026-09-20)

- Semantic Versioning 2.0.0: patch for fixes, minor for backward-compatible additions, major for
  breaks; no cap, no carry. rsxtk is at `0.5.1` (released by CI 2026-11-09), so consumers' carets treat every minor as
  potentially breaking until `1.0.0`, where its API promise begins. Keep a README Breaking Changes
  table as the migration guide (none exists yet).
- **A bump is gated** (workspace rule, `../../cmem/layout-and-packaging.md` § "The compatibility
  gate"): rsxtk's surface is its CLI — flags, positional shapes, exit codes, output a script may
  parse. A committed golden `--help` and the test suite are the snapshot; a bump script diffs them
  and refuses a patch when a command or flag was removed or reshaped. The suite exists
  (2026-10-23, `testing.md`); the golden `--help` and the bump script do not yet.
- The `Cargo.toml` version line is the only source. `README.md` no longer restates it (its
  "New Features in v0.4.2" heading drifted against 0.4.4; it became "Features" on 2026-10-23) —
  keep version numbers out of the README. Since 0.5.0 `main.rs` takes it from Cargo
  (`#[command(name = "rsxtk", version)]` = `CARGO_PKG_VERSION`), so a bump is ONE line plus the
  lockfile's own entry; the `--version` test guards it.
- **0.5.0 was a minor bump** (2026-10-23): new behaviour (`build.rs`/`rsc/` mirrored, library
  scripts built as `cdylib`) and fixes; `--help` lists the same 16 commands as 0.4.4 (compared
  binary to binary), so nothing was removed. A `build.rs` that 0.4.4 ignored now RUNS — a reason
  not to call it a patch.
- **The crate excludes `cmem/`, `.github/`, `scripts/`** (`exclude` in `Cargo.toml`; `cmem/`
  2026-10-23, the other two 2026-11-09): they are for working on rsxtk, not for crate users; they
  stay public on GitHub. `cargo package --list` for 0.5.0: `.cargo_vcs_info.json`, `.gitignore`,
  `Cargo.lock`, `Cargo.toml`(`.orig`), both licenses, `README.md`, `rsxtk_logo.png`,
  `src/main.rs`, `tests/cli.rs`.
- Nothing is published unless the owner says so. **Since 2026-11-09 CI publishes** (owner: "the
  cargo registry token has been added to the secrets for the project. please setup the CI and the
  publish for crates.io"): the owner's go = bump the `Cargo.toml` version, run the gate, commit,
  push `main`, push the tag `rsxtk_vX.Y.Z` (lightweight, like the older tags). No hand
  `cargo publish` any more; the repo secret `CARGO_REGISTRY_TOKEN` is the only credential.
- 🔒 **`.github/workflows/release.yml`, on a pushed `rsxtk_v*` tag — three jobs, each with only
  what it needs** (workspace C30; H30 no `gh`, in CI too; H32 CI-made Release pages):
  - `checks` (contents: read, no secret, clone without credentials): the tag equals `rsxtk_v` +
    the `Cargo.toml` version; Rust `nightly-2026-11-04` (the local gate's nightly; rustfmt, clippy,
    `wasm32-wasip1` — the suite needs all three) — `cargo build --release --locked`,
    `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`; then
    `bun test scripts/`. Everything that runs third-party build code runs here only.
  - `publish` (contents: read): refuses unless the tag still peels to the commit `checks` passed;
    asks crates.io for the version — 200 = skip (a re-run is safe), 404 = `cargo publish --locked
    --no-verify` with `CARGO_REGISTRY_TOKEN` in that one step, anything else = stop.
  - `release` (contents: write, NO checkout): runs once crates.io serves the version, even if
    `publish` failed after its upload (C34); fetches only `scripts/github-release.ts` at
    `github.sha` and runs it: the tag must exist and peel to `github.sha` (a `POST /releases`
    would CREATE a missing tag), an existing page = done, else POST `generate_release_notes` with
    the crates.io link on top, read back, peel again; 30 s per request.
  Move `RUST_TOOLCHAIN` when the local gate's nightly moves (the CI gate must BE the local gate,
  ★ 2.4); move `BUN_VERSION` + `BUN_SHA256` with the installed Bun (Tooling rule 10). Adapted from
  universalWasmLoader-rs (rsxtk's tags are `rsxtk_vX.Y.Z`, not `vX.Y.Z`).
  **No backfill** (owner: "only new tags will get release pages"): `rsxtk_v0.4.0` keeps its one
  manual page (by `jrmarcum`, read through the REST API 2026-11-09); `rsxtk_v0.5.0` stays without
  one. First run: 0.5.1 (item 10).
- **0.5.1 is a patch** (2026-11-09): `src/` and `tests/` are unchanged since 0.5.0 apart from the
  CRLF → LF renormalization (`git diff --ignore-cr-at-eol rsxtk_v0.5.0 -- src tests` empty); the
  manifest gained the two `exclude` entries. It is the first release published by CI.
