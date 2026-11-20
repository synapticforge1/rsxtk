use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use anyhow::{Result, bail, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::Value;
use wasmtime::{Engine, Linker, Module, ResourceLimiter, Store, Val, Config, OptLevel};
use wasmtime_wasi::{FsPerms, WasiCtxBuilder};
use wasmtime_wasi::p1::{self, WasiP1Ctx};

#[derive(Parser)]
#[command(name = "rsxtk", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Most host memory a guest's memories and tables may use together, in MiB (run, mod, bench)
    #[arg(long, global = true, default_value_t = 4096, value_parser = clap::value_parser!(u64).range(1..=1_048_576))]
    max_memory: u64,
    /// Let the guest create, change and delete files in its folder (run, mod, bench); read-only without it
    #[arg(long, global = true)]
    allow_write: bool,
}

/// What a guest may use (owner, 2026-11-20): its folder read-only unless `--allow-write`, and a
/// memory budget for all its memories and tables together; no time limit.
#[derive(Clone, Copy)]
struct Sandbox { allow_write: bool, max_memory_bytes: usize }

#[derive(Subcommand)]
#[command(rename_all = "lowercase")]
enum Commands {
    /// 🏃 Run a WASI app or script
    Run { path: PathBuf },
    /// 🧩 Execute functions from a library
    Mod { 
        path: PathBuf, 
        #[arg(allow_hyphen_values = true)]
        args: Vec<String> 
    },
    /// 🚀 Native execution via 'cargo -Z script'
    Csrun { path: PathBuf },
    /// 🔨 Build a universal .wasm or .wat file
    Build { 
        path: PathBuf, 
        target: BuildTarget 
    },
    /// 🔄 Convert between .wasm and .wat
    Convert { input: PathBuf, #[arg(short, long)] output: Option<PathBuf> },
    /// ✨ Init a new script template
    Init { name: String },
    /// 📦 Init a new library module template
    InitMod { name: String },
    /// 📂 Create 'rsc' folder for local files
    Resource { path: Option<PathBuf> },
    /// ⏱️ Benchmark execution speed
    Bench { 
        path: PathBuf, 
        #[arg(short, long, default_value = "5", value_parser = clap::value_parser!(u32).range(1..))] iterations: u32
    },
    /// ➕ Add a dependency (Auto-injects manifest if missing)
    Add { path: PathBuf, crate_name: String, version: Option<String> },
    /// ➖ Remove a dependency
    Remove { path: PathBuf, crate_name: String },
    /// 📦 List script dependencies
    List { path: PathBuf },
    /// ✨ Format Rust code (Preflight check for manifests)
    Fmt { path: PathBuf },
    /// 🧹 Optimize WASM via Walrus
    Optimize { #[arg(short, long)] input: PathBuf, #[arg(short, long)] output: Option<PathBuf> },
    /// 🗑️ Wipe the .tk build cache
    Clean,
    /// 📝 View WASM metadata (wasmtk style)
    Info { path: PathBuf },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[value(rename_all = "lowercase")]
enum BuildTarget { Wasi, Wasm, Wat }

struct MyState { wasi: WasiP1Ctx, budget: GuestBudget }

/// One budget for EVERYTHING a guest grows: all its memories and all its tables, together
/// (review N1: per-object caps let a module declaring 60 memories and 60 tables take 5.6 GB under a
/// 64 MiB setting). A growth that would pass the budget is refused, so `memory.grow` / `table.grow`
/// return -1 and an oversized declaration fails to instantiate.
struct GuestBudget { limit: usize, used: usize }

/// What one table element costs the host: a funcref / externref slot is one pointer.
const TABLE_ELEMENT_BYTES: usize = std::mem::size_of::<usize>();

impl GuestBudget {
    fn take(&mut self, bytes: usize) -> bool {
        match self.used.checked_add(bytes) {
            Some(total) if total <= self.limit => { self.used = total; true }
            _ => false,
        }
    }
}

impl ResourceLimiter for GuestBudget {
    fn memory_growing(&mut self, current: usize, desired: usize, _max: Option<usize>) -> wasmtime::Result<bool> {
        Ok(self.take(desired.saturating_sub(current)))
    }
    fn table_growing(&mut self, current: usize, desired: usize, _max: Option<usize>) -> wasmtime::Result<bool> {
        Ok(self.take(desired.saturating_sub(current).saturating_mul(TABLE_ELEMENT_BYTES)))
    }
}

/// A linker with WASI preview 1, and a store whose guest sees the module's folder as `.` —
/// READ-ONLY unless `--allow-write` (review S1: a module saved into a shared folder could rewrite
/// every file beside it) — plus stdio and arguments, never the environment, with its memories
/// capped by `sandbox`.
fn new_store(engine: &Engine, module_path: &Path, sandbox: Sandbox) -> Result<(Linker<MyState>, Store<MyState>)> {
    let mut linker: Linker<MyState> = Linker::new(engine);
    p1::add_to_linker_sync(&mut linker, |state| &mut state.wasi)?;
    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio().inherit_args();
    let perms = if sandbox.allow_write { FsPerms::ReadWrite } else { FsPerms::ReadOnly };
    builder.preopened_dir(module_path.parent().unwrap(), ".", perms)?;
    let budget = GuestBudget { limit: sandbox.max_memory_bytes, used: 0 };
    let mut store = Store::new(engine, MyState { wasi: builder.build_p1(), budget });
    store.limiter(|state| &mut state.budget);
    Ok((linker, store))
}

fn create_engine() -> Result<Engine> {
    let mut config = Config::new();
    config.cranelift_opt_level(OptLevel::Speed);
    config.wasm_simd(true);
    config.wasm_bulk_memory(true);
    config.wasm_multi_memory(true);
    Ok(Engine::new(&config)?)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let engine = create_engine()?;
    let sandbox = Sandbox {
        allow_write: cli.allow_write,
        max_memory_bytes: usize::try_from(cli.max_memory.saturating_mul(1024 * 1024)).unwrap_or(usize::MAX),
    };

    match cli.command {
        Commands::Run { path } => run_wasm(&engine, &path, sandbox),
        Commands::Mod { path, args } => run_wasm_mod(&engine, &path, args, sandbox),
        Commands::Csrun { path } => run_cargo_script(&path),
        Commands::Build { path, target } => {
            match target {
                BuildTarget::Wasi | BuildTarget::Wasm => build_with_virtual_cargo(&engine, &path).map(|_| ()),
                BuildTarget::Wat => {
                    let wasm = build_with_virtual_cargo(&engine, &path)?;
                    convert_wasm_wat(&wasm, None)
                },
            }
        },
        Commands::Convert { input, output } => convert_wasm_wat(&input, output),
        Commands::Init { name } => init_script(&name, false),
        Commands::InitMod { name } => init_script(&name, true),
        Commands::Resource { path } => create_resource_dir(path.as_deref()),
        Commands::Bench { path, iterations } => benchmark_wasm(&engine, &path, iterations, sandbox),
        Commands::Add { path, crate_name, version } => {
            let ver = match version { Some(v) => v, None => get_latest_version(&crate_name)? };
            edit_deps(&path, &crate_name, Some(&ver))
        },
        Commands::Remove { path, crate_name } => edit_deps(&path, &crate_name, None),
        Commands::List { path } => list_deps(&path),
        Commands::Fmt { path } => format_script(&path),
        Commands::Optimize { input, output } => optimize_wasm(&engine, &input, output),
        Commands::Clean => clean_cache(),
        Commands::Info { path } => print_info(&engine, &path),
    }
}

// --- WASM Loading & Execution ---

fn run_wasm(engine: &Engine, path: &Path, sandbox: Sandbox) -> Result<()> {
    let abs_path = fs::canonicalize(path)?;
    let module = load_module(engine, &abs_path)?;
    execute_module(engine, module, &abs_path, sandbox)
}

fn run_wasm_mod(engine: &Engine, path: &Path, args: Vec<String>, sandbox: Sandbox) -> Result<()> {
    let abs_path = fs::canonicalize(path)?;
    let module = load_module(engine, &abs_path)?;

    let (linker, mut store) = new_store(engine, &abs_path, sandbox)?;
    let instance = linker.instantiate(&mut store, &module)?;

    if args.is_empty() {
        print_info(engine, &abs_path)?;
        println!("💡 Tip: Use 'rsxtk mod <path> <function> [args]' to execute.");
        return Ok(());
    }

    let func_name = &args[0];
    let func = instance.get_func(&mut store, func_name)
        .ok_or_else(|| anyhow!("Func '{}' not found", func_name))?;
    
    let func_type = func.ty(&store);
    let param_tys = func_type.params();
    
    let mut wasm_params = Vec::new();
    for (i, ty) in param_tys.enumerate() {
        if i + 1 >= args.len() { bail!("Missing argument for parameter {}", i); }
        let val = &args[i + 1];
        wasm_params.push(match ty {
            wasmtime::ValType::I32 => Val::I32(val.parse()?),
            wasmtime::ValType::I64 => Val::I64(val.parse()?),
            wasmtime::ValType::F32 => Val::F32(val.parse::<f32>()?.to_bits()),
            wasmtime::ValType::F64 => Val::F64(val.parse::<f64>()?.to_bits()),
            _ => bail!("Unsupported parameter type"),
        });
    }
    // Review H13: extra arguments were silently dropped.
    if args.len() - 1 > wasm_params.len() {
        bail!("❌ '{}' takes {} argument(s); {} were given.", func_name, wasm_params.len(), args.len() - 1);
    }

    let mut results = vec![Val::I32(0); func_type.results().len()];
    match func.call(&mut store, &wasm_params, &mut results) {
        Ok(()) => print_val_results(&results),
        // A guest that calls proc_exit (review 2, D3: `mod … _start` with exit 0 failed with a backtrace).
        Err(e) => guest_exit(e)?,
    }
    Ok(())
}

/// What a guest's trap means for rsxtk, the same in `run` and `mod`: `proc_exit(0)` is a clean
/// finish; `proc_exit(N)` is the error "Exited status: N" (rsxtk exits 1, as 1.0.0 does — exit
/// codes are part of the promised CLI); anything else is an error.
fn guest_exit(e: wasmtime::Error) -> Result<()> {
    match e.downcast::<wasmtime_wasi::I32Exit>() {
        Ok(status) if status.0 == 0 => Ok(()),
        Ok(status) => bail!("Exited status: {}", status.0),
        Err(e) => Err(e.into()),
    }
}

fn execute_module(engine: &Engine, module: Module, abs_path: &Path, sandbox: Sandbox) -> Result<()> {
    let (linker, mut store) = new_store(engine, abs_path, sandbox)?;
    let instance = linker.instantiate(&mut store, &module)?;

    if let Some(func) = instance.get_func(&mut store, "_start") {
        if let Err(e) = func.call(&mut store, &[], &mut []) { guest_exit(e)?; }
    } else {
        println!("💡 Library module loaded.");
        print_info(engine, abs_path)?;
    }
    Ok(())
}

fn load_module(engine: &Engine, path: &Path) -> Result<Module> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "rs" => {
            // The full path, so `info x.rs` and `run x.rs` key one cache (review 2: a relative path
            // made them rebuild each other).
            let path = &fs::canonicalize(path)?;
            let (artifact, _) = build_and_cache_script(engine, path)?;
            match Module::new(engine, &fs::read(&artifact)?) {
                Ok(module) => Ok(module),
                // A damaged cache (review H10: truncated by a crash or a full disk) is dropped and
                // the script built once more; a second failure is reported.
                Err(_) => {
                    let _ = fs::remove_file(artifact.with_file_name("source.id"));
                    let (artifact, _) = build_and_cache_script(engine, path)?;
                    Ok(Module::new(engine, &fs::read(artifact)?)?)
                }
            }
        },
        "wat" => {
            let bytes = fs::read(path)?;
            Ok(Module::new(engine, &wat::parse_bytes(without_bom(&bytes))?)?)
        },
        "wasm" => Ok(Module::new(engine, &fs::read(path)?)?),
        // A .cwasm is precompiled native code that the engine would run without validating it,
        // outside the sandbox. rsxtk never loads one (owner, 2026-11-19).
        "cwasm" => bail!(
            "❌ '{}' is a precompiled .cwasm: native code that cannot be checked, so rsxtk does not run it.\n\
            💡 Run the .wasm, .wat or .rs it was built from instead.",
            path.display()
        ),
        _ => bail!("Unsupported file type: {}", ext),
    }
}

// --- Build & Caching ---

/// Builds a `.rs` script and returns `(optimized, wasm)` paths: the walrus-optimized module that
/// is run (cached as `.tk/<stem>/<stem>.wasm`) and Cargo's own output. A script without `fn main`
/// is a library module (`initmod`) and is mirrored as a `cdylib`; otherwise it is a binary.
/// A `build.rs` and an `rsc/` folder beside the script are mirrored with it. The cache is plain
/// wasm, validated before it is written and again by `Module::new` on every run — never a `.cwasm`.
fn build_and_cache_script(engine: &Engine, script_path: &Path) -> Result<(PathBuf, PathBuf)> {
    let stem = script_stem(script_path)?;
    check_script_name(stem)?;
    let cache_root = std::env::current_dir()?.join(".tk").join(stem);
    // One build of this cache at a time (review 2, D1: parallel runs of one script raced on the
    // mirror and failed with "os error 32"); released when this function returns, before the guest runs.
    let _build_lock = lock_cache(&cache_root, stem)?;
    let cached_path = cache_root.join(format!("{}.wasm", stem));

    let content = fs::read_to_string(script_path)?;
    let Some((manifest, code)) = split_manifest(&content)? else {
        bail!(
            "❌ Missing script manifest in '{}'.\n\n\
            WASI scripts require a header block for dependencies.\n\
            💡 Fix: Run 'rsxtk add {} <crate>' or 'rsxtk fmt {}' to inject it.",
            script_path.display(),
            stem_file_name(script_path),
            stem_file_name(script_path)
        );
    };
    let code = code.trim_start();
    let is_lib = !has_main_fn(code);
    // Cargo names a library's output after the crate name, with '-' turned into '_'.
    let artifact = if is_lib { stem.replace('-', "_") } else { stem.to_string() };
    let wasm_output = cache_root.join("t/wasm32-wasip1/release").join(format!("{}.wasm", artifact));

    let script_dir = script_path.parent().unwrap_or(Path::new("."));
    let build_rs = script_dir.join("build.rs");
    // A script that is itself named build.rs is not also its own build script.
    let has_build_rs = build_rs.is_file() && !script_path.file_name().is_some_and(|n| n == "build.rs");
    let rsc_dir = script_dir.join("rsc");
    let mirror_build_rs = cache_root.join("build.rs");
    let mirror_rsc = cache_root.join("src/rsc");

    // The cache is reused only for the exact sources it was built from: the same script (by path
    // AND content), the same build.rs and the same rsc/ files. Timestamps are not trusted.
    let fingerprint = source_fingerprint(script_path, has_build_rs.then_some(build_rs.as_path()), &rsc_dir)?;
    let id_path = cache_root.join("source.id");
    if cached_path.exists() && wasm_output.exists()
        && fs::read_to_string(&id_path).is_ok_and(|id| id == fingerprint)
    {
        return Ok((cached_path, wasm_output));
    }
    // A build that fails part-way must not leave the old fingerprint pointing at new sources.
    let _ = fs::remove_file(&id_path);

    fs::create_dir_all(cache_root.join("src"))?;
    let (src_file, stale_file, lib_section) = if is_lib {
        ("src/lib.rs", "src/main.rs", "[lib]\ncrate-type = [\"cdylib\"]\n\n")
    } else {
        ("src/main.rs", "src/lib.rs", "")
    };
    // A leftover file from the other kind would be built too, so remove it.
    let _ = fs::remove_file(cache_root.join(stale_file));
    // The mirror is written like every other output: never through a link planted in `.tk/`.
    write_atomic(&cache_root.join("Cargo.toml"), format!("[package]\nname=\"{}\"\nversion=\"0.1.0\"\nedition=\"2021\"\n\n{}{}", stem, lib_section, manifest.trim()).as_bytes())?;
    write_atomic(&cache_root.join(src_file), format!("\n{}", code).as_bytes())?;

    // Cargo runs build.rs from the package root. rsc/ goes under src/, so include_str!("rsc/…")
    // resolves as it does beside the script (and under `csrun`). Stale copies are removed.
    if has_build_rs { write_atomic(&mirror_build_rs, &fs::read(&build_rs)?)?; }
    else { let _ = fs::remove_file(&mirror_build_rs); }
    if rsc_dir.is_dir() { copy_dir_fresh(&rsc_dir, &mirror_rsc)?; }
    else if mirror_rsc.exists() { fs::remove_dir_all(&mirror_rsc)?; }

    let status = std::process::Command::new("cargo")
        .env("CARGO_TARGET_DIR", cache_root.join("t"))
        .arg("build").arg("--manifest-path").arg(cache_root.join("Cargo.toml"))
        .args(["--target", "wasm32-wasip1", "--release"])
        .status()?;

    if !status.success() { bail!("Build failed."); }

    let mut module = walrus::Module::from_buffer(&fs::read(&wasm_output)?)?;
    let optimized_wasm = module.emit_wasm();

    Module::validate(engine, &optimized_wasm)
        .map_err(|e| anyhow!("The optimized module of '{}' does not validate: {}", script_path.display(), e))?;
    // Module first, then its fingerprint: an interrupted run leaves no fingerprint, so the next
    // run rebuilds instead of trusting a half-written module.
    write_atomic(&cached_path, &optimized_wasm)?;
    write_atomic(&id_path, fingerprint.as_bytes())?;
    // Up to 0.5.2 the cache was a precompiled .cwasm plus a machine id; neither is read any more.
    let _ = fs::remove_file(cache_root.join(format!("{}.cwasm", stem)));
    let _ = fs::remove_file(cache_root.join("machine.id"));

    Ok((cached_path, wasm_output))
}

/// The script's file stem, which names its Cargo package and its `.tk/` folder. A name that is not
/// valid UTF-8 is an error, not a panic (review 2, U2).
fn script_stem(script_path: &Path) -> Result<&str> {
    script_path.file_stem().and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("❌ '{}': a script's file name must be valid UTF-8.", script_path.display()))
}

