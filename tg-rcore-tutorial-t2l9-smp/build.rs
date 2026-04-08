use std::{env, fs, path::PathBuf, process::Command};

const TARGET_ARCH: &str = "riscv64gc-unknown-none-elf";
const APP_BASE: u64 = 0x8040_0000;
const APP_NAMES: &[&str] = &["00hello_world", "08power_3", "09power_5", "10power_7"];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=user-src/src");

    if env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() != "riscv64" {
        return;
    }

    write_linker();
    build_apps();
}

fn write_linker() {
    let ld = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
    fs::write(&ld, tg_linker::SCRIPT)
        .unwrap_or_else(|err| panic!("failed to write linker script to {}: {err}", ld.display()));
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}

fn build_apps() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let template_root = manifest_dir.join("user-src");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let user_root = out_dir.join("generated-user");
    prepare_user_crate(&template_root, &user_root);
    let target_dir = user_root.join("target").join(TARGET_ARCH).join("debug");
    let mut bins = Vec::with_capacity(APP_NAMES.len());

    for name in APP_NAMES {
        let status = Command::new("cargo")
            .args([
                "build",
                "--manifest-path",
                user_root.join("Cargo.toml").to_string_lossy().as_ref(),
                "--bin",
                name,
                "--target",
                TARGET_ARCH,
            ])
            .env("BASE_ADDRESS", APP_BASE.to_string())
            .status()
            .expect("failed to execute cargo build for user app");
        assert!(status.success(), "failed to build user app {name}");

        let elf = target_dir.join(name);
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
        assert!(status.success(), "rust-objcopy failed for {}", elf.display());
        bins.push(bin);
    }

    let app_asm = out_dir.join("app.asm");
    write_app_asm(&app_asm, &bins);
    println!("cargo:rustc-env=APP_ASM={}", app_asm.display());
}

fn prepare_user_crate(template_root: &PathBuf, user_root: &PathBuf) {
    if user_root.exists() {
        fs::remove_dir_all(user_root)
            .unwrap_or_else(|err| panic!("failed to clean {}: {err}", user_root.display()));
    }
    fs::create_dir_all(user_root.join("src").join("bin"))
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", user_root.display()));

    fs::write(user_root.join("Cargo.toml"), generated_user_manifest())
        .unwrap_or_else(|err| panic!("failed to write generated user Cargo.toml: {err}"));
    fs::write(user_root.join("build.rs"), generated_user_build_rs())
        .unwrap_or_else(|err| panic!("failed to write generated user build.rs: {err}"));

    copy_dir_recursive(&template_root.join("src"), &user_root.join("src"));
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) {
    for entry in fs::read_dir(src).unwrap_or_else(|err| panic!("failed to read {}: {err}", src.display())) {
        let entry = entry.unwrap_or_else(|err| panic!("failed to read dir entry in {}: {err}", src.display()));
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)
                .unwrap_or_else(|err| panic!("failed to create {}: {err}", dst_path.display()));
            copy_dir_recursive(&src_path, &dst_path);
        } else {
            fs::copy(&src_path, &dst_path).unwrap_or_else(|err| {
                panic!(
                    "failed to copy template {} -> {}: {err}",
                    src_path.display(),
                    dst_path.display()
                )
            });
        }
    }
}

fn generated_user_manifest() -> &'static str {
    r#"[package]
name = "joshua912815-rcore-tutorial-t2l9-smp-user"
version = "0.1.0-preview.3"
edition = "2024"
publish = false

[lib]
name = "user_lib"
path = "src/lib.rs"

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"

[dependencies]
tg-console = { package = "tg-rcore-tutorial-console", version = "0.4.8" }
tg-syscall = { package = "tg-rcore-tutorial-syscall", version = "0.4.8", features = ["user"] }
customizable-buddy = "0.0.2"
"#
}

fn generated_user_build_rs() -> &'static str {
    r#"fn main() {
    use std::{env, fs, path::PathBuf};

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=BASE_ADDRESS");

    if let Some(base) = env::var("BASE_ADDRESS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
    {
        let text = format!(
            "\
OUTPUT_ARCH(riscv)
ENTRY(_start)
SECTIONS {{
    . = {base};
    .text : {{
        *(.text.entry)
        *(.text .text.*)
    }}
    .rodata : {{
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }}
    .data : {{
        *(.data .data.*)
        *(.sdata .sdata.*)
    }}
    .bss : {{
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }}
}}"
        );
        let ld = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
        fs::write(&ld, text).unwrap();
        println!("cargo:rustc-link-arg=-T{}", ld.display());
    }
}"#
}

fn write_app_asm(path: &PathBuf, bins: &[PathBuf]) {
    use std::io::Write;

    let mut asm = fs::File::create(path)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", path.display()));

    writeln!(
        asm,
        "\
.global apps
.section .data
.align 3
apps:
    .quad {APP_BASE:#x}
    .quad 0
    .quad {}",
        bins.len(),
    )
    .unwrap();

    for i in 0..bins.len() {
        writeln!(asm, "    .quad app_{i}_start").unwrap();
    }
    writeln!(asm, "    .quad app_{}_end", bins.len() - 1).unwrap();

    for (i, bin) in bins.iter().enumerate() {
        writeln!(
            asm,
            "\
app_{i}_start:
    .incbin {bin:?}
app_{i}_end:"
        )
        .unwrap();
    }
}
