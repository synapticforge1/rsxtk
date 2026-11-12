# open-work.md — what is outstanding in rsxtk

Only open items. Created 2026-09-20. Numbers are stable (other files cite them); a closed item's
number is not reused (closed 2026-10-23: 1 — README synced, `build.rs`/`rsc/` mirrored; 5 — history
purged, `overview.md` § History; 6 — siblings' stale `jrmarcum/rsxtk` links fixed in their repos,
verified read-only: wasmtk `vision.md:83`, `polyglot-producers.md:74`, `roadmap.md:1418` and wasmrt
`loaders.md:28` all name `synapticforge1/rsxtk`; the two remaining mentions record the move; 2 —
the test suite, `testing.md`; 4 — clippy and the manifest lint at 0 warnings; 7 — `add`/`remove`
prefix match fixed, `design-decisions.md`; 8 — 0.5.0 published to crates.io, `overview.md` §
Publishing; closed 2026-10-26: 9 — line endings pinned to LF by `.gitattributes` and the five CRLF
files renormalized, `bae61ee`, release binary proved identical, `testing.md` § Comparing binaries;
closed 2026-11-09: 10 — the release workflow's first real run, 0.5.1, all three jobs green,
`design-decisions.md` § Versioning and release).

- **3. The wasmrt switch**: measure first (rlib size; end-to-end time wasmtime vs wasmrt,
  interleaved, `.cwasm` off), then swap `wasmtime` + `wasmtime-wasi` for `wasmrt-core`; keep `wat`
  / `wasmprinter` / `walrus`. Owner decision that rsxtk goes first with wasmtk (2026-09-20). The
  referee is `cargo test` (engine-agnostic assertions); it must pass unchanged after the swap.
- **11. 🗓️ OWNER — crates.io trusted publishing for rsxtk** (D2, letter H33; added 2026-11-12):
  registered on crates.io by the owner (owner, 2026-11-12: "Yes, it's registered" — `rsxtk` →
  Settings → Trusted Publishing → GitHub, `synapticforge1/rsxtk`, workflow `release.yml`);
  UNMEASURED until 0.5.2 publishes through it. After that, delete the repo secret
  `CARGO_REGISTRY_TOKEN` and revoke its crates.io token (owner).
