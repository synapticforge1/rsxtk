# open-work.md — what is outstanding in rsxtk

Open items only. Last verified 2026-11-20. Numbers are stable (other files cite them) and never
reused; closed items are in [archive/open-work-closed.md](archive/open-work-closed.md).

- **3. The wasmrt switch** — 🗓️ **NOT YET (owner, 2026-11-20):** "we are not quite ready for that
  step. there is much more work with wasmrt/wazmrt before a decision can be made." Do not start it
  or propose it as next until the owner says wasmrt is ready. When it is: measure first (rlib size; end-to-end time wasmtime vs wasmrt,
  interleaved), then swap `wasmtime` + `wasmtime-wasi` for `wasmrt-core`; keep `wat` /
  `wasmprinter` / `walrus`. Owner decision that rsxtk goes first with wasmtk (2026-09-20). The
  referee is `cargo test` (engine-agnostic assertions); it must pass unchanged after the swap.
