/**
 * Release the bumped version: check everything, then push `main` ONLY (workspace letter H33, slot
 * U4; the owner's order, 2026-11-10: "CI to check first, then push the tag, and then create the
 * release page from the tag, then publish the package to the registry").
 *
 *   bun scripts/bump.ts <level>      # first: makes the `bump to vX.Y.Z` commit
 *   bun scripts/release.ts           # then: the guards, the cold gate, ONE push of `main`
 *   bun scripts/release.ts --dry-run # every guard and the gate, nothing pushed
 *
 * It makes and pushes NO tag, ever (no `git tag`, no `-f`): CI's `checks` job runs on the pushed
 * commit, and only when green, for an untagged version, CI tags it `rsxtk_vX.Y.Z`, makes the
 * Release page from the tag, then publishes to crates.io (`.github/workflows/release.yml`). A
 * published tag never moves.
 *
 * Guards, each refusing with nothing pushed:
 *   - the tree is clean (untracked files count);
 *   - on `main`; HEAD = `origin/main` (just fetched) + exactly one commit, `bump to vX.Y.Z`, that
 *     changes only Cargo.toml and Cargo.lock, and X.Y.Z = Cargo.toml's version;
 *   - the tag `rsxtk_vX.Y.Z` absent here and on `origin`;
 *   - crates.io does not list X.Y.Z — read from the crate's version LIST, never the version's own
 *     URL (a caching registry may cache the miss; loader-go, 2026-11-10);
 *   - the cold gate, CI's own: `cargo build --release --locked`, `cargo clippy --all-targets
 *     --locked -- -D warnings`, `cargo test --locked`, `bun test scripts/`,
 *     `bun scripts/compat.ts --release` (fresh surface.txt, CLI floor, the previous tag's tests,
 *     `cargo package`).
 * Then `git push origin main` (never forced) and a read-back: `origin/main` = HEAD, and still no
 * tag on origin. Adapted from universalWasmLoader-rs's release.ts. `node:` builtins and `fetch`.
 */
import { spawnSync } from "node:child_process";
import process from "node:process";
import { ROOT, TAG_PREFIX, cargoVersion, releaseGate, run } from "./compat.ts";

const CRATE = "rsxtk";
const UA = "rsxtk release script (https://github.com/synapticforge1/rsxtk)";

function fail(msg: string, code = 1): never {
  console.error(`release: ${msg} — nothing pushed`);
  process.exit(code);
}

let dryRun = false;
for (const arg of process.argv.slice(2)) {
  if (arg === "--dry-run" || arg === "-n") dryRun = true;
  else fail(`unknown argument "${arg}" — use --dry-run or nothing`, 2);
}

function git(...args: string[]): string {
  const r = run("git", args);
  if (r.status !== 0) fail(`git ${args.join(" ")} failed:\n${r.stderr.trim()}`);
  return r.stdout.trim();
}

/** Run a gate step with inherited output; refuse on a non-zero exit. */
function step(label: string, cmd: string, args: string[]): void {
  console.log(`== ${label}`);
  const r = spawnSync(cmd, args, { cwd: ROOT, stdio: "inherit" });
  if (r.status !== 0) fail(`${label} failed (exit ${r.status})`);
}

// ── the tree and the branch ──
const dirty = git("status", "--porcelain", "--untracked-files=all");
if (dirty) fail(`the tree is not clean (untracked files count):\n${dirty}`);
const branch = git("rev-parse", "--abbrev-ref", "HEAD");
if (branch !== "main") fail(`on "${branch}", not main — merge first, then release from main`);

