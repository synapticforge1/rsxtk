# STATE: rsxtk

Updated: 2026-11-20, at the end of the session

## Version
- Released: 0.5.2 on 2026-11-12 (crates.io, by CI in the owner's order)
- Working branch: main (local commits not pushed)

## Now
- wasmtime / wasmtime-wasi 50.0.3 (was 40.0.4): `cargo audit` 0 vulnerabilities; no new C.
- rsxtk never runs a `.cwasm` (owner, 2026-11-19); the script cache is plain wasm; no `unsafe`.
- CI only publishes: every gate runs in `release.ts`, after the review gate (H37 (d), (e)).
- Gates: privacy, licence inventory (`licences.txt`), `cargo audit`, memory size.

## Review
- Run for: none in progress; the first review under the final standard is due before the next release
- Position: not applicable
- Open issues: 0
- Platforms passed: win-x64 (full gate on wasmtime 50, 2026-11-20)

## Next
1. "review" — the five required reviews (open-work 16); `release.ts` refuses without one.
2. The next release is a minor, 0.6.0 (open-work 15).
3. The wasmrt switch, measured first (open-work 3).

## Waiting on owner
- open-work 11: retire the old crates.io token.
- The test machines for the platforms marked `pending` in `platforms.md`.

## Read next
- review-process.md: before the first review under the final standard
- design-decisions.md § Engine: before touching wasmtime's features or version (no C rule)
