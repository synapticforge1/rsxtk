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
    func.call(&mut store, &wasm_params, &mut results)?;
    print_val_results(&results);
    Ok(())
}

fn execute_module(engine: &Engine, module: Module, abs_path: &Path, sandbox: Sandbox) -> Result<()> {
    let (linker, mut store) = new_store(engine, abs_path, sandbox)?;
    let instance = linker.instantiate(&mut store, &module)?;

    if let Some(func) = instance.get_func(&mut store, "_start") {
        if let Err(e) = func.call(&mut store, &[], &mut []) {
            match e.downcast::<wasmtime_wasi::I32Exit>() {
                Ok(status) if status.0 == 0 => (),
                Ok(status) => bail!("Exited status: {}", status.0),
                Err(e) => return Err(e.into()),
            }
        }
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
            Ok(Module::new(engine, &wat::parse_bytes(&bytes)?)?)
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
    let stem = script_path.file_stem().unwrap().to_str().unwrap();
    let cache_root = std::env::current_dir()?.join(".tk").join(stem);
    let cached_path = cache_root.join(format!("{}.wasm", stem));

    let content = fs::read_to_string(script_path)?;
    let Some((manifest, code)) = split_manifest(&content)? else {
        bail!(
            "❌ Missing script manifest in '{}'.\n\n\
            WASI scripts require a header block for dependencies.\n\
            💡 Fix: Run 'rsxtk add {} <crate>' or 'rsxtk fmt {}' to inject it.",
            script_path.display(),
            script_path.file_name().unwrap().to_str().unwrap(),
            script_path.file_name().unwrap().to_str().unwrap()
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
    fs::write(cache_root.join("Cargo.toml"), format!("[package]\nname=\"{}\"\nversion=\"0.1.0\"\nedition=\"2021\"\n\n{}{}", stem, lib_section, manifest.trim()))?;
    fs::write(cache_root.join(src_file), format!("\n{}", code))?;

    // Cargo runs build.rs from the package root. rsc/ goes under src/, so include_str!("rsc/…")
    // resolves as it does beside the script (and under `csrun`). Stale copies are removed.
    if has_build_rs { fs::copy(&build_rs, &mirror_build_rs)?; }
    else { let _ = fs::remove_file(&mirror_build_rs); }
    if rsc_dir.is_dir() { copy_dir_fresh(&rsc_dir, &mirror_rsc)?; }
    else if mirror_rsc.exists() { fs::remove_dir_all(&mirror_rsc)?; }

    let status = std::process::Command::new("cargo")
        .env("CARGO_TARGET_DIR", cache_root.join("t"))
        .args(["build", "--manifest-path", cache_root.join("Cargo.toml").to_str().unwrap(), "--target", "wasm32-wasip1", "--release"])
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

/// Writes `bytes` to `path` through a temporary file beside it and a rename, so an interrupted or
/// failed write never leaves a half-written file, and readers see the old or the new file whole.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let name = path.file_name().ok_or_else(|| anyhow!("'{}' has no file name", path.display()))?;
    let tmp = path.with_file_name(format!(".{}.rsxtk-{}.tmp", name.to_string_lossy(), std::process::id()));
    if let Err(e) = fs::write(&tmp, bytes).and_then(|_| fs::rename(&tmp, path)) {
        let _ = fs::remove_file(&tmp);
        return Err(anyhow!("could not write '{}': {}", path.display(), e));
    }
    Ok(())
}

/// True when the code defines `fn main` (followed by optional whitespace and `(`).
fn has_main_fn(code: &str) -> bool {
    code.match_indices("fn main").any(|(i, m)| code[i + m.len()..].trim_start().starts_with('('))
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
        else { fs::copy(entry.path(), target)?; }
    }
    Ok(())
}

// --- Utilities & Helpers ---

fn init_script(name: &str, is_mod: bool) -> Result<()> {
    let filename = if name.ends_with(".rs") { name.to_string() } else { format!("{}.rs", name) };
    let content = if is_mod {
        "---\n[dependencies]\n---\n#[no_mangle]\npub extern \"C\" fn add(a: i32, b: i32) -> i32 {\n    a + b\n}"
    } else {
        "---\n[dependencies]\n---\nfn main() {\n    println!(\"Hello from rsxtk!\");\n}"
    };
    // create_new: an existing file is never replaced (review H4).
    let mut file = match fs::OpenOptions::new().write(true).create_new(true).open(&filename) {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            bail!("❌ '{}' already exists; rsxtk init does not overwrite it.", filename)
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
    let bytes = if is_wasm { wasmprinter::print_bytes(&data)?.into_bytes() } else { wat::parse_bytes(&data)?.into_owned() };
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
    write_atomic(path, format!("---\n{}\n---\n{}", edited.trim(), code.trim_start()).as_bytes())?;
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
/// (`$XDG_CACHE_HOME/rsxtk/locks`, else `~/.cache/rsxtk/locks`).
/// [win-x64, win-arm64, mac-x64, mac-arm64] the temp folder is already per-user (`%TEMP%`, `$TMPDIR`).
fn lock_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let cache = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from).filter(|p| p.is_absolute()).map(|h| h.join(".cache")));
        if let Some(cache) = cache { return cache.join("rsxtk").join("locks"); }
    }
    std::env::temp_dir().join("rsxtk-locks")
}

