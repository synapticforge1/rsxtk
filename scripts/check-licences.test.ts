import { describe, expect, test } from "bun:test";
import { allowed, inventory, parse, problems, type Pkg } from "./check-licences.ts";

const pkg = (name: string, license: string | null, license_file: string | null = null): Pkg => ({
  name,
  version: "1.0.0",
  license,
  license_file,
});

describe("SPDX expressions", () => {
  test("OR needs one allowed side; AND needs every side", () => {
    expect(allowed("MIT OR Apache-2.0")).toBe(true);
    expect(allowed("MIT OR Apache-2.0 OR LGPL-2.1-or-later")).toBe(true);
    expect(allowed("LGPL-2.1-or-later")).toBe(false);
    expect(allowed("(MIT OR Apache-2.0) AND Unicode-3.0")).toBe(true);
    expect(allowed("Apache-2.0 AND GPL-3.0-only")).toBe(false);
  });
  test("AND binds tighter than OR", () => {
    // GPL-3.0-only OR (MIT AND ISC): allowed through the right side.
    expect(allowed("GPL-3.0-only OR MIT AND ISC")).toBe(true);
    // (GPL-3.0-only AND MIT) OR GPL-2.0-only: not allowed.
    expect(allowed("GPL-3.0-only AND MIT OR GPL-2.0-only")).toBe(false);
  });
  test("WITH is one licence; the legacy slash is OR", () => {
    expect(allowed("Apache-2.0 WITH LLVM-exception")).toBe(true);
    expect(allowed("GPL-2.0-only WITH LLVM-exception")).toBe(false);
    expect(allowed("MIT/Apache-2.0")).toBe(true);
    expect(allowed("Apache-2.0 / MIT")).toBe(true);
  });
  test("a malformed expression is refused, not guessed", () => {
    expect(() => parse("MIT OR")).toThrow();
    expect(() => parse("(MIT")).toThrow();
    expect(() => parse("MIT Apache-2.0")).toThrow();
    expect(() => parse("WITH MIT")).toThrow();
  });
});

describe("problems", () => {
  test("a disallowed, missing or malformed licence is a problem; allowed ones are not", () => {
    const found = problems([
      pkg("good", "MIT"),
      pkg("copyleft", "GPL-3.0-only"),
      pkg("fileonly", null, "LICENSE"),
      pkg("broken", "MIT OR"),
    ]);
    expect(found.map((f) => f.split(" ")[0])).toEqual(["copyleft", "fileonly", "broken"]);
  });
  test("rsxtk's own licence must be MIT OR Apache-2.0", () => {
    expect(problems([pkg("rsxtk", "MIT OR Apache-2.0")])).toEqual([]);
    expect(problems([pkg("rsxtk", "MIT")])).toHaveLength(1);
  });
});

test("the inventory is sorted, one line per package", () => {
  const text = inventory([pkg("zeta", "MIT"), pkg("alpha", null, "COPYING")]);
  expect(text.split("\n").slice(1, 3)).toEqual(["alpha 1.0.0\tlicense-file: COPYING", "zeta 1.0.0\tMIT"]);
});

test("rsxtk itself is not in the inventory, so a version bump never makes it stale", () => {
  const at = (version: string) => inventory([pkg("itoa", "MIT"), { ...pkg("rsxtk", "MIT OR Apache-2.0"), version }]);
  expect(at("0.5.2").split("\n").filter((l) => l.startsWith("rsxtk "))).toEqual([]);
  expect(at("1.0.0")).toBe(at("0.5.2"));
});
