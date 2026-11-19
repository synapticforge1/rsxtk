# review-process.md — rsxtk's slots in the review standard

The standard is `../workspace/cmem/review-process.md` (🔒 owner, 2026-11-17, letter H37); this file
fills only what differs for rsxtk (its § 12). Last verified 2026-11-19. Briefs are the standard's
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

- None under this standard yet. rsxtk had no open review issues in a tracked ledger on 2026-11-19
  (H37 (c): nothing to move).

## Slots

- **Private-data paths:** `cmem/local/`, `cmem/review-issues.md`, `.claude/`, the root `CLAUDE.md`
  — all gitignored.
- **Reference results** (basic review: "reference results hold"): `cargo test` 41 pass;
  `bun test scripts/` (count in `testing.md`); `surface.txt` fresh; clippy 0 warnings. Any change
  in a count is a finding until explained.
- **Support matrix and platform notes:** [platforms.md](platforms.md).
- **Licence inventory and notices:** `licences.txt` (committed; every package in `Cargo.lock`
  with its SPDX licence from the package's own `Cargo.toml`), checked by
  `bun scripts/check-licences.ts` against the allowlist in that script (MIT, Apache-2.0,
  Apache-2.0 WITH LLVM-exception, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, Unlicense — exactly
  those in use, 2026-11-19; 284 packages after that day's security updates). rsxtk's own licence is `MIT OR Apache-2.0`
  (`Cargo.toml`, `LICENSE-MIT`, `LICENSE-APACHE`, both shipped in the crate; the gate checks all
  three). rsxtk ships SOURCE only: no dependency is bundled, so no third-party notice file ships.
- **Outputs that can carry user facts:**
  - `.tk/<stem>/` in the current folder: the mirrored script, `build.rs` and `rsc/`, Cargo's build
    output, and the cached optimized `.wasm` (no machine id since 2026-11-19);
  - error messages, which print the user's paths;
  - network: `add` without a version asks crates.io for the latest (requested by the user; a fixed
    `User-Agent`); nothing else talks to the network. No telemetry.
  - a guest sees only the script's parent folder (the one preopen).
- **Gate scripts** (run locally by `scripts/release.ts`, after `scripts/review-gate.ts`; CI only
  publishes): `cargo build --release --locked`; `cargo clippy --all-targets --locked -- -D
  warnings`; `cargo test --locked`; `bun test scripts/`; `bun scripts/check-privacy.ts` (privacy
  and credential scan); `bun scripts/check-licences.ts` (licence inventory); `bun
  scripts/check-cmem.ts`; `cargo audit` (known vulnerabilities, RustSec; `cargo-audit` installed
  2026-11-19 — fails today on wasmtime 40, open-work 17); `bun scripts/compat.ts --release` (API
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