/// Creates `dir` (and its parents) if missing. [linux-*, mac-*] The folder itself gets mode 0700,
/// so other local users cannot list which scripts were edited when (privacy re-review 2).
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        if let Some(parent) = dir.parent() { fs::create_dir_all(parent)?; }
        match fs::DirBuilder::new().mode(0o700).create(dir) {
            Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => return Err(e),
            _ => return Ok(()),
        }
    }
    #[cfg(not(unix))]
    fs::create_dir_all(dir)
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
    let mut has_deps = false;
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
            if in_deps { deps_end = None; has_deps = true; }
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
    } else if !has_deps {
        out.insert(0, "[dependencies]".to_string());
    }
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
    if version.trim().is_empty() || version.chars().any(|c| c == '"' || c == '\\' || c.is_control()) {
        bail!("❌ '{}' is not a valid version requirement.", version.escape_debug());
    }
    Ok(())
}

/// The crate a manifest line declares (`name = …`, `name.workspace = …`, `"name" = …`), if any.
/// Exact, so `serde` does not match `serde_json`.
fn dep_key(line: &str) -> Option<&str> {
    if !line.contains('=') { return None; }
    let key = line.split(['=', '.']).next()?.trim().trim_matches('"');
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
    write_atomic(path, format!("---\n{}\n---\n{}", manifest.trim(), formatted).as_bytes())?;
    println!("✨ Formatted {}", path.display());
    Ok(())
}

/// The code formatted by `rustfmt` as edition 2021 (the edition rsxtk builds scripts with), through
/// stdin and stdout — no temporary file. A failure (bad syntax, no rustfmt) is an error, never a
/// silent success.
fn rustfmt(code: &str) -> Result<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("❌ rustfmt could not be started ({}). Install it: rustup component add rustfmt", e))?;
    // rustfmt reads all of stdin before it writes, so writing first cannot deadlock.
    child.stdin.take().ok_or_else(|| anyhow!("rustfmt's stdin is not available"))?.write_all(code.as_bytes())?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!("❌ rustfmt failed ({}):\n{}", out.status, String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8(out.stdout)?)
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
    write_atomic(&output.unwrap_or_else(|| input.to_path_buf()), &module.emit_wasm())?;
    Ok(())
}

fn benchmark_wasm(engine: &Engine, path: &Path, iterations: u32, sandbox: Sandbox) -> Result<()> {
    let start = Instant::now();
    for _ in 0..iterations { run_wasm(engine, path, sandbox)?; }
    println!("📊 Avg Execution: {:?}", start.elapsed() / iterations);
    Ok(())
}