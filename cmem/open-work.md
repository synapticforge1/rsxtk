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
5. **Siblings still name the pre-move URL `jrmarcum/rsxtk`** (2026-10-20; repo is now
   `synapticforge1/rsxtk`, `overview.md` § Hosting): wasmtk `cmem/vision.md:83`,
   `cmem/polyglot-producers.md:74`, `cmem/roadmap.md:1418`; wasmrt `cmem/loaders.md:28`. Not ours to
   edit (best-practices 8.1). 🔒 Owner, 2026-10-20: these get fixed when wasmtk moves to the org;
   no handoff letter. Nothing for rsxtk to do — close when a grep for `jrmarcum/rsxtk` in those
   trees comes back empty.