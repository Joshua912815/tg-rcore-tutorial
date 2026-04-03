use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::PathBuf, process::Command};

const TARGET_ARCH: &str = "riscv64gc-unknown-none-elf";
const TG_USER_MANIFEST: &str = "Cargo.toml";
const TG_USER_TEMPLATE: &str = "Cargo.user.toml";

// 以下三项均从 .cargo/config.toml [env] 节读取，不再硬编码：
//   TG_USER_CRATE     — crates.io 包名
//   TG_USER_LOCAL_DIR — 本地缓存目录名
//   TG_USER_VERSION   — 版本号

#[derive(Deserialize, Default)]
struct Cases {
    base: Option<u64>,
    step: Option<u64>,
    cases: Option<Vec<String>>,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LOG");
    println!("cargo:rerun-if-env-changed=TG_USER_DIR");
    println!("cargo:rerun-if-env-changed=TG_USER_CRATE");
    println!("cargo:rerun-if-env-changed=TG_USER_LOCAL_DIR");
    println!("cargo:rerun-if-env-changed=TG_USER_VERSION");

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    // 只在 RISC-V64 架构上使用链接脚本
    if target_arch == "riscv64" {
        write_linker();
        if should_skip_build_apps() {
            write_dummy_app_asm();
        } else {
            build_apps();
        }
    }
}

fn should_skip_build_apps() -> bool {
    if env::var_os("TG_SKIP_USER_APPS").is_some() {
        return true;
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let manifest_dir = manifest_dir.to_string_lossy();
    manifest_dir.contains("/target/package/") || manifest_dir.contains("\\target\\package\\")
}

fn write_linker() {
    let ld = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
    fs::write(&ld, tg_linker::NOBIOS_SCRIPT).unwrap_or_else(|err| {
        panic!("failed to write linker script to {}: {}", ld.display(), err)
    });
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}

fn build_apps() {
    let tg_user_source = ensure_tg_user();
    let cases_path = tg_user_source.join("cases.toml");
    println!("cargo:rerun-if-changed={}", cases_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        tg_user_source.join(TG_USER_TEMPLATE).display()
    );
    println!("cargo:rerun-if-changed={}", tg_user_source.join("src").display());

    let cfg = fs::read_to_string(&cases_path).unwrap_or_else(|err| {
        panic!("failed to read cases.toml from {}: {}", cases_path.display(), err)
    });
    let mut cases_map: HashMap<String, Cases> = toml::from_str(&cfg).unwrap_or_else(|err| {
        panic!("failed to parse cases.toml: {err}")
    });

    let cases = cases_map.remove("ch2").unwrap_or_default();
    let base = cases.base.unwrap_or(0);
    let step = cases.step.unwrap_or(0);
    let names = cases.cases.unwrap_or_default();

    if names.is_empty() {
        panic!("no user cases found for ch2 in {}", cases_path.display());
    }

    let tg_user_build_root = prepare_tg_user_build_dir(&tg_user_source);
    let target_dir = tg_user_build_root.join("target").join(TARGET_ARCH).join("debug");
    let mut bins: Vec<PathBuf> = Vec::with_capacity(names.len());

    for (i, name) in names.iter().enumerate() {
        let base_address = base + i as u64 * step;
        build_user_app(&tg_user_build_root, name, base_address);
        let elf = target_dir.join(name);
        let app_path = if base_address != 0 {
            objcopy_to_bin(&elf)
        } else {
            elf
        };
        bins.push(app_path);
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let app_asm = out_dir.join("app.asm");
    write_app_asm(&app_asm, base, step, &bins);
    println!("cargo:rustc-env=APP_ASM={}", app_asm.display());
}

fn build_user_app(tg_user_root: &PathBuf, name: &str, base_address: u64) {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "build",
        "--manifest-path",
        tg_user_root.join(TG_USER_MANIFEST).to_string_lossy().as_ref(),
        "--bin",
        name,
        "--target",
        TARGET_ARCH,
    ]);

    if base_address != 0 {
        cmd.env("BASE_ADDRESS", base_address.to_string());
    }

    let status = cmd.status().expect("failed to execute cargo build for user app");
    if !status.success() {
        panic!("failed to build user app {name}");
    }
}

