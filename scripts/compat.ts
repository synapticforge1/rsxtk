/**
 * rsxtk's compatibility gate (workspace letters H6, H33 slots U3 and U5), one implementation for
 * every caller: `scripts/bump.ts` (the floor before a bump), `scripts/release.ts` (the cold gate
 * before `main` is pushed) and CI's `checks` job (on the pushed commit, before any tag exists).
 *
 *   bun scripts/compat.ts surface           # (re)write the golden surface.txt from the binary
 *   bun scripts/compat.ts surface --check   # refuse when surface.txt is stale
 *   bun scripts/compat.ts --release         # the whole release-mode gate
 *
 * rsxtk has no library API, so `cargo semver-checks` has nothing to read: its SURFACE is the CLI.
 * `surface.txt` (committed) is every `--help` page — `rsxtk --help` and `rsxtk <command> --help`
 * for each command — with the binary's name normalized (`rsxtk.exe` on Windows). The FLOOR
 * compares the STRUCTURE of two surfaces, never the descriptions:
 *   entries: each command; each positional argument with its position and shape (`<REQUIRED>`,
 *   `[OPTIONAL]`, `...`); each possible value; each option with its flags and value name.
 *   removed or reshaped entry → major; a new REQUIRED positional on an existing command → major;
 *   any other new entry (a new command with its arguments included) → minor; identical → patch.
 * It sees what `--help` shows, not exit codes or output formats: the previous tag's tests cover
 * behaviour.
 *
 * Release mode, against the PREVIOUS release (the highest `rsxtk_v*` tag below Cargo.toml's
 * version; a tag for this version is allowed only on HEAD — a re-run):
 *   1. surface.txt is fresh (equals this build's `--help` pages);
 *   2. the floor of the change since <prev> is at most the version bump;
 *   3. <prev>'s `tests/` against this `src/`, in a throwaway `.compat/` crate; red refuses a patch
 *      or minor, a major may proceed;
 *   4. `cargo package --locked` (the publish dry run; it builds the packaged crate).
 * <prev>'s surface is `git show <prev>:surface.txt`; a tag from before the golden existed (0.5.1
 * and older) is built in a throwaway `.compat-prev/` and asked for its `--help` pages.
 *
 * Throwaway crates build in their own subdirectories of the target directory (CARGO_TARGET_DIR,
 * else ./target): `compat/` and `compat-prev/` — never the repo's own (see `targetDir`). They
 * persist between runs, so dependencies are built once per subdirectory. `node:` builtins only.
 */
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

export const KINDS = ["patch", "minor", "major"] as const;
export type Kind = (typeof KINDS)[number];

export const ROOT = fileURLToPath(new URL("..", import.meta.url));
export const TAG_PREFIX = "rsxtk_v";
export const SURFACE_FILE = "surface.txt";
const VERSION_LINE = /^version[ \t]*=[ \t]*"(\d+\.\d+\.\d+)"/m;

/**
 * The target directory for a build of `which` crate. ⚠️ Each throwaway crate gets its OWN
 * subdirectory: cargo keys a workspace member's build by name + version, not by its folder, so a
 * copy of rsxtk at the same version sharing the repo's target dir is the "same" unit — the repo's
 * build then reads as fresh after a source edit and `target/debug/rsxtk` may be the copy's binary
 * (found 2026-11-12 by inverting this gate: a renamed flag still showed the old name).
 */
export function targetDir(which: "repo" | "compat" | "compat-prev" = "repo"): string {
  const base = process.env.CARGO_TARGET_DIR || join(ROOT, "target");
  return which === "repo" ? base : join(base, which);
}

export function run(cmd: string, args: string[], cwd = ROOT, target = targetDir()) {
  const r = spawnSync(cmd, args, {
    cwd,
    encoding: "utf8",
    env: { ...process.env, CARGO_TARGET_DIR: target },
    maxBuffer: 64 * 1024 * 1024,
  });
  return { status: r.status ?? 1, stdout: r.stdout ?? "", stderr: r.stderr ?? "", error: r.error };
}

