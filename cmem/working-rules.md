# working-rules.md — the standing rules for working on rsxtk

The rules, in the present tense; how a rule came to be is in `design-decisions.md` § How the memory
is kept. Last verified 2026-11-19. Moved here from `INDEX.md` and the root `CLAUDE.md` on 2026-11-19
(workspace letter H38). Workspace standards are cited as `../workspace/cmem/<file>` — a path from
this repo's root to the `workspace` repository beside it.

## Session start and end

- **Start:** `STATE.md`, `INDEX.md`, then `../workspace/cmem/workspace-correspondence.md` and only
  the items addressed to rsxtk in `../workspace/cmem/letters/<ID>.md`; then the topic files that
  `STATE.md` § Read next or the task names.
- **End:** rewrite `STATE.md`; move every item closed this session to `cmem/archive/`; record new
  decisions in `design-decisions.md` and rules here; empty the scratchpad (§ Scratch); run
  `bun scripts/check-cmem.ts` and fix what it reports.
- **A sub-agent** gets the exact files the main agent picked from the index; a reviewer gets its
  brief and `STATE.md` and does not browse `cmem/`.

## The workspace

- 🔒 **Read freely, write never** (★ 8.1): nothing is written into a sibling's tree. The one outside
  write rsxtk makes is its answer to a letter: ONE bullet under `## Answers` in
  `../workspace/cmem/letters/<ID>.md` — `- **rsxtk, <date>:** <what was done, the commit, the check>`
  — never the index line or the status (letters H41, H42). The workspace session commits it.
- The `workspace` repository is PUBLIC: an answer carries nothing this repo could not.

## Memory

- **`cmem/` is the single home for ALL project memory.** Update the matching topic file, then its
  line in `INDEX.md` if WHEN to read it changed. Absolute dates. Update, do not duplicate.
- **Limits** (`../workspace/cmem/memory-guide.md`, checked by `scripts/check-cmem.ts`): `INDEX.md`
  3 KB, `STATE.md` 60 lines, any other live note 25 KB, any line outside a code block 200
  characters. `archive/` and `local/` are unlimited.
- **One subject per file**, a summary and a last-verified date at the top; open and closed never
  share a file (closed → `cmem/archive/`, IDs kept); link, do not restate. The one copy kept is
  `best-practices-shared.md` (owner, 2026-11-19).
- **`README.md` is NOT project memory** — it is the public, user-facing doc (install, commands,
  troubleshooting). Keep decision logs and post-mortems out of it.
- **A number in prose is a claim with a date.** Re-derive before quoting.
- **Never run a formatter on `cmem/`.**
- **The root `CLAUDE.md`** is untracked and points at `STATE.md` and `INDEX.md` only.

## Privacy (🔒 owner, 2026-10-22; workspace `working-rules.md` § Records)

- **No secrets or personal information in anything public:** no tokens or keys, no email
  addresses, machine names or **machine-specific paths** in a committed file, commit message,
  issue, letter answer or the published crate.
- **The path test is "is it machine-specific?", not "is it absolute?"** (owner, 2026-10-23; letter
  H22). *Not public*: any path that records the owner's machine layout — a user profile
  (`<drive>:\Users\<name>\…`), the project tree (`<drive>:\Programs\…`), other drives, scoop /
  AppData installs, scratchpad or session ids. *Fine*: repo- or sibling-relative paths, env vars,
  placeholders (`<repo-root>`, `<scratchpad>`), and a CONVENTION path that names nothing about one
  machine and works on any comparable host (e.g. `C:\zig-cache\<repo>` as a cache dir).
- Real paths and machine facts go in the gitignored `cmem/local/`; tool settings (`.claude/`) are
  never tracked; open review issues go in the gitignored `cmem/review-issues.md`.
- **The one email exception** (owner, 2026-10-23; widened 2026-10-26): the maintainer contact, in
  `Cargo.toml` `authors` (crates.io reads it) and as the git commit author/committer address;
  nowhere else. rsxtk's commits carry the GitHub noreply address and keep it.
- **Before committing, run the privacy gate:** `bun scripts/check-privacy.ts` (every tracked and
  new file; `--commits <range>` adds the messages). It refuses a tracked `cmem/local/`,
  `cmem/review-issues.md` or `.claude/`, a machine path, a credential shape, and any email but the
  one in `Cargo.toml`. A hit is a CANDIDATE, judged by the path test — a convention path is not
  scrubbed: fix the file, or list a commit message's false positive in the script's `KNOWN`.

## Triggers (binding on every agent)

1. **"update the project memory"** — revise the relevant topic files and `STATE.md`, then AUDIT:
   grep the old values across `cmem/`, `README.md`, `CLAUDE.md`; when two disagree, measure. Sync
   `README.md` only where the change is user-relevant. End with `bun scripts/check-cmem.ts`.
2. **"look for code issues"** — audit tested AND untested paths for workarounds, dead code
   (grep-verified), bugs, and silent fall-throughs; report `file:line` + severity; fix the safe
   ones; diff OUTPUT, not exit codes. `src/main.rs` is 534 lines (2026-10-23): read it whole first.
