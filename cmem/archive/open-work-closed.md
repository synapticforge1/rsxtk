# open-work-closed.md — rsxtk's closed open-work items (archive)

Moved here from `open-work.md`'s header on 2026-11-19 (workspace letter H38, migration step 4).
Numbers are stable and never reused. Read only when the owner asks about past work.

## Closed 2026-10-23 (0.5.0)

- **1** — README synced; `build.rs` / `rsc/` mirrored.
- **2** — the test suite (`testing.md`).
- **4** — clippy and the manifest lint at 0 warnings.
- **5** — history purged (`overview.md` § History).
- **6** — siblings' stale `jrmarcum/rsxtk` links fixed in their repos, verified read-only: wasmtk
  `vision.md:83`, `polyglot-producers.md:74`, `roadmap.md:1418` and wasmrt `loaders.md:28` all name
  `synapticforge1/rsxtk`; the two remaining mentions record the move.
- **7** — `add`/`remove` prefix match fixed (`design-decisions.md`).
- **8** — 0.5.0 published to crates.io (`overview.md` § Publishing).

## Closed 2026-10-26

- **9** — line endings pinned to LF by `.gitattributes` and the five CRLF files renormalized,
  `bae61ee`; release binary proved identical (`testing.md` § Comparing binaries).

## Closed 2026-11-09 (0.5.1)

- **10** — the release workflow's first real run, 0.5.1, all three jobs green
  (`design-decisions.md` § Versioning and release).

## Closed 2026-11-19

- **12** — H37 (d) + (e): `release.ts` runs `review-gate.ts` first and every gate locally;
  `release.yml` runs no tests, and keeps a credential-less `cargo package --locked` for a
  release (owner's option 1). `release.md` § How the release path got here.
- **13** — the licence inventory: `scripts/check-licences.ts` + `licences.txt`.