const version = cargoVersion();
const tag = `${TAG_PREFIX}${version}`;
git("fetch", "-q", "origin", "main");
const upstream = git("rev-parse", "origin/main");
const head = git("rev-parse", "HEAD");
const ahead = git("rev-list", "--count", `${upstream}..HEAD`);
const behind = git("rev-list", "--count", `HEAD..${upstream}`);
if (behind !== "0") fail(`main is ${behind} commit(s) behind origin/main — pull first`);
if (ahead !== "1") fail(`main must be origin/main + exactly the bump commit; it is ${ahead} ahead`);
const subject = git("log", "-1", "--format=%s", "HEAD");
if (subject !== `bump to v${version}`) fail(`the commit on top is "${subject}", not "bump to v${version}" (run bun scripts/bump.ts <level>)`);
const files = git("show", "--name-only", "--format=", "HEAD").split(/\r?\n/).filter(Boolean).sort();
if (files.join(",") !== "Cargo.lock,Cargo.toml") fail(`the bump commit changes ${files.join(", ")}, not only Cargo.toml + Cargo.lock`);

// ── the tag: absent here and on origin ──
if (run("git", ["rev-parse", "-q", "--verify", `refs/tags/${tag}`]).status === 0) fail(`tag ${tag} exists locally`);
const remoteTag = run("git", ["ls-remote", "--exit-code", "--tags", "origin", `refs/tags/${tag}`]);
if (remoteTag.status === 0) fail(`tag ${tag} exists on origin`);
if (remoteTag.status !== 2) fail(`could not ask origin whether ${tag} exists (git ls-remote exit ${remoteTag.status})`);

// ── crates.io: the version LIST (never the version URL before it exists) ──
let listed: string[];
try {
  const res = await fetch(`https://crates.io/api/v1/crates/${CRATE}/versions`, {
    headers: { "User-Agent": UA },
    signal: AbortSignal.timeout(30_000),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  listed = ((await res.json()) as { versions: { num: string }[] }).versions.map((v) => v.num);
} catch (e) {
  fail(`could not read crates.io's version list for ${CRATE}: ${(e as Error).message}`);
}
if (listed.includes(version)) fail(`crates.io already lists ${CRATE} ${version}`);
console.log(`guards: clean, main = origin/main + "bump to v${version}", ${tag} absent here and on origin, crates.io without ${version}`);

// ── the cold gate (CI's checks, in CI's order) ──
step("cargo build --release --locked", "cargo", ["build", "--release", "--locked"]);
step("cargo clippy --all-targets --locked -- -D warnings", "cargo", ["clippy", "--all-targets", "--locked", "--", "-D", "warnings"]);
step("cargo test --locked", "cargo", ["test", "--locked"]);
step("bun test scripts/", process.execPath, ["test", "scripts/"]);
console.log("== bun scripts/compat.ts --release");
try {
  for (const line of releaseGate()) console.log(line);
} catch (e) {
  fail(`compat --release: ${(e as Error).message}`);
}
const after = git("status", "--porcelain", "--untracked-files=all");
if (after) fail(`the gate left the tree dirty:\n${after}`);

if (dryRun) {
  console.log(`release (dry-run): every guard and the gate passed for ${tag}; nothing pushed`);
  process.exit(0);
}

// ── ONE push of main, read back; no tag ──
const push = spawnSync("git", ["push", "origin", "main"], { cwd: ROOT, stdio: "inherit" });
if (push.status !== 0) {
  console.error(`release: git push origin main failed (exit ${push.status})`);
  process.exit(1);
}
const remoteMain = git("ls-remote", "origin", "refs/heads/main").split(/\s+/)[0];
if (remoteMain !== head) {
  console.error(`release: after the push, origin/main is ${remoteMain}, not ${head}`);
  process.exit(1);
}
if (run("git", ["ls-remote", "--exit-code", "--tags", "origin", `refs/tags/${tag}`]).status === 0) {
  console.error(`release: a tag ${tag} appeared on origin — nothing here makes one; investigate`);
  process.exit(1);
}
console.log(`pushed main = ${head} (read back), no tag. CI now: checks -> tag ${tag} -> Release page -> crates.io.`);
console.log(`Watch: https://github.com/synapticforge1/rsxtk/actions`);