export function parse(v: string): [number, number, number] {
  const m = v.match(new RegExp(`^(?:${TAG_PREFIX}|v)?(\\d+)\\.(\\d+)\\.(\\d+)$`));
  if (!m) throw new Error(`not a version: ${v}`);
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

export function compare(a: string, b: string): number {
  const [x, y] = [parse(a), parse(b)];
  for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] - y[i];
  return 0;
}

/** The level of the change from `a` to `b` (b > a); null when equal. */
export function bumpKind(a: string, b: string): Kind | null {
  const [x, y] = [parse(a), parse(b)];
  if (x[0] !== y[0]) return "major";
  if (x[1] !== y[1]) return "minor";
  if (x[2] !== y[2]) return "patch";
  return null;
}

export function atLeast(k: Kind, floor: Kind): boolean {
  return KINDS.indexOf(k) >= KINDS.indexOf(floor);
}

export function cargoVersion(root = ROOT): string {
  const m = readFileSync(join(root, "Cargo.toml"), "utf8").match(VERSION_LINE);
  if (!m) throw new Error("no version line in Cargo.toml");
  return m[1];
}

/** The highest `rsxtk_v*` tag strictly below `version`, or null. */
export function previousTag(version: string, root = ROOT): string | null {
  const re = new RegExp(`^${TAG_PREFIX}\\d+\\.\\d+\\.\\d+$`);
  const tags = run("git", ["tag", "-l", `${TAG_PREFIX}*`], root).stdout.split(/\r?\n/).filter((t) => re.test(t));
  const below = tags.filter((t) => compare(t, version) < 0).sort(compare);
  return below.at(-1) ?? null;
}

// ── the surface ──

/** `rsxtk <args>` through `cargo run` in `dir`; its stdout, CRLF and the binary name normalized. */
function help(dir: string, args: string[], target: string): string {
  const r = run("cargo", ["run", "--quiet", "--locked", "--", ...args], dir, target);
  if (r.status !== 0) throw new Error(`cargo run -- ${args.join(" ")} in ${dir} failed (exit ${r.status}):\n${r.stderr.trim()}`);
  return r.stdout.replace(/\r\n/g, "\n").replace(/\brsxtk\.exe\b/g, "rsxtk").trimEnd();
}

/** The command names listed under "Commands:" (without `help`). */
export function commandsOf(top: string): string[] {
  const out: string[] = [];
  let inCommands = false;
  for (const line of top.split("\n")) {
    if (/^\S/.test(line)) inCommands = line.trim() === "Commands:";
    else if (inCommands) {
      const m = line.match(/^ {2}(\S+)/);
      if (m && m[1] !== "help") out.push(m[1]);
    }
  }
  return out;
}

/** Every `--help` page of the binary built from `dir`, as the golden file's text. */
export function surfaceText(dir = ROOT, target = targetDir()): string {
  const top = help(dir, ["--help"], target);
  const pages = [`## rsxtk --help\n${top}`];
  for (const c of commandsOf(top)) pages.push(`## rsxtk ${c} --help\n${help(dir, [c, "--help"], target)}`);
  return `${pages.join("\n\n")}\n`;
}

/** The structural entries of a surface text, and which of them are required positionals. */
export function entries(text: string): { all: Set<string>; required: Set<string> } {
  const all = new Set<string>();
  const required = new Set<string>();
  let page = "";
  let section = "";
  let position = 0;
  for (const line of text.replace(/\r\n/g, "\n").split("\n")) {
    const head = line.match(/^## rsxtk (?:(\S+) )?--help$/);
    if (head) {
      page = head[1] ?? "rsxtk";
      section = "";
      position = 0;
      continue;
    }
    if (/^\S/.test(line)) {
      section = line.trim().replace(/:$/, "");
      continue;
    }
    const item = line.match(/^ {2}(\S.*?)(?: {2,}(.*))?$/);
    if (!item) continue;
    const [, name, desc = ""] = item;
    if (section === "Commands" && page === "rsxtk") {
      if (name !== "help") all.add(`command ${name}`);
    } else if (section === "Arguments") {
      position++;
      const e = `${page} argument ${position} ${name}`;
      all.add(e);
      if (name.startsWith("<")) required.add(e);
      const values = desc.match(/\[possible values: ([^\]]+)\]/);
      if (values) for (const v of values[1].split(",")) all.add(`${page} argument ${name} value ${v.trim()}`);
    } else if (section === "Options") {
      all.add(`${page} option ${name}`);
    }
  }
  return { all, required };
}

