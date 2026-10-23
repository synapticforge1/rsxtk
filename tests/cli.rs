//! End-to-end tests: each one runs the real `rsxtk` binary in its own temporary folder and checks
//! stdout and the exit code. Every command has a passing case and a case that must fail.
//! Nothing here needs the network (`add` always gets an explicit version).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const HELLO_WAT: &str = r#"(module
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory 1)
  (export "memory" (memory 0))
  (data (i32.const 8) "Hello, Rosetta Code!\n")
  (func $main (export "_start")
    (i32.store (i32.const 0) (i32.const 8))
    (i32.store (i32.const 4) (i32.const 21))
    (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 24))
    drop))
"#;

const LIB_WAT: &str = r#"(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add))
"#;

const SCRIPT: &str = "---\n[dependencies]\n---\nfn main() {\n    println!(\"script ran\");\n}\n";

/// A fresh, empty folder for one test.
fn workdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("rsxtk-tests").join(name);
    if dir.exists() { fs::remove_dir_all(&dir).unwrap(); }
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn rsxtk(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rsxtk")).args(args).current_dir(dir).output().unwrap()
}

fn stdout(out: &Output) -> String { String::from_utf8_lossy(&out.stdout).into_owned() }
fn stderr(out: &Output) -> String { String::from_utf8_lossy(&out.stderr).into_owned() }

fn assert_ok(out: &Output) {
    assert!(out.status.success(), "expected success, got {:?}\nstdout:\n{}\nstderr:\n{}", out.status, stdout(out), stderr(out));
}

fn assert_fails(out: &Output) {
    assert!(!out.status.success(), "expected failure, got success\nstdout:\n{}", stdout(out));
}

// --- version ---

