# open-work.md — what is outstanding in rsxtk

Only open items. Created 2026-09-20.

1. **`rsc/` and `build.rs` are never mirrored into `.tk/`** — the README promised both from 0.4.2
   on; no version of `main.rs` in history ever copied them (checked 2026-10-23). Scripts DO read
   `rsc/…` at run time, through the preopen of the script's folder (verified with a release
   binary: positive prints the file, negative fails `NotFound`). `build.rs` is not supported at
   all. README now states the real behaviour. 🗓️ OWNER: implement `build.rs` (and/or `rsc/`)
   mirroring, or leave it out.
2. **No test suite** — `testing.md` § "What a real suite needs". This is the referee for the engine
   switch and should exist before it.
3. **The wasmrt switch**: measure first (rlib size; end-to-end time wasmtime vs wasmrt, interleaved,
   `.cwasm` off), then swap `wasmtime` + `wasmtime-wasi` for `wasmrt-core`; keep `wat` /
   `wasmprinter` / `walrus`. Owner decision that rsxtk goes first with wasmtk (2026-09-20).
4. **Two clippy warnings in untouched code** (2026-10-23): `map_or` can be simplified
   (`convert_wasm_wat`, `main.rs:366`); collapsible `if` (`edit_deps`, `main.rs:400`). Plus the
   Cargo manifest lint `redundant_homepage` (`homepage` = `repository`).
5. **History purge (workspace letter H19) + ~410 MB of build output** — one rewrite does both.
   Measured 2026-10-20/23: one pack, 416.55 MiB; `target/debug` 245.8 MB / 3106 blobs,
   `target/package` 164 MB / 90 blobs, committed before `/target` was ignored (2026-04-16). Personal
   information in history: the owner's personal address as author on 33 of 34 commits (committer
   on 20; every commit from `80769c2` on uses noreply — recount 2026-10-23); absolute drive paths in 8 commits, ALL inside `target/`, none in messages. Plan: drop
   `target/` and rewrite the owner's address → noreply over every ref (`main`, `origin/0.4.0`, tag
   `rsxtk_v0.4.0`), `git filter-branch` (no host Python), verify zero hits, force-push with lease,
   prune, record the old→new map. `Cargo.toml` `authors` stays (allowed maintainer contact).
   🗓️ OWNER: tag `rsxtk_v0.4.0` (`56c71ea`) holds 2579 `target/` files, 427 with paths — purging
   them changes a tagged tree, which H19 forbids by default (a Go-checksum concern; crates.io does
   not hash the git tree, but its 0.4.x `.cargo_vcs_info.json` keep the old hashes forever).
6. **Siblings still name the pre-move URL `jrmarcum/rsxtk`** (2026-10-20; repo is now
   `synapticforge1/rsxtk`, `overview.md` § Hosting): wasmtk `cmem/vision.md:83`,
   `cmem/polyglot-producers.md:74`, `cmem/roadmap.md:1418`; wasmrt `cmem/loaders.md:28`. Not ours to
   edit (best-practices 8.1). 🔒 Owner, 2026-10-20: these get fixed when wasmtk moves to the org;
   no handoff letter. Nothing for rsxtk to do — close when a grep for `jrmarcum/rsxtk` in those
   trees comes back empty.