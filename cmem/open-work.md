# open-work.md — what is outstanding in rsxtk

Only open items. Created 2026-09-20. Numbers are stable (other files cite them); a closed item's
number is not reused (closed 2026-10-23: 1 — README synced, `build.rs`/`rsc/` mirrored; 5 — history
purged, `overview.md` § History; 6 — siblings' stale `jrmarcum/rsxtk` links fixed in their repos,
verified read-only: wasmtk `vision.md:83`, `polyglot-producers.md:74`, `roadmap.md:1418` and wasmrt
`loaders.md:28` all name `synapticforge1/rsxtk`; the two remaining mentions record the move; 2 —
the test suite, `testing.md`; 4 — clippy and the manifest lint at 0 warnings; 7 — `add`/`remove`
prefix match fixed, `design-decisions.md`).

3. **The wasmrt switch**: measure first (rlib size; end-to-end time wasmtime vs wasmrt, interleaved,
   `.cwasm` off), then swap `wasmtime` + `wasmtime-wasi` for `wasmrt-core`; keep `wat` /
   `wasmprinter` / `walrus`. Owner decision that rsxtk goes first with wasmtk (2026-09-20). The
   referee is `cargo test` (engine-agnostic assertions); it must pass unchanged after the swap.