/// Text bytes without a leading UTF-8 byte-order mark, which Windows editors write (review 2: a
/// `.wat` with one was refused while a `.rs` script with one was accepted).
fn without_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes)
}

/// The file name, for messages.
fn stem_file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

/// A script is built as a Cargo package named after its file, so its name must be one Cargo
/// builds (review 2, C2: `init 2fib.rs` succeeded and `run` then failed with a raw Cargo error): a
/// letter or `_` first, then letters, digits, `-` or `_`, and not one of the names Cargo keeps for
/// its build folders. Measured 2026-11-20 with `cargo build` of 2021-edition packages: those four
/// names and `2fib` / `my.script` fail; Rust keywords (`fn`, `self`, `try`, `gen`, …) BUILD, so they
/// are allowed — refusing them would break names that work.
fn check_script_name(stem: &str) -> Result<()> {
    const CARGO_RESERVED: &[&str] = &["build", "deps", "examples", "incremental"];
    // [win-x64, win-arm64] Only `nul` is refused: measured on 1.0.0, `run nul.rs` failed with "os
    // error 3", while con, prn, aux, com1-9 and lpt1-9 ran (with a Cargo warning). Refusing those
    // would break scripts that work under 1.0.0 (re-review 3).
    const WINDOWS_DEVICES: &[&str] = &["nul"];
    // Compared without case: on Windows and macOS `Build.rs` IS `build.rs` (re-review 2: `init
    // Build` passed and the file was then taken as a build script).
    let is = |list: &[&str]| list.iter().any(|r| r.eq_ignore_ascii_case(stem));
    let why = if !stem.starts_with(|c: char| c.is_alphabetic() || c == '_') {
        Some("it must start with a letter or '_'")
    } else if !stem.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        Some("it may hold only letters, digits, '-' and '_'")
    } else if is(CARGO_RESERVED) {
        Some("Cargo reserves it for its build folders")
    } else if cfg!(windows) && is(WINDOWS_DEVICES) {
        Some("Windows keeps it as a device name")
    } else {
        None
    };
    match why {
        Some(why) => bail!("❌ '{}' cannot be a script name: rsxtk builds the script as a Cargo package named after the file, and {}. Rename it (for example '{}.rs').", stem, why, suggest_script_name(stem)),
        None => Ok(()),
    }
}

