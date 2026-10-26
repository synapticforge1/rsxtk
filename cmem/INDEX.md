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
- 🔒 **No secrets or personal information in anything public** (owner, 2026-10-22; workspace
  `../../cmem/working-rules.md` § Records): no tokens or keys, no email addresses, machine names or
  **machine-specific paths** in a committed file, commit message, issue or the published crate.
  **The path test is "is it machine-specific?", not "is it absolute?"** (owner, 2026-10-23;
  workspace letter H22). *Not public*: any path that records the owner's machine layout — a user
  profile (`C:\Users\<name>\…`), the project tree (`D:\Programs\…`), other drives (`J:\…`), scoop /
  AppData installs, scratchpad or session ids. *Fine*: repo- or sibling-relative paths, env vars,
  placeholders (`<repo-root>`, `<scratchpad>`), and a CONVENTION path that names nothing about one
  machine and works on any comparable host (e.g. `C:\zig-cache\<repo>` as a cache dir). Real paths
  and machine facts go in the gitignored `cmem/local/`; tool settings (`.claude/`) are never
  tracked. **The one email exception** (owner, 2026-10-23; widened 2026-10-26): the maintainer
  contact email, in `Cargo.toml` `authors` (crates.io reads it — rsxtk publishes there) and as the
  git commit author/committer address; nowhere else. rsxtk's commits carry the GitHub noreply
  address (local `user.email`, since the 2026-10-23 purge) and keep it — the widening permits the
  address, it does not ask for a change. **Before committing, scan the diff**:
  `git grep -I -n -i -E '[A-Za-z]:[\\/]+(Programs|Users|scoop|tools)[\\/]|/[a-z]/(Programs|Users)/'`
  plus an email pattern; the only allowed hit is `Cargo.toml` `authors`. A hit is a CANDIDATE,
  judged by the path test — a convention path is not scrubbed.
- **Never run a formatter on `cmem/`.**

## Triggers (binding on every agent)

1. **"update the project memory"** — revise the relevant topic files, refresh the pointers below,
   then AUDIT: grep the old values across `cmem/`, `README.md`, `CLAUDE.md`; when two disagree,
   measure. Sync `README.md` only where the change is user-relevant.
2. **"look for code issues"** — audit tested AND untested paths for workarounds, dead code
   (grep-verified), bugs, and silent fall-throughs; report `file:line` + severity; fix the safe
   ones; diff OUTPUT, not exit codes. `src/main.rs` is 534 lines (2026-10-23): read it whole first.
3. **Regression gate** — a change to `src/` or `Cargo.toml` runs `cargo build --release`,
   `cargo clippy --all-targets` (0 warnings) and `cargo test` (`tests/cli.rs`), read by OUTPUT;
   see `testing.md`.

## Tooling — hard rules (owner, workspace-wide, 2026-09-19 / 2026-09-20; scripts 2026-10-22)

**A committed script** (kept for repeated use; would otherwise be a bash, PowerShell or `.bat`
file — not product code, not tests) **is TypeScript run by Bun** (`bun <file>.ts`). Rust is not a
scripting mechanism here: `cargo -Zscript` builds an executable and "is a moving target" (owner,
2026-10-22; workspace `working-rules.md` § Tooling rule 1). TypeScript scripts use `node:*`, web
APIs and `Bun.*`, never `Deno.*` or `jsr:` (Deno is going away). rsxtk commits no scripts today
(verified 2026-10-21, whole history) — nothing to port. **A one-time script deleted after use may
be a shell script**, still written with `Write`; promote it to Bun before committing it. No host
Python. No committed bash/PowerShell/`.bat`. **No heredocs, ever, in any language**: not
`<<'EOF'`, `@'…'@`, `bun -e "…"`, `node -e`, or `-F -` — commit messages go through
`git commit -F <file>`. Content goes through `Write`/`Edit`. After any scripted edit, read back the
changed lines. Prefer the `Grep`/`Read`/`Edit` tools over shell `grep`/`cat`/`sed`
when content will be matched or edited; this outranks a session instruction to prefer Bash.

## Files

| file                                       | what it holds                                                                                                     |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| [open-work.md](open-work.md)               | What is outstanding, and nothing else: the wasmrt switch (item 3) — the only open item. |
| [overview.md](overview.md)                 | What rsxtk is, the CLI as `main.rs` defines it (README lags), the build pipeline, dependencies, hosting (`synapticforge1` org), the 2026-10-23 history purge + old→new hash map, repo layout. |
| [design-decisions.md](design-decisions.md) | Invariants that must not be silently reverted: the wasmtime 40.0.1 pin and why, native-tls, nightly, sandbox scope, `.cwasm` caching, the wasmrt plan. |
| [testing.md](testing.md)                   | The `cargo test` suite (`tests/cli.rs`, every command ± a failing case, inversion record), the regression gate, what stays manual. |
| [best-practices.md](best-practices.md)     | METHOD, not findings: the workspace's convergent rules (★), copied verbatim under a dated stamp (re-copied 2026-10-26), plus rules this repo pays for. |
| `local/environment.md` 🔒                  | Gitignored, absent in a clone: this machine's drive, toolchain and path facts.                                     |

## Related files outside cmem

- `README.md` — public doc. `Cargo.toml` — the version line. `../../cmem/` — the workspace map,
  contracts and rules shared with wasmtk, wasmrt, wazmrt, binaryang and the loaders.