/** The least level the change from `before` to `after` needs, with the entries that set it. */
export function floorOf(before: string, after: string): { kind: Kind; reasons: string[] } {
  const [a, b] = [entries(before), entries(after)];
  const removed = [...a.all].filter((e) => !b.all.has(e));
  const added = [...b.all].filter((e) => !a.all.has(e));
  // A required positional breaks callers only on a command they could already call.
  const existed = (e: string) => {
    const page = e.split(" ")[0];
    return page === "rsxtk" || a.all.has(`command ${page}`);
  };
  const newRequired = added.filter((e) => b.required.has(e) && existed(e));
  if (removed.length || newRequired.length) {
    return { kind: "major", reasons: [...removed.map((e) => `removed: ${e}`), ...newRequired.map((e) => `new required: ${e}`)] };
  }
  if (added.length) return { kind: "minor", reasons: added.map((e) => `added: ${e}`) };
  return { kind: "patch", reasons: [] };
}

/** surface.txt as committed in the work tree, or throws when it is missing or stale. */
export function freshSurface(root = ROOT): string {
  const file = join(root, SURFACE_FILE);
  if (!existsSync(file)) throw new Error(`${SURFACE_FILE} is missing — bun scripts/compat.ts surface`);
  const golden = readFileSync(file, "utf8").replace(/\r\n/g, "\n");
  const now = surfaceText(root);
  if (golden !== now) {
    throw new Error(`${SURFACE_FILE} is stale: this build's --help pages differ — run bun scripts/compat.ts surface, review the diff, commit it`);
  }
  return golden;
}

/** <tag>'s surface: its committed golden, else its binary's `--help` pages (built in `.compat-prev/`). */
export function surfaceAt(tag: string, root = ROOT): string {
  const shown = run("git", ["show", `${tag}:${SURFACE_FILE}`], root);
  if (shown.status === 0) return shown.stdout.replace(/\r\n/g, "\n");
  const prev = join(root, ".compat-prev");
  rmSync(prev, { recursive: true, force: true });
  try {
    mkdirSync(prev);
    extract(tag, [], prev, root);
    return surfaceText(prev, targetDir("compat-prev"));
  } finally {
    rmSync(prev, { recursive: true, force: true });
  }
}

/** `git archive <tag> [paths]` extracted into `dir`. */
function extract(tag: string, paths: string[], dir: string, root: string): void {
  const tar = join(dir, "tag.tar");
  const archived = run("git", ["archive", "--format=tar", "-o", tar, tag, ...paths], root);
  const untarred = archived.status === 0 && run("tar", ["-xf", "tag.tar"], dir).status === 0;
  if (!untarred) throw new Error(`could not extract ${tag}${paths.length ? `:${paths.join(" ")}` : ""}\n${archived.stderr.trim()}`);
  rmSync(tar);
}