/// A valid name close to `stem`, for the refusal's hint: other characters become `_`, and `my_`
/// goes in front (which also clears a reserved or device name).
fn suggest_script_name(stem: &str) -> String {
    let body: String = stem.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    format!("my_{}", body.trim_start_matches(['_', '-']))
}

/// A lock on one script's build cache for as long as the returned file lives: a second `run`,
/// `build` or `info` of the same script waits — saying so — instead of racing on the mirror. No
/// time limit: the holder is a Cargo build, which has none (owner, 2026-11-20).
///
/// The `.tk` folder is checked first (open items before 1.0.1): a link at any path rsxtk keeps in
/// it — `.tk/<stem>`, its `src`, `src/rsc`, `t`, the cached module, `source.id`, the lock file — is refused, since a build through it would
/// write (and later load a module from) wherever it points; and on Unix a `.tk` another user owns
/// is refused, so a shared folder's cache cannot be swapped under the user. `.tk` itself may be a
/// link (a cache on another drive), as long as on Unix the folder it names is the user's own.
fn lock_cache(cache_root: &Path, stem: &str) -> Result<fs::File> {
    let tk = cache_root.parent().ok_or_else(|| anyhow!("no .tk folder for '{}'", cache_root.display()))?;
    fs::create_dir_all(tk)?;
    check_owned(tk)?;
    let lock_path = tk.join(format!("{}.lock", stem));
    // Every path rsxtk keeps under `.tk/<stem>`: a link at Cargo's target folder `t` would have the
    // module built — and loaded — wherever it points (re-review 14).
    for path in [
        lock_path.clone(), cache_root.to_path_buf(), cache_root.join("src"), cache_root.join("src/rsc"),
        cache_root.join("t"), cache_root.join(format!("{}.wasm", stem)), cache_root.join("source.id"),
    ] { refuse_link(&path)?; }
    let file = fs::OpenOptions::new().create(true).truncate(false).write(true).open(&lock_path)
        .map_err(|e| anyhow!("❌ could not open the build lock '{}': {}", lock_path.display(), e))?;
    match file.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => {
            eprintln!("⏳ Another rsxtk run is building '{}'; waiting for it.", stem);
            file.lock().map_err(|e| anyhow!("❌ could not lock '{}': {}", lock_path.display(), e))?;
        }
        Err(fs::TryLockError::Error(e)) => bail!("❌ could not lock '{}': {}", lock_path.display(), e),
    }
    Ok(file)
}

