# STATE: rsxtk

Updated: 2026-11-20, at the end of the session

## Version
- Released: 0.5.2 on 2026-11-12 (crates.io, by CI in the owner's order)
- Working branch: main (review/0.6.0 merged locally; nothing pushed)

## Now
- The review for 0.6.0 is complete at 4f2afa6 (five classes; last re-reviews all empty).
- wasmtime / wasmtime-wasi 50.0.3; `cargo audit` 0 vulnerabilities; no C library in the build.
- Guest sandbox: read-only folder unless `--allow-write`; one memory budget, `--max-memory`.
- Every gate runs locally in `release.ts` (review gate first); CI only publishes.

## Review
- Run for: 0.6.0 — complete
- Position: done (Review complete: 4f2afa6)
- Open issues: 0
- Platforms passed: win-x64 (full suite 2026-11-20); others pending

## Next
1. "release" with level minor → 0.6.0 (open-work 15); `release.ts` will accept the review.
2. The wasmrt switch, measured first (open-work 3).

## Waiting on owner
- open-work 11: retire the old crates.io token.
- The test machines for the platforms marked `pending` in `platforms.md`.

## Read next
- release.md: before "release" (0.6.0 is a minor; the Tested on line is win-x64 only)
- design-decisions.md § Review 0.6.0: the owner's decisions from the review
