/**
 * Tests for the CLI-surface floor in scripts/compat.ts (workspace letter H33, slot U3): `entries`
 * and `floorOf` on small hand-written surfaces shaped like clap's `--help` pages, plus the
 * version helpers. Run with `bun test scripts/`. The binary-driven parts (`surfaceText`,
 * `freshSurface`, `prevTagTests`) run in `compat.ts --release` itself.
 *
 * Every item below has a description, so no line ends in whitespace (an editor may strip it);
 * the real surface.txt keeps clap's trailing spaces and parses the same way.
 */
import { expect, test } from "bun:test";
import { bumpKind, commandsOf, compare, entries, floorOf, parse } from "./compat.ts";

const TOP = [
  "## rsxtk --help",
  "Usage: rsxtk <COMMAND>",
  "",
  "Commands:",
  "  run    🏃 Run a WASI app or script",
  "  build  🔨 Build",
  "  help   Print this message or the help of the given subcommand(s)",
  "",
  "Options:",
  "  -h, --help     Print help",
  "  -V, --version  Print version",
].join("\n");

const RUN = [
  "## rsxtk run --help",
  "🏃 Run a WASI app or script",
  "",
  "Usage: rsxtk run <PATH>",
  "",
  "Arguments:",
  "  <PATH>  the script",
  "",
  "Options:",
  "  -h, --help  Print help",
].join("\n");

const BUILD = [
  "## rsxtk build --help",
  "🔨 Build",
  "",
  "Usage: rsxtk build <PATH> <TARGET>",
  "",
  "Arguments:",
  "  <PATH>    the input",
  "  <TARGET>  [possible values: wasi, wasm, wat]",
  "",
  "Options:",
  "  -o, --output <OUTPUT>  where to write",
  "  -h, --help             Print help",
].join("\n");

const BASE = [TOP, RUN, BUILD].join("\n\n");

/** Replace exactly one occurrence, or fail the test (a fixture edit that matches nothing proves nothing). */
function edit(text: string, from: string, to: string): string {
  const at = text.indexOf(from);
  expect(at).toBeGreaterThanOrEqual(0);
  expect(text.indexOf(from, at + 1)).toBe(-1);
  return text.slice(0, at) + to + text.slice(at + from.length);
}

test("entries: commands (not help), positions, values, options", () => {
  const e = entries(BASE);
  expect([...e.all]).toEqual([
    "command run", "command build", "rsxtk option -h, --help", "rsxtk option -V, --version",
    "run argument 1 <PATH>", "run option -h, --help",
    "build argument 1 <PATH>", "build argument 2 <TARGET>",
    "build argument <TARGET> value wasi", "build argument <TARGET> value wasm", "build argument <TARGET> value wat",
    "build option -o, --output <OUTPUT>", "build option -h, --help",
  ]);
  expect([...e.required]).toEqual(["run argument 1 <PATH>", "build argument 1 <PATH>", "build argument 2 <TARGET>"]);
});

test("entries: clap's trailing spaces after a bare argument parse the same", () => {
  const bare = BASE.replace("  <PATH>  the script", "  <PATH>  ");
  expect([...entries(bare).all]).toEqual([...entries(BASE).all]);
});

test("commandsOf: the Commands section without help", () => {
  expect(commandsOf(TOP.split("\n").slice(1).join("\n"))).toEqual(["run", "build"]);
});

test("floor: identical structure → patch, even when descriptions change", () => {
  expect(floorOf(BASE, BASE)).toEqual({ kind: "patch", reasons: [] });
  expect(floorOf(BASE, edit(BASE, "the script", "any script")).kind).toBe("patch");
});

test("floor: a removed command, option or value → major", () => {
  expect(floorOf(BASE, [edit(TOP, "  build  🔨 Build\n", ""), RUN].join("\n\n")).kind).toBe("major");
  expect(floorOf(BASE, edit(BASE, "  -o, --output <OUTPUT>  where to write\n", "")).kind).toBe("major");
  const f = floorOf(BASE, edit(BASE, "wasi, wasm, wat", "wasi, wasm"));
  expect(f.kind).toBe("major");
  expect(f.reasons).toEqual(["removed: build argument <TARGET> value wat"]);
});

test("floor: a renamed flag or a reordered positional → major", () => {
  expect(floorOf(BASE, edit(BASE, "-o, --output <OUTPUT>", "-o, --out <OUTPUT>")).kind).toBe("major");
  const swapped = edit(BASE, "  <PATH>    the input\n  <TARGET>  [possible values: wasi, wasm, wat]",
    "  <TARGET>  [possible values: wasi, wasm, wat]\n  <PATH>    the input");
  expect(floorOf(BASE, swapped).kind).toBe("major");
});

test("floor: a new required positional on an existing command → major", () => {
  const f = floorOf(BASE, edit(BASE, "  <PATH>  the script\n", "  <PATH>  the script\n  <MODE>  how\n"));
  expect(f.kind).toBe("major");
  expect(f.reasons).toEqual(["new required: run argument 2 <MODE>"]);
});

test("floor: a new optional positional, option or value → minor", () => {
  expect(floorOf(BASE, edit(BASE, "  <PATH>  the script\n", "  <PATH>  the script\n  [MODE]  how\n")).kind).toBe("minor");
  expect(floorOf(BASE, edit(BASE, "  -h, --help  Print help", "  -q, --quiet  be quiet\n  -h, --help  Print help")).kind).toBe("minor");
  expect(floorOf(BASE, edit(BASE, "wasi, wasm, wat", "wasi, wasm, wat, cwasm")).kind).toBe("minor");
});

test("floor: a new command with required arguments → minor (nobody called it before)", () => {
  const NEW = ["## rsxtk lint --help", "Lint", "", "Arguments:", "  <PATH>  the file", "", "Options:", "  -h, --help  Print help"].join("\n");
  const after = [edit(TOP, "  help ", "  lint   Lint\n  help "), RUN, BUILD, NEW].join("\n\n");
  const f = floorOf(BASE, after);
  expect(f.kind).toBe("minor");
  expect(f.reasons).toContain("added: lint argument 1 <PATH>");
});

test("versions: rsxtk_v tags parse, compare and classify", () => {
  expect(parse("rsxtk_v0.5.1")).toEqual([0, 5, 1]);
  expect(compare("rsxtk_v0.5.1", "0.10.0")).toBeLessThan(0);
  expect(bumpKind("rsxtk_v0.5.1", "0.5.2")).toBe("patch");
  expect(bumpKind("rsxtk_v0.5.1", "0.6.0")).toBe("minor");
  expect(bumpKind("rsxtk_v0.5.1", "1.0.0")).toBe("major");
  expect(() => parse("main")).toThrow();
});
