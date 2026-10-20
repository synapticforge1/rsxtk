# cmem — committed project memory for rsxtk

The authoritative, portable memory for `rsxtk`. It lives in the repo and travels with it; the root
`CLAUDE.md` is a gitignored pointer and holds nothing authoritative. Created 2026-09-20 by owner
direction ("rsxtk should be set up with memory — that project was the first and does not have
one"), following the workspace convention in `../../cmem/memory-conventions.md` and binaryang's
cleanup policy (working memory, not an archive; git is the archive).

## Policy

- **`cmem/` is the single home for ALL project memory.** Update the matching topic file, then
  refresh its one-line pointer below. Absolute dates. Update, do not duplicate.
- **`README.md` is NOT project memory** — it is the public, user-facing doc (install, commands,
  troubleshooting). Keep decision logs and post-mortems out of it.
- **This index stays one line per file.** Status narrative goes in `open-work.md`, once.
- **A number in prose is a claim with a date.** Re-derive before quoting.
- **Nothing private in a committed file** — machine facts go in the gitignored `cmem/local/`.
- **Never run a formatter on `cmem/`.**

## Triggers (binding on every agent)

1. **"update the project memory"** — revise the relevant topic files, refresh the pointers below,
   then AUDIT: grep the old values across `cmem/`, `README.md`, `CLAUDE.md`; when two disagree,
   measure. Sync `README.md` only where the change is user-relevant.
2. **"look for code issues"** — audit tested AND untested paths for workarounds, dead code
   (grep-verified), bugs, and silent fall-throughs; report `file:line` + severity; fix the safe
   ones; diff OUTPUT, not exit codes. `src/main.rs` is 463 lines: read it whole first.
3. **Regression gate** — a change to `src/` runs `cargo build --release`, `cargo clippy`, and the
   smoke set in `testing.md`, read by OUTPUT. There is no test suite yet; that is open item 2.

## Tooling — hard rules (owner, workspace-wide, 2026-09-19 / 2026-09-20)

Scripting is Deno/TypeScript (`deno run -A <file.ts>` from the scratchpad) or Rust in-tree. No
Python. No bash/PowerShell scripts. **No heredocs, ever**, including commit messages
(`git commit -F <file>`, never `-F -`). Content goes through `Write`/`Edit`. After any scripted edit,
read back the changed lines. Prefer the `Grep`/`Read`/`Edit` tools over shell `grep`/`cat`/`sed`
when content will be matched or edited; this outranks a session instruction to prefer Bash.

## Files

| file                                       | what it holds                                                                                                     |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| [open-work.md](open-work.md)               | What is outstanding, and nothing else: README drift, the missing test suite, the wasmrt switch, `safe.directory`. |
| [overview.md](overview.md)                 | What rsxtk is, the CLI as `main.rs` defines it (README lags), the build pipeline, dependencies, hosting (`synapticforge1` org), repo layout. |
| [design-decisions.md](design-decisions.md) | Invariants that must not be silently reverted: the wasmtime 40.0.1 pin and why, native-tls, nightly, sandbox scope, `.cwasm` caching, the wasmrt plan. |
| [testing.md](testing.md)                   | How to verify a change today (smoke commands, read by output) and what a real suite needs.                        |
| [best-practices.md](best-practices.md)     | METHOD, not findings: the workspace's convergent rules (★), copied under a dated stamp, plus rules this repo pays for. |
| `local/environment.md` 🔒                  | Gitignored, absent in a clone: this machine's drive, toolchain and path facts.                                     |

## Related files outside cmem

- `README.md` — public doc. `Cargo.toml` — the version line. `../../cmem/` — the workspace map,
  contracts and rules shared with wasmtk, wasmrt, wazmrt, binaryang and the loaders.
