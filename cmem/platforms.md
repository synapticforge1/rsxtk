# platforms.md — where rsxtk is tested, and what differs per platform

The support matrix, then notes per platform ID only where there is something to say. Common facts
(the six IDs, statuses, testing tiers, the Linux baseline): `../workspace/cmem/platforms.md`.
Created 2026-11-19 (letter H37, work item (g)). Machine names stay in `cmem/local/`.

## Support matrix

rsxtk ships SOURCE (the crates.io crate); a user builds it with `cargo install`. "Target name" is
the Rust host triple it was built and tested as.

| Platform | Status | Target name | Tested on |
| --- | --- | --- | --- |
| win-x64 | tested | x86_64-pc-windows-gnu | the development machine |
| win-arm64 | pending | aarch64-pc-windows-gnullvm | — |
| mac-x64 | pending | x86_64-apple-darwin | — |
| mac-arm64 | pending | aarch64-apple-darwin | — |
| linux-x64 | pending | x86_64-unknown-linux-gnu | — |
| linux-arm64 | pending | aarch64-unknown-linux-gnu | — |

- Until 0.5.2, CI's `checks` job ran the gate on GitHub's `ubuntu-latest` (linux-x64). That was CI,
  not a test machine; the tests left CI on 2026-11-19 (H37 (e)); linux-x64 stays `pending` until
  a local run on a named distro. CI still runs `cargo package` there for a release (a build, not
  a test).
- A release states one `Tested on: <OS and version> (<platform ID>)` line per platform actually run
  — on the Release page, in its `release.md` entry and in the README — and "other platforms: not
  yet tested" (owner, 2026-11-19). The first release to carry it is the next one.

## Linux distros tested

| Distro | Version | Platform | Date |
| --- | --- | --- | --- |
| — | — | — | none yet |

- **glibc 2.28 baseline:** unaffected — rsxtk publishes no Linux binary; a user's build links the
  user's own glibc. No musl build is made (none is shipped).

## win-x64

- **TLS:** `reqwest` uses `native-tls`, which is SChannel here (`Cargo.toml`, the comment tagged
  `[win-x64]`): no cmake or NASM, which this machine lacks. On Linux the same feature is OpenSSL and
  needs its development headers to build; on macOS, Security.framework.
- **Toolchain:** the GNU host (MinGW `dlltool`); `CARGO_TARGET_DIR` must be SHORT — a deep path
  breaks `dlltool` in `windows-sys` ("Can't open def file") (`testing.md`).
- **Filesystem:** the development drive is exFAT — `target/` there poisons after one build, so the
  target dir lives on NTFS; `git` needs `safe.directory` (`working-rules.md` § Machine).
- **Binaries:** two release builds of one source differ in the PE `TimeDateStamp` and `CheckSum`
  only (`testing.md` § Comparing binaries).

## Every platform

- The script cache is a plain `.wasm` (portable): no `.cwasm`, no machine id, nothing
  platform-specific in `.tk/` except Cargo's own build output.
- `csrun` needs the nightly toolchain; the build needs the `wasm32-wasip1` target on every host.