/// An error when `path` is a symbolic link (on Windows also a junction); a missing path is fine.
fn refuse_link(path: &Path) -> Result<()> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        bail!("❌ '{}' is a link; rsxtk does not build through a link in .tk/, which could point anywhere. Remove it and run again.", path.display());
    }
    Ok(())
}

/// [linux-x64, linux-arm64, mac-x64, mac-arm64] An error when this user cannot change `dir`'s mode
/// — it belongs to someone else. Setting the mode it already has changes nothing and fails only
/// for a non-owner (root may change any). [win-x64, win-arm64] Not checked: a folder's access is
/// its ACL, which the folder's owner set.
fn check_owned(dir: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let perms = fs::metadata(dir)?.permissions();
        fs::set_permissions(dir, perms).map_err(|e| anyhow!(
            "❌ '{}' is not yours ({}); rsxtk will not build in another user's .tk folder. Remove it or run the script from a folder you own.", dir.display(), e))?;
    }
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

/// The `---`-fenced manifest and the code after it. A UTF-8 byte-order mark and blank lines may
/// come before the opening fence; each fence is a line of exactly `---`. `Ok(None)` when the file
/// has no manifest; an error when the closing fence is missing, or when other text comes before a
/// `---` line (a manifest must start the file — injecting a second one would corrupt it).
fn split_manifest(content: &str) -> Result<Option<(String, String)>> {
    let body = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut offset = 0;
    let mut open = None;
    for line in body.split_inclusive('\n') {
        let fence = line.trim() == "---";
        match open {
            None if line.trim().is_empty() => {}
            None if fence => open = Some(offset + line.len()),
            None => break,
            Some(start) if fence => {
                return Ok(Some((body[start..offset].to_string(), body[offset + line.len()..].to_string())));
            }
            Some(_) => {}
        }
        offset += line.len();
    }
    if open.is_some() { bail!("❌ Invalid format: Manifest block is incomplete (no closing '---' line)."); }
    if body.lines().any(|l| l.trim() == "---") {
        bail!("❌ Text comes before the '---' manifest block; the manifest must start the file.");
    }
    Ok(None)
}

/// Rewrites the user's own file in place (`add`, `remove`, `fmt`, `optimize` without `-o`): a
/// symlinked file is written THROUGH to its target, so the link stays a link (review 2, U1). Only
/// for a file the user named to be edited — every other output goes through `write_atomic`, which
/// never follows a link.
fn rewrite_script(path: &Path, bytes: &[u8]) -> Result<()> {
    let target = match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => fs::canonicalize(path)
            .map_err(|e| anyhow!("could not follow the link '{}': {}", path.display(), e))?,
        _ => path.to_path_buf(),
    };
    write_atomic(&target, bytes)
}

/// Writes `bytes` to `path` through a temporary file beside it and a rename, so an interrupted or
/// failed write never leaves a half-written file, and readers see the old or the new file whole.
/// An existing file keeps its permissions (an executable script stays executable; review 2, U1).
/// A symlink AT `path` is replaced, never followed: a link planted at an output name (the `.tk`
/// cache, `build`'s or `convert`'s output) cannot redirect the write elsewhere (re-review 2).
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let target = path.to_path_buf();
    let name = target.file_name().ok_or_else(|| anyhow!("'{}' has no file name", target.display()))?;
    // Permissions of an existing REGULAR file only (symlink_metadata: a link's target is not read).
    let perms = fs::symlink_metadata(&target).ok().filter(|m| m.is_file()).map(|m| m.permissions());
    // The temporary file is created EXCLUSIVELY (`create_new`: an existing name — a planted
    // symlink included — is refused, never followed) under a name that is unique per attempt
    // (re-review 3: `fs::write` to a pid-only name followed a symlink planted at that name).
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let (mut file, tmp) = (0..32u32)
        .find_map(|attempt| {
            let tmp = target.with_file_name(format!(".{}.rsxtk-{}-{:x}-{}.tmp", name.to_string_lossy(), std::process::id(), stamp, attempt));
            match fs::OpenOptions::new().write(true).create_new(true).open(&tmp) {
                Ok(f) => Some(Ok((f, tmp))),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => None,
                Err(e) => Some(Err(e)),
            }
        })
        .unwrap_or_else(|| Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "no free temporary name")))
        .map_err(|e| anyhow!("could not write '{}': {}", target.display(), e))?;
    // Bytes and permissions go through the open handle, never through the path again.
    let written = file.write_all(bytes)
        .and_then(|_| match perms { Some(p) => file.set_permissions(p), None => Ok(()) });
    // Closed on every path, so a failed write's temp can be removed on Windows too (re-review 4).
    drop(file);
    let written = written.and_then(|_| fs::rename(&tmp, &target));
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(anyhow!("could not write '{}': {}", target.display(), e));
    }
    Ok(())
}

/// True when the code defines `fn main` (followed by optional whitespace and `(`) in CODE — not in
/// a comment or a string (review 2, C1: a library whose doc comment said "`fn main()`" was built as
/// a binary and failed with E0601).
fn has_main_fn(code: &str) -> bool {
    let code = rust_code_only(code);
    code.match_indices("fn main").any(|(i, m)| {
        let starts_token = code[..i].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        starts_token && code[i + m.len()..].trim_start().starts_with('(')
    })
}

/// The code with every comment and string or char literal replaced by spaces (line breaks kept),
/// so a search sees only Rust tokens: `//` and nested `/* */` comments; `"…"` and `b"…"` with
/// escapes; raw strings `r"…"`, `r#"…"#`, `br#"…"#`; char literals `'x'`, `'\n'` (a lifetime such as
/// `'a` is code and is kept).
fn rust_code_only(code: &str) -> String {
    let c: Vec<char> = code.chars().collect();
    let mut out = String::with_capacity(code.len());
    let blank = |out: &mut String, ch: char| out.push(if ch == '\n' { '\n' } else { ' ' });
    let ident = |ch: char| ch.is_alphanumeric() || ch == '_';
    let mut i = 0;
    while i < c.len() {
        let next = c.get(i + 1).copied();
        if c[i] == '/' && next == Some('/') {
            while i < c.len() && c[i] != '\n' { blank(&mut out, c[i]); i += 1; }
        } else if c[i] == '/' && next == Some('*') {
            let mut depth = 0;
            while i < c.len() {
                if c[i] == '/' && c.get(i + 1) == Some(&'*') { depth += 1; blank(&mut out, c[i]); blank(&mut out, c[i + 1]); i += 2; }
                else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                    depth -= 1; blank(&mut out, c[i]); blank(&mut out, c[i + 1]); i += 2;
                    if depth == 0 { break; }
                } else { blank(&mut out, c[i]); i += 1; }
            }
        } else if (c[i] == 'r' || (c[i] == 'b' && next == Some('r')))
            && (i == 0 || !ident(c[i - 1]))
            && { let s = if c[i] == 'b' { i + 2 } else { i + 1 }; let h = c[s..].iter().take_while(|&&x| x == '#').count(); c.get(s + h) == Some(&'"') }
        {
            // A raw string: r, then n hashes, a quote … a quote and n hashes.
            let s = if c[i] == 'b' { i + 2 } else { i + 1 };
            let hashes = c[s..].iter().take_while(|&&x| x == '#').count();
            let mut j = s + hashes + 1;
            while j < c.len() && !(c[j] == '"' && c[j + 1..].iter().take(hashes).filter(|&&x| x == '#').count() == hashes) { j += 1; }
            let end = (j + 1 + hashes).min(c.len());
            while i < end { blank(&mut out, c[i]); i += 1; }
        } else if c[i] == '"' || (c[i] == 'b' && next == Some('"') && (i == 0 || !ident(c[i - 1]))) {
            if c[i] == 'b' { blank(&mut out, c[i]); i += 1; }
            blank(&mut out, c[i]); i += 1;
            while i < c.len() && c[i] != '"' {
                if c[i] == '\\' && i + 1 < c.len() { blank(&mut out, c[i]); i += 1; }
                blank(&mut out, c[i]); i += 1;
            }
            if i < c.len() { blank(&mut out, c[i]); i += 1; }
        } else if c[i] == '\'' && (next == Some('\\') || c.get(i + 2) == Some(&'\'')) {
            // A char literal ('x' or an escape); a lifetime ('a without a closing quote) is code.
            blank(&mut out, c[i]); i += 1;
            while i < c.len() && c[i] != '\'' {
                if c[i] == '\\' && i + 1 < c.len() { blank(&mut out, c[i]); i += 1; }
                blank(&mut out, c[i]); i += 1;
            }
            if i < c.len() { blank(&mut out, c[i]); i += 1; }
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

/// What a cached module was built from: the script's path and bytes, the bytes of the `build.rs`
/// mirrored with it (or its absence), and every file under `rsc/` by relative path and bytes. Two
/// scripts with one name, or an edit that keeps an older timestamp, give different values.
/// `DefaultHasher::new()` uses fixed keys, so the value is stable for one rsxtk binary; a new
/// binary may compute another value, which only costs one rebuild.
fn source_fingerprint(script_path: &Path, build_rs: Option<&Path>, rsc_dir: &Path) -> Result<String> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    script_path.hash(&mut h);
    fs::read(script_path)?.hash(&mut h);
    match build_rs {
        Some(b) => { 1u8.hash(&mut h); fs::read(b)?.hash(&mut h); }
        None => 0u8.hash(&mut h),
    }
    hash_tree(rsc_dir, Path::new(""), &mut h)?;
    Ok(format!("{:016x}", h.finish()))
}

/// Feeds every file under `dir` (sorted, by path relative to the root, then its bytes) to `h`;
/// a missing `dir` counts as empty but distinct from an empty folder.
fn hash_tree(dir: &Path, rel: &Path, h: &mut impl std::hash::Hasher) -> Result<()> {
    use std::hash::Hash;
    if !dir.is_dir() { 0u8.hash(h); return Ok(()); }
    1u8.hash(h);
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<std::io::Result<_>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = rel.join(entry.file_name());
        name.hash(h);
        if entry.file_type()?.is_dir() { hash_tree(&entry.path(), &name, h)?; }
        else { fs::read(entry.path())?.hash(h); }
    }
    Ok(())
}

/// Copies `src` into `dst` recursively, replacing whatever `dst` held.
fn copy_dir_fresh(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() { fs::remove_dir_all(dst)?; }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() { copy_dir_fresh(&entry.path(), &target)?; }
        else { write_atomic(&target, &fs::read(entry.path())?)?; }
    }
    Ok(())
}

// --- Utilities & Helpers ---

fn init_script(name: &str, is_mod: bool) -> Result<()> {
    let filename = if name.ends_with(".rs") { name.to_string() } else { format!("{}.rs", name) };
    // Only a name `run` can build (review 2, C2).
    check_script_name(script_stem(Path::new(&filename))?)?;
    let content = if is_mod {
        "---\n[dependencies]\n---\n#[no_mangle]\npub extern \"C\" fn add(a: i32, b: i32) -> i32 {\n    a + b\n}"
    } else {
        "---\n[dependencies]\n---\nfn main() {\n    println!(\"Hello from rsxtk!\");\n}"
    };
    // create_new: an existing file is never replaced (review H4).
    let mut file = match fs::OpenOptions::new().write(true).create_new(true).open(&filename) {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            bail!("❌ '{}' already exists; rsxtk {} does not overwrite it.", filename, if is_mod { "initmod" } else { "init" })
        }
        other => other?,
    };
    std::io::Write::write_all(&mut file, content.as_bytes())?;
    println!("✨ Initialized {}", filename);
    Ok(())
}

fn print_val_results(results: &[Val]) {
    for res in results {
        match res {
            Val::I32(v) => println!("{}", v),
            Val::I64(v) => println!("{}", v),
            Val::F32(v) => println!("{}", f32::from_bits(*v)),
            Val::F64(v) => println!("{}", f64::from_bits(*v)),
            _ => println!("{:?}", res),
        }
    }
}

fn print_info(engine: &Engine, path: &Path) -> Result<()> {
    let module = load_module(engine, path)?;
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
    
    println!("\n📄 Module Info: {}", filename);
    println!("────────────────────────────────────────");

    println!("🚀 User Callable Functions:");
    let mut count = 0;
    for export in module.exports() {
        if let Some(func_type) = export.ty().func() {
            count += 1;
            let params: Vec<String> = func_type.params().map(|t| t.to_string()).collect();
            let results: Vec<String> = func_type.results().map(|t| t.to_string()).collect();
            let res_str = if results.is_empty() { String::new() } else { format!(" -> {}", results.join(", ")) };
            println!("  - {}({}){}", export.name(), params.join(", "), res_str);
        }
    }
    if count == 0 { println!("  (None)"); }

    let has_wasi = module.imports().any(|i| i.module() == "wasi_snapshot_preview1" || i.module().starts_with("wasi:"));
    println!("\n🛠️  WASI Support: {}", if has_wasi { "Yes" } else { "No" });
    println!("────────────────────────────────────────\n");

    Ok(())
}

fn build_with_virtual_cargo(engine: &Engine, path: &Path) -> Result<PathBuf> {
    let abs_path = fs::canonicalize(path)?;
    let (_, wasm_path) = build_and_cache_script(engine, &abs_path)?;
    let local_wasm = abs_path.with_extension("wasm");
    write_atomic(&local_wasm, &fs::read(wasm_path)?)?;
    println!("🔨 Built: {}", local_wasm.display());
    Ok(local_wasm)
}

