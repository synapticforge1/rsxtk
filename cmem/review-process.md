# review-process.md — rsxtk's slots in the review standard

The standard is `../workspace/cmem/review-process.md` (🔒 owner, 2026-11-17, letter H37); this file
fills only what differs for rsxtk (its § 12). Last verified 2026-11-20. Briefs are the standard's
item lists word for word plus the scope; they are not copied here.

## The loop, as rsxtk runs it

- "review" runs basic → hardening → licensing → security → privacy (efficiency first, only when
  asked), by the standard's § 9. Scope of a first review: everything since the last release tag
  (`git diff rsxtk_v<last>..HEAD`, plus the code those changes reach).
- Fixes go on a local branch `review/<version>`, merged to `main` when the review completes.
- Issues: `cmem/review-issues.md` — **gitignored**; the privacy gate refuses it if tracked.
  Each row: an ID (`<class letter><n>`: B, H, L, S, P, E), the platform ID where it is one, the
  finding, the decision (accepted / rejected with its reason), the fix commit.
- When a re-review comes back empty and the full suite has passed, § Last review records ONE
  bullet `- Review complete: <commit>` (the reviewed commit; replace the previous one) in a
  memory-only commit. `scripts/review-gate.ts`, the first step of `release.ts`, refuses a release
  unless that commit is an ancestor and nothing outside `cmem/` changed since.
- Reviewer briefs say: work in a folder under the main session's scratchpad, which the main
  session removes at triage (Tooling rule 15); never commit, push or release.

## Last review

- Review complete: 940e335
- For 0.6.0, 2026-11-20 (owner: "review"); scope `rsxtk_v0.5.2..264b408`, branch `review/0.6.0`.
  First review 22 findings (basic 4, hardening 14 — one of them, H7, rejected by the owner's
  decision — security 2, privacy 2, licensing 0) + 1 owner question; the five re-review rounds found
  12 more (basic 5, hardening 7) and were fixed; the last re-review of every class was empty. Counts only — the issues stay in
  the gitignored `review-issues.md`. Owner decisions in `design-decisions.md` § Review 0.6.0.
- Full suite once on win-x64 at 4f2afa6 (Windows 11, nightly-2026-11-04): release build, clippy 0,
  `cargo test` 75/75, `bun test scripts/` 73/73, check-privacy, check-licences (269), `cargo audit`
  0 vulnerabilities, check-cmem. `compat.ts --release` runs at the bump (it refuses an already
  released version by design). Other platforms: pending (`platforms.md`).
- At "release" (2026-11-20) the H37 `Tested on` lines were found missing and added (9f13e9b: README,
  the Release page from the README at the released commit, a `release.ts` guard); re-reviewed by
  basic, hardening + security and privacy — all empty at the floor. Below the floor, noted:
  `release.ts`'s README guard has no test of its own (its twin in `github-release.ts` has).
- Then the owner asked "did we run the checks on wsl?": the first linux-x64 run (WSL 2, Ubuntu
  26.04.1 LTS) found clippy failing on unix and the lock folder staying 0755 — fixed in 78812de
  (linux-only test, failing on 9f13e9b); the full suite then passed on linux-x64 (cargo test 76/76,
  clippy 0, bun 75, every gate but `cargo audit`, which reads only the lockfile). README 940e335 adds
  the linux-x64 Tested on line. Re-review 7 (two agents for a two-line change: basic + hardening,
  security + privacy — a recorded deviation) was empty → **Review complete: 940e335**.
- Platforms passed for 0.6.0: win-x64 (Windows 11), linux-x64 (Ubuntu 26.04.1 LTS under WSL 2).

## Slots

- **Private-data paths:** `cmem/local/`, `cmem/review-issues.md`, `.claude/`, the root `CLAUDE.md`
  — all gitignored.
- **Reference results** (basic review: "reference results hold"): `cargo test` 75 pass;
  `bun test scripts/` 73; `surface.txt` fresh (94 entries); clippy 0 warnings (2026-11-20). Any change
  in a count is a finding until explained.
- **Support matrix and platform notes:** [platforms.md](platforms.md).
- **Licence inventory and notices:** `licences.txt` (committed; every package in `Cargo.lock`
  with its SPDX licence from the package's own `Cargo.toml`), checked by
  `bun scripts/check-licences.ts` against the allowlist in that script (MIT, Apache-2.0,
  Apache-2.0 WITH LLVM-exception, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, Unlicense — exactly
  those in use, 2026-11-19; 269 packages since wasmtime 50, 2026-11-20). rsxtk's own licence is `MIT OR Apache-2.0`
  (`Cargo.toml`, `LICENSE-MIT`, `LICENSE-APACHE`, both shipped in the crate; the gate checks all
  three). rsxtk ships SOURCE only: no dependency is bundled, so no third-party notice file ships.
- **Outputs that can carry user facts:**
  - `.tk/<stem>/` in the current folder: the mirrored script, `build.rs` and `rsc/`, Cargo's build
    output, the cached optimized `.wasm` and `source.id` (a hash of the script's path and sources);
  - lock files, one per script edited by add / remove / fmt, kept: `<temp>/rsxtk-locks/` (windows,
    mac), `~/.cache/rsxtk/locks/` (linux, 0700) — empty, named by a hash of the script's path;
  - error messages, which print the user's paths;
  - network: `add` without a version asks crates.io for the latest (requested by the user; a fixed
    `User-Agent`); nothing else talks to the network. No telemetry.
  - a guest sees only the script's parent folder (the one preopen), read-only unless `--allow-write`.
- **Gate scripts** (run locally by `scripts/release.ts`, after `scripts/review-gate.ts`; CI only
  publishes): `cargo build --release --locked`; `cargo clippy --all-targets --locked -- -D
  warnings`; `cargo test --locked`; `bun test scripts/`; `bun scripts/check-privacy.ts` (privacy
  and credential scan); `bun scripts/check-licences.ts` (licence inventory); `bun
  scripts/check-cmem.ts`; `cargo audit` (known vulnerabilities, RustSec; `cargo-audit` installed
  2026-11-19; 0 vulnerabilities since wasmtime 50, 2026-11-20); `bun scripts/compat.ts --release` (API
  surface + previous tag's tests + `cargo package`).

## Items that do not apply, and why

- **Cross-implementation agreement** (basic): rsxtk has no second implementation of an interface.
- **Unintended API breaks** (basic): rsxtk has no library API; its surface is the CLI, checked by
  `surface.txt` and the previous tag's tests.
- **Native library loading** (security): a loaders' item; rsxtk loads no native library.

## Where the security review looks first

- **Exploitable memory corruption** does not apply to rsxtk's own code: `src/main.rs` has no
  `unsafe` since 2026-11-19. Until then two blocks deserialized `.cwasm` files (native code run
  without validation); the owner's rule now is that rsxtk never runs a `.cwasm` — check that no
  `deserialize*` / `precompile_*` call has come back (`design-decisions.md` § Execution model).
- A script's `build.rs` runs on the host, unsandboxed, like any Cargo build script (documented in
  the README; `design-decisions.md` § Execution model).
- **Machine paths in binaries** (privacy): no binary is published; the crate is source.
