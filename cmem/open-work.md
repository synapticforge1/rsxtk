# open-work.md — what is outstanding in rsxtk

Only open items. Created 2026-09-20. Numbers are stable (other files cite them); a closed item's
number is not reused (closed 2026-10-23: 1 — README synced, `build.rs`/`rsc/` mirrored; 5 — history
purged, `overview.md` § History).

2. **No test suite** — `testing.md` § "What a real suite needs". This is the referee for the engine
   switch and should exist before it.
3. **The wasmrt switch**: measure first (rlib size; end-to-end time wasmtime vs wasmrt, interleaved,
   `.cwasm` off), then swap `wasmtime` + `wasmtime-wasi` for `wasmrt-core`; keep `wat` /
   `wasmprinter` / `walrus`. Owner decision that rsxtk goes first with wasmtk (2026-09-20).
4. **Two clippy warnings in untouched code** (2026-10-23): `map_or` can be simplified
   (`convert_wasm_wat`, `main.rs:414`); collapsible `if` (`edit_deps`, `main.rs:448`). Plus the
   Cargo manifest lint `redundant_homepage` (`homepage` = `repository`).
6. **Siblings still name the pre-move URL `jrmarcum/rsxtk`** (2026-10-20; repo is now
   `synapticforge1/rsxtk`, `overview.md` § Hosting): wasmtk `cmem/vision.md:83`,
   `cmem/polyglot-producers.md:74`, `cmem/roadmap.md:1418`; wasmrt `cmem/loaders.md:28`. Not ours to
   edit (best-practices 8.1). 🔒 Owner, 2026-10-20: these get fixed when wasmtk moves to the org;
   no handoff letter. Nothing for rsxtk to do — close when a grep for `jrmarcum/rsxtk` in those
   trees comes back empty.