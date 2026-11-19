/**
 * The licence inventory gate (workspace review standard, letter H37: "the licence inventory against
 * an allowlist" is a GATE, run locally). Created 2026-11-19 (rsxtk open-work 13).
 *
 *   bun scripts/check-licences.ts           # check: the inventory is fresh and every licence allowed
 *   bun scripts/check-licences.ts --write   # regenerate licences.txt (review the diff, commit it)
 *
 * What it reads: `cargo metadata --format-version 1 --locked` — every package in `Cargo.lock`, all
 * targets, with the `license` field from the package's OWN `Cargo.toml` in the downloaded source
 * (not the registry's listing). A package with only `license-file` is refused until judged by hand.
 *
 * What it refuses:
 *   - a package whose SPDX expression the allowlist does not satisfy (OR: one side is enough; AND:
 *     every side; `X WITH E`: the pair is listed); the legacy `/` is read as OR;
 *   - rsxtk's own licence differing from `MIT OR Apache-2.0`, or a missing LICENSE-MIT /
 *     LICENSE-APACHE (both ship in the crate);
 *   - a stale `licences.txt` (the committed inventory: one line per package).
 *
 * rsxtk publishes SOURCE only: no dependency is bundled, so no third-party notice file ships; the
 * licences reach users through their own `cargo install`. The allowlist holds exactly the licences
 * in use on 2026-11-19 — a new one fails here until it is judged and added, with its reason.
 */
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";

export const OWN_LICENCE = "MIT OR Apache-2.0";
export const INVENTORY = "licences.txt";

/** Permissive licences, each compatible with distributing rsxtk's source as MIT OR Apache-2.0. */
export const ALLOWED: ReadonlySet<string> = new Set([
  "MIT",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "Unicode-3.0",
  "Unlicense",
]);

type Node = { id: string } | { op: "AND" | "OR"; left: Node; right: Node };

/** Parses an SPDX licence expression (with the legacy `/` as OR). Throws on a malformed one. */
export function parse(expr: string): Node {
  const tokens = expr.replace(/\//g, " OR ").replace(/([()])/g, " $1 ").split(/\s+/).filter(Boolean);
  let i = 0;
  const peek = () => tokens[i];
  function atom(): Node {
    const t = tokens[i++];
    if (t === undefined) throw new Error(`"${expr}": ends early`);
    if (t === "(") {
      const n = or();
      if (tokens[i++] !== ")") throw new Error(`"${expr}": missing )`);
      return n;
    }
    if (t === ")" || t === "AND" || t === "OR" || t === "WITH") throw new Error(`"${expr}": unexpected ${t}`);
    if (peek() === "WITH") {
      i++;
      const e = tokens[i++];
      if (e === undefined || e === "(" || e === ")") throw new Error(`"${expr}": WITH needs an exception`);
      return { id: `${t} WITH ${e}` };
    }
    return { id: t };
  }
  function and(): Node {
    let n = atom();
    while (peek() === "AND") {
      i++;
      n = { op: "AND", left: n, right: atom() };
    }
    return n;
  }
  function or(): Node {
    let n = and();
    while (peek() === "OR") {
      i++;
      n = { op: "OR", left: n, right: and() };
    }
    return n;
  }
  const n = or();
  if (i !== tokens.length) throw new Error(`"${expr}": unexpected ${tokens[i]}`);
  return n;
}

/** True when the allowlist satisfies the expression. */
export function allowed(expr: string, allow: ReadonlySet<string> = ALLOWED): boolean {
  const walk = (n: Node): boolean =>
    "id" in n ? allow.has(n.id) : n.op === "OR" ? walk(n.left) || walk(n.right) : walk(n.left) && walk(n.right);
  return walk(parse(expr));
}

export interface Pkg {
  name: string;
  version: string;
  license: string | null;
  license_file: string | null;
}

/** The inventory text: one sorted line per package, `name version<TAB>licence`. */
export function inventory(pkgs: readonly Pkg[]): string {
  const lines = pkgs
    .map((p) => `${p.name} ${p.version}\t${p.license ?? `license-file: ${p.license_file ?? "NONE"}`}`)
    .sort();
  return `# rsxtk's dependency licences, from Cargo.lock (bun scripts/check-licences.ts --write)\n${lines.join("\n")}\n`;
}

/** Every problem with these packages: a missing or disallowed licence. */
export function problems(pkgs: readonly Pkg[], own = "rsxtk"): string[] {
  const out: string[] = [];
  for (const p of pkgs) {
    const at = `${p.name} ${p.version}`;
    if (p.name === own) {
      if (p.license !== OWN_LICENCE) out.push(`${at}: own licence is "${p.license}", not "${OWN_LICENCE}"`);
      continue;
    }
    if (p.license === null) {
      out.push(`${at}: no SPDX licence (license-file ${p.license_file ?? "none"}) — judge it by hand`);
      continue;
    }
    try {
      if (!allowed(p.license)) out.push(`${at}: "${p.license}" is not satisfied by the allowlist`);
    } catch (e) {
      out.push(`${at}: ${(e as Error).message}`);
    }
  }
  return out;
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const write = args.includes("--write");
  if (args.some((a) => a !== "--write")) {
    console.error("check-licences: use --write or nothing");
    process.exit(2);
  }
  const root = join(import.meta.dir, "..");
  const r = spawnSync("cargo", ["metadata", "--format-version", "1", "--locked"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 1 << 28,
    timeout: 300_000,
  });
  if (r.status !== 0) {
    console.error(`check-licences: cargo metadata failed (exit ${r.status}):\n${r.stderr}`);
    process.exit(1);
  }
  const pkgs = (JSON.parse(r.stdout) as { packages: Pkg[] }).packages;
  if (pkgs.length < 50) {
    console.error(`check-licences: only ${pkgs.length} packages — the listing is broken, not the tree small`);
    process.exit(1);
  }
  const found = problems(pkgs);
  for (const f of ["LICENSE-MIT", "LICENSE-APACHE"]) {
    if (!existsSync(join(root, f))) found.push(`${f} is missing`);
  }
  const text = inventory(pkgs);
  const path = join(root, INVENTORY);
  if (write) {
    writeFileSync(path, text);
    console.log(`check-licences: wrote ${INVENTORY} (${pkgs.length} packages)`);
  } else {
    const old = existsSync(path) ? readFileSync(path, "utf8").replace(/\r\n/g, "\n") : "";
    if (old !== text) found.push(`${INVENTORY} is stale — run bun scripts/check-licences.ts --write and review the diff`);
  }
  if (found.length > 0) {
    console.error(`check-licences: ${found.length} problem(s):`);
    for (const f of found) console.error(`  ${f}`);
    process.exit(1);
  }
  console.log(`check-licences: ${pkgs.length} packages, every licence allowed${write ? "" : `, ${INVENTORY} fresh`}`);
}