fn prepare_tg_user_build_dir(source_root: &PathBuf) -> PathBuf {
    let build_root = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("tg-user-build");
    if build_root.exists() {
        fs::remove_dir_all(&build_root)
            .unwrap_or_else(|err| panic!("failed to reset {}: {}", build_root.display(), err));
    }
    fs::create_dir_all(&build_root)
        .unwrap_or_else(|err| panic!("failed to create {}: {}", build_root.display(), err));

    copy_file_if_exists(source_root, &build_root, "README.md");
    copy_file_if_exists(source_root, &build_root, "build.rs");
    copy_file_if_exists(source_root, &build_root, "cases.toml");
    copy_file_if_exists(source_root, &build_root, "Cargo.lock");

    let cargo_dir = source_root.join(".cargo");
    if cargo_dir.exists() {
        copy_dir_all(&cargo_dir, &build_root.join(".cargo"));
    }

    copy_dir_all(&source_root.join("src"), &build_root.join("src"));

    let manifest_src = if source_root.join(TG_USER_TEMPLATE).exists() {
        source_root.join(TG_USER_TEMPLATE)
    } else {
        source_root.join(TG_USER_MANIFEST)
    };
    fs::copy(&manifest_src, build_root.join(TG_USER_MANIFEST)).unwrap_or_else(|err| {
        panic!(
            "failed to copy tg-user manifest from {}: {}",
            manifest_src.display(),
            err
        )
    });
    ensure_workspace_table(&build_root);
    build_root
}

fn copy_file_if_exists(source_root: &PathBuf, target_root: &PathBuf, relative: &str) {
    let src = source_root.join(relative);
    if src.exists() {
        let dst = target_root.join(relative);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("failed to create {}: {}", parent.display(), err));
        }
        fs::copy(&src, &dst)
            .unwrap_or_else(|err| panic!("failed to copy {} to {}: {}", src.display(), dst.display(), err));
    }
}

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) {
    fs::create_dir_all(dst)
        .unwrap_or_else(|err| panic!("failed to create {}: {}", dst.display(), err));

    for entry in fs::read_dir(src).unwrap_or_else(|err| panic!("failed to read {}: {}", src.display(), err)) {
        let entry = entry.unwrap_or_else(|err| panic!("failed to access {}: {}", src.display(), err));
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if entry
            .file_type()
            .unwrap_or_else(|err| panic!("failed to inspect {}: {}", path.display(), err))
            .is_dir()
        {
            copy_dir_all(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap_or_else(|err| {
                panic!("failed to copy {} to {}: {}", path.display(), target.display(), err)
            });
        }
    }
}

fn objcopy_to_bin(elf: &PathBuf) -> PathBuf {
    let bin = elf.with_extension("bin");
    let objcopy = find_objcopy();
    let status = Command::new(&objcopy)
        .args([
            elf.to_string_lossy().as_ref(),
            "--strip-all",
            "-O",
            "binary",
            bin.to_string_lossy().as_ref(),
        ])
        .status()
        .unwrap_or_else(|err| panic!("failed to execute {}: {}", objcopy.display(), err));
    if !status.success() {
        panic!("{} failed for {}", objcopy.display(), elf.display());
    }
    bin
}

fn find_objcopy() -> PathBuf {
    if let Some(path) = env::var_os("RUST_OBJCOPY") {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }
    }

    if command_exists("rust-objcopy") {
        return PathBuf::from("rust-objcopy");
    }

    let sysroot = command_output("rustc", &["--print", "sysroot"]);
    let host = env::var("HOST").expect("HOST is not set by cargo");
    let llvm_objcopy = PathBuf::from(sysroot)
        .join("lib")
        .join("rustlib")
        .join(host)
        .join("bin")
        .join("llvm-objcopy");
    if llvm_objcopy.exists() {
        return llvm_objcopy;
    }

    panic!(
        "cannot find objcopy tool; install cargo-binutils or llvm-tools-preview, \
or set RUST_OBJCOPY to a usable binary"
    );
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn command_output(name: &str, args: &[&str]) -> String {
    let output = Command::new(name)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("failed to execute {}: {}", name, err));
    if !output.status.success() {
        panic!("{} {:?} failed", name, args);
    }
    String::from_utf8(output.stdout)
        .unwrap_or_else(|err| panic!("{} {:?} returned non-utf8 output: {}", name, args, err))
        .trim()
        .to_owned()
}

fn write_app_asm(path: &PathBuf, base: u64, step: u64, bins: &[PathBuf]) {
    use std::io::Write;
    let mut asm = fs::File::create(path)
        .unwrap_or_else(|err| panic!("failed to create {}: {}", path.display(), err));

    writeln!(
        asm,
        "\
.global apps
.section .data
.align 3
apps:
    .quad {base:#x}
    .quad {step:#x}
    .quad {}",
        bins.len(),
    )
    .unwrap();

    for i in 0..bins.len() {
        writeln!(asm, "    .quad app_{i}_start").unwrap();
    }

    writeln!(asm, "    .quad app_{}_end", bins.len() - 1).unwrap();

    for (i, path) in bins.iter().enumerate() {
        writeln!(
            asm,
            "\
app_{i}_start:
    .incbin {path:?}
app_{i}_end:",
        )
        .unwrap();
    }
}

fn write_dummy_app_asm() {
    use std::io::Write;

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let app_asm = out_dir.join("app.asm");
    let mut asm = fs::File::create(&app_asm)
        .unwrap_or_else(|err| panic!("failed to create {}: {}", app_asm.display(), err));

    writeln!(
        asm,
        "\
.global apps
.section .data
.align 3
apps:
    .quad 0
    .quad 0
    .quad 0
    .quad 0"
    )
    .unwrap();

    println!("cargo:rustc-env=APP_ASM={}", app_asm.display());
}

fn ensure_tg_user() -> PathBuf {
    // 优先使用 TG_USER_DIR 显式指定的目录
    if let Ok(dir) = env::var("TG_USER_DIR") {
        let path = PathBuf::from(dir);
        if path.join(TG_USER_TEMPLATE).exists() || path.join(TG_USER_MANIFEST).exists() {
            return path;
        }
    }

    // 从 .cargo/config.toml [env] 读取三个配置项
    let crate_name = env::var("TG_USER_CRATE")
        .expect("TG_USER_CRATE not set; add it to .cargo/config.toml [env]");
    let local_dir_name = env::var("TG_USER_LOCAL_DIR")
        .expect("TG_USER_LOCAL_DIR not set; add it to .cargo/config.toml [env]");
    let version = env::var("TG_USER_VERSION")
        .expect("TG_USER_VERSION not set; add it to .cargo/config.toml [env]");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let tg_user_dir = manifest_dir.join(&local_dir_name);

    // 本地缓存目录已存在则直接使用
    if tg_user_dir.join(TG_USER_TEMPLATE).exists() || tg_user_dir.join(TG_USER_MANIFEST).exists() {
        return tg_user_dir;
    }

    // 从 crates.io 克隆指定包
    let crate_spec = format!("{crate_name}@{version}");
    let status = Command::new("cargo")
        .args([
            "clone",
            crate_spec.as_str(),
            "--",
            tg_user_dir.to_string_lossy().as_ref(),
        ])
        .status()
        .unwrap_or_else(|e| panic!("failed to execute cargo clone {crate_spec}: {e}"));

    if !status.success() {
        panic!(
            "failed to clone {crate_spec} into {}; ensure cargo-clone is installed or set TG_USER_DIR",
            tg_user_dir.display()
        );
    }

    if !tg_user_dir.join(TG_USER_MANIFEST).exists() {
        panic!(
            "{crate_spec} clone did not produce a valid crate at {}",
            tg_user_dir.display()
        );
    }

    tg_user_dir
}

/// 若 Cargo.toml 末尾尚无 [workspace] 表，则追加一个空的，
/// 使该 crate 成为独立 workspace 根，避免父 workspace 冲突。
fn ensure_workspace_table(dir: &PathBuf) {
    let cargo_toml = dir.join(TG_USER_MANIFEST);
    let content = fs::read_to_string(&cargo_toml).unwrap_or_default();
    if !content.contains("[workspace]") {
        fs::write(&cargo_toml, format!("{}\n[workspace]\n", content))
            .unwrap_or_else(|err| panic!("failed to patch Cargo.toml in {}: {}", dir.display(), err));
    }
}
