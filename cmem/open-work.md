# open-work.md — what is outstanding in rsxtk

Only open items. Created 2026-09-20.

1. **README lags `main.rs`**: the feature list is headed "New Features in v0.4.2" (Cargo is
   0.4.4) and the command table omits `mod`, `convert`, `initmod`, `bench`, `optimize`, `info`.
   Sync from `overview.md`'s table; user-relevant, so README is the right home.
2. **No test suite** — `testing.md` § "What a real suite needs". This is the referee for the engine
   switch and should exist before it.
3. **The wasmrt switch**: measure first (rlib size; end-to-end time wasmtime vs wasmrt, interleaved,
   `.cwasm` off), then swap `wasmtime` + `wasmtime-wasi` for `wasmrt-core`; keep `wat` /
   `wasmprinter` / `walrus`. Owner decision that rsxtk goes first with wasmtk (2026-09-20).
4. **`component-model` feature enabled and unused** — measure the size cost, then remove or record why it stays.
5. **This machine**: `git config --global --add safe.directory <this path>` is missing (git refuses
   the exFAT checkout); record in `local/environment.md` once added.