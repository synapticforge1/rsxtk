# STATE: rsxtk

Updated: 2026-11-20, full review 2 complete; releasing 1.0.1

## Version
- Released: 1.0.0 on 2026-11-20 (crates.io, by CI in the owner's order; run 44779881233)
- Next: 1.0.1 (patch; owner's level), after full review 2
- Working branch: `review/1.0.1` → merged into main (review complete at cb2a6e7)

## Now
- Full review 2 (owner: "do a full review one more time before releasing", then "scope the open
  items and fix them before the release") is complete at cb2a6e7: five classes, re-reviews 10–19
  until empty. It fixed: library detection, script names and versions = Cargo's, build lock,
  rustfmt limit (tested), guest exit, outputs never through a link, the `.tk` checks and private
  cache, the per-user lock fallback, the checksum read-back, README Install with `--locked`.
- win-x64 cargo 89, linux-x64 96 (WSL 2), bun 79, every gate, at cb2a6e7.
- wasmtime / wasmtime-wasi 50.0.3; `cargo audit` 0 vulnerabilities; no C library in the build.
## Review
- Run for: 1.0.1 — complete (Review complete: cb2a6e7)
- Position: done; next is the release
- Open issues: see the gitignored `review-issues.md` (recorded per round)
- Platforms passed (at cb2a6e7): win-x64, linux-x64 (Ubuntu 26.04.1 under WSL 2)

## Next
1. `bun scripts/bump.ts patch` → 1.0.1; `bun scripts/release.ts`; follow CI; read back (with the
   checksum); install; the `release.md` entry. Clear caches and the scratchpad.
2. The wasmrt switch, measured first (open-work 3).
## Waiting on owner
- open-work 11: retire the old crates.io token.
- The test machines for the platforms marked `pending` in `platforms.md`.

## Read next
- design-decisions.md § Full review 2: the owner's decisions and the rules the fixes set
- review-issues.md (gitignored): every finding, round by round
