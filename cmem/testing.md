# testing.md — how to verify a change

Created 2026-09-20; rewritten 2026-10-23 when the suite landed. Read results by OUTPUT (an exit
code alone is not evidence), on the tree as it will be committed.

## The suite: `cargo test` (tests/cli.rs, 2026-10-23)

```
CARGO_TARGET_DIR=<short NTFS dir> cargo test           # 43 pass, 0 ignored (2026-11-19, ~3 s)
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
cargo clippy --all-targets -- -D warnings   # 0 warnings since 2026-10-23 — keep it at 0
cargo test
```

plus, since 2026-11-12, `bun test scripts/` and `bun scripts/compat.ts surface --check` (a CLI
change regenerates `surface.txt`: `bun scripts/compat.ts surface`, review the diff, commit it).

plus the gates `bun scripts/check-privacy.ts`, `bun scripts/check-licences.ts` (a dependency
change regenerates `licences.txt` with `--write`: review the diff, commit it) and
`bun scripts/check-cmem.ts`.

**Every gate runs locally, and only locally** (owner, 2026-11-17, letter H37 (e)): CI no longer
repeats it; `scripts/release.ts` runs all of it before the push, after the review gate. Until
0.5.2, CI's `checks` repeated the gate on every push of `main`. Run
2026-11-12 locally (Windows, nightly 2026-11-04): release build OK, clippy 0 warnings with `-D
warnings`, 41 pass, 45 script tests pass, `surface.txt` fresh (60 entries). Re-run 2026-11-19
(Windows 11, win-x64, same nightly): the same, with 57 script tests; release build 3 min 29 s.
Again after the `.cwasm` removal, from a COLD target dir: release build 3 min 43 s, clippy 0,
`cargo test` **43** pass (+ `run_cwasm_is_refused`, `run_rs_ignores_an_old_cwasm_cache`, both
seen to fail against their mutants), 70 script tests, every gate green; 6 min 23 s in all.

Use `CARGO_TARGET_DIR` on NTFS and SHORT: a deep path breaks MinGW `dlltool` in `windows-sys`
("Can't open def file"); `%TEMP%\rsxtk-t` works (`local/environment.md`).

**Debug builds are fine.** Corrected 2026-10-23: the stack overflow on `run` ("thread 'main' has
overflowed its stack") came only from the OLD April debug binary left in the D:-drive `target/`
(it fails on every fresh build, 2/2); a debug build of today's code passes fresh and cached
builds. Do not use the stale binaries in `target/`.

## Comparing binaries (measured 2026-10-26)

**A whole-file hash of `rsxtk.exe` is not evidence**: two release builds of the SAME source differ
(MinGW's linker stamps the PE header). Measured: they differ in exactly two fields, the COFF
`TimeDateStamp` (offset `0x88`) and the optional header `CheckSum` (`0xd8`); everything else of
13,806,080 bytes is identical. So "same binary" means: byte-diff with those two fields masked, and
show the control (two builds of one source) first. Used for item 9: the CRLF tree (`aef343e`) and
the LF tree (`bae61ee`), built from the same path and target dir, differ only in those two fields
— no code change, as expected (rustc normalizes CRLF to LF before lexing; `main.rs:244` is the one
multi-line literal, a `\` continuation).

## Line endings

`.gitattributes` (2026-10-26): `* text=auto eol=lf`, `*.png binary`. Check with
`git ls-files --eol` — every text file `i/lf w/lf`. A CRLF file in the working tree after a
renormalize is refreshed by deleting it and `git checkout -- <file>`.

## The release scripts: `bun test scripts/` (2026-11-12)

70 tests in 6 files, all pass on Bun 1.4.4 (2026-11-19; 45 in 3 files before that day's gates):
- `check-licences.test.ts` (7) — SPDX OR / AND / precedence / WITH / the legacy `/`, malformed
  expressions refused, a missing licence, rsxtk's own licence, the inventory's order. **Inverted**:
  4 mutants (AND read as OR, `/` not OR, trailing tokens ignored, own licence unchecked), each
  caught; on the real tree, Unicode-3.0 removed from the allowlist → 19 problems, as expected.
- `review-gate.test.ts` (6) — the `Review complete` bullet (prose with a real commit id is not
  one); no record, two, an unknown or a non-ancestor commit refuse; anything outside `cmem/`
  changed since refuses. **Inverted**: 5 mutants, each caught (the fifth only after a test for a
  commit id in prose was added — the first version of the suite missed it).
- `check-privacy.test.ts` (12, 2026-11-19) — machine paths (drive, WSL, Git Bash, macOS, Linux;
  the CI runner's home and placeholders pass), token shapes, a URL password, masking, the one
  maintainer address in `Cargo.toml` only, tracked private files, UTF-16 text, arguments. Planted
  values are built by concatenation so the file passes its own gate. **Inverted** with three
  one-line mutants (issues file not private, any file may hold the maintainer, UTF-16 skipped),
  each caught by its own test.
- `github-release.test.ts` (21) — `state` / `tag` / `page` as a child process against a mock
  GitHub REST API (`Bun.serve`): new / rerun / none; the annotated tag created and read back,
  already there, refused elsewhere, a bad read-back, a rejected ref; the page in H32's order
  (missing tag, tag elsewhere, existing page, POST rejected, draft read-back, tag moved by the
  POST, dry-run, no token); a stalled API hits the time limit; non-X.Y.Z refused.
- `crates-io.test.ts` (14) — `has` / `token` / `revoke` / `readback` against one mock playing
  GitHub's OIDC endpoint and crates.io: the token masked and never printed, a refused exchange,
  the read-back's polling, yanked = fatal.
- `compat.test.ts` (10) — the CLI floor on hand-written `--help` pages: descriptions ignored;
  removed / renamed / reordered → major; a new required argument on an existing command → major;
  a new optional argument, option, value, or a whole new command → minor; the version helpers.
**Inverted** 2026-11-12 on scratch copies, one-line mutants each caught by its own test: the floor
ignoring removals (2 fail), no tag read-back (1), rerun misreported as new (1), yanked accepted
(1). The binary-driven half (`surfaceText`, stale golden, the previous tag's tests) was inverted
with the real binary in the release rehearsal (`release.md` § How the release path got here).
On 2026-11-09 the first page-only script was inverted the same way (2 mutants).

## Manual checks still worth a look

The suite does not cover: `add` WITHOUT a version (fetches the latest from crates.io; falls back
to `*` offline), `optimize` without `-o` (overwrites its input), and a real `.cwasm` from another
machine (machine-id mismatch message).

Rules inherited from the workspace: diff the output against the previous run, not the exit code;
rebuild the unmodified commit before blaming a toolchain failure; a hang is a measurement — read
CPU time first; never pipe when the exit code is the measurement.
