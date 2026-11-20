# STATE: rsxtk

Updated: 2026-11-20, during full review 2 (before 1.0.1)

## Version
- Released: 1.0.0 on 2026-11-20 (crates.io, by CI in the owner's order; run 44779881233)
- Next: 1.0.1 (patch; owner's level), after full review 2
- Working branch: `review/1.0.1` (from main at 868a4aa; tip 97a4277), merged into main when complete

## Now
- Full review 2 (owner: "do a full review one more time before releasing"): five classes, then
  re-reviews 10–13, all fixed; then the owner's "scope the open items and fix them before the
  release" → fixed in 97a4277 (`.tk` links / owner, per-user lock fallback, checksum read-back,
  fmt limit test, Windows symlink tests, README Install with `--locked`).
- Running: the re-review of 440f260..97a4277 (security + hardening, basic) and linux-x64 at
  97a4277 (plus a hand check, as root, that another user's `.tk` is refused).
- win-x64 at 97a4277: cargo test 88/88, bun 79, clippy 0; every new test FAILS on the commit
  before its fix.
- wasmtime / wasmtime-wasi 50.0.3; `cargo audit` 0 vulnerabilities; no C library in the build.

## Review
- Run for: 1.0.1 — in progress (full review 2)
- Position: re-reviews of the open-item fixes (97a4277)
- Open issues: see the gitignored `review-issues.md` (recorded per round)
- Platforms passed (at 440f260): win-x64, linux-x64 (Ubuntu 26.04.1 under WSL 2); 97a4277 pending

## Next
1. Re-reviews until empty; the full suite on win-x64 and linux-x64 at the final commit.
2. `Review complete: <commit>` in `review-process.md`; merge `review/1.0.1` into main.
3. `bun scripts/bump.ts patch` → 1.0.1; `bun scripts/release.ts`; follow CI; read back (now with
   the checksum); install; the `release.md` entry. Clear caches and the scratchpad.
4. The wasmrt switch, measured first (open-work 3).

## Waiting on owner
- open-work 11: retire the old crates.io token.
- The test machines for the platforms marked `pending` in `platforms.md`.

## Read next
- design-decisions.md § Full review 2: the owner's decisions and the rules the fixes set
- review-issues.md (gitignored): every finding, round by round