fn convert_wasm_wat(input: &Path, output: Option<PathBuf>) -> Result<()> {
    let data = fs::read(input)?;
    let is_wasm = input.extension().is_some_and(|e| e == "wasm");
    let out = output.unwrap_or_else(|| input.with_extension(if is_wasm { "wat" } else { "wasm" }));
    // Review H9: `-o` naming the input replaced it with the other format.
    if same_file(input, &out) {
        bail!("❌ The output '{}' is the input file; choose another name with -o.", out.display());
    }
    let bytes = if is_wasm { wasmprinter::print_bytes(&data)?.into_bytes() } else { wat::parse_bytes(without_bom(&data))?.into_owned() };
    write_atomic(&out, &bytes)?;
    println!("🔄 Converted to: {}", out.display());
    Ok(())
}

fn run_cargo_script(path: &Path) -> Result<()> {
    let status = std::process::Command::new("cargo").arg("-Z").arg("script").arg(fs::canonicalize(path)?).status()?;
    if !status.success() { bail!("Cargo script failed."); }
    Ok(())
}

fn edit_deps(path: &Path, crate_name: &str, version: Option<&str>) -> Result<()> {
    check_crate_name(crate_name)?;
    if let Some(v) = version { check_version(v)?; }
    let _lock = lock_script(path)?;
    let content = fs::read_to_string(path)?;
    // No manifest yet: one is injected in front of the code (the BOM, if any, is dropped).
    let (manifest, code) = split_manifest(&content)?.unwrap_or_else(|| {
        (String::from("[dependencies]\n"), content.strip_prefix('\u{feff}').unwrap_or(&content).to_string())
    });
    let edited = edit_manifest(&manifest, crate_name, version)?;
    if version.is_none() && edited.trim() == manifest.trim() {
        println!("ℹ️ '{}' is not in the [dependencies] of {}; nothing changed.", crate_name, path.display());
        return Ok(());
    }
    rewrite_script(path, format!("---\n{}\n---\n{}", edited.trim(), code.trim_start()).as_bytes())?;
    println!("✅ Manifest updated in {}", path.display());
    Ok(())
}

/// An exclusive lock on one script for as long as the returned file lives, so parallel `add`,
/// `remove` and `fmt` runs on it queue instead of losing each other's edits (review N3). The lock
/// is an empty file `<lock folder>/<hash of the script's full path>.lock`; it is kept afterwards,
/// because deleting a lock file other processes may be waiting on races. A run that waits says so,
/// and gives up after `LOCK_WAIT` naming the lock file (review NEW-3: it waited forever, silently).
fn lock_script(path: &Path) -> Result<fs::File> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    fs::canonicalize(path)?.hash(&mut h);
    let dir = lock_dir();
    create_private_dir(&dir).map_err(|e| anyhow!("❌ could not create the lock folder '{}': {}", dir.display(), e))?;
    let lock_path = dir.join(format!("{:016x}.lock", h.finish()));
    let file = fs::OpenOptions::new().create(true).truncate(false).write(true).open(&lock_path)
        .map_err(|e| anyhow!("❌ could not open the lock file '{}': {}", lock_path.display(), e))?;
    let started = std::time::Instant::now();
    let mut told = false;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(fs::TryLockError::WouldBlock) => {
                if !told {
                    eprintln!("⏳ Another rsxtk run is editing {}; waiting (lock: {})", path.display(), lock_path.display());
                    told = true;
                }
                if started.elapsed() >= LOCK_WAIT {
                    bail!("❌ {} is still locked after {} s by another process (lock: {}); nothing changed.",
                        path.display(), LOCK_WAIT.as_secs(), lock_path.display());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(fs::TryLockError::Error(e)) => bail!("❌ could not lock '{}': {}", lock_path.display(), e),
        }
    }
}

/// How long `add` / `remove` / `fmt` wait for another run's lock on the same script.
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(60);

/// Where the lock files live: a folder only this user can use.
/// [linux-x64, linux-arm64] `/tmp` is shared by every user, so a lock folder there could be
/// pre-created or held by someone else (security re-review 2): the per-user cache folder is used
/// (`$XDG_CACHE_HOME/rsxtk/locks`, else `~/.cache/rsxtk/locks`, else `/tmp/rsxtk-locks-<uid>`).
/// [win-x64, win-arm64, mac-x64, mac-arm64] the temp folder is already per-user (`%TEMP%`, `$TMPDIR`).
fn lock_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let cache = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from).filter(|p| p.is_absolute()).map(|h| h.join(".cache")));
        if let Some(cache) = cache { return cache.join("rsxtk").join("locks"); }
        // No usable HOME: the shared temp folder, under a name of this user's own (its uid, the
        // owner of /proc/self), so one user's folder never blocks another's (open items before 1.0.1).
        use std::os::unix::fs::MetadataExt;
        if let Ok(me) = fs::metadata("/proc/self") {
            return std::env::temp_dir().join(format!("rsxtk-locks-{}", me.uid()));
        }
    }
    std::env::temp_dir().join("rsxtk-locks")
}

/// Creates `dir` (and its parents) if missing. [linux-x64, linux-arm64, mac-x64, mac-arm64] The
/// folder is then set to mode 0700 whether rsxtk made it or found it, so other local users cannot
/// list which scripts were edited when (privacy re-review 2). The linux-x64 run of 2026-11-20 found
/// a folder made by someone else (there: a test) kept at 0755; a folder another user owns cannot
/// be changed, which is an error, not a silent pass.
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// The manifest with `crate_name` removed from its `[dependencies]` table and, given a version,
/// added at the END OF THAT TABLE (a `[dependencies]` table is created first if there is none).
/// Every other table — `[build-dependencies]`, `[features]`, `[target.…]` — is kept as it is. An
/// entry spanning several lines (`x = { … \n … }`) is removed whole, and so is the table form
/// `[dependencies.<crate>]` with its lines (review B6: it was left in place, so `add` wrote a
/// duplicate key and `remove` removed nothing).
fn edit_manifest(manifest: &str, crate_name: &str, version: Option<&str>) -> Result<String> {
    // The scanner reads one line at a time, so a TOML multi-line string (which may hold `[`, `#`
    // or a line that looks like a table header) would be misread: refuse instead of guessing
    // (review NEW-2). Dependency manifests do not need them.
    if manifest.contains("\"\"\"") || manifest.contains("'''") {
        bail!("❌ The manifest uses a multi-line string (\"\"\" or '''); rsxtk add/remove cannot edit it safely — edit it by hand.");
    }
    let crate_table = format!("[dependencies.{}]", crate_name);
    let crate_subtable = format!("[dependencies.{}.", crate_name);
    let mut out: Vec<String> = Vec::new();
    let mut in_deps = false;
    let mut in_crate_table = false; // inside `[dependencies.<crate>]`: dropped
    let mut deps_end: Option<usize> = None;
    let mut open_brackets = 0i32; // still inside a removed multi-line entry
    for line in manifest.lines() {
        // The line without its comment (review N2: `[dependencies] # deps` was not recognised, and
        // a `[` inside a comment or a string threw the bracket count off and deleted later lines).
        let t = toml_code(line).trim();
        if open_brackets > 0 {
            // A table header before the removed entry closed: the entry never closes. Refuse rather
            // than drop every line up to the end (review NEW-1: `remove` ate the rest of the manifest).
            if is_table_header(t) {
                bail!("❌ The entry for '{}' opens a '{{' or '[' that never closes; edit the manifest by hand.", crate_name);
            }
            open_brackets += bracket_balance(t);
            continue;
        }
        if t.starts_with('[') { // a table header: `[x]` or `[[x]]`
            if in_deps { deps_end = Some(out.len()); }
            // `[dependencies.serde]`, `[ dependencies . "serde" ]`, `[dependencies.'serde']` name one
            // table: spaces and both kinds of TOML key quotes are ignored (review B8: `'` was not).
            let header: String = t.chars().filter(|c| !matches!(c, ' ' | '\t' | '"' | '\'')).collect();
            in_deps = header == "[dependencies]";
            if in_deps { deps_end = None; }
            // The crate's own table and its sub-tables (`[dependencies.serde.features]`, review B7).
            in_crate_table = !crate_name.is_empty() && (header == crate_table || header.starts_with(&crate_subtable));
            if !in_crate_table { out.push(line.to_string()); }
            continue;
        }
        if in_crate_table { continue; }
        if in_deps && dep_key(t) == Some(crate_name) {
            open_brackets = bracket_balance(t).max(0);
            continue;
        }
        out.push(line.to_string());
    }
    if open_brackets > 0 {
        bail!("❌ The entry for '{}' opens a '{{' or '[' that never closes; edit the manifest by hand.", crate_name);
    }
    if in_deps { deps_end = Some(out.len()); }
    if let Some(v) = version {
        let entry = format!("{} = \"{}\"", crate_name, v);
        match deps_end {
            Some(mut at) => {
                while at > 0 && out[at - 1].trim().is_empty() { at -= 1; }
                out.insert(at, entry);
            }
            None => { out.insert(0, entry); out.insert(0, "[dependencies]".to_string()); }
        }
    }
    // A `remove` never adds a `[dependencies]` table: removing an absent crate changes nothing
    // (review 2: it inserted an empty table and said "Manifest updated").
    Ok(out.join("\n"))
}

