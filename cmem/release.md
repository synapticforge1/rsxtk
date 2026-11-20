# release.md — how a release runs here, and every release (newest first)

🔒 The ONE place rsxtk's releases are recorded (owner, 2026-11-10; workspace letter H33 D3). Other
files point here. The workspace's uniform shape is `../workspace/cmem/release-process.md` § 5 (slots
U1–U12); this file says how rsxtk fills each slot. Reference repo for crates.io:
universalWasmLoader-rs (`../../universalWasmLoader/universalWasmLoader-rs/cmem/release.md`), whose
1.0.5 ran this order on GitHub first (2026-11-11).

## How a release runs (since 2026-11-12, the owner's order)

The owner says **"release"** (H34, `working-rules.md` § Owner commands): memory → commit → merge into
`main` → push → then this process. **The level (patch / minor / major) is the owner's; ask if it
was not given.** Run from the repo root on `main`, with `CARGO_TARGET_DIR` on NTFS
(`local/environment.md`) and the pinned nightly as the default toolchain.

1. **U1–U3 `bun scripts/bump.ts <level>`** — the level is REQUIRED (no default). Refuses a dirty
   tree (untracked files count). Gate (`scripts/compat.ts`): `surface.txt` is fresh (equals this
   build's `--help` pages — a stale snapshot is refused); the **CLI floor** against tag
   `rsxtk_v<current>` (removed or reshaped command / argument / option / value → major, a new
   required argument on an existing command → major, anything added → minor, same → patch) — a
   lower level is refused; for a patch or minor, the tag's `tests/` against this `src/` (red
   refuses). Then it writes `Cargo.toml` + `Cargo.lock` and makes the commit **`bump to vX.Y.Z`**
   itself (exactly those two files, read back). `--dry-run` writes and commits nothing.
   - rsxtk has no library API, so `cargo semver-checks` (§ 6's crates.io tool) has nothing to
     read: **the floor tool is the CLI surface** (`surface.txt`, the plan in
     `design-decisions.md` since 2026-09-20). It sees what `--help` shows, not exit codes or
     output formats; the previous tag's tests cover behaviour.
2. **U4 `bun scripts/release.ts`** — FIRST the review gate (`scripts/review-gate.ts`, H37 (d)):
   `review-process.md` § Last review records `Review complete: <commit>`, an ancestor of the
   candidate, with nothing outside `cmem/` changed since — so "review" runs before "release".
   Then the guards: clean tree; on `main`; HEAD = `origin/main` (just fetched) + exactly that bump
   commit; the tag `rsxtk_vX.Y.Z` absent here and on `origin`; crates.io's version LIST without
   X.Y.Z (never the version's own URL before it exists). Cold gate — the ONLY run of the gates
   (CI only publishes, H37 (e)): `cargo build --release --locked`, `cargo clippy --all-targets
   --locked -- -D warnings`, `cargo test --locked`, `bun test scripts/`, `check-privacy.ts`,
   `check-licences.ts`, `check-cmem.ts`, `bun scripts/compat.ts --release` (fresh golden, CLI
   floor over the previous tag, its tests, `cargo package --locked`). Then ONE `git push origin
   main` (never forced), read back with `ls-remote`. **It makes no tag, ever.** `--dry-run` stops
   before the push. The gate runs on win-x64 today; each platform run is a `Tested on:` line.
3. **CI, `.github/workflows/release.yml`, on that push of `main`** — 🔒 "CI to check first, then
   push the tag, and then create the release page from the tag, then publish the package to the
   registry", in ONE run (a `GITHUB_TOKEN` tag starts no run, so the jobs chain by `needs:`):
   - **U5 `checks`** {contents: read; credential-less clone at `github.sha`}: the version line;
     `state` — the version's tag absent → **new**, on this commit → **rerun**, elsewhere →
     **none** (an ordinary push: nothing else runs); for a release, `cargo package --locked` on
     the pinned nightly — the verify build runs here, with no secret, so `publish` builds nothing
     (owner's option 1, 2026-11-19). **No tests run on GitHub** (owner, 2026-11-17, H37 (e)).
   - **U6 `tag`** {contents: write; no checkout}: `github-release.ts tag` — the annotated tag
     `rsxtk_vX.Y.Z` on `github.sha` via `git/tags` + `git/refs`, read back; already here = done,
     elsewhere = refused (a published tag never moves).
   - **U7 `release`** {contents: write; no checkout}: `github-release.ts page` — the Release page
     "rsxtk X.Y.Z" FROM the tag, H32's order, BEFORE the publish; its notes open with the README's
     `Tested on: <OS> (<platform ID>)` lines and "Other platforms: …" (read from README.md at the
     released commit; none → refused, H37), then the crates.io link. `release.ts` refuses earlier,
     before the push, when the README has no such line.
   - **U8 `publish`** {id-token: write, contents: read}: the tag still peels here; skip when
     crates.io already lists the version; **crates.io trusted publishing** (owner's D2) — GitHub
     OIDC (audience `crates.io`) → `POST /api/v1/trusted_publishing/tokens` → a short-lived token;
     `cargo publish --locked --no-verify`; the token revoked (`DELETE`); then the read-back,
     FATAL: crates.io serves X.Y.Z, not yanked, with the checksum of this commit's package
     (`CRATE_FILE`; a re-run that skipped the publish repackages with `--no-verify` — reproducible,
     measured on 1.0.0; added before 1.0.1) (crates.io keeps no provenance attestation; it does
     record `trustpub_data` — loader-rs measured it on 1.0.5).
   - **U9:** no `uses:`; Rust `nightly-2026-11-04` via the runner's rustup (signed manifests) —
     the local gate's nightly (the package CI verifies is built by the compiler the tests passed
     on), move both together; Bun 1.4.4 by version + sha256; top-level
     `permissions: {}`; `concurrency` per commit, never cancelled (a per-tag group cannot exist
     before the tag).
4. **U10, as a user (by hand, after the run):** `cargo install rsxtk --version X.Y.Z --locked
   --root <scratch>` from crates.io, then `<scratch>/bin/rsxtk --version` and a `run` of a `.wat`
   hello in a scratch folder.
5. **U11:** an entry below, newest first, with what changed, the run id and the read-backs.
6. **U12, on a failure:** "Re-run failed jobs" in the web UI (owner): each job skips what is done
   (`tag` = done, page = done, crates.io lists it = no upload). Never a new tag for the same code,
   never a moved tag. A CHANGED release job is rehearsed before it lands (best-practices 7.22).

**🗓️ Owner items for trusted publishing (D2), before the next release:** on crates.io, `rsxtk` →
Settings → Trusted Publishing → add GitHub: repository `synapticforge1/rsxtk`, workflow
`release.yml` (no environment) — **registered by the owner, 2026-11-12; MEASURED by 0.5.2**
(crates.io's `trustpub_data` names this repository and run). Without it the `publish` job stops at "Trusted-publishing token"
with crates.io's "No matching Trusted Publishing config" (after the tag and the page exist — U12's
re-run finishes it). After the first release publishes through it, delete the GitHub secret
`CARGO_REGISTRY_TOKEN` and revoke that crates.io API token: nothing references it any more.

### How the release path got here (dated)

- **2026-01-06 → 2026-10-23** — published by hand (`cargo publish`), tags by hand.
- **2026-11-09** — H32: a tag-triggered workflow made the Release page (REST API, H30); then on
  the owner's word CI also published with the `CARGO_REGISTRY_TOKEN` secret — 0.5.1 below.
- **2026-11-12** — H33 / D1 / D2 / D3: this file's process. `bump.ts`, `release.ts`, `compat.ts`
  and `crates-io.ts` added; the workflow moved from a tag trigger to a push of `main` with CI
  tagging. **Proof before landing (best-practices 7.22):** `bun test scripts/` 45 cases (mock
  GitHub REST, GitHub's OIDC endpoint, crates.io; the CLI floor on hand-written surfaces); four
  one-line mutants each caught by its own case (floor ignoring removals, no tag read-back, rerun
  misreported, yanked accepted). Read-only against the REAL GitHub and crates.io: `state` rerun
  (0.5.1 @ `5732d04`) / none (0.5.1 @ `848829d`) / new (0.5.2); `tag` done / refused (would move)
  / would create; `page` exists / refused (no tag); `has` 0.5.1 true, 9.9.9 false; `readback`
  0.5.1 served. **Rehearsal** in a scratch clone with a scratch bare `origin`: `bump` refused no
  level, two levels and an untracked file, then committed `bump to v0.5.2` (floor patch over
  `rsxtk_v0.5.1`, whose `--help` pages were built from the tag; its tests 41/41); `release.ts`
  refused another branch, an untracked file, a commit on top, a local tag and a tag on origin,
  each with `origin/main` untouched; the real run passed the cold gate, pushed `main` = the bump,
  read back, NO tag; a second run refused. **The gate inverted** with the real binary: `convert`'s
  `--output` renamed `--out` → "surface.txt is stale"; golden regenerated → "needs at least major
  … removed: convert option -o, --output <OUTPUT>". ⚠️ **The inversion FOUND a defect, fixed
  before landing:** the throwaway `.compat/` crate (rsxtk at the same version) shared the repo's
  target directory, and afterwards the edited clone's build read as Fresh and `--help` still
  showed the old flag — the gate would have checked a stale binary. Each throwaway crate now
  builds in its own target subdirectory (`compat.ts` `targetDir`); the repo-wide re-run, from a
  fresh clone and a fresh target directory, was green end to end. **Measured on GitHub by 0.5.2**
  (run 43271903460, below).
- **2026-11-19** — H37 (d) / (e), the final review standard: `release.ts` runs the review gate
  first and every gate locally (privacy, licences, memory added); `release.yml`'s `checks` lost
  its build, clippy, tests, surface and compat steps and keeps the version, `state` and, for a
  release, a credential-less `cargo package --locked` (owner's option 1). Landed in `ca614b9`.
  **Rehearsed** in a scratch clone with a scratch bare `origin` (`release.ts --dry-run`), origin
  untouched each time: no record → refused at the review gate; a record, then `src/main.rs`
  changed → refused, naming the file; a valid record with `bump to v0.6.0` on top → "HEAD~1 is
  covered by the review of …", then refused at the NEXT guard (an untracked file) — so the review
  gate runs first and passes; two records → refused. `release.yml` parsed (Bun's YAML): the
  four jobs and their permissions unchanged; no build, clippy, test, compat or `uses:` step.
  **Unmeasured on GitHub**: the next push runs `checks` with state none (version and `state`
  only); the package step first runs at the next release — the same `cargo package --locked`
  that `compat.ts --release` runs locally and 0.5.2's CI ran green.

## Release History

Every version on crates.io, newest first (39 versions, 15 not yanked — crates.io API,
2026-11-20, after 1.0.1). Before 0.5.0 the repo kept no memory and its commit messages say
"update"; those entries hold what is known and say so.

### 1.0.1 — 2026-11-20 (patch) — full review 2 and the open items

**Tested on: Windows 11 (win-x64)** · **Tested on: Ubuntu 26.04 LTS under WSL 2 (linux-x64)** ·
Other platforms: not yet tested. Platforms verified at cb2a6e7: win-x64 (`cargo test` 89, bun 79,
clippy 0, every gate; again by `release.ts` at 32bcedb with `cargo audit` 0 vulnerabilities, 1
allowed warning: pin-utils unmaintained); linux-x64 (Ubuntu 26.04.1, WSL 2: `cargo test` 96 incl. the
linux-only tests, bun 79, clippy 0, every gate). Symlink tests run on Windows too (Developer Mode).

**What changed (for users):** a library whose comment mentions `fn main` is a library; script names
are what Cargo builds (only `nul` refused among Windows device names — con, prn, … ran under 1.0.0);
`add` accepts exactly Cargo's version requirements; parallel runs of one script wait for its build;
`fmt` stops rustfmt after 2 minutes; a guest's exit is handled alike in `run` and `mod`; `.wat` with
a BOM runs; outputs never follow a symlink (temp files exclusive); a symlinked script or module is
rewritten through its link; `.tk` checks (links inside refused; another user's or a world-writable
`.tk` refused; each script's cache 0700, an open one rebuilt, fail closed); the Linux lock folder is
per-user with no HOME; README Install (`--locked`), the Breaking Changes table for 1.0.0, the
shared-folder limit. Release path: the crates.io read-back compares the checksum. The CLI floor over
1.0.0 is patch and 1.0.0's 75 tests pass unchanged.
**How (owner: "do a full review one more time before releasing", "scope the open items and fix them
before the release"; level patch):** review complete at cb2a6e7 (`review-process.md`); record
`759f316`. The first `release.ts` refused (main 16 ahead of origin/main, nothing pushed), so
`759f316` was pushed first (run 44812729588, non-release, green), then `bump.ts patch` → `32bcedb`
(the first bump attempt failed only because CARGO_TARGET_DIR was unset: `target/` on exFAT) and
`release.ts` pushed `main` = `32bcedb`, no tag. **Run 44813275188**, every job green, first attempt:
tag `rsxtk_v1.0.1` (object `1fb44ac`, `github-actions[bot]`, 20:40:45Z, → `32bcedb`); page "rsxtk
1.0.1", not a draft, Tested on lines first; publish with the checksum read-back.
**Read back:** crates.io 1.0.1 created 20:41:15Z, not yanked, `MIT OR Apache-2.0`, 824,810 bytes,
checksum `9a835389…6d96` = the local `cargo package` output; **`trustpub_data` = {github,
synapticforge1/rsxtk, run 44813275188, sha `32bcedb`}**.
**As a user (U10):** `cargo install rsxtk --version =1.0.1 --locked` into a scratch root (391 s) →
`rsxtk 1.0.1`; `run hello.wat` printed "hello from 1.0.1", exit 0; `add s.rs itoa 1-rc` refused, exit 1.

### 1.0.0 — 2026-11-20 (major) — the first release under the final review standard

**Tested on: Windows 11 (win-x64)** · **Tested on: Ubuntu 26.04 LTS under WSL 2 (linux-x64)** ·
Other platforms: not yet tested. Platforms verified: win-x64, full suite + every gate at 4f2afa6 and
again by `release.ts` at 3abc9f4 (`cargo test` 75, `bun test scripts/` 76, clippy 0, `cargo audit`
0); linux-x64 (Ubuntu 26.04.1 LTS, WSL 2), full suite at 78812de (`cargo test` 76 incl. the
linux-only test, bun 75, clippy 0, every gate but `cargo audit`, which reads only the lockfile).

**What changed (for users):** a module's folder is READ-ONLY to the guest unless `--allow-write`;
`--max-memory <MiB>` (default 4096) is one budget for all guest memories and tables; `.cwasm` files
are never run, and the `.rs` cache is a plain `.wasm` keyed by a fingerprint of the sources (not
timestamps); wasmtime / wasmtime-wasi 40 → 50.0.3 (32 advisories gone; no C library added);
`add` / `remove` edit only `[dependencies]` (every quoting and table form), refuse what they cannot
edit safely, validate names and versions, and lock the script; `fmt` is honest about rustfmt
(edition 2021); `init` never overwrites; atomic writes; `mod` refuses extra arguments; `bench -i 0`
refused; README Privacy section. Release path: CI only publishes (H37 (e)); every gate local in
`release.ts`, the review gate first; the Release page opens with the README's Tested on lines.
**Why a major:** `bump.ts minor` was refused — one 0.5.2 test checks the removed `.cwasm` cache (a
deliberate break) — and the owner chose 1.0.0 over teaching the gate 0.x (2026-11-20). 1.0.0 starts
the API promise; a README Breaking Changes table is still to come (open-work 15).
**Review:** the five required reviews and five re-review rounds (`review-process.md` § Last review);
at "release", the H37 Tested on lines were added and the first linux-x64 run found and fixed two
unix defects, each re-reviewed; `release.ts` then found `licences.txt` listing rsxtk itself (stale
after every bump) — fixed in review tooling (63d71b6) and the bump redone.
**How (owner: "release", level asked → minor, then 1.0.0):** pushed `3ea4d99` (run 44777844354: the
new workflow, state none — version and state only), then `9be571b`; `bump.ts major` → `3abc9f4`;
`release.ts` passed the review gate, the guards and the local gate and pushed `main` = `3abc9f4`, no
tag. **Run 44779881233**, every job and step green, first attempt (UTC): `checks` 17:05:34–17:07:32
(version, state new, `cargo package --locked`); `tag` 17:07:35–41 (annotated `rsxtk_v1.0.0`, object
`50d5651`, tagger `github-actions[bot]`, → `3abc9f4`); `release` 17:07:44–49 (page "rsxtk 1.0.0" by
`github-actions[bot]`, not a draft, `releases/latest`, opening with both Tested on lines);
`publish` 17:07:52–08:11 (OIDC token, `cargo publish --no-verify`, revoked, read back).
**Read back:** crates.io 1.0.0 created 17:08:06Z, not yanked, `MIT OR Apache-2.0`, 810,459 bytes;
**`trustpub_data` = {github, synapticforge1/rsxtk, run 44779881233, sha `3abc9f4`}**.
**As a user (U10):** `cargo install rsxtk --version =1.0.0 --locked` into a scratch root (351 s) →
`rsxtk 1.0.0`; `run hello.wat` printed "hello from 1.0.0", exit 0; `run x.cwasm` refused, exit 1.

### 0.5.2 — 2026-11-12 (patch) — the first release in the owner's order

**What changed:** nothing in `src/` or `tests/`; the CLI is 0.5.1's (floor patch). The release
process (workspace H33 / H34, the owner's D1–D3): `bump.ts` requires the level and commits;
`release.ts` pushes `main` only; CI runs checks → tag → page → publish; crates.io **trusted
publishing** replaces the stored token; the CLI-surface golden `surface.txt` is the bump floor;
this file is the one release record. The crate's manifest also excludes `surface.txt`.
**How (owner: "release", level patch, trusted publisher registered — asked and answered):** the
memory commit `7c2f970` was pushed first; its run **43267892511** was the non-release path —
`checks` green (state none; the golden fresh on Linux too, so `rsxtk.exe` → `rsxtk`
normalization holds), the release-mode gate, `tag`, `release`, `publish` skipped. Then
`bump.ts patch` made `bump to v0.5.2` `0a19a93` (floor patch over `rsxtk_v0.5.1`, its tests
41/41). `release.ts` refused ONCE at "cargo package --locked failed" with nothing pushed (the
message was cut by my output filter — cause UNMEASURED; `cargo package --locked` alone then
passed, and the re-run passed every guard and the gate) and then pushed `main` = `0a19a93`, no
tag. **Run 43271903460**, every job and step green, first attempt, in the owner's order (UTC,
2026-11-12):
- `checks` 17:40:27–17:49:05, including `compat.ts --release`.
- `tag` 17:49:08–14: the annotated `rsxtk_v0.5.2` (object `f2273fb`, tagger
  `github-actions[bot]`, 17:49:11Z), peeling to `0a19a93`.
- `release` 17:49:17–24: the page "rsxtk 0.5.2" by `github-actions[bot]`, not a draft, crates.io
  link on top, `releases/latest`.
- `publish` 17:49:27–44: OIDC → token, `cargo publish --no-verify`, revoke, read-back.
**Read back:** crates.io 0.5.2 created 17:49:40Z, not yanked; **`trustpub_data` = {provider
github, repository synapticforge1/rsxtk, run_id 43271903460, sha `0a19a93`}** — the owner's
trusted publisher is MEASURED. `git ls-remote`: `rsxtk_v0.5.2` → `f2273fb`, `^{}` → `0a19a93`.
**As a user (U10):** `cargo install rsxtk --version =0.5.2 --locked` into a scratch root, own
target dir → `rsxtk --version` = `rsxtk 0.5.2`; `rsxtk run hello.wat` (a WASI `fd_write`) printed
"hello from 0.5.2", exit 0.

### 0.5.1 — 2026-11-09 (patch) — the first release by CI

**What changed:** nothing in `src/` or `tests/` apart from the CRLF → LF renormalization (`git
diff --ignore-cr-at-eol rsxtk_v0.5.0 -- src tests` empty); `Cargo.toml` excludes `.github/` and
`scripts/` from the crate. CI: the gate, the crates.io publish (token secret) and the Release page.
**How (owner: "commit, publish then release"):** tag `rsxtk_v0.5.1` → `5732d04` (lightweight,
pushed by hand — the order before D1), run **42720045933**: `checks`, `publish`, `release` all
success, every step success (public Actions API). **Read back:** crates.io 0.5.1 created
2026-11-09T23:30:37Z, not yanked; Release `rsxtk_v0.5.1`, "rsxtk 0.5.1", by
`github-actions[bot]`, not a draft, crates.io link on top; the tag unmoved after the run.

### 0.5.0 — 2026-10-23 (minor)

**What changed:** a script's `build.rs` and `rsc/` are mirrored into the build project (a
`build.rs` that 0.4.4 ignored now RUNS — why it was not a patch); library scripts build as
`cdylib`; `info` on a library from another folder fixed; `add` / `remove` match the exact crate
(was a prefix match: `serde` hit `serde_json`); the end-to-end suite `tests/cli.rs`; clippy clean;
`--version` from Cargo; `homepage` dropped; `cmem/` excluded from the crate. `--help` lists the
same 16 commands as 0.4.4 (compared binary to binary).
**How:** by hand — `cargo login` + `cargo publish` from `be056bd` (779.1 KB; SHA-256 equal to the
scanned `--dry-run` package), tag `rsxtk_v0.5.0` → `be056bd` (lightweight), pushed. No Release
page (no backfill — owner, 2026-11-09).

### 0.4.4 — 2026-04-20 · 0.4.3 — 2026-04-17

**What changed:** not recorded. The commits of 2026-04-16 → 04-20 are a "project rebuild" with
`target/` removed from the tree and a `.gitignore` added, then version bumps.
**How:** by hand; no tags.

### 0.4.0 — 2026-01-22 · 0.4.1, 0.4.2 — 2026-01-23

**What changed:** not recorded; the `0.4.0` branch was merged as pull request #1 on 2026-01-22.
Tag `rsxtk_v0.4.0` → `8263a86` (a 2026-01-13 commit; by hand), the only tag with a manual
Release page (by `jrmarcum`).

### 0.3.6 – 0.3.9 — 2026-01-16 → 01-19 · 0.3.3 — 2026-01-13

**What changed:** not recorded. Not yanked.

### Yanked: 0.1.0 – 0.2.9 (2026-01-06 → 01-07), 0.3.0 – 0.3.2, 0.3.4, 0.3.5 (→ 2026-01-15)

24 versions, every one yanked (there is no 0.2.3); the first publishes of the crate. What changed
in each is not recorded.
