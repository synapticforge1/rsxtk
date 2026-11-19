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
- When a re-review comes back empty and the full suite has passed, this file records
  `Review complete: <commit>` under § Last review; the release tooling reads it (open-work 12).
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
- **Licence inventory and notices:** the project's own licence is `MIT OR Apache-2.0`
  (`Cargo.toml` `license`, `LICENSE-MIT`, `LICENSE-APACHE`, both shipped in the crate). No
  inventory of the dependencies exists yet (open-work 13); rsxtk ships SOURCE only, so dependency
  licences reach users through their own `cargo install`, and no notice file is bundled.
- **Outputs that can carry user facts:**
  - `.tk/<stem>/` beside a script: the mirrored script, `build.rs` and `rsc/`, Cargo's build output,
    the `.cwasm`, and an id file holding a hash of OS + arch + hostname (`get_machine_id`, not the
    hostname itself);
  - error messages, which print the user's paths;
  - network: `add` without a version asks crates.io for the latest (requested by the user; a fixed
    `User-Agent`); nothing else talks to the network. No telemetry.
  - a guest sees only the script's parent folder (the one preopen).
- **Gate scripts** (run locally; CI only publishes once open-work 12 lands):
  `cargo build --release --locked`; `cargo clippy --all-targets --locked -- -D warnings`;
  `cargo test --locked`; `bun test scripts/`; `bun scripts/compat.ts surface --check` and
  `--release` (API surface + previous tag's tests + `cargo package`); `bun scripts/check-privacy.ts`
  (privacy and credential scan); `bun scripts/check-cmem.ts`. Missing: a vulnerability check
  (`cargo audit` is not installed) and the licence inventory (open-work 13).

## Items that do not apply, and why

- **Cross-implementation agreement** (basic): rsxtk has no second implementation of an interface.
- **Unintended API breaks** (basic): rsxtk has no library API; its surface is the CLI, checked by
  `surface.txt` and the previous tag's tests.
- **Native library loading** (security): a loaders' item; rsxtk loads no native library.

## Where the security review looks first

- `src/main.rs` has two `unsafe` blocks (measured 2026-11-19): `Module::deserialize_file` for the
  cached `.cwasm` of a `.rs` script and for a `.cwasm` given directly to `run`. A `.cwasm` is
  precompiled native code that wasmtime trusts without validation, so `run <file>.cwasm` on a file
  from elsewhere runs native code outside the sandbox. In scope for "untrusted input runs nothing
  it was not asked to" and "exploitable memory corruption".
- A script's `build.rs` runs on the host, unsandboxed, like any Cargo build script (documented in
  the README; `design-decisions.md` § Execution model).
- **Machine paths in binaries** (privacy): no binary is published; the crate is source.
