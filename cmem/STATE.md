# STATE: rsxtk

Updated: 2026-11-19, at the end of the session

## Version
- Released: 0.5.2 on 2026-11-12 (crates.io, by CI in the owner's order)
- Working branch: main (local commits not pushed)

## Now
- Workspace letters H37, H38, H41, H42, H43 done in this repo; answers written to the letters.
- rsxtk never runs a `.cwasm` (owner, 2026-11-19); the script cache is plain wasm; no `unsafe`.
- CI only publishes: every gate runs in `release.ts`, after the review gate (H37 (d), (e)).
- Gates added: privacy, licence inventory (`licences.txt`), `cargo audit`, memory size.
- `cargo audit` fails on wasmtime 40 (32 advisories, no 40.x fix): no release passes until it moves.

## Review
- Run for: none in progress; the first review under the final standard is due before the next release
- Position: not applicable
- Open issues: 0
- Platforms passed: win-x64 (full gate, cold target dir, 2026-11-19)

## Next
1. Move wasmtime off 40 (open-work 17), once the owner decides; measure the build first.
2. "review" — the five required reviews (open-work 16); `release.ts` refuses without one.
3. The next release is a minor, 0.6.0 (open-work 15).
4. The wasmrt switch, measured first (open-work 3).

## Waiting on owner
- open-work 11: retire the old crates.io token.
- open-work 17: upgrade wasmtime (≥ 43.0.1, wasi ≥ 45) or go straight to wasmrt (item 3).
- The test machines for the platforms marked `pending` in `platforms.md`.

## Read next
- review-process.md: before the first review under the final standard
- open-work.md: items 15–17; design-decisions.md § Engine for the wasmtime pin