/// A TOML table header line (comment already removed): `[name]` or `[[name]]`, where the name is
/// keys and dots — not an inline array such as `["derive"]`.
fn is_table_header(code: &str) -> bool {
    let inner = code.trim_start_matches('[').trim_end_matches(']');
    code.starts_with('[') && code.ends_with(']') && !inner.is_empty()
        && inner.chars().all(|c| c.is_alphanumeric() || "_-. \"'".contains(c))
        && !inner.contains(", ")
}

/// Opening minus closing `{` / `[` brackets on a manifest line, outside strings and comments.
fn bracket_balance(line: &str) -> i32 {
    let mut balance = 0;
    toml_scan(line, |c| match c { '{' | '[' => balance += 1, '}' | ']' => balance -= 1, _ => {} });
    balance
}

/// The part of a manifest line before its `#` comment (a `#` inside a string does not count).
fn toml_code(line: &str) -> &str {
    let mut end = line.len();
    // The scan stops at the first `#` outside a string, so this sees at most one.
    toml_scan_with_index(line, |i, c| if c == '#' { end = i; });
    &line[..end]
}

/// Calls `f` for every character of `line` that is outside a TOML string, up to a comment.
fn toml_scan(line: &str, mut f: impl FnMut(char)) {
    toml_scan_with_index(line, |_, c| if c != '#' { f(c) });
}

/// Calls `f(byte index, char)` for every character outside a TOML string (basic `"…"` with
/// escapes, literal `'…'`), including the `#` that starts a comment, then stops there.
fn toml_scan_with_index(line: &str, mut f: impl FnMut(usize, char)) {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        match quote {
            Some(q) => {
                if escaped { escaped = false; }
                else if q == '"' && c == '\\' { escaped = true; }
                else if c == q { quote = None; }
            }
            None => {
                if c == '"' || c == '\'' { quote = Some(c); continue; }
                f(i, c);
                if c == '#' { return; }
            }
        }
    }
}

/// A crate name as crates.io allows it: an ASCII letter, then letters, digits, `-` or `_`, at most
/// 64 characters. Anything else would be written into the manifest and a URL unescaped.
fn check_crate_name(name: &str) -> Result<()> {
    let ok = name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok { bail!("❌ '{}' is not a valid crate name (letters, digits, '-' and '_', starting with a letter).", name); }
    Ok(())
}

/// A version requirement goes between quotes in the manifest, so it may not contain a quote, a
/// backslash or a line break.
fn check_version(version: &str) -> Result<()> {
    if !is_version_req(version) {
        bail!("❌ '{}' is not a valid version requirement (for example 1, 1.2, ^1.2.3, ~0.4, >=1, <2, *).", version.escape_debug());
    }
    Ok(())
}

/// A Cargo version requirement (review 2, C3: `add x banana` wrote `x = "banana"` and said
/// success), as Cargo's parser takes it (checked against `cargo metadata`, re-review 2): `*` / `x` /
/// `X` as the whole requirement, or comma-separated comparators — an optional `^ ~ = > >= < <=`, then
/// MAJOR[.MINOR[.PATCH]] with no leading zeros, each fitting a u64, where a wildcard (`*` / `x` /
/// `X`) may replace the parts after MAJOR but is never followed by a number, and an optional
/// `-pre` / `+build` only on a full MAJOR.MINOR.PATCH (a numeric `pre` identifier has no leading zero).
fn is_version_req(req: &str) -> bool {
    fn comparator(c: &str) -> bool {
        let c = c.trim();
        let c = [">=", "<=", "^", "~", "=", ">", "<"].iter()
            .find_map(|op| c.strip_prefix(op)).unwrap_or(c).trim_start();
        let (core, rest) = c.split_at(c.find(['-', '+']).unwrap_or(c.len()));
        let ident_ok = |p: &str| !p.is_empty() && p.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-');
        let no_leading_zero = |p: &str| !(p.len() > 1 && p.starts_with('0') && p.chars().all(|ch| ch.is_ascii_digit()));
        let pre_ok = |s: &str| !s.is_empty() && s.split('.').all(|p| ident_ok(p) && no_leading_zero(p));
        let build_ok = |s: &str| !s.is_empty() && s.split('.').all(ident_ok);
        let (pre, build) = match rest.split_once('+') { Some((p, b)) => (p, Some(b)), None => (rest, None) };
        let tail_ok = (pre.is_empty() || pre.strip_prefix('-').is_some_and(pre_ok)) && build.is_none_or(build_ok);
        let parts: Vec<&str> = core.split('.').collect();
        // Each number fits a u64, as in Cargo (re-review 3: `18446744073709551616` was taken).
        let number = |p: &str| !p.is_empty() && p.chars().all(|ch| ch.is_ascii_digit()) && no_leading_zero(p) && p.parse::<u64>().is_ok();
        let wild = |p: &str| matches!(p, "*" | "x" | "X");
        let first_wild = parts.iter().position(|p| wild(p)).unwrap_or(parts.len());
        (1..=3).contains(&parts.len())
            && number(parts[0])
            && parts[1..first_wild].iter().all(|p| number(p))
            && parts[first_wild..].iter().all(|p| wild(p))
            && tail_ok
            // A `-pre` / `+build` tail needs the full MAJOR.MINOR.PATCH (re-review 3: `1-rc` was taken).
            && (rest.is_empty() || (parts.len() == 3 && first_wild == parts.len()))
    }
    !req.trim().is_empty() && !req.chars().any(|c| c == '"' || c == '\\' || c.is_control())
        // A bare wildcard only as the whole requirement: Cargo refuses `>=1,*` (re-review 4).
        && (matches!(req.trim(), "*" | "x" | "X") || req.split(',').all(comparator))
}

