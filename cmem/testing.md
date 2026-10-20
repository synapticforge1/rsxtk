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