/** <tag>'s `tests/` against this `src/` in a throwaway `.compat/` crate. */
export function prevTagTests(tag: string, root = ROOT): { green: boolean; summary: string; failures: string } {
  const compat = join(root, ".compat");
  rmSync(compat, { recursive: true, force: true });
  try {
    mkdirSync(compat);
    for (const f of ["Cargo.toml", "Cargo.lock", "README.md"]) cpSync(join(root, f), join(compat, f));
    cpSync(join(root, "src"), join(compat, "src"), { recursive: true });
    extract(tag, ["tests"], compat, root);
    const r = run("cargo", ["test", "--locked", "--tests"], compat, targetDir("compat"));
    const out = `${r.stdout}\n${r.stderr}`;
    let passed = 0;
    let failed = 0;
    for (const m of out.matchAll(/test result: \w+\. (\d+) passed; (\d+) failed/g)) {
      passed += Number(m[1]);
      failed += Number(m[2]);
    }
    const green = r.status === 0 && passed > 0 && failed === 0;
    const failures = green ? "" : out.split("\n").filter((l) => /FAILED|panicked|error(\[|:)/.test(l)).join("\n");
    return { green, summary: `${passed} passed, ${failed} failed (exit ${r.status})`, failures };
  } finally {
    rmSync(compat, { recursive: true, force: true });
  }
}

/** The whole release-mode gate. Returns the lines to print; throws with the reason on a refusal. */
export function releaseGate(root = ROOT): string[] {
  const out: string[] = [];
  const version = cargoVersion(root);
  const here = run("git", ["rev-parse", "HEAD"], root).stdout.trim();
  const own = run("git", ["rev-parse", "-q", "--verify", `refs/tags/${TAG_PREFIX}${version}^{commit}`], root);
  if (own.status === 0 && own.stdout.trim() !== here) {
    throw new Error(`tag ${TAG_PREFIX}${version} exists on ${own.stdout.trim()}, not on HEAD ${here}: this version is already released`);
  }
  const prev = previousTag(version, root);
  if (prev === null) throw new Error(`no ${TAG_PREFIX}* tag below ${version}: nothing to compare with`);
  const kind = bumpKind(prev, version);
  if (kind === null) throw new Error(`Cargo.toml ${version} equals ${prev}`);
  out.push(`release ${version} over ${prev}: a ${kind} bump`);

  // 1. the golden is fresh
  const now = freshSurface(root);
  out.push(`${SURFACE_FILE} is fresh (${entries(now).all.size} surface entries)`);

  // 2. the floor
  const f = floorOf(surfaceAt(prev, root), now);
  if (!atLeast(kind, f.kind)) {
    throw new Error(`the CLI surface since ${prev} needs at least ${f.kind}; a ${kind} release is too small:\n${f.reasons.join("\n")}`);
  }
  out.push(`CLI floor over ${prev}: ${f.kind}${f.reasons.length ? ` (${f.reasons.join("; ")})` : ""} — the ${kind} bump covers it`);

  // 3. behaviour
  const t = prevTagTests(prev, root);
  out.push(`${prev} tests/ against this src/: ${t.green ? "green" : "RED"} — ${t.summary}`);
  if (!t.green && kind !== "major") {
    throw new Error(`${prev}'s tests are red against this src/; a ${kind} release is refused (classify each failure)\n${t.failures}`);
  }

  // 4. packaging (the publish dry run)
  const pkg = run("cargo", ["package", "--locked", "--quiet"], root);
  if (pkg.status !== 0) throw new Error(`cargo package --locked failed:\n${pkg.stderr.trim()}`);
  out.push("cargo package --locked: the packaged crate builds");
  return out;
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const usage = "usage — bun scripts/compat.ts surface [--check] | --release";
  try {
    if (args.length === 1 && args[0] === "--release") {
      for (const line of releaseGate()) console.log(line);
      console.log("compat --release: green");
    } else if (args[0] === "surface" && args.length === 1) {
      const text = surfaceText();
      writeFileSync(join(ROOT, SURFACE_FILE), text);
      console.log(`wrote ${SURFACE_FILE}: ${entries(text).all.size} surface entries`);
    } else if (args[0] === "surface" && args.length === 2 && args[1] === "--check") {
      const text = freshSurface();
      console.log(`${SURFACE_FILE} is fresh: ${entries(text).all.size} surface entries`);
    } else {
      console.error(`compat: ${usage}`);
      process.exit(2);
    }
  } catch (e) {
    console.error(`compat: ${(e as Error).message}`);
    process.exit(1);
  }
}