/// The crate a manifest line declares (`name = …`, `name.workspace = …`, `"name" = …`), if any.
/// Exact, so `serde` does not match `serde_json`.
fn dep_key(line: &str) -> Option<&str> {
    if !line.contains('=') { return None; }
    // Both TOML key quotes: `"serde" = …` and `'serde' = …` (review B9: `'` was not trimmed).
    let key = line.split(['=', '.']).next()?.trim().trim_matches(['"', '\'']);
    (!key.is_empty()).then_some(key)
}

/// Identifies rsxtk to crates.io, as its crawler policy asks.
const CRATES_IO_USER_AGENT: &str =
    concat!("rsxtk/", env!("CARGO_PKG_VERSION"), " (+https://github.com/synapticforge1/rsxtk)");

/// The newest version of `crate_name` on crates.io. A crate crates.io does not know is an error.
/// When crates.io cannot be reached or answers oddly, `*` is written (as the README documents) with
/// a warning, so the user knows to pin a version later.
fn get_latest_version(crate_name: &str) -> Result<String> {
    check_crate_name(crate_name)?;
    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let response = reqwest::blocking::Client::builder()
        .user_agent(CRATES_IO_USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()?
        .get(&url)
        .send();
    let why = match response {
        Ok(res) if res.status() == reqwest::StatusCode::NOT_FOUND => {
            bail!("❌ crates.io has no crate named '{}'.", crate_name)
        }
        Ok(res) if res.status().is_success() => {
            match res.json::<Value>().ok().and_then(|j| j["crate"]["max_version"].as_str().map(str::to_string)) {
                Some(v) => return Ok(v),
                None => "its answer had no version".to_string(),
            }
        }
        Ok(res) => format!("it answered {}", res.status()),
        Err(e) => format!("it could not be reached: {}", e),
    };
    eprintln!("⚠️ crates.io lookup for '{}' failed ({}); writing \"*\" — pin a version later.", crate_name, why);
    Ok("*".to_string())
}

fn create_resource_dir(path: Option<&Path>) -> Result<()> {
    let base = path.unwrap_or(Path::new("."));
    fs::create_dir_all(base.join("rsc"))?;
    println!("📂 Created rsc/ folder.");
    Ok(())
}

fn clean_cache() -> Result<()> {
    let cache = std::env::current_dir()?.join(".tk");
    if cache.exists() { fs::remove_dir_all(cache)?; println!("🗑️ Cache cleared."); }
    Ok(())
}

fn list_deps(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)?;
    match split_manifest(&content)? {
        Some((manifest, _)) => println!("📦 Dependencies:\n{}", manifest.trim()),
        None => println!("⚠️ No manifest found in this script."),
    }
    Ok(())
}

fn format_script(path: &Path) -> Result<()> {
    let _lock = lock_script(path)?;
    let content = fs::read_to_string(path)?;
    let (manifest, code) = match split_manifest(&content)? {
        Some(parts) => parts,
        None => {
            println!("✨ No manifest detected. Injecting '---' header...");
            ("[dependencies]".to_string(), content.strip_prefix('\u{feff}').unwrap_or(&content).to_string())
        }
    };
    let formatted = rustfmt(&code)
        .map_err(|e| anyhow!("{}\n'{}' is unchanged.", e, path.display()))?;
    rewrite_script(path, format!("---\n{}\n---\n{}", manifest.trim(), formatted).as_bytes())?;
    println!("✨ Formatted {}", path.display());
    Ok(())
}

/// The code formatted by `rustfmt` as edition 2021 (the edition rsxtk builds scripts with), through
/// stdin and stdout — no temporary file. A failure (bad syntax, no rustfmt) is an error, never a
/// silent success.
fn rustfmt(code: &str) -> Result<String> {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("❌ rustfmt could not be started ({}). Install it: rustup component add rustfmt", e))?;
    // Feed stdin and drain stdout / stderr on their own threads, so this thread can watch the
    // clock: rustfmt has a time limit (review 2, D2 — it ran past 60 s on deeply nested code while
    // holding the script's lock; the owner's no-time-limit rule covers guests and Cargo builds).
    let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("rustfmt's stdin is not available"))?;
    let mut stdout = child.stdout.take().ok_or_else(|| anyhow!("rustfmt's stdout is not available"))?;
    let mut stderr = child.stderr.take().ok_or_else(|| anyhow!("rustfmt's stderr is not available"))?;
    let input = code.to_owned();
    let feeder = std::thread::spawn(move || { let _ = stdin.write_all(input.as_bytes()); });
    let out_reader = std::thread::spawn(move || { let mut v = Vec::new(); let _ = stdout.read_to_end(&mut v); v });
    let err_reader = std::thread::spawn(move || { let mut v = Vec::new(); let _ = stderr.read_to_end(&mut v); v });
    let limit = rustfmt_limit();
    let deadline = Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait()? { break status; }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("❌ rustfmt did not finish within {} s and was stopped.", limit.as_secs_f32());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let _ = feeder.join();
    let out = out_reader.join().map_err(|_| anyhow!("reading rustfmt's output failed"))?;
    let err = err_reader.join().map_err(|_| anyhow!("reading rustfmt's errors failed"))?;
    if !status.success() {
        bail!("❌ rustfmt failed ({}):\n{}", status, String::from_utf8_lossy(&err).trim());
    }
    Ok(String::from_utf8(out)?)
}

/// How long `fmt` lets rustfmt run.
const RUSTFMT_LIMIT: std::time::Duration = std::time::Duration::from_secs(120);

/// `RUSTFMT_LIMIT`, which a DEBUG build lets a test shorten with `RSXTK_TEST_RUSTFMT_LIMIT_MS` so
/// the limit itself is tested (open items before 1.0.1). A release build — every `cargo install` —
/// reads no such variable.
fn rustfmt_limit() -> std::time::Duration {
    #[cfg(debug_assertions)]
    if let Some(ms) = std::env::var("RSXTK_TEST_RUSTFMT_LIMIT_MS").ok().and_then(|v| v.parse::<u64>().ok()) {
        return std::time::Duration::from_millis(ms);
    }
    RUSTFMT_LIMIT
}

/// True when both paths name one existing file.
fn same_file(a: &Path, b: &Path) -> bool {
    matches!((fs::canonicalize(a), fs::canonicalize(b)), (Ok(x), Ok(y)) if x == y)
}

fn optimize_wasm(engine: &Engine, input: &Path, output: Option<PathBuf>) -> Result<()> {
    let bytes = fs::read(input)?;
    // Review H8: walrus panics (exit 101) on some invalid modules; validate first, so an invalid
    // input is a named refusal and nothing is written.
    Module::validate(engine, &bytes)
        .map_err(|e| anyhow!("❌ '{}' is not a valid WebAssembly module: {}", input.display(), e))?;
    let mut module = walrus::Module::from_buffer(&bytes)?;
    // Without -o the input is replaced, as documented — atomically, so it is never half-written.
    match output {
        Some(out) => write_atomic(&out, &module.emit_wasm())?,
        // Without -o the user's own file is rewritten in place, a symlinked one through its link,
        // as `add` / `fmt` do (re-review 3: it replaced the link with a file).
        None => rewrite_script(input, &module.emit_wasm())?,
    }
    Ok(())
}

fn benchmark_wasm(engine: &Engine, path: &Path, iterations: u32, sandbox: Sandbox) -> Result<()> {
    let start = Instant::now();
    for _ in 0..iterations { run_wasm(engine, path, sandbox)?; }
    println!("📊 Avg Execution: {:?}", start.elapsed() / iterations);
    Ok(())
}