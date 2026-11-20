# design-decisions.md — invariants that must not be silently reverted

Dated; the reason travels with the rule. Created 2026-09-20 from `Cargo.toml` comments, `README.md`
§ Troubleshooting, and the sibling memories that record decisions about rsxtk.

## Engine and dependencies

- 🔒 **wasmtime and wasmtime-wasi `"50.0.3"` (caret), `default-features = false`**, features
  `cranelift`, `runtime`, `anyhow` / `p1` (owner, 2026-11-20). 🔒 **The rule that does not change:
  no C library and no cmake / NASM in rsxtk's build** (owner: "we pinned that version of wasmtime
  because it didn't have c dependencies"). Measured 2026-11-20, `cargo tree -i` + the build
  scripts, on 40.0.4 and 50.0.3 alike: the only C built is two small files through `cc` —
  wasmtime's `src/runtime/vm/helpers.c` and wasmtime-internal-fiber's `src/windows.c` — with the
  Rust toolchain's own C compiler; no `aws-lc-sys` / `aws-lc-rs`, `cmake` or `ring` is built
  (`ring` sits in `Cargo.lock` as an unused optional dependency of `rustls-webpki`, also on 40).
  `aws-lc-sys` comes only with wasmtime's DEFAULT features: keep `default-features = false`, and
  re-run that measurement before adding any wasmtime feature or moving its major version.
- **How the 40 pin ended** (2026-11-19 → 20): it was set because wasmtime 42+ with default
  features pulls `aws-lc-sys` (cmake + NASM). `cargo audit` then found 32 advisories in wasmtime /
  wasmtime-wasi 40.0.4 — the sandbox — with no 40.x fix; the owner chose to move. The port from 40
  was small: `preopened_dir` takes `FsPerms::ReadWrite` (= the old `DirPerms::all()` +
  `FilePerms::all()`), and wasmtime's own error type converts into anyhow's through its `anyhow`
  feature. `cargo test` passed unchanged (43/43); `cargo audit` 0 vulnerabilities.
- 🔒 **`reqwest` on `native-tls`** (Windows SChannel) for the same reason — no cmake/NASM.
- 🔒 **`wat` / `wasmprinter` / `walrus` are toolkit scope**, not engine scope; they stay when the
  engine is swapped (wasmrt `cmem/loaders.md`).
- **`[profile.dev] debug = "line-tables-only"`** (2026-11-19; owner asked "do we need that much
  for this?" of a 4.3 GB target dir): the debug profile was 3.0 GB of it, mostly full debug info
  of wasmtime and Cranelift; line tables keep file:line in backtraces. Only rsxtk's own builds
  (clippy, `cargo test`) read it; `cargo install` uses the release profile.
- **Every dependency licence is allowlisted** (2026-11-19, `scripts/check-licences.ts`, H37
  licensing): the allowlist holds exactly the licences in use; a new one fails the gate until it is
  judged and added here with its reason. `licences.txt` is the committed inventory.
- `component-model` dropped from rsxtk's own wasmtime features (2026-10-23): rsxtk never used it,
  and the measurement showed it costs nothing to declare or drop — `wasmtime-wasi` (`p2`, implied
  by `p1`) and `wasmtime-wasi-io` turn it on regardless. `cargo tree -e features -i wasmtime`
  lists the same 8 features with and without the line, so the build is identical. It goes away
  only with the wasmtime → wasmrt switch.

## Execution model

- **Nightly toolchain is a prerequisite**: `csrun` uses `cargo -Z script`.
- **Zero-config WASI sandbox**: the script's parent directory is the only preopen. Widening it is
  a security decision, not a convenience.
- 🔒 **rsxtk never runs a `.cwasm`** (owner, 2026-11-19: "do not run .cwasm files"). A `.cwasm`
  is precompiled native code that wasmtime loads with `unsafe` `Module::deserialize_file` and does
  not validate, so a file from elsewhere ran native code outside the sandbox. Now `run x.cwasm`
  (and `mod`, `bench`, `info`, which share `load_module`) refuses by extension before reading the
  file, and `src/main.rs` has no `unsafe`. Do not reintroduce `deserialize*` or `precompile_*`.
- **The script cache is plain wasm**: `.tk/<stem>/<stem>.wasm`, walrus-optimized and validated
  (`Module::validate`) before it is written, then validated and compiled by `Module::new` on each
  run. Fresh while newer than the script, `build.rs` and `rsc/`. Repeated runs skip `cargo` but
  not Cranelift (the cost of the rule above). No machine id: a wasm module is portable, so the
  hostname hash is gone too. A `.cwasm` / `machine.id` left by 0.5.2 or earlier is never read and
  is deleted at the next build. No wasmtime disk cache is configured (its `cache` feature is off).
  This is a behaviour break (`run x.cwasm` used to work): the next release is at least a minor.
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

## Review 0.6.0 — the owner's decisions (2026-11-20)

- 🔒 **The guest's folder is READ-ONLY; `--allow-write` grants write** (security S1: a module saved
  into a shared folder could rewrite every file beside it). The folder stays the only preopen.
