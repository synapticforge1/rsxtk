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
    // The cache is plain wasm, never a precompiled .cwasm.
    assert!(dir.join(".tk/script/script.wasm").exists());
    assert!(!dir.join(".tk/script/script.cwasm").exists());
    // Second run comes from the cache and prints the same.
    let again = rsxtk(&dir, &["run", "script.rs"]);
    assert_ok(&again);
    assert_eq!(stdout(&again), "script ran\n");
}

#[test]
fn run_cwasm_is_refused() {
    let dir = workdir("run_cwasm");
    // Any bytes will do: the file must be refused by its kind, before anything reads it.
    fs::write(dir.join("hello.cwasm"), b"\x7fELF not really").unwrap();
    let out = rsxtk(&dir, &["run", "hello.cwasm"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("does not run it"), "stderr:\n{}", stderr(&out));
    assert_eq!(stdout(&out), "");
}

#[test]
fn run_rs_ignores_an_old_cwasm_cache() {
    let dir = workdir("run_rs_old_cache");
    fs::write(dir.join("script.rs"), SCRIPT).unwrap();
    // A cache left by 0.5.2 or earlier, newer than the script: it must not be read, and is removed.
    fs::create_dir_all(dir.join(".tk/script")).unwrap();
    fs::write(dir.join(".tk/script/script.cwasm"), b"planted").unwrap();
    fs::write(dir.join(".tk/script/machine.id"), "0").unwrap();
    let out = rsxtk(&dir, &["run", "script.rs"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "script ran\n");
    assert!(!dir.join(".tk/script/script.cwasm").exists());
    assert!(!dir.join(".tk/script/machine.id").exists());
}

#[test]
fn scripts_with_one_name_in_two_folders_each_run_their_own() {
    // Review B1: the cache was keyed by name and timestamp only, so b/p.rs ran a/p.rs's module.
    let dir = workdir("run_rs_same_name");
    fs::create_dir_all(dir.join("a")).unwrap();
    fs::create_dir_all(dir.join("b")).unwrap();
    fs::write(dir.join("b/p.rs"), "---\n[dependencies]\n---\nfn main() {\n    println!(\"from b\");\n}\n").unwrap();
    fs::write(dir.join("a/p.rs"), "---\n[dependencies]\n---\nfn main() {\n    println!(\"from a\");\n}\n").unwrap();
    let a = rsxtk(&dir, &["run", "a/p.rs"]);
    assert_ok(&a);
    assert_eq!(stdout(&a), "from a\n");
    let b = rsxtk(&dir, &["run", "b/p.rs"]);
    assert_ok(&b);
    assert_eq!(stdout(&b), "from b\n");
}

#[test]
fn run_rs_rebuilds_an_edit_with_an_older_timestamp() {
    // Review B1: an edit that keeps an older mtime (cp -p, unzip, a restore) was not rebuilt.
    let dir = workdir("run_rs_old_mtime");
    fs::write(dir.join("t.rs"), SCRIPT).unwrap();
    assert_eq!(stdout(&rsxtk(&dir, &["run", "t.rs"])), "script ran\n");
    let file = fs::OpenOptions::new().write(true).open(dir.join("t.rs")).unwrap();
    file.set_len(0).unwrap();
    drop(file);
    fs::write(dir.join("t.rs"), "---\n[dependencies]\n---\nfn main() {\n    println!(\"edited\");\n}\n").unwrap();
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3 * 24 * 3600);
    fs::OpenOptions::new().write(true).open(dir.join("t.rs")).unwrap().set_modified(old).unwrap();
    let out = rsxtk(&dir, &["run", "t.rs"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "edited\n");
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

#[test]
fn bench_zero_iterations_is_refused_not_a_panic() {
    // Review B3: `-i 0` divided by zero (exit 101).
    let dir = workdir("bench_zero");
    fs::write(dir.join("hello.wat"), HELLO_WAT).unwrap();
    let out = rsxtk(&dir, &["bench", "hello.wat", "-i", "0"]);
    assert_eq!(out.status.code(), Some(2), "stderr:\n{}", stderr(&out));
    assert!(!stderr(&out).contains("panicked"), "{}", stderr(&out));
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

#[test]
fn fmt_formats_as_edition_2021() {
    // Review B2: rustfmt ran as edition 2015 and refused `async fn`, and rsxtk said "Formatted".
    let dir = workdir("fmt_2021");
    fs::write(dir.join("a.rs"), "---\n[dependencies]\n---\nasync fn   f( ){}\nfn main(){  }\n").unwrap();
    assert_ok(&rsxtk(&dir, &["fmt", "a.rs"]));
    let text = fs::read_to_string(dir.join("a.rs")).unwrap();
    assert!(text.contains("async fn f() {}"), "{}", text);
    assert!(text.contains("fn main() {}"), "{}", text);
}

#[test]
fn fmt_of_code_rustfmt_rejects_fails_and_keeps_the_file() {
    // Review B2: a rustfmt failure was reported as success (exit 0).
    let dir = workdir("fmt_reject");
    let original = "---\n[dependencies]\n---\nfn main( {\n";
    fs::write(dir.join("bad.rs"), original).unwrap();
    let out = rsxtk(&dir, &["fmt", "bad.rs"]);
    assert_fails(&out);
    assert!(!stdout(&out).contains("Formatted"), "{}", stdout(&out));
    assert!(stderr(&out).contains("rustfmt failed"), "{}", stderr(&out));
    assert_eq!(fs::read_to_string(dir.join("bad.rs")).unwrap(), original);
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

// --- review 0.6.0: security ---

/// Creates `out.txt` in its folder (preopen fd 3) and writes "hi"; exits with path_open's errno.
const WRITE_WAT: &str = r#"(module
  (import "wasi_snapshot_preview1" "path_open" (func $po (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fw (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "proc_exit" (func $exit (param i32)))
  (memory (export "memory") 1)
  (data (i32.const 100) "out.txt")
  (data (i32.const 200) "hi")
  (func (export "_start") (local $err i32)
    (local.set $err (call $po (i32.const 3) (i32.const 0) (i32.const 100) (i32.const 7) (i32.const 1)
      (i64.const 536870911) (i64.const 536870911) (i32.const 0) (i32.const 300)))
    (if (local.get $err) (then (call $exit (local.get $err))))
    (i32.store (i32.const 400) (i32.const 200))
    (i32.store (i32.const 404) (i32.const 2))
    (drop (call $fw (i32.load (i32.const 300)) (i32.const 400) (i32.const 1) (i32.const 500)))))
"#;

#[test]
fn guest_folder_is_read_only_without_allow_write() {
    // Review S1 (owner: read-only + --allow-write): a guest could create and overwrite any file
    // beside the module.
    let dir = workdir("s1_read_only");
    fs::write(dir.join("w.wat"), WRITE_WAT).unwrap();
    let out = rsxtk(&dir, &["run", "w.wat"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("Exited status"), "{}", stderr(&out));
    assert!(!dir.join("out.txt").exists());
}

#[test]
fn allow_write_lets_the_guest_write_its_folder() {
    let dir = workdir("s1_allow_write");
    fs::write(dir.join("w.wat"), WRITE_WAT).unwrap();
    let out = rsxtk(&dir, &["run", "--allow-write", "w.wat"]);
    assert_ok(&out);
    assert_eq!(fs::read_to_string(dir.join("out.txt")).unwrap(), "hi");
}

// --- review 0.6.0: hardening (unplanned input fails loudly and safely) ---

#[test]
fn add_puts_the_crate_in_dependencies_not_in_a_later_table() {
    // Review H1: the new entry was appended at the END of the manifest, inside [build-dependencies].
    let dir = workdir("h1_tables");
    fs::write(dir.join("c.rs"), "---\n[dependencies]\nitoa = \"1\"\n\n[build-dependencies]\ncc = \"1\"\n---\nfn main() {}\n").unwrap();
    assert_ok(&rsxtk(&dir, &["add", "c.rs", "anyhow", "1"]));
    let text = fs::read_to_string(dir.join("c.rs")).unwrap();
    assert!(text.starts_with("---\n[dependencies]\nitoa = \"1\"\nanyhow = \"1\"\n\n[build-dependencies]\ncc = \"1\"\n---\n"), "{}", text);
}

#[test]
fn remove_takes_a_multi_line_entry_whole() {
    // Review H1: only the first line of `x = { … }` spanning lines was removed; the rest stayed.
    let dir = workdir("h1_multiline");
    fs::write(dir.join("c.rs"), "---\n[dependencies]\nserde = { version = \"1\",\n  features = [\"derive\"] }\nitoa = \"1\"\n---\nfn main() {}\n").unwrap();
    assert_ok(&rsxtk(&dir, &["remove", "c.rs", "serde"]));
    let text = fs::read_to_string(dir.join("c.rs")).unwrap();
    assert_eq!(text, "---\n[dependencies]\nitoa = \"1\"\n---\nfn main() {}\n");
}

#[test]
fn add_and_remove_understand_the_dependencies_table_form() {
    // Review B6: `[dependencies.serde]` was left in place — `add` wrote a duplicate key (cargo
    // refuses it) and `remove` removed nothing, both reporting success.
    let dir = workdir("b6_table_form");
    let original = "---\n[dependencies]\nitoa = \"1\"\n\n[dependencies.serde]\nversion = \"1\"\nfeatures = [\"derive\"]\n\n[build-dependencies]\ncc = \"1\"\n---\nfn main() {}\n";
    fs::write(dir.join("t.rs"), original).unwrap();
    assert_ok(&rsxtk(&dir, &["add", "t.rs", "serde", "1.0.5"]));
    let added = fs::read_to_string(dir.join("t.rs")).unwrap();
    assert_eq!(added, "---\n[dependencies]\nitoa = \"1\"\nserde = \"1.0.5\"\n\n[build-dependencies]\ncc = \"1\"\n---\nfn main() {}\n");
    fs::write(dir.join("t.rs"), original).unwrap();
    assert_ok(&rsxtk(&dir, &["remove", "t.rs", "serde"]));
    let removed = fs::read_to_string(dir.join("t.rs")).unwrap();
    assert_eq!(removed, "---\n[dependencies]\nitoa = \"1\"\n\n[build-dependencies]\ncc = \"1\"\n---\nfn main() {}\n");
}

#[test]
fn a_commented_dependencies_header_is_still_the_dependencies_table() {
    // Review N2a: `[dependencies] # deps` was not recognised, so `add` wrote a second table.
    let dir = workdir("n2_commented_header");
    fs::write(dir.join("t.rs"), "---\n[dependencies] # deps\nrand = \"0.8\"\n---\nfn main() {}\n").unwrap();
    assert_ok(&rsxtk(&dir, &["add", "t.rs", "serde", "1"]));
    let text = fs::read_to_string(dir.join("t.rs")).unwrap();
    assert_eq!(text, "---\n[dependencies] # deps\nrand = \"0.8\"\nserde = \"1\"\n---\nfn main() {}\n");
}

#[test]
fn remove_ignores_brackets_in_comments_and_strings() {
    // Review N2c: `# [old` opened a "multi-line entry" and `remove` deleted every later line.
    let dir = workdir("n2_brackets");
    let original = "---\n[dependencies]\nserde = \"1\" # [old\nrand = \"1\"\nx = { version = \"2\", path = \"a[b\" }\n\n[features]\ndefault = []\n---\nfn main() {}\n";
    fs::write(dir.join("t.rs"), original).unwrap();
    assert_ok(&rsxtk(&dir, &["remove", "t.rs", "serde"]));
    let text = fs::read_to_string(dir.join("t.rs")).unwrap();
    assert_eq!(text, "---\n[dependencies]\nrand = \"1\"\nx = { version = \"2\", path = \"a[b\" }\n\n[features]\ndefault = []\n---\nfn main() {}\n");
}

#[test]
fn remove_of_an_absent_crate_says_nothing_changed() {
    let dir = workdir("n2_absent");
    fs::write(dir.join("t.rs"), SCRIPT).unwrap();
    let out = rsxtk(&dir, &["remove", "t.rs", "serde"]);
    assert_ok(&out);
    assert!(stdout(&out).contains("nothing changed"), "{}", stdout(&out));
    assert_eq!(fs::read_to_string(dir.join("t.rs")).unwrap(), SCRIPT);
}

#[test]
fn add_waits_for_the_scripts_lock() {
    // Review N3: parallel `add`s all said "Manifest updated" and an entry was lost. A race is too
    // timing-dependent to test (it passed 3/3 on the unfixed code), so this holds the script's
    // lock itself and checks that `add` waits for it. The lock's name is rsxtk's
    // (`lock_script`): the same std `DefaultHasher` over the script's canonical path.
    // Review NEW-3: it waited silently; now it says what it waits for.
    use std::hash::{Hash, Hasher};
    let dir = workdir("n3_lock");
    fs::write(dir.join("p.rs"), SCRIPT).unwrap();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    fs::canonicalize(dir.join("p.rs")).unwrap().hash(&mut h);
    let locks = lock_dir_like_rsxtk();
    fs::create_dir_all(&locks).unwrap();
    let held = fs::OpenOptions::new().create(true).truncate(false).write(true)
        .open(locks.join(format!("{:016x}.lock", h.finish()))).unwrap();
    held.lock().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_rsxtk")).args(["add", "p.rs", "itoa", "1"]).current_dir(&dir)
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1500));
    assert!(child.try_wait().unwrap().is_none(), "add finished while the script's lock was held");
    assert_eq!(fs::read_to_string(dir.join("p.rs")).unwrap(), SCRIPT);
    held.unlock().unwrap();
    let out = child.wait_with_output().unwrap();
    assert_ok(&out);
    assert!(stderr(&out).contains("waiting"), "no waiting message:\n{}", stderr(&out));
    assert!(fs::read_to_string(dir.join("p.rs")).unwrap().contains("itoa = \"1\""));
}

/// rsxtk's lock folder (`lock_dir` in src/main.rs), computed the same way.
fn lock_dir_like_rsxtk() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let cache = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from).filter(|p| p.is_absolute()).map(|h| h.join(".cache")));
        if let Some(cache) = cache { return cache.join("rsxtk").join("locks"); }
    }
    std::env::temp_dir().join("rsxtk-locks")
}

#[test]
fn remove_refuses_an_entry_that_never_closes_and_keeps_the_file() {
    // Review NEW-1: `serde = { version = "1"` without its `}` made `remove` drop every later line.
    let dir = workdir("new1_unclosed");
    let original = "---\n[dependencies]\nserde = { version = \"1\"\nrand = \"1\"\n\n[features]\nx = []\n---\nfn main() {}\n";
    fs::write(dir.join("t.rs"), original).unwrap();
    let out = rsxtk(&dir, &["remove", "t.rs", "serde"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("never closes"), "{}", stderr(&out));
    assert_eq!(fs::read_to_string(dir.join("t.rs")).unwrap(), original);
}

#[test]
fn add_refuses_a_manifest_with_multi_line_strings_and_keeps_the_file() {
    // Review NEW-2: a `"""…"""` value holding `[` left fragments behind or made a `[x]` line inside
    // the string read as a table header.
    let dir = workdir("new2_multiline");
    let original = "---\n[dependencies]\nrand = \"1\"\n[package.metadata]\ndoc = \"\"\"\n[x]\n\"\"\"\n---\nfn main() {}\n";
    fs::write(dir.join("t.rs"), original).unwrap();
    let out = rsxtk(&dir, &["remove", "t.rs", "rand"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("multi-line string"), "{}", stderr(&out));
    assert_eq!(fs::read_to_string(dir.join("t.rs")).unwrap(), original);
}

#[test]
fn single_quoted_crate_tables_are_the_crates_too() {
    // Review B8: `[dependencies.'serde']` (a TOML literal-string key) was not recognised: `add`
    // defined serde twice and `remove` left `[dependencies.'serde'.features]` behind.
    let dir = workdir("b8_single_quotes");
    let original = "---\n[dependencies]\nanyhow = \"1\"\n\n[dependencies.'serde']\nversion = \"1\"\n\n[dependencies.'serde'.features]\nx = 1\n---\nfn main() {}\n";
    fs::write(dir.join("t.rs"), original).unwrap();
    assert_ok(&rsxtk(&dir, &["add", "t.rs", "serde", "2"]));
    assert_eq!(fs::read_to_string(dir.join("t.rs")).unwrap(), "---\n[dependencies]\nanyhow = \"1\"\nserde = \"2\"\n---\nfn main() {}\n");
}

#[test]
fn a_single_quoted_dependency_key_is_the_same_crate() {
    // Review B9: `'serde' = "1"` was not matched: `add serde 2` wrote a second serde key.
    let dir = workdir("b9_single_quoted_key");
    fs::write(dir.join("u.rs"), "---\n[dependencies]\n'serde' = \"1\"\nitoa = \"1\"\n---\nfn main() {}\n").unwrap();
    assert_ok(&rsxtk(&dir, &["add", "u.rs", "serde", "2"]));
    assert_eq!(fs::read_to_string(dir.join("u.rs")).unwrap(), "---\n[dependencies]\nitoa = \"1\"\nserde = \"2\"\n---\nfn main() {}\n");
    assert_ok(&rsxtk(&dir, &["remove", "u.rs", "serde"]));
    assert_eq!(fs::read_to_string(dir.join("u.rs")).unwrap(), "---\n[dependencies]\nitoa = \"1\"\n---\nfn main() {}\n");
}

#[test]
fn add_and_remove_take_a_crates_sub_tables_with_it() {
    // Review B7: `[dependencies.serde.features]` stayed behind as an orphan / conflicting table.
    let dir = workdir("b7_subtable");
    let original = "---\n[dependencies]\nanyhow = \"1\"\n\n[dependencies.serde]\nversion = \"1\"\n\n[dependencies.serde.features]\nx = 1\n---\nfn main() {}\n";
    fs::write(dir.join("t.rs"), original).unwrap();
    assert_ok(&rsxtk(&dir, &["remove", "t.rs", "serde"]));
    assert_eq!(fs::read_to_string(dir.join("t.rs")).unwrap(), "---\n[dependencies]\nanyhow = \"1\"\n---\nfn main() {}\n");
}

const TWO_MEMORIES_WAT: &str = r#"(module
  (memory $a 1)
  (memory $b 1)
  (func (export "grow_both") (param i32) (result i32)
    (drop (memory.grow $a (local.get 0)))
    (memory.grow $b (local.get 0))))
"#;

#[test]
fn max_memory_is_one_budget_for_all_memories_together() {
    // Review N1 (round 2): caps were per memory, so many memories passed the setting many times.
    let dir = workdir("n1_total");
    fs::write(dir.join("m.wat"), TWO_MEMORIES_WAT).unwrap();
    // 600 pages = 37.5 MiB each: the first fits a 64 MiB budget, the second does not (-1).
    let capped = rsxtk(&dir, &["--max-memory", "64", "mod", "m.wat", "grow_both", "600"]);
    assert_ok(&capped);
    assert_eq!(stdout(&capped).trim(), "-1");
    let roomy = rsxtk(&dir, &["--max-memory", "128", "mod", "m.wat", "grow_both", "600"]);
    assert_ok(&roomy);
    assert_eq!(stdout(&roomy).trim(), "1");
}

const TABLE_WAT: &str = r#"(module
  (table 1 funcref)
  (func (export "grow") (param i32) (result i32)
    (table.grow (ref.null func) (local.get 0))))
"#;

#[test]
fn guest_tables_are_capped_by_max_memory_too() {
    // Review N1: `table.grow` took 19.5 GB of host memory under a 64 MiB cap.
    let dir = workdir("n1_tables");
    fs::write(dir.join("t.wat"), TABLE_WAT).unwrap();
    // 64 MiB / 16 = 4 Mi elements: growing by 10 million is refused (-1), by 1000 granted (old size 1).
    let refused = rsxtk(&dir, &["--max-memory", "64", "mod", "t.wat", "grow", "10000000"]);
    assert_ok(&refused);
    assert_eq!(stdout(&refused).trim(), "-1");
    let granted = rsxtk(&dir, &["--max-memory", "64", "mod", "t.wat", "grow", "1000"]);
    assert_ok(&granted);
    assert_eq!(stdout(&granted).trim(), "1");
}

#[test]
fn add_to_a_script_with_a_byte_order_mark_keeps_one_manifest() {
    // Review H2: a BOM hid the manifest, and `add` injected a second one in front of it.
    let dir = workdir("h2_bom");
    fs::write(dir.join("b.rs"), "\u{feff}---\n[dependencies]\n---\nfn main() {}\n").unwrap();
    assert_ok(&rsxtk(&dir, &["add", "b.rs", "anyhow", "1"]));
    let text = fs::read_to_string(dir.join("b.rs")).unwrap();
    assert_eq!(text, "---\n[dependencies]\nanyhow = \"1\"\n---\nfn main() {}\n");
}

#[test]
fn add_refuses_text_before_the_manifest_and_keeps_the_file() {
    // Review H2: a line before `---` made `add` inject a second manifest (exit 0).
    let dir = workdir("h2_leading");
    let original = "// note\n---\n[dependencies]\n---\nfn main() {}\n";
    fs::write(dir.join("l.rs"), original).unwrap();
    let out = rsxtk(&dir, &["add", "l.rs", "anyhow", "1"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("manifest must start the file"), "{}", stderr(&out));
    assert_eq!(fs::read_to_string(dir.join("l.rs")).unwrap(), original);
}

#[test]
fn fmt_of_an_unclosed_manifest_fails_without_a_panic() {
    // Review H3: `---` without a closing fence indexed past the end (exit 101).
    let dir = workdir("h3_unclosed");
    fs::write(dir.join("a.rs"), "---\nfn main(){}\n").unwrap();
    let out = rsxtk(&dir, &["fmt", "a.rs"]);
    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr(&out));
    assert!(stderr(&out).contains("Manifest block is incomplete"), "{}", stderr(&out));
}

#[test]
fn init_refuses_to_overwrite_an_existing_file() {
    // Review H4: `init keep` replaced keep.rs with the template.
    let dir = workdir("h4_init");
    fs::write(dir.join("keep.rs"), "precious").unwrap();
    let out = rsxtk(&dir, &["init", "keep"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("already exists"), "{}", stderr(&out));
    assert_eq!(fs::read_to_string(dir.join("keep.rs")).unwrap(), "precious");
    assert_fails(&rsxtk(&dir, &["initmod", "keep"]));
    assert_eq!(fs::read_to_string(dir.join("keep.rs")).unwrap(), "precious");
}

#[test]
fn fmt_leaves_a_users_rs_tmp_file_alone() {
    // Review H5: fmt wrote and then deleted a fixed `<name>.rs.tmp`.
    let dir = workdir("h5_tmp");
    fs::write(dir.join("x.rs"), "---\n[dependencies]\n---\nfn main(){}\n").unwrap();
    fs::write(dir.join("x.rs.tmp"), "user data").unwrap();
    assert_ok(&rsxtk(&dir, &["fmt", "x.rs"]));
    assert_eq!(fs::read_to_string(dir.join("x.rs.tmp")).unwrap(), "user data");
}

const GROW_WAT: &str = r#"(module
  (memory 1)
  (func (export "grow") (param i32) (result i32)
    (memory.grow (local.get 0))))
"#;

#[test]
fn guest_memory_is_capped_by_max_memory() {
    // Review H6 (owner: a memory cap, no time limit): memory.grow took gigabytes unchecked.
    let dir = workdir("h6_memory");
    fs::write(dir.join("grow.wat"), GROW_WAT).unwrap();
    // 1000 pages = 62.5 MiB: refused (-1) under a 16 MiB cap, granted (old size 1) under the default.
    let capped = rsxtk(&dir, &["--max-memory", "16", "mod", "grow.wat", "grow", "1000"]);
    assert_ok(&capped);
    assert_eq!(stdout(&capped).trim(), "-1");
    let default = rsxtk(&dir, &["mod", "grow.wat", "grow", "1000"]);
    assert_ok(&default);
    assert_eq!(stdout(&default).trim(), "1");
    // The default cap is 4096 MiB: 65536 pages (4 GiB) more are refused.
    let huge = rsxtk(&dir, &["mod", "grow.wat", "grow", "65536"]);
    assert_ok(&huge);
    assert_eq!(stdout(&huge).trim(), "-1");
}

#[test]
fn optimize_refuses_an_invalid_module_and_writes_nothing() {
    // Review H8: walrus panicked (exit 101) on some invalid modules; they are validated first now.
    let dir = workdir("h8_optimize");
    // Parses as wasm, but the function returns an i64 where an i32 is declared.
    fs::write(dir.join("bad.wasm"), wat::parse_str("(module (func (result i32) i64.const 1))").unwrap()).unwrap();
    let out = rsxtk(&dir, &["optimize", "-i", "bad.wasm", "-o", "out.wasm"]);
    assert_eq!(out.status.code(), Some(1), "stderr:\n{}", stderr(&out));
    assert!(stderr(&out).contains("not a valid WebAssembly module"), "{}", stderr(&out));
    assert!(!dir.join("out.wasm").exists());
}

#[test]
fn convert_refuses_to_overwrite_its_input() {
    // Review H9: `convert e.wasm -o e.wasm` replaced the binary with WAT text.
    let dir = workdir("h9_convert");
    let bytes = wat::parse_str(LIB_WAT).unwrap();
    fs::write(dir.join("e.wasm"), &bytes).unwrap();
    let out = rsxtk(&dir, &["convert", "e.wasm", "-o", "e.wasm"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("is the input file"), "{}", stderr(&out));
    assert_eq!(fs::read(dir.join("e.wasm")).unwrap(), bytes);
}

#[test]
fn a_truncated_script_cache_is_rebuilt() {
    // Review H10: a half-written cached module counted as fresh and failed on every run.
    let dir = workdir("h10_cache");
    fs::write(dir.join("hello.rs"), SCRIPT).unwrap();
    assert_ok(&rsxtk(&dir, &["run", "hello.rs"]));
    let cached = dir.join(".tk/hello/hello.wasm");
    let file = fs::OpenOptions::new().write(true).open(&cached).unwrap();
    file.set_len(100).unwrap();
    drop(file);
    let out = rsxtk(&dir, &["run", "hello.rs"]);
    assert_ok(&out);
    assert_eq!(stdout(&out), "script ran\n");
}

#[test]
fn add_refuses_an_invalid_crate_name_or_version() {
    // Review H11: names and versions went into the manifest unchecked (quotes, line breaks).
    let dir = workdir("h11_names");
    fs::write(dir.join("s.rs"), SCRIPT).unwrap();
    let bad_name = rsxtk(&dir, &["add", "s.rs", "evil\"\n[x]", "1"]);
    assert_fails(&bad_name);
    assert!(stderr(&bad_name).contains("not a valid crate name"), "{}", stderr(&bad_name));
    let bad_version = rsxtk(&dir, &["add", "s.rs", "anyhow", "1\"\nx = \"2"]);
    assert_fails(&bad_version);
    assert!(stderr(&bad_version).contains("not a valid version"), "{}", stderr(&bad_version));
    assert_eq!(fs::read_to_string(dir.join("s.rs")).unwrap(), SCRIPT);
}

#[test]
fn mod_refuses_extra_arguments() {
    // Review H13: `mod lib.wat add 1 2 3` silently dropped the 3.
    let dir = workdir("h13_mod_extra");
    fs::write(dir.join("lib.wat"), LIB_WAT).unwrap();
    let out = rsxtk(&dir, &["mod", "lib.wat", "add", "1", "2", "3"]);
    assert_fails(&out);
    assert!(stderr(&out).contains("takes 2 argument(s); 3 were given"), "{}", stderr(&out));
}
