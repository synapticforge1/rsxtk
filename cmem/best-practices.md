# best-practices.md — METHOD, not findings

Post-mortems stay in their home file; only the transferable rule comes here, as one bold sentence,
the incident that paid for it, and the origin tag. Rules rsxtk pays for itself are appended under
§ R with `[rsxtk]`. Last verified 2026-11-20.

The workspace's convergent rules (★), cited here as "★ 2.4", "best-practices 7.22" and so on, are
copied in [best-practices-shared.md](best-practices-shared.md) (moved there 2026-11-19, letter H38).

---

## § R — rules rsxtk has paid for

- **R.1 A build-time dependency of a dependency is part of your pin.** wasmtime 42+ pulled
  `aws-lc-sys`, which needs NASM + cmake; the constraint is recorded in the specifier and in
  `design-decisions.md`, with the condition under which it lifts. [rsxtk, 2026-04]
- **R.2 A README feature list headed with a version is a claim that goes stale on the next bump.**
  "New Features in v0.4.2" under Cargo 0.4.4, with six subcommands missing. Generate the command
  table from `main.rs` or stop dating the heading. [rsxtk, found 2026-09-20]
- **R.3 An abridged copy of a canonical block is a second enumeration (★ 4.1): it drifts silently
  even with "do not edit in place" above it.** The 2026-09-20 ★ copy shortened every rule; by
  2026-10-26, 12 of 28 bold sentences differed from the canonical file and nothing had fired.
  Copy verbatim, and check by script. [rsxtk, found 2026-10-26]
- **R.4 "Same binary" needs a same-source control before it means anything.** A whole-file hash
  of the release exe was the planned proof that renormalizing CRLF → LF changed no code; it
  differed — and so did two builds of the SAME source (MinGW stamps the PE `TimeDateStamp` and
  `CheckSum`). Build one source twice, find the bytes that vary, mask exactly those, then compare
  (`testing.md` § Comparing binaries). [rsxtk, found 2026-10-26]
- **R.5 A throwaway copy of the crate never shares the real crate's target directory.** The
  release gate built rsxtk's previous-tag tests in a `.compat/` copy at the same name and version,
  sharing the target dir to save the dependency build; after that, an edited clone's build read as
  Fresh and `--help` still showed a flag that had been renamed — the gate would have compared a
  stale binary and passed. Found only by inverting the gate with a real CLI change. Each copy now
  builds in its own target subdirectory (`scripts/compat.ts` `targetDir`). The mechanism (cargo
  keying the build by name and version, not folder) is the likely cause, not a measured one.
  [rsxtk, found 2026-11-12]
- **R.6 A restored source must look NEWER than the build it replaces, and must be read and written
  as UTF-8 bytes.** In the 0.6.0 review, mutants were made with PowerShell 5.1's `Get-Content -Raw`
  (it decodes BOM-less UTF-8 as the ANSI code page: every emoji double-encoded) and the original put
  back with `Copy-Item` (it keeps the file's OLD timestamp, so cargo kept the mutant's binary as
  fresh). The next `surface` regeneration ran the stale mutant and wrote mojibake into the golden;
  only an encoding check caught it. Read/write with `[IO.File]` and an explicit UTF-8 encoding (or
  `cmd /c "git show … > file"` for exact bytes), and touch restored files before the next build.
  [rsxtk, found 2026-11-20]
- **R.7 A validator that claims to match a tool is checked against that tool, never by reading.**
  `add`'s version check was "re-done" by reading Cargo's grammar three times; each time a Cargo
  oracle (`cargo metadata` on 100+ requirements) still found forms one side took and the other
  refused (`x`, `1-rc`, u64 overflow, `>=1,*`). Run the oracle on every change. [rsxtk, 2026-11-20]
- **R.8 A fix that refuses input must be measured against the last release first.** Refusing all
  Windows device names looked safe; on 1.0.0, con / prn / aux / com / lpt scripts ran, so the
  refusal was a break. Measure what the release did, then refuse only what never worked. [rsxtk,
  2026-11-20]
