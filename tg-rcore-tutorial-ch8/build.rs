use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use tg_easy_fs::{BlockDevice, EasyFileSystem};

const TARGET_ARCH: &str = "riscv64gc-unknown-none-elf";
const BLOCK_SZ: usize = 512;
const DOOM_IMAGE: &str = "tg-rcore-doom-build";
const FS_BLOCKS: u32 = 128 * 2048;

struct PackedArtifact {
    name: String,
    host_path: PathBuf,
}

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
    println!("cargo:rerun-if-env-changed=TG_SKIP_USER_APPS");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_EXERCISE");

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    // 只在 RISC-V64 架构上使用链接脚本
    if target_arch == "riscv64" {
        write_linker();
        if should_skip_build_apps() {
            return;
        }
        build_apps_and_pack_fs();
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

fn build_apps_and_pack_fs() {
    let tg_user_root = ensure_tg_user();
    let cases_path = tg_user_root.join("cases.toml");
    println!("cargo:rerun-if-changed={}", cases_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        tg_user_root.join("Cargo.toml").display()
    );
    println!("cargo:rerun-if-changed={}", tg_user_root.join("src").display());

    let cfg = fs::read_to_string(&cases_path).unwrap_or_else(|err| {
        panic!("failed to read cases.toml from {}: {}", cases_path.display(), err)
    });
    let mut cases_map: HashMap<String, Cases> = toml::from_str(&cfg).unwrap_or_else(|err| {
        panic!("failed to parse cases.toml: {err}")
    });

    let case_key = if env::var("CARGO_FEATURE_EXERCISE").is_ok() {
        "ch8_exercise"
    } else {
        "ch8"
    };
    let cases = cases_map.remove(case_key).unwrap_or_default();
    let base = cases.base.unwrap_or(0);
    let step = cases.step.unwrap_or(0);
    let names = cases.cases.unwrap_or_default();

    if names.is_empty() {
        panic!("no user cases found for {case_key} in {}", cases_path.display());
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let fs_target_dir = manifest_dir
        .join("target")
        .join(TARGET_ARCH)
        .join("debug");
    let app_target_dir = tg_user_root
        .join("target")
        .join(TARGET_ARCH)
        .join("debug");
    let mut artifacts = Vec::new();

    for (i, name) in names.iter().enumerate() {
        let base_address = base + i as u64 * step;
        build_user_app(&tg_user_root, name, base_address);
        artifacts.push(PackedArtifact {
            name: name.clone(),
            host_path: app_target_dir.join(name),
        });
    }

    build_doom_port(&manifest_dir, &fs_target_dir);
    artifacts.push(PackedArtifact {
        name: "doom".to_string(),
        host_path: fs_target_dir.join("doom"),
    });
    artifacts.extend(doom_assets(&manifest_dir));

    easy_fs_pack(&artifacts, &fs_target_dir).unwrap_or_else(|err| {
        panic!(
            "failed to pack easy-fs image in {}: {err}",
            fs_target_dir.display()
        )
    });
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

    if name == "initproc" {
        cmd.env("CHAPTER", "doom");
    }

    if base_address != 0 {
        cmd.env("BASE_ADDRESS", base_address.to_string());
    }

    let status = cmd.status().expect("failed to execute cargo build for user app");
    if !status.success() {
        panic!("failed to build user app {name}");
    }
}

struct BlockFile(std::sync::Mutex<std::fs::File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        use std::io::{Seek, SeekFrom, Write};
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }
}

fn easy_fs_pack(artifacts: &[PackedArtifact], fs_target: &PathBuf) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Read;
    use std::sync::Arc;

    fs::create_dir_all(fs_target)?;
    let fs_file = fs_target.join("fs.img");
    println!("cargo:rerun-if-changed={}", fs_file.display());
    let block_file = Arc::new(BlockFile(std::sync::Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(fs_file)?;
        f.set_len(FS_BLOCKS as u64 * BLOCK_SZ as u64).unwrap();
        f
    })));

    let efs = EasyFileSystem::create(block_file, FS_BLOCKS, 1);
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));

    for artifact in artifacts {
        let mut host_file = std::fs::File::open(&artifact.host_path)?;
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        let inode = root_inode.create(artifact.name.as_str()).unwrap();
        inode.write_at(0, all_data.as_slice());
    }

    Ok(())
}

