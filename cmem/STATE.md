# STATE: rsxtk

Updated: 2026-11-20, at the end of the session

## Version
- Released: 1.0.1 on 2026-11-20 (crates.io, by CI in the owner's order; run 44813275188)
- Working branch: main (`review/1.0.1` merged)

## Now
- 1.0.1 is out: tag, page (Tested on win-x64 + linux-x64), crates.io with its checksum read back;
  user install works.
- Full review 2 (owner: "do a full review one more time before releasing", then "scope the open
  items and fix them before the release") is complete at cb2a6e7: five classes, re-reviews 10–19
  until empty. It fixed: library detection, script names and versions = Cargo's, build lock,
  rustfmt limit (tested), guest exit, outputs never through a link, the `.tk` checks and private
  cache, the per-user lock fallback, the checksum read-back, README Install with `--locked`.
- win-x64 cargo 89, linux-x64 96 (WSL 2), bun 79, every gate, at cb2a6e7.
- wasmtime / wasmtime-wasi 50.0.3; `cargo audit` 0 vulnerabilities; no C library in the build.

## Review
- Run for: 1.0.1 — complete, released (Review complete: cb2a6e7)
- Position: done
- Open issues: see the gitignored `review-issues.md` (recorded per round)
- Platforms passed (at cb2a6e7): win-x64, linux-x64 (Ubuntu 26.04.1 under WSL 2)

## Next
- Nothing scheduled; ask the owner. The wasmrt switch (open-work 3) waits until the owner says
  wasmrt is ready (2026-11-20: much more work on wasmrt/wazmrt first).

## Waiting on owner
- open-work 11: revoke the old API token on crates.io (the GitHub secret is deleted).
- The test machines for the platforms marked `pending` in `platforms.md`.

## Read next
- release.md § 1.0.1: what this release changed and how it ran
- design-decisions.md § Full review 2: the owner's decisions and the rules the fixes set
- review-issues.md (gitignored): every finding, round by round
