use flate2::read::GzDecoder;
use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::PathBuf, process::Command};
use tar::Archive;

const TARGET_ARCH: &str = "riscv64gc-unknown-none-elf";

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
    println!("cargo:rerun-if-env-changed=TG_USER_VERSION");
    println!("cargo:rerun-if-env-changed=TG_USER_CRATE");
    println!("cargo:rerun-if-env-changed=TG_USER_LOCAL_DIR");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_EXERCISE");
    println!("cargo:rerun-if-changed=assets/tg-rcore-tutorial-user.tar.gz");

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
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
    is_packaged_build()
}

fn write_linker() {
    let ld = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
    fs::write(&ld, tg_linker::NOBIOS_SCRIPT).unwrap_or_else(|err| {
        panic!("failed to write linker script to {}: {}", ld.display(), err)
    });
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}

fn is_packaged_build() -> bool {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let out_dir = out_dir.to_string_lossy();

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let manifest_dir = manifest_dir.to_string_lossy();

    out_dir.contains("/target/package/")
        || out_dir.contains("\\target\\package\\")
        || manifest_dir.contains("/target/package/")
        || manifest_dir.contains("\\target\\package\\")
}

fn build_apps() {
    let tg_user_root = ensure_tg_user();
    let cases_path = tg_user_root.join("cases.toml");
    println!("cargo:rerun-if-changed={}", cases_path.display());
    println!("cargo:rerun-if-changed={}", tg_user_root.join("Cargo.toml").display());
    println!("cargo:rerun-if-changed={}", tg_user_root.join("src").display());

    let cfg = fs::read_to_string(&cases_path).unwrap_or_else(|err| {
        panic!("failed to read cases.toml from {}: {}", cases_path.display(), err)
    });
    let mut cases_map: HashMap<String, Cases> = toml::from_str(&cfg).unwrap_or_else(|err| {
        panic!("failed to parse cases.toml: {err}")
    });

    let case_key = if env::var("CARGO_FEATURE_EXERCISE").is_ok() {
        "ch3_exercise"
    } else {
        "ch3"
    };
    let cases = cases_map.remove(case_key).unwrap_or_default();
    let base = cases.base.unwrap_or(0);
    let step = cases.step.unwrap_or(0);
    let names = cases.cases.unwrap_or_default();

    if names.is_empty() {
        panic!("no user cases found for {case_key} in {}", cases_path.display());
    }

    let target_dir = tg_user_root.join("target").join(TARGET_ARCH).join("debug");
    let mut bins: Vec<PathBuf> = Vec::with_capacity(names.len());

    for (i, name) in names.iter().enumerate() {
        let base_address = base + i as u64 * step;
        build_user_app(&tg_user_root, name, base_address);
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
        tg_user_root.join("Cargo.toml").to_string_lossy().as_ref(),
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

fn objcopy_to_bin(elf: &PathBuf) -> PathBuf {
    let bin = elf.with_extension("bin");
    let status = Command::new("rust-objcopy")
        .args([
            elf.to_string_lossy().as_ref(),
            "--strip-all",
            "-O",
            "binary",
            bin.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("failed to execute rust-objcopy");
    if !status.success() {
        panic!("rust-objcopy failed for {}", elf.display());
    }
    bin
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
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());

    if let Ok(dir) = env::var("TG_USER_DIR") {
        let path = resolve_manifest_relative(&manifest_dir, dir);
        if path.join("Cargo.toml").exists() {
            return path;
        }
    }

    let vendored = manifest_dir.join("vendor").join("tg-rcore-tutorial-user");
    if vendored.join("Cargo.toml").exists() {
        return vendored;
    }

    if let Some(extracted) = extract_tg_user_archive(&manifest_dir) {
        return extracted;
    }

    let crate_name = env::var("TG_USER_CRATE")
        .expect("TG_USER_CRATE not set; add it to .cargo/config.toml [env]");
    let local_dir_name = env::var("TG_USER_LOCAL_DIR")
        .expect("TG_USER_LOCAL_DIR not set; add it to .cargo/config.toml [env]");
    let version = env::var("TG_USER_VERSION")
        .expect("TG_USER_VERSION not set; add it to .cargo/config.toml [env]");

    let tg_user_dir = resolve_manifest_relative(&manifest_dir, local_dir_name);
    if tg_user_dir.join("Cargo.toml").exists() {
        ensure_workspace_table(&tg_user_dir);
        return tg_user_dir;
    }

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

    if !tg_user_dir.join("Cargo.toml").exists() {
        panic!(
            "{crate_spec} clone did not produce a valid crate at {}",
            tg_user_dir.display()
        );
    }

    ensure_workspace_table(&tg_user_dir);
    tg_user_dir
}

fn resolve_manifest_relative(manifest_dir: &PathBuf, path: String) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        manifest_dir.join(path)
    }
}

fn ensure_workspace_table(dir: &PathBuf) {
    let cargo_toml = dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml).unwrap_or_default();
    if !content.contains("[workspace]") {
        fs::write(&cargo_toml, format!("{content}\n[workspace]\n"))
            .unwrap_or_else(|e| panic!("failed to append [workspace] to {}: {e}", cargo_toml.display()));
    }
}

fn extract_tg_user_archive(manifest_dir: &PathBuf) -> Option<PathBuf> {
    let archive_path = manifest_dir.join("assets").join("tg-rcore-tutorial-user.tar.gz");
    if !archive_path.exists() {
        return None;
    }

    let extracted_root = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("tg-user-archive");
    let extracted_dir = extracted_root.join("tg-rcore-tutorial-user");
    if extracted_dir.join("Cargo.toml").exists() {
        ensure_workspace_table(&extracted_dir);
        return Some(extracted_dir);
    }

    if extracted_root.exists() {
        fs::remove_dir_all(&extracted_root).unwrap_or_else(|e| {
            panic!(
                "failed to remove old extracted tg-user archive at {}: {e}",
                extracted_root.display()
            )
        });
    }
    fs::create_dir_all(&extracted_root).unwrap_or_else(|e| {
        panic!(
            "failed to create extraction directory for tg-user archive at {}: {e}",
            extracted_root.display()
        )
    });

    let file = fs::File::open(&archive_path)
        .unwrap_or_else(|e| panic!("failed to open {}: {e}", archive_path.display()));
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(&extracted_root).unwrap_or_else(|e| {
        panic!(
            "failed to unpack tg-user archive {} into {}: {e}",
            archive_path.display(),
            extracted_root.display()
        )
    });

    if !extracted_dir.join("Cargo.toml").exists() {
        panic!(
            "tg-user archive {} did not produce a crate at {}",
            archive_path.display(),
            extracted_dir.display()
        );
    }

    ensure_workspace_table(&extracted_dir);
    Some(extracted_dir)
}