fn build_doom_port(manifest_dir: &Path, fs_target_dir: &Path) {
    let doom_dir = manifest_dir.join("doom");
    let vendor_dir = manifest_dir.join("vendor").join("doomgeneric");
    let dockerfile = doom_dir.join("Dockerfile");
    let output = fs_target_dir.join("doom");

    println!("cargo:rerun-if-changed={}", doom_dir.display());
    println!("cargo:rerun-if-changed={}", vendor_dir.display());

    fs::create_dir_all(fs_target_dir).unwrap_or_else(|err| {
        panic!(
            "failed to create Doom output directory {}: {}",
            fs_target_dir.display(),
            err
        )
    });

    if !ensure_doom_builder_image(manifest_dir, &dockerfile) && output.exists() {
        println!(
            "cargo:warning=Docker unavailable; reusing cached Doom binary at {}",
            output.display()
        );
        return;
    }

    let mount_root = manifest_dir;
    let script = format!(
        "set -e\n\
         SRC=\"$(sed -n 's/^SRC_DOOM = //p' vendor/doomgeneric/Makefile | tr ' ' '\\n' | \
         sed 's/\\.o$/.c/' | grep -v 'doomgeneric_xlib.c' | \
         sed 's#^#vendor/doomgeneric/#' | tr '\\n' ' ')\"\n\
         mkdir -p target/{target_arch}/debug\n\
         riscv64-unknown-elf-gcc \
         -specs=/usr/lib/picolibc/riscv64-unknown-elf/picolibc.specs \
         -nostartfiles -march=rv64gc -mabi=lp64d -Os \
         -D_DEFAULT_SOURCE -DNORMALUNIX \
         -DDOOMGENERIC_RESX=640 -DDOOMGENERIC_RESY=400 \
         -Ivendor/doomgeneric -Idoom \
         -T doom/link.ld \
         doom/start.c doom/tg_picolibc.c doom/doomgeneric_tg.c $SRC \
         -o target/{target_arch}/debug/doom\n",
        target_arch = TARGET_ARCH,
    );

    let status = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/work", mount_root.display()),
            "-w",
            "/work",
            DOOM_IMAGE,
            "bash",
            "-lc",
            &script,
        ])
        .status()
        .unwrap_or_else(|err| panic!("failed to run Docker for Doom build: {err}"));

    if !status.success() {
        panic!("failed to build Doom port");
    }
}

fn ensure_doom_builder_image(manifest_dir: &Path, dockerfile: &Path) -> bool {
    let inspect_ok = Command::new("docker")
        .args(["image", "inspect", DOOM_IMAGE])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if inspect_ok {
        return true;
    }

    let status = Command::new("docker")
        .args([
            "build",
            "-t",
            DOOM_IMAGE,
            "-f",
            dockerfile.to_string_lossy().as_ref(),
            manifest_dir.to_string_lossy().as_ref(),
        ])
        .status();

    status.map(|s| s.success()).unwrap_or(false)
}

fn doom_assets(manifest_dir: &Path) -> Vec<PackedArtifact> {
    let asset_dir = manifest_dir.join("doom").join("assets");
    println!("cargo:rerun-if-changed={}", asset_dir.display());

    for name in ["doom1.wad", "freedoom1.wad"] {
        let host_path = asset_dir.join(name);
        if host_path.exists() {
            return vec![PackedArtifact {
                name: name.to_string(),
                host_path,
            }];
        }
    }

    panic!(
        "no IWAD found in {}; add doom1.wad or freedoom1.wad",
        asset_dir.display()
    );
}