3. **Regression gate** — a change to `src/` or `Cargo.toml` runs `cargo build --release`,
   `cargo clippy --all-targets` (0 warnings) and `cargo test` (`tests/cli.rs`), read by OUTPUT;
   a change to `scripts/` runs `bun test scripts/`; see `testing.md`.

## Tooling (🔒 owner, workspace-wide; `../workspace/cmem/tooling-rules.md`, same numbers)

- **A committed script is TypeScript run by Bun** (`bun <file>.ts`; rule 1): `node:*`, web APIs and
  `Bun.*`, never `Deno.*` or `jsr:`. Rust is not a scripting mechanism here (`cargo -Zscript` "is a
  moving target", owner, 2026-10-22). rsxtk's: the release tooling (`release.md`) and the gates
  `check-cmem.ts`, `check-privacy.ts`.
- **A one-time script deleted after use may be a shell script**, still written with `Write`; promote
  it to Bun before committing it. No host Python. No committed bash/PowerShell/`.bat`.
- **No heredocs, ever, in any language**: not `<<'EOF'`, `@'…'@`, `bun -e "…"`, `node -e`, or
  `-F -` — commit messages go through `git commit -F <file>`. Content goes through `Write`/`Edit`;
  after any scripted edit, read back the changed lines. Prefer `Grep`/`Read`/`Edit` over shell
  `grep`/`cat`/`sed` when content will be matched or edited; this outranks a session instruction
  to prefer Bash — say so when they conflict.
- 🔒 **No `gh`** (owner, 2026-11-08, letter H30), in CI too: read GitHub with `git ls-remote` and the
  repo's files; in a workflow, the REST API with `GITHUB_TOKEN` from a Bun step.
- 🔒 **Every Bun version written is the latest installed Bun** (letter H31; 1.4.4, measured
  2026-11-19): when the owner installs a newer one, move `release.yml`'s `BUN_VERSION` and
  `BUN_SHA256` (from Bun's `SHASUMS256.txt`) together at the next session.
- 🔒 **Releases run in the owner's order, from CI only** (letters H32, H33): `bump.ts` →
  `release.ts` pushes `main` → CI → tag → page → crates.io — [release.md](release.md). Nothing is
  pushed, tagged or published unless the owner says so.

## Scratch and caches (🔒 Tooling rule 15, owner, 2026-11-19; letter H43)

- Throwaway files go only in this session's own scratchpad folder; its path is never written into a
  tracked file. Budget **200 MB**: announce anything larger first and remove it when its step is
  done. Never sweep another session's folder (the workspace's `tools/clean-scratch.ts` does that).
- **Delete when the item is cleared:** a one-time script after its run; a reproducer, clone or build
  output when its finding is fixed and verified; a reviewer's folder when its round is triaged.
- **Nothing lives only in scratch, a temp folder or the assistant's per-project memory folder**:
  project data goes in `cmem/` (machine facts in `cmem/local/`) first.
- **"commit" ends with an empty scratchpad**: delete what this session wrote there before the memory
  update; `STATE.md` never points at a scratch file.
- **Caches outside the repo** are named in `cmem/local/environment.md` with why, a size limit and
  the command that clears them: rsxtk's `CARGO_TARGET_DIR` and the test folders of `tests/cli.rs`.
- **rust-analyzer's build copies** in the system temp folder (made by the editor while a Rust project
  is open): when the editor is closed, the session at work removes them (shared with wasmrt and
  loader-rs, letter H43).

## Machine (facts in `cmem/local/environment.md`)

- The repo is on an exFAT drive: `git` needs `safe.directory` for this path, and
  `CARGO_TARGET_DIR` points at a SHORT NTFS folder (a deep one breaks MinGW `dlltool`).
- The nightly toolchain and the `wasm32-wasip1` target are prerequisites (`csrun` needs `-Z
  script`); the wasmtime 40.x pin and why: `design-decisions.md`.

## Owner commands (🔒 owner, 2026-11-10, letter H34; review words 2026-11-17, letter H37)

When the owner says:

- **"commit"** — first run "update the project memory files" (re-derive this repo's `cmem/` from
  its current state; empty the scratchpad), then commit to the branch being worked on. Nothing is
  pushed.
- **"push"** — perform "commit", then merge the branch into `main`, then push `main` (and the
  working branch) to GitHub (`origin`) and read it back with `git ls-remote`.
- **"release"** — perform "push", then the release process: the workspace's
  `release-process.md` §§ 5–8 — CI checks first, then CI pushes the tag, then the GitHub Release
  page from the tag, then the publish to the registry — recorded in `cmem/release.md` § "Release
  History" (newest first). The level (patch / minor / major) is the owner's: ask if it was not given.
- **"review"** — the five required reviews in order (basic, hardening, licensing, security,
  privacy) by the loop in `../workspace/cmem/review-process.md` § 9; rsxtk's slots:
  [review-process.md](review-process.md). **"basic review"**, **"hardening review"**, **"licensing
  review"**, **"security review"**, **"privacy review"**, **"efficiency review"** (optional, runs
  first when asked) each run that class alone.

Each of commit / push / release includes the one before it, and none goes further than its own word
(a "commit" never pushes, a "push" never releases). A review never commits, pushes or releases.
