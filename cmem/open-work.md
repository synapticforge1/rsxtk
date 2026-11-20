# open-work.md — what is outstanding in rsxtk

Open items only. Last verified 2026-11-19. Numbers are stable (other files cite them) and never
reused; closed items are in [archive/open-work-closed.md](archive/open-work-closed.md).

- **3. The wasmrt switch**: measure first (rlib size; end-to-end time wasmtime vs wasmrt,
  interleaved), then swap `wasmtime` + `wasmtime-wasi` for `wasmrt-core`; keep `wat` /
  `wasmprinter` / `walrus`. Owner decision that rsxtk goes first with wasmtk (2026-09-20). The
  referee is `cargo test` (engine-agnostic assertions); it must pass unchanged after the swap.
- **11. 🗓️ OWNER — retire the old crates.io token** (D2, letter H33; added 2026-11-12): trusted
  publishing is registered and MEASURED (0.5.2's `trustpub_data`, `release.md`), so nothing uses
  the repo secret `CARGO_REGISTRY_TOKEN` any more: delete it (GitHub → Settings → Secrets and
  variables → Actions) and revoke that API token on crates.io.
- **15. The next release is 1.0.0** (owner, 2026-11-20, at "release"): `run x.cwasm` stopped
  working and the guest folder became read-only by default; a 0.5.2 test checks the removed cache,
  so the gate requires a major. 1.0.0 starts the API promise: a README Breaking Changes table (the
  migration guide named in `design-decisions.md`) is still to be written.
