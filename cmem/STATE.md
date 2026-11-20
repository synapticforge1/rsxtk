# STATE: rsxtk

Updated: 2026-11-20, at the end of the session

## Version
- Released: 1.0.0 on 2026-11-20 (crates.io, by CI in the owner's order; run 44779881233)
- Working branch: main

## Now
- 1.0.0 is out: tag, page (Tested on win-x64 + linux-x64) and crates.io read back; user install works.
- The review that preceded it is complete (product at 940e335; record at 63d71b6).
- wasmtime / wasmtime-wasi 50.0.3; `cargo audit` 0 vulnerabilities; no C library in the build.
- Guest sandbox: read-only folder unless `--allow-write`; one memory budget, `--max-memory`.
- Every gate runs locally in `release.ts` (review gate first); CI only publishes.

## Review
- Run for: 1.0.0 — complete, released
- Position: done (Review complete: 63d71b6)
- Open issues: 0
- Platforms passed: win-x64, linux-x64 (Ubuntu 26.04.1 under WSL 2), 2026-11-20; others pending

## Next
1. A README Breaking Changes table for 1.0.0 (open-work 15) — a README change, so a review first.
2. The wasmrt switch, measured first (open-work 3).

## Waiting on owner
- open-work 11: retire the old crates.io token.
- The test machines for the platforms marked `pending` in `platforms.md`.

## Read next
- release.md § 1.0.0: what this release changed and how it ran
- design-decisions.md § Review 0.6.0: the owner's decisions from the review
