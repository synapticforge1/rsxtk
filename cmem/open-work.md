# open-work.md — what is outstanding in rsxtk

Open items only. Last verified 2026-11-19. Numbers are stable (other files cite them) and never
reused; closed items are in [archive/open-work-closed.md](archive/open-work-closed.md).

- **3. The wasmrt switch**: measure first (rlib size; end-to-end time wasmtime vs wasmrt,
  interleaved, `.cwasm` off), then swap `wasmtime` + `wasmtime-wasi` for `wasmrt-core`; keep `wat`
  / `wasmprinter` / `walrus`. Owner decision that rsxtk goes first with wasmtk (2026-09-20). The
  referee is `cargo test` (engine-agnostic assertions); it must pass unchanged after the swap.
- **11. 🗓️ OWNER — retire the old crates.io token** (D2, letter H33; added 2026-11-12): trusted
  publishing is registered and MEASURED (0.5.2's `trustpub_data`, `release.md`), so nothing uses
  the repo secret `CARGO_REGISTRY_TOKEN` any more: delete it (GitHub → Settings → Secrets and
  variables → Actions) and revoke that API token on crates.io.
- **12. H37 (d) + (e): the release path under the final review standard** (added 2026-11-19).
  (d) `scripts/release.ts` runs the review set first: refuse unless `review-process.md` records
  `Review complete: <commit>` and no code changed since. (e) `release.yml` loses its test steps
  (build, clippy, `cargo test`, `bun test`, the surface and compat gates — all already run by
  `release.ts`); CI keeps checkout, version, "is this a release?", tag, page, publish, read-back.
  🗓️ **Waiting on the owner:** what replaces the `checks` packaging that `cargo publish
  --no-verify` relies on — (1) a credential-less `cargo package --locked` kept in `checks`,
  (2) drop `--no-verify` (builds beside the id-token), or (3) trust the local gate's package.
  Rehearse the changed job before it lands (best-practices 7.22); never mid-release.
- **13. A licence inventory** (H37, licensing review): none exists. Before the first review, a
  gate that lists every crate in `Cargo.lock` with its licence read from the source, checked
  against an allowlist compatible with `MIT OR Apache-2.0` (`review-process.md` § Slots).
