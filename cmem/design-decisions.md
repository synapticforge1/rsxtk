# design-decisions.md — invariants that must not be silently reverted

Dated; the reason travels with the rule. Created 2026-09-20 from `Cargo.toml` comments, `README.md`
§ Troubleshooting, and the sibling memories that record decisions about rsxtk.

## Engine and dependencies

- 🔒 **`wasmtime = 40.0.1`, `default-features = false`.** Versions 42.x+ reintroduce `aws-lc-sys`
  in the dependency chain, which needs NASM and cmake to build; this machine has neither, and the
  build fails with `Missing dependency: cmake`. Do not upgrade until NASM + cmake are available or
  a pure-Rust crypto path exists in that version. A caret would not protect this: the constraint is
  correctness of the build, so it lives in the specifier.
- 🔒 **`reqwest` on `native-tls`** (Windows SChannel) for the same reason — no cmake/NASM.
- 🔒 **`wat` / `wasmprinter` / `walrus` are toolkit scope**, not engine scope; they stay when the
  engine is swapped (wasmrt `cmem/loaders.md`).
- ⚠️ `component-model` is enabled on wasmtime and nothing uses it (wasmrt `cmem/loaders.md`).
  Recorded, not removed: removing it is a size measurement first.

## Execution model

- **Nightly toolchain is a prerequisite**: `csrun` uses `cargo -Z script`.
- **Zero-config WASI sandbox**: the script's parent directory is the only preopen. Widening it is
  a security decision, not a convenience.
- **`.cwasm` pre-compilation + wasmtime disk cache** make repeated runs skip Cranelift; the cache
  key includes a machine id (OS, arch, hostname). ⚠️ Under an interpreter engine (wasmrt) `.cwasm`
  would not be the default — re-decide when the engine changes.
- **Smart entry**: `_start` if exported, else list callable exports; `mod` calls one by name.
- **The manifest is the `---`-fenced TOML header**; `fmt` must preserve it (a formatter that eats
  the header eats the dependencies).

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
  breaks; no cap, no carry. rsxtk is at `0.4.4`, so consumers' carets treat every minor as
  potentially breaking until `1.0.0`, where its API promise begins. Keep a README Breaking Changes
  table as the migration guide (none exists yet).
- **A bump is gated** (workspace rule, `../../cmem/layout-and-packaging.md` § "The compatibility
  gate"): rsxtk's surface is its CLI — flags, positional shapes, exit codes, output a script may
  parse. A committed golden `--help` and the smoke set are the snapshot; a bump script diffs them
  and refuses a patch when a command or flag was removed or reshaped. Needs the test suite first
  (`open-work.md` item 2).
- The `Cargo.toml` version line is the only source; `README.md` restates it and has already
  drifted (0.4.2 vs 0.4.4) — see `open-work.md`.
- Nothing is published unless the owner says so; `cargo publish` from a clean tree, tag after.
