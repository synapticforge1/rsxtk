# testing.md — how to verify a change

Created 2026-09-20; rewritten 2026-10-23 when the suite landed. Read results by OUTPUT (an exit
code alone is not evidence), on the tree as it will be committed.

## The suite: `cargo test` (tests/cli.rs, 2026-10-23)

```
CARGO_TARGET_DIR=<short NTFS dir> cargo test           # 41 pass, 0 ignored (2026-10-23, ~3 s)
```

- **End to end, one file.** Each test runs the real binary (`CARGO_BIN_EXE_rsxtk`) in its own
  fresh folder under `%TEMP%\rsxtk-tests\<test>` and checks stdout and the exit code. Inputs are
  written inline (no committed fixtures); `.wasm` inputs come from the `wat` crate. No network:
  `add` always gets an explicit version.
- **Every command** has a passing case and a case that must fail: `run` (`.wat`, `.wasm`, `.rs`
  with cache, missing file, unsupported extension, no manifest, a library from another folder),
  `mod` (call, missing argument, unknown function), `init`/`initmod`, `build` (`wat`, bad target),
  `convert` (round trip, invalid WAT), `resource`, `rsc/` at run time and embedded (with rebuild
  on change), `build.rs`, `bench`, `add`/`list`/`remove` (incl. manifest injection, an
  incomplete manifest, and crates sharing a prefix: `serde` vs `serde_json`), `fmt`, `optimize`,
  `info`, `clean`, `csrun` (needs nightly), and
  `--version` = `CARGO_PKG_VERSION` (catches `main.rs`'s repeated version string drifting).
- **Inversion done 2026-10-23**: four deliberate breaks (bare-filename library info, library
  detection forced off, `rsc/` copy disabled, `--version` changed) made exactly their four tests
  fail; then the old prefix match in `edit_deps` made exactly `remove_keeps_crates_that_share_a_prefix`
  and `add_replaces_only_the_named_crate` fail (the plain add/list/remove test still passed).
  Restored each time, the suite passed again. A new test counts only once it has been seen to fail.
- **Engine-agnostic**: the assertions are rsxtk's own output and the guest's stdout, never
  wasmtime messages, so the suite should run unchanged on wasmrt — it is the referee for item 3.

## The regression gate (a change to `src/` or `Cargo.toml`)

```
cargo build --release
cargo clippy --all-targets      # 0 warnings since 2026-10-23 — keep it at 0
cargo test
```

Use `CARGO_TARGET_DIR` on NTFS and SHORT: a deep path breaks MinGW `dlltool` in `windows-sys`
("Can't open def file"); `%TEMP%\rsxtk-t` works (`local/environment.md`).

**Debug builds are fine.** Corrected 2026-10-23: the stack overflow on `run` ("thread 'main' has
overflowed its stack") came only from the OLD April debug binary left in the D:-drive `target/`
(it fails on every fresh build, 2/2); a debug build of today's code passes fresh and cached
builds. Do not use the stale binaries in `target/`.

## Manual checks still worth a look

The suite does not cover: `add` WITHOUT a version (fetches the latest from crates.io; falls back
to `*` offline), `optimize` without `-o` (overwrites its input), and a real `.cwasm` from another
machine (machine-id mismatch message).

Rules inherited from the workspace: diff the output against the previous run, not the exit code;
rebuild the unmodified commit before blaming a toolchain failure; a hang is a measurement — read
CPU time first; never pipe when the exit code is the measurement.