fn ensure_tg_user() -> PathBuf {
    // 优先使用 TG_USER_DIR 显式指定的目录
    if let Ok(dir) = env::var("TG_USER_DIR") {
        let path = PathBuf::from(dir);
        if path.join("Cargo.toml").exists() {
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
    let bundled_user_dir = manifest_dir.join("bundled-user");

    // 本地缓存目录已存在则直接使用
    if tg_user_dir.join("Cargo.toml").exists() {
        ensure_workspace_table(&tg_user_dir);
        return tg_user_dir;
    }

    if bundled_user_dir.exists() {
        materialize_bundled_user(&manifest_dir, &bundled_user_dir, &tg_user_dir);
        ensure_workspace_table(&tg_user_dir);
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

    if !tg_user_dir.join("Cargo.toml").exists() {
        panic!(
            "{crate_spec} clone did not produce a valid crate at {}",
            tg_user_dir.display()
        );
    }

    // 克隆后补加 [workspace]，防止父 workspace 将其识别为非成员而报错
    ensure_workspace_table(&tg_user_dir);

    tg_user_dir
}

fn materialize_bundled_user(manifest_dir: &Path, bundled_user_dir: &Path, target_dir: &Path) {
    copy_dir_all(bundled_user_dir, target_dir).unwrap_or_else(|err| {
        panic!(
            "failed to materialize bundled user sources from {} to {}: {}",
            bundled_user_dir.display(),
            target_dir.display(),
            err
        )
    });

    let syscall_local = manifest_dir
        .join("support-crates")
        .join("ai4ose-tg-rcore-tutorial-syscall-doom")
        .join("Cargo.toml");

    let syscall_dep = if syscall_local.exists() {
        r#"tg-syscall = { package = "ai4ose-tg-rcore-tutorial-syscall-doom", path = "../support-crates/ai4ose-tg-rcore-tutorial-syscall-doom", version = "0.1.0-preview.1", features = ["user"] }"#
            .to_string()
    } else {
        r#"tg-syscall = { package = "ai4ose-tg-rcore-tutorial-syscall-doom", version = "0.1.0-preview.1", features = ["user"] }"#
            .to_string()
    };

    let cargo_toml = format!(
        r#"[package]
name = "tg-rcore-tutorial-user"
description = "Minimal user-space runtime snapshot bundled with the AI4OSE ch8-doom crate."
version = "0.1.0-preview.1"
edition = "2024"
authors = ["Joshua"]
repository = "https://github.com/Joshua912815/tg-rcore-tutorial"
homepage = "https://github.com/Joshua912815/tg-rcore-tutorial/tree/ch8-doom/tg-rcore-tutorial-ch8"
documentation = "https://github.com/Joshua912815/tg-rcore-tutorial/tree/ch8-doom/tg-rcore-tutorial-ch8/bundled-user"
license = "GPL-3.0"
readme = "README.md"

[lib]
name = "user_lib"
path = "src/lib.rs"

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"

[dependencies]
tg-console = {{ package = "tg-rcore-tutorial-console", version = "0.4.8" }}
{syscall_dep}
customizable-buddy = "0.0.2"
"#,
        syscall_dep = syscall_dep,
    );

    fs::write(target_dir.join("Cargo.toml"), cargo_toml).unwrap_or_else(|err| {
        panic!(
            "failed to write generated bundled-user Cargo.toml in {}: {}",
            target_dir.display(),
            err
        )
    });

    let cases_toml = r#"[ch8]
cases = [
    "initproc",
]

[ch8_exercise]
cases = [
    "initproc",
]
"#;
    fs::write(target_dir.join("cases.toml"), cases_toml).unwrap_or_else(|err| {
        panic!(
            "failed to write generated bundled-user cases.toml in {}: {}",
            target_dir.display(),
            err
        )
    });
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// 若 Cargo.toml 末尾尚无 [workspace] 表，则追加一个空的，
/// 使该 crate 成为独立 workspace 根，避免父 workspace 冲突。
fn ensure_workspace_table(dir: &PathBuf) {
    let cargo_toml = dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml).unwrap_or_default();
    if !content.contains("[workspace]") {
        fs::write(&cargo_toml, format!("{}
[workspace]
", content))
            .unwrap_or_else(|err| panic!("failed to patch Cargo.toml in {}: {}", dir.display(), err));
    }
}