#[test]
fn version_matches_cargo_toml() {
    let out = rsxtk(&workdir("version"), &["--version"]);
    assert_ok(&out);
    assert_eq!(stdout(&out).trim(), format!("rsxtk {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_command_fails() {
    assert_fails(&rsxtk(&workdir("unknown"), &["nosuchcommand"]));
}

// --- run ---

#[test]
fn run_wat_prints_hello() {
    let dir = workdir("run_wat");
    fs::write(dir.join("hello.wat"), HELLO_WAT).unwrap();
    let out = rsxtk(&dir, &["run", "hello.wat"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "Hello, Rosetta Code!\n");
}

#[test]
fn run_wasm_prints_hello() {
    let dir = workdir("run_wasm");
    fs::write(dir.join("hello.wasm"), wat::parse_str(HELLO_WAT).unwrap()).unwrap();
    let out = rsxtk(&dir, &["run", "hello.wasm"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "Hello, Rosetta Code!\n");
}

#[test]
fn run_missing_file_fails() {
    assert_fails(&rsxtk(&workdir("run_missing"), &["run", "nope.wasm"]));
}

#[test]
fn run_unsupported_extension_fails() {
    let dir = workdir("run_ext");
    fs::write(dir.join("notes.txt"), "hi").unwrap();
    let out = rsxtk(&dir, &["run", "notes.txt"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("Unsupported file type"));
}

#[test]
fn run_rs_script_builds_and_runs() {
    let dir = workdir("run_rs");
    fs::write(dir.join("script.rs"), SCRIPT).unwrap();
    let out = rsxtk(&dir, &["run", "script.rs"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "script ran\n");
    assert!(dir.join(".tk/script/script.cwasm").exists());
    // Second run comes from the cache and prints the same.
    let again = rsxtk(&dir, &["run", "script.rs"]);
    assert_ok(&again);
    assert_eq!(stdout(&again), "script ran\n");
}

#[test]
fn run_rs_without_manifest_fails() {
    let dir = workdir("run_rs_nomanifest");
    fs::write(dir.join("bare.rs"), "fn main() {}\n").unwrap();
    let out = rsxtk(&dir, &["run", "bare.rs"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("Missing script manifest"));
}

#[test]
fn run_library_from_another_folder_lists_exports() {
    let dir = workdir("run_lib_elsewhere");
    fs::create_dir_all(dir.join("libs")).unwrap();
    fs::write(dir.join("libs/lib.wat"), LIB_WAT).unwrap();
    let out = rsxtk(&dir, &["run", "libs/lib.wat"]);
    assert_ok(&out);
    assert!(stdout(&out).contains("add(i32, i32) -> i32"), "{}", stdout(&out));
}

// --- mod ---

#[test]
fn mod_calls_export_with_arguments() {
    let dir = workdir("mod_wat");
    fs::write(dir.join("lib.wat"), LIB_WAT).unwrap();
    let out = rsxtk(&dir, &["mod", "lib.wat", "add", "2", "3"]);
    assert_ok(&out);
    assert_eq!(stdout(&out).trim(), "5");
}

#[test]
fn mod_missing_argument_fails() {
    let dir = workdir("mod_missing_arg");
    fs::write(dir.join("lib.wat"), LIB_WAT).unwrap();
    let out = rsxtk(&dir, &["mod", "lib.wat", "add", "2"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("Missing argument for parameter 1"));
}

#[test]
fn mod_unknown_function_fails() {
    let dir = workdir("mod_unknown");
    fs::write(dir.join("lib.wat"), LIB_WAT).unwrap();
    let out = rsxtk(&dir, &["mod", "lib.wat", "nosuch", "1"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("Func 'nosuch' not found"));
}

// --- init / initmod ---

#[test]
fn init_writes_a_runnable_template() {
    let dir = workdir("init");
    assert_ok(&rsxtk(&dir, &["init", "fresh"]));
    let text = fs::read_to_string(dir.join("fresh.rs")).unwrap();
    assert!(text.starts_with("---\n[dependencies]\n---\n"));
    let out = rsxtk(&dir, &["run", "fresh.rs"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "Hello from rsxtk!\n");
}

#[test]
fn init_into_missing_folder_fails() {
    assert_fails(&rsxtk(&workdir("init_missing_dir"), &["init", "no/such/dir/x.rs"]));
}

#[test]
fn initmod_template_builds_and_mod_calls_it() {
    let dir = workdir("initmod");
    assert_ok(&rsxtk(&dir, &["initmod", "my-lib"]));
    let out = rsxtk(&dir, &["mod", "my-lib.rs", "add", "4", "5"]);
    assert_ok(&out);
    assert_eq!(stdout(&out).trim(), "9");
}

// --- build ---

#[test]
fn build_wat_writes_wasm_and_wat() {
    let dir = workdir("build_wat");
    fs::write(dir.join("script.rs"), SCRIPT).unwrap();
    assert_ok(&rsxtk(&dir, &["build", "script.rs", "wat"]));
    assert!(dir.join("script.wasm").exists());
    assert!(fs::read_to_string(dir.join("script.wat")).unwrap().starts_with("(module"));
}

#[test]
fn build_unknown_target_fails() {
    let dir = workdir("build_bad_target");
    fs::write(dir.join("script.rs"), SCRIPT).unwrap();
    assert_fails(&rsxtk(&dir, &["build", "script.rs", "cwasm"]));
}

// --- convert ---

#[test]
fn convert_round_trips_wat_and_wasm() {
    let dir = workdir("convert");
    fs::write(dir.join("lib.wat"), LIB_WAT).unwrap();
    assert_ok(&rsxtk(&dir, &["convert", "lib.wat"]));
    assert_eq!(fs::read(dir.join("lib.wasm")).unwrap(), wat::parse_str(LIB_WAT).unwrap());
    assert_ok(&rsxtk(&dir, &["convert", "lib.wasm", "-o", "back.wat"]));
    assert!(fs::read_to_string(dir.join("back.wat")).unwrap().contains("i32.add"));
}

#[test]
fn convert_invalid_wat_fails() {
    let dir = workdir("convert_bad");
    fs::write(dir.join("bad.wat"), "(module (func (export \"f\") i32.bogus))").unwrap();
    assert_fails(&rsxtk(&dir, &["convert", "bad.wat"]));
}

// --- resource / rsc / build.rs ---

#[test]
fn resource_creates_rsc_folder() {
    let dir = workdir("resource");
    assert_ok(&rsxtk(&dir, &["resource"]));
    assert!(dir.join("rsc").is_dir());
}

#[test]
fn resource_with_a_file_path_fails() {
    let dir = workdir("resource_file");
    fs::write(dir.join("script.rs"), SCRIPT).unwrap();
    assert_fails(&rsxtk(&dir, &["resource", "script.rs"]));
}

#[test]
fn script_reads_rsc_at_run_time() {
    let dir = workdir("rsc_runtime");
    fs::create_dir_all(dir.join("rsc")).unwrap();
    fs::write(dir.join("rsc/greeting.txt"), "hello from rsc\n").unwrap();
    fs::write(dir.join("read.rs"), "---\n[dependencies]\n---\nfn main() {\n    print!(\"{}\", std::fs::read_to_string(\"rsc/greeting.txt\").unwrap());\n}\n").unwrap();
    let out = rsxtk(&dir, &["run", "read.rs"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "hello from rsc\n");
    fs::remove_file(dir.join("rsc/greeting.txt")).unwrap();
    assert_fails(&rsxtk(&dir, &["run", "read.rs"]));
}

#[test]
fn script_embeds_rsc_and_rebuilds_when_it_changes() {
    let dir = workdir("rsc_embed");
    fs::create_dir_all(dir.join("rsc")).unwrap();
    fs::write(dir.join("rsc/embed.txt"), "v1\n").unwrap();
    fs::write(dir.join("embed.rs"), "---\n[dependencies]\n---\nfn main() {\n    print!(\"{}\", include_str!(\"rsc/embed.txt\"));\n}\n").unwrap();
    let out = rsxtk(&dir, &["run", "embed.rs"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "v1\n");
    // Make sure the edit is newer than the cached build on coarse-mtime file systems.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    fs::write(dir.join("rsc/embed.txt"), "v2\n").unwrap();
    let out = rsxtk(&dir, &["run", "embed.rs"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "v2\n");
    fs::remove_dir_all(dir.join("rsc")).unwrap();
    assert_fails(&rsxtk(&dir, &["run", "embed.rs"]));
    assert!(!dir.join(".tk/embed/src/rsc").exists());
}

#[test]
fn build_rs_beside_the_script_is_used() {
    let dir = workdir("build_rs");
    fs::write(dir.join("build.rs"), "fn main() {\n    println!(\"cargo:rustc-env=RSXTK_TEST_MSG=from build.rs\");\n}\n").unwrap();
    fs::write(dir.join("gen.rs"), "---\n[dependencies]\n---\nfn main() {\n    println!(\"{}\", env!(\"RSXTK_TEST_MSG\"));\n}\n").unwrap();
    let out = rsxtk(&dir, &["run", "gen.rs"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "from build.rs\n");
    fs::remove_file(dir.join("build.rs")).unwrap();
    assert_fails(&rsxtk(&dir, &["run", "gen.rs"]));
    assert!(!dir.join(".tk/gen/build.rs").exists());
}

// --- bench ---

#[test]
fn bench_runs_n_times_and_reports_average() {
    let dir = workdir("bench");
    fs::write(dir.join("hello.wat"), HELLO_WAT).unwrap();
    let out = rsxtk(&dir, &["bench", "hello.wat", "-i", "3"]);
    assert_ok(&out);
    let text = stdout(&out);
    assert_eq!(text.matches("Hello, Rosetta Code!").count(), 3);
    assert!(text.contains("Avg Execution"));
}

#[test]
fn bench_missing_file_fails() {
    assert_fails(&rsxtk(&workdir("bench_missing"), &["bench", "nope.wat"]));
}

// --- add / remove / list ---

#[test]
fn add_list_and_remove_a_dependency() {
    let dir = workdir("deps");
    fs::write(dir.join("script.rs"), SCRIPT).unwrap();
    assert_ok(&rsxtk(&dir, &["add", "script.rs", "anyhow", "1.0"]));
    assert!(fs::read_to_string(dir.join("script.rs")).unwrap().contains("anyhow = \"1.0\""));
    let listed = rsxtk(&dir, &["list", "script.rs"]);
    assert_ok(&listed);
    assert!(stdout(&listed).contains("anyhow = \"1.0\""));
    assert_ok(&rsxtk(&dir, &["remove", "script.rs", "anyhow"]));
    let text = fs::read_to_string(dir.join("script.rs")).unwrap();
    assert!(!text.contains("anyhow"));
    assert!(text.contains("fn main()"));
}

#[test]
fn add_injects_a_manifest_when_missing() {
    let dir = workdir("deps_inject");
    fs::write(dir.join("bare.rs"), "fn main() {}\n").unwrap();
    assert_ok(&rsxtk(&dir, &["add", "bare.rs", "anyhow", "1.0"]));
    let text = fs::read_to_string(dir.join("bare.rs")).unwrap();
    assert!(text.starts_with("---\n[dependencies]\nanyhow = \"1.0\"\n---\n"), "{}", text);
}

#[test]
fn add_to_an_incomplete_manifest_fails() {
    let dir = workdir("deps_incomplete");
    fs::write(dir.join("broken.rs"), "---\n[dependencies]\nfn main() {}\n").unwrap();
    let out = rsxtk(&dir, &["add", "broken.rs", "anyhow", "1.0"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("Manifest block is incomplete"));
}

#[test]
fn list_without_a_manifest_says_so() {
    let dir = workdir("list_none");
    fs::write(dir.join("bare.rs"), "fn main() {}\n").unwrap();
    let out = rsxtk(&dir, &["list", "bare.rs"]);
    assert_ok(&out);
    assert!(stdout(&out).contains("No manifest found"));
}

#[test]
fn remove_keeps_crates_that_share_a_prefix() {
    let dir = workdir("deps_prefix");
    fs::write(dir.join("script.rs"), "---\n[dependencies]\nserde = \"1\"\nserde_json = \"1\"\n---\nfn main() {}\n").unwrap();
    assert_ok(&rsxtk(&dir, &["remove", "script.rs", "serde"]));
    let text = fs::read_to_string(dir.join("script.rs")).unwrap();
    assert!(!text.contains("serde = "));
    assert!(text.contains("serde_json = \"1\""), "{}", text);
}

#[test]
fn add_replaces_only_the_named_crate() {
    let dir = workdir("deps_replace");
    fs::write(dir.join("script.rs"), "---\n[dependencies]\nserde = \"1.0.100\"\nserde_json = \"1\"\n---\nfn main() {}\n").unwrap();
    assert_ok(&rsxtk(&dir, &["add", "script.rs", "serde", "1.0.200"]));
    let text = fs::read_to_string(dir.join("script.rs")).unwrap();
    assert!(text.contains("serde = \"1.0.200\""), "{}", text);
    assert!(!text.contains("1.0.100"), "{}", text);
    assert!(text.contains("serde_json = \"1\""), "{}", text);
}

// --- fmt ---

#[test]
fn fmt_formats_code_and_keeps_the_manifest() {
    let dir = workdir("fmt");
    fs::write(dir.join("messy.rs"), "---\n[dependencies]\nanyhow = \"1.0\"\n---\nfn main(){println!(\"x\");}\n").unwrap();
    assert_ok(&rsxtk(&dir, &["fmt", "messy.rs"]));
    let text = fs::read_to_string(dir.join("messy.rs")).unwrap();
    assert!(text.starts_with("---\n[dependencies]\nanyhow = \"1.0\"\n---\n"), "{}", text);
    assert!(text.contains("fn main() {\n    println!(\"x\");\n}"), "{}", text);
}

#[test]
fn fmt_missing_file_fails() {
    assert_fails(&rsxtk(&workdir("fmt_missing"), &["fmt", "nope.rs"]));
}

// --- optimize ---

#[test]
fn optimize_writes_a_valid_module() {
    let dir = workdir("optimize");
    fs::write(dir.join("lib.wasm"), wat::parse_str(LIB_WAT).unwrap()).unwrap();
    assert_ok(&rsxtk(&dir, &["optimize", "-i", "lib.wasm", "-o", "opt.wasm"]));
    let out = rsxtk(&dir, &["mod", "opt.wasm", "add", "2", "3"]);
    assert_ok(&out);
    assert_eq!(stdout(&out).trim(), "5");
}

#[test]
fn optimize_invalid_input_fails() {
    let dir = workdir("optimize_bad");
    fs::write(dir.join("junk.wasm"), b"not wasm").unwrap();
    assert_fails(&rsxtk(&dir, &["optimize", "-i", "junk.wasm"]));
}

// --- info ---

#[test]
fn info_lists_signatures_and_wasi_use() {
    let dir = workdir("info");
    fs::write(dir.join("lib.wat"), LIB_WAT).unwrap();
    fs::write(dir.join("hello.wat"), HELLO_WAT).unwrap();
    let lib = rsxtk(&dir, &["info", "lib.wat"]);
    assert_ok(&lib);
    assert!(stdout(&lib).contains("- add(i32, i32) -> i32"));
    assert!(stdout(&lib).contains("WASI Support: No"));
    let hello = rsxtk(&dir, &["info", "hello.wat"]);
    assert_ok(&hello);
    assert!(stdout(&hello).contains("WASI Support: Yes"));
}

#[test]
fn info_invalid_module_fails() {
    let dir = workdir("info_bad");
    fs::write(dir.join("junk.wasm"), b"not wasm").unwrap();
    assert_fails(&rsxtk(&dir, &["info", "junk.wasm"]));
}

// --- clean ---

#[test]
fn clean_removes_the_build_cache() {
    let dir = workdir("clean");
    fs::write(dir.join("script.rs"), SCRIPT).unwrap();
    assert_ok(&rsxtk(&dir, &["run", "script.rs"]));
    assert!(dir.join(".tk").exists());
    assert_ok(&rsxtk(&dir, &["clean"]));
    assert!(!dir.join(".tk").exists());
    // Nothing to clean is not an error.
    assert_ok(&rsxtk(&dir, &["clean"]));
}

// --- csrun (native, nightly `cargo -Z script`) ---

#[test]
fn csrun_runs_natively() {
    let dir = workdir("csrun");
    fs::write(dir.join("native.rs"), SCRIPT).unwrap();
    let out = rsxtk(&dir, &["csrun", "native.rs"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "script ran\n");
}

#[test]
fn csrun_missing_file_fails() {
    assert_fails(&rsxtk(&workdir("csrun_missing"), &["csrun", "nope.rs"]));
}
