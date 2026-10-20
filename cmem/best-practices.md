# best-practices.md — METHOD, not findings

Post-mortems stay in their home file; only the transferable rule comes here, as one bold sentence,
the incident that paid for it, and the origin tag. Rules rsxtk pays for itself are appended under
§ R with `[rsxtk]`.

---

## The convergent rules of the workspace (★)

> Copied from `wasmExamples/cmem/best-practices.md` on 2026-09-20 (owner: canonical there,
> duplicated here so this repo can be investigated on its own — open-book philosophy). Change them
> there and re-copy; do not edit this block in place. Numbers are the canonical file's.

- **1.1 ★ An exit code is not evidence; diff the OUTPUT counts, per file.** Exit-0 runs silently
  dropped passes in four repos. [wabt-ts + binaryen-ts + wazmrt + wasmtk + wasmrt]
- **1.2 ★ A green suite is evidence about the tests, not the code.** The fuzzer had zero
  `makeLoad`; `deno test` collected 513 of 908 and exited 0. [binaryen-ts + wabt-ts]
- **1.3 ★ Re-derive before quoting any number; a count in prose is a dated claim.** [every repo]
- **1.4 ★ When the corpus can move, the denominator is a measurement, and a comparison must print
  it.** Failure totals matched by coincidence after +4 files. [wasmrt + binaryang]
- **1.11 ★ Validate the harness against a known positive; a broken harness can score BETTER.**
  [wabt-ts + binaryen-ts]
- **2.1 ★ A new test that has never failed has not been shown to test anything: invert it and
  watch it fail for the RIGHT reason.** [wazmrt + wabt-ts + binaryen-ts + binaryang + wasmtk]
- **2.3 ★ A gate that cannot fail is decoration; a gate with no trigger, a goal with no gate, a
  gate that only prints, and a property asserted in a doc comment are all preferences.**
  [wazmrt + wasmrt + binaryang]
- **2.4 ★ The local gate must BE CI's gate: read the steps from the workflow file.** [binaryang + wabt-ts]
- **2.16 ★ A change's own new surface is the one place the audit that produced it will not look —
  re-run the governing audit against the new members before landing.** [wazmrt + wasmrt + wasmtk]
- **3.1 ★ A round trip proves agreement with yourself; when a bug is visible only to a third party
  the test has to BE a third party — hand the bytes to an outside reader as a GATE.** Legacy EH:
  every internal check green, wasmtime rejected all ten modules. [wazmrt + binaryen-ts + wabt-ts + wasmrt + wasmtk]
- **3.2 ★ Agreement between components that learned the convention from each other is not
  evidence, and there can be three.** [binaryen-ts + wabt-ts + wazmrt + wasmrt]
- **3.3 ★ Producer/consumer pairs are the recurring blind spot; fix the gap, not a post-process
  workaround, and pin both directions.** [binaryang + wazmrt + wabt-ts]
- **3.8 ★ A stale vendored snapshot is indistinguishable from live data without recorded
  provenance: stamp source + date, and never let a check read an artifact it did not just
  produce.** [wabt-ts + wasmtk + wasmrt + wazmrt]
- **3.10 ★ A claim about another project's code is a hypothesis until you grep their source; a
  stated BENEFIT is a hypothesis about someone else's code.** [wasmtk + wasmrt + wazmrt]
- **4.1 ★ One authoritative enumeration; a second copy drifts into silence; one reader per grammar
  production, and a production has more than one site.** [binaryen-ts + wabt-ts + wasmrt + wazmrt]
- **4.2 ★ A value read and discarded is a decision; a wrong decode made harmless by a guard in
  another file is a load-bearing coincidence.** [binaryen-ts + wabt-ts + wasmrt]
- **4.3 ★ Silent-wrong output is the worst class: ask "what would a wrong answer look like?"
  first, and prefer a hard error over a stub.** [wasmrt + wasmtk + wabt-ts]
- **4.16 ★ A rationale expires; the code it justified does not: a comment justifying a deviation
  names the authority it defers to; "by design" ages badly by definition.** [wasmrt + wazmrt + binaryang]
- **5.1 ★ Prove, do not assume: prove the REASON, not just the answer; UNVERIFIED is a legitimate
  status.** [wazmrt + wasmrt]
- **5.2 ★ Identify the LAYER before naming a cause; the stage that REPORTS a failure is usually not
  the stage that caused it; vary one thing at a time.** [wazmrt + binaryen-ts + wasmtk + wasmrt]
- **6.1 ★ A written result is a CLAIM: a commit message, a status line, a doc of a safety guard.
  Compare it to the artifact; put a pin where the tool enforces it.** [binaryang + wabt-ts + wasmrt]
- **6.2 ★ Write down the thing you only said out loud; write the negatives, the calibration, and
  what you did NOT check; distinguish clean / measured / UNMEASURED.** [wabt-ts + binaryang]
- **6.3 ★ Record findings that were WRONG so they are not fixed again; a retraction re-checks the
  REASONING, not the requirement; a correction is a dated claim too.** [wazmrt + wasmtk + binaryen-ts + wabt-ts]
- **7.1 ★ Do not author file CONTENT through a shell heredoc; scripts go in files, always; commit
  messages through `-F <file>`.** Five corruptions including a NUL byte in `.gitattributes`.
  [binaryang + wasmtk + wazmrt + wasmrt]
- **7.2 ★ When the exit code IS the measurement, do not pipe.** [wasmtk + wasmrt + wabt-ts]
- **7.3 ★ A change that silently does nothing is indistinguishable from one that worked: prefer
  "fail if absent"; edit scripts assert each match exactly once; read back.** [binaryang + wazmrt + wasmrt]
- **7.6 ★ Line endings: pin in `.gitattributes` wildcard-first; measure with a positive control;
  never fix with `git config`.** [wasmtk + binaryang + wazmrt + wasmrt]
- **8.1 ★ Read freely, write never: nothing is written into a sibling's tree unless the owner
  directs that write, and then it is logged in both memories.** [wasmrt + wazmrt + binaryang + wabt-ts]

---

## § R — rules rsxtk has paid for

- **R.1 A build-time dependency of a dependency is part of your pin.** wasmtime 42+ pulled
  `aws-lc-sys`, which needs NASM + cmake; the constraint is recorded in the specifier and in
  `design-decisions.md`, with the condition under which it lifts. [rsxtk, 2026-04]
- **R.2 A README feature list headed with a version is a claim that goes stale on the next bump.**
  "New Features in v0.4.2" under Cargo 0.4.4, with six subcommands missing. Generate the command
  table from `main.rs` or stop dating the heading. [rsxtk, found 2026-09-20]