- 🔒 **`--max-memory <MiB>` (default 4096) is ONE budget for all of a guest's memories and tables**
  (`GuestBudget`, a `ResourceLimiter`; a table element counts as a pointer). **No time limit** on a
  guest or on `cargo build` (Ctrl-C ends a run; deep-nesting compile time, H7, was rejected on this).
- **Manifest editing** (`add` / `remove`): only `[dependencies]` is edited; a crate's
  `[dependencies.<crate>]` table and sub-tables, in any quoting, are its entry; comments and strings
  are skipped; an entry that never closes or a manifest with multi-line strings is refused, never
  guessed. The manifest must start the file (a BOM and blank lines may precede it).
- **A lock per script** for add / remove / fmt (kept files; per-user folder; 60-s wait, then a
  named error). **The script cache** is keyed by a fingerprint of the sources, not timestamps.
- 🔒 **GitHub rulesets (owner, 2026-11-20):** `main` — no deletion, no force-push; tags
  `refs/tags/rsxtk_v*` — no deletion, force-push or update. **The logo** was made with Gemini's help
  (owner); its terms were not verified by the review (not a finding).

## Full review 2 (before 1.0.1) — decisions and the rules the fixes set (2026-11-20)

- 🔒 **Publishing gate and tag creation: left as they are** (owner: "Leave as is", "Record as
  accepted"). Any writer's push to main can publish; the `rsxtk_v*` ruleset has no `creation` rule
  (CI refuses a version whose tag peels elsewhere).
- **1.0.0's CLI is kept**: a guest's `proc_exit(N)` stays exit 1 + "Exited status: N"; of the
  Windows device names only `nul` is refused (con, prn, aux, com1-9, lpt1-9 ran under 1.0.0).
- **Script names are what Cargo builds** (letter or `_` first; letters, digits, `-`, `_`; not
  build / deps / examples / incremental, in any case). Rust keywords build, so they are allowed.
- **`add`'s version check is Cargo's grammar**, checked against `cargo metadata` (a tail needs a
  full MAJOR.MINOR.PATCH; numbers fit a u64; a bare `*`/`x`/`X` only alone). Re-check it against
  Cargo, not by reading, when it changes.
- **Two writers:** `rewrite_script` writes THROUGH a link (only the user's named file: add, remove,
  fmt, optimize without `-o`); `write_atomic` replaces whatever is at the name and never follows a
  link, its temp made with `create_new` under a pid+time+attempt name. Every other output, the
  `.tk` mirror included, goes through `write_atomic`.
- **`.tk`**: a link at the paths rsxtk keeps in it is refused; `.tk` itself may be a link (a cache
  on another drive); on Unix a `.tk` another user owns or every user may write is refused, and
  `.tk/<stem>` is created 0700 and kept 0700 (an existing open one is removed, then tightened —
  fail closed). Linux lock folder with no usable HOME: `<temp>/rsxtk-locks-<uid>`; a link at the
  lock folder's name is refused.
- **Threat model (re-review 17):** whoever can write the folder a script lives in can edit it or add
  a `build.rs` (native code), so swap races by such a user are out of scope; the `.tk` checks are
  defence in depth, and the README says a shared folder is not made safe.
- **`rsc/` links are followed** (the user's own tree; refusing them would break 1.0.0) — README.
- **The crates.io read-back compares checksums** with this commit's package; `cargo package` is
  reproducible (1.0.0 repackaged = crates.io's checksum).
- **Test-only knob:** a DEBUG build reads `RSXTK_TEST_RUSTFMT_LIMIT_MS`; release builds never do.

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
  breaks; no cap, no carry. 🔒 **rsxtk is at `1.0.0`** (released by CI 2026-11-20; owner's choice
  when the gate refused a 0.x minor): its API promise has begun — a removed or reshaped command,
  option or behaviour needs `2.0.0`. Keep a README Breaking Changes table as the migration guide
  (none exists yet — open-work 15). The gate has no 0.x rule: red previous-release tests need a major.
- **A bump is gated** (workspace rule, `../workspace/cmem/layout-and-packaging.md` § "The
  compatibility gate"): rsxtk's surface is its CLI — flags, positional shapes, exit codes, output a script may
  parse. **The committed golden `surface.txt`** (every `--help` page, since 2026-11-12) and the
  test suite are the snapshot; `scripts/bump.ts` and `compat.ts --release` diff the golden's
  STRUCTURE against the previous tag's (descriptions do not count) and refuse a level below the
  floor: a removed or reshaped command / argument / option / value, or a new required argument on
  an existing command → major; an addition → minor. A stale golden is refused. What `--help`
  does not show (exit codes, output formats) is covered by running the previous tag's tests. How
  a release runs: [release.md](release.md).
- The `Cargo.toml` version line is the only source. `README.md` no longer restates it (its
  "New Features in v0.4.2" heading drifted against 0.4.4; it became "Features" on 2026-10-23) —
  keep version numbers out of the README. Since 0.5.0 `main.rs` takes it from Cargo
  (`#[command(name = "rsxtk", version)]` = `CARGO_PKG_VERSION`), so a bump is ONE line plus the
  lockfile's own entry; the `--version` test guards it.
- **The crate excludes `cmem/`, `.github/`, `scripts/`, `surface.txt`** (`exclude` in
  `Cargo.toml`; `cmem/` 2026-10-23, `.github/` and `scripts/` 2026-11-09, `surface.txt`
  2026-11-12): they are for working on rsxtk, not for crate users; they stay public on GitHub.
  `cargo package --list` for 0.5.1: `.cargo_vcs_info.json`, `.gitattributes`, `.gitignore`,
  `Cargo.lock`, `Cargo.toml`(`.orig`), both licenses, `README.md`, `rsxtk_logo.png`,
  `src/main.rs`, `tests/cli.rs`.
- Nothing is published unless the owner says so ("release", H34). 🔒 **Releases run in the
  owner's order** (2026-11-10, letter H33): `bump.ts <level>` → `release.ts` pushes `main` only →
  CI checks → CI tags `rsxtk_vX.Y.Z` → the Release page from the tag → crates.io through trusted
  publishing; **nobody tags or publishes by hand.** The whole process, the owner items and every
  release: [release.md](release.md). Invariants that must not be silently reverted:
  - 🔒 **CI only publishes** (owner, 2026-11-17, H37 (e)): every gate and test suite runs
    locally in `release.ts`, after the review gate (H37 (d)); CI verifies the version, decides
    "is this a release?", packages the crate (`cargo package --locked`, no secret — owner's
    option 1, 2026-11-19, so `publish` keeps `--no-verify` and builds nothing beside the
    id-token), tags, makes the page, publishes, skips a published version, reads back;
  - the tag is CI's, annotated, made only after `checks` is green on that commit, and never moves;
  - every job holds only its own permission (workspace C30): `checks` no secret, `tag` / `release`
    no checkout, `publish` only `id-token: write` + read;
  - no `gh` and no `uses:` (H30); Bun pinned by version + sha256 (Tooling rule 10);
  - CI's Rust is the local gate's dated nightly (`RUST_TOOLCHAIN`), so the package CI verifies is
    built by the compiler the tests passed on; the tests need nightly (`csrun`), rustfmt (`fmt`)
    and `wasm32-wasip1`, locally;
  - the release scripts' throwaway crates never share the repo's target directory
    (best-practices R.5).
  **No backfill** (owner, 2026-11-09: "only new tags will get release pages").

## How the memory is kept (and how its rules changed)

- **2026-09-20** — `cmem/` created by owner direction ("rsxtk should be set up with memory — that
  project was the first and does not have one"), after the workspace convention then in
  `memory-conventions.md` and binaryang's cleanup policy (working memory, not an archive; git is
  the archive). First committed 2026-10-20. The root `CLAUDE.md` became an untracked pointer.
- **2026-10-22 / 10-23 / 10-26** — the privacy policy and the path test (letters H19, H22), the one
  email exception and its widening (H28); now `working-rules.md` § Privacy.
- **2026-11-10** — the owner's "commit" / "push" / "release" written into `INDEX.md` by the
  workspace on the owner's direction (H34); moved unchanged to `working-rules.md` on 2026-11-19.
- 🔒 **2026-11-19 — the owner's memory guide** (letter H38, `../workspace/cmem/memory-guide.md`):
  `STATE.md` added; `INDEX.md` cut from 8.9 KB to a router, its policy, triggers, tooling and owner
  commands moved to the new `working-rules.md`; the ★ block moved unchanged to
  `best-practices-shared.md` (owner: keep the copy, in a file of its own); closed open-work items
  and the record of letters answered moved to `cmem/archive/`; `scripts/check-cmem.ts` copied from
  the workspace's `tools/`. The workspace memory itself moved to the `workspace` repository beside
  this one (H42), and its correspondence became one file per item (H41).
- 🔒 **2026-11-19 — the final review standard** (letter H37): open issues live in the gitignored
  `cmem/review-issues.md`; the manual privacy `git grep` became the gate `scripts/check-privacy.ts`
  (adapted from LeptonPad's, which came from binaryang's).
