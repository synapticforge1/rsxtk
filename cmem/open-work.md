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
- **15. The next release is at least a MINOR** (added 2026-11-19): `run x.cwasm` stopped working
  (owner: rsxtk never runs a `.cwasm`), a behaviour break `surface.txt` cannot see (`--help` is
  unchanged). `bump.ts` would allow a patch: choose `minor` (0.6.0). The README notes it.
- **16. The first review under the final standard** (added 2026-11-19): `release.ts` now refuses
  without `Review complete: <commit>` in `review-process.md`, so "review" must run before the
  next "release".
