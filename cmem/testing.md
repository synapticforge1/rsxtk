# testing.md — how to verify a change today, and what a suite needs

Created 2026-09-20. **There is no test suite.** Until there is, every `src/` change is verified by
the smoke set below, read by OUTPUT (an exit code is not evidence), on the committed tree after the
last edit.

## Smoke set (run from the repo root; write results to a file, then `echo "exit=$?"`)

```
cargo build --release
cargo clippy --all-targets
./target/release/rsxtk init smoke_script.rs
./target/release/rsxtk run smoke_script.rs                 # expect the template's output
./target/release/rsxtk build smoke_script.rs wat           # expect a .wat beside the .tk artifact
./target/release/rsxtk run <the README hello.wat>          # expect "Hello, Rosetta Code!"
./target/release/rsxtk info <any .wasm>                    # expect export signatures
./target/release/rsxtk clean
```

Also worth running (2026-10-23): `rsxtk mod <lib>.wat add 2 3` → `5`; the same with an argument
missing → `Missing argument for parameter 1`, exit 1; `rsxtk run nope.wasm` → exit 1;
`rsxtk initmod my-lib` then `rsxtk mod my-lib.rs add 4 5` → `9` (library mirror, hyphen name);
`rsxtk mod my-lib.rs nosuch 1` → `Func 'nosuch' not found`, exit 1; `rsxtk run <dir>/lib.wat`
from ANOTHER folder → the export listing, exit 0.

Mirroring checks (2026-10-23, each in its own folder): a script with
`include_str!("rsc/embed.txt")` prints the file; edit the file → the next run prints the new text
(cache rebuilt); remove `rsc/` → build fails and `.tk/<stem>/src/rsc` is gone. A `build.rs` that
emits `cargo:rustc-env=X=…` and a script printing `env!("X")` → the value; edit `build.rs` → the new
value; remove `build.rs` → compile error and `.tk/<stem>/build.rs` is gone.

**Run the smoke set with a RELEASE binary.** The debug build overflows its stack on `run`
(`thread 'main' has overflowed its stack`, exit `0xC00000FD`, 2026-10-23) before the script
starts, so the negative control dies the same way as the positive one, and neither proves
anything. Build with `CARGO_TARGET_DIR` on NTFS and SHORT (a deep path breaks MinGW `dlltool` in
`windows-sys`: "Can't open def file").

Rules inherited from the workspace: diff the output against the previous run, not the exit code;
rebuild the unmodified commit before blaming a toolchain failure; a hang is a measurement — read
CPU time first; never pipe when the exit code is the measurement.

## What a real suite needs (open item 2)

- A `tests/` directory with fixture scripts (`.rs` with manifest, a `.wat`, a `.wasm`, a library
  module for `mod`), and `cargo test` integration tests that spawn the binary and compare stdout.
- One negative case per command (missing file, bad manifest, unknown export) asserting the ERROR
  outcome, invalid for exactly one reason.
- Inversion: each new test must be seen to fail (break the thing on purpose) before it counts.
- Every test must pass on wasmtime 40 today AND be engine-agnostic enough to run unchanged when
  the engine becomes wasmrt — the suite is the referee for the switch.
- Generated artifacts (`.tk/`, `.cwasm`) stay untracked.
