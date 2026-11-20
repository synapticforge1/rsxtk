# open-work.md — what is outstanding in rsxtk

Open items only. Last verified 2026-11-20. Numbers are stable (other files cite them) and never
reused; closed items are in [archive/open-work-closed.md](archive/open-work-closed.md).

- **3. The wasmrt switch** — 🗓️ **NOT YET (owner, 2026-11-20):** "we are not quite ready for that
  step. there is much more work with wasmrt/wazmrt before a decision can be made." Do not start it
  or propose it as next until the owner says wasmrt is ready. When it is: measure first (rlib size; end-to-end time wasmtime vs wasmrt,
  interleaved), then swap `wasmtime` + `wasmtime-wasi` for `wasmrt-core`; keep `wat` /
  `wasmprinter` / `walrus`. Owner decision that rsxtk goes first with wasmtk (2026-09-20). The
  referee is `cargo test` (engine-agnostic assertions); it must pass unchanged after the swap.
- **11. 🗓️ OWNER — retire the old crates.io token** (D2, letter H33; added 2026-11-12): trusted
  publishing is registered and MEASURED (0.5.2's `trustpub_data`, `release.md`), so nothing uses
  the repo secret `CARGO_REGISTRY_TOKEN` any more: delete it (GitHub → Settings → Secrets and
  variables → Actions) and revoke that API token on crates.io. **GitHub secret deleted (owner,
  2026-11-20; not readable without auth, so not verified here).** Left: revoke the token on
  crates.io (Account Settings → API Tokens).
