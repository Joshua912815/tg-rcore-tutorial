# ai4ose-tg-rcore-tutorial-ch8-doom

`ai4ose-tg-rcore-tutorial-ch8-doom` is a publishable AI4OSE game experiment crate based on `tg-rcore-tutorial-ch8`.

It ports the user-space cross-platform Doom implementation from `doomgeneric`, extends the Chapter 8 tutorial kernel to support framebuffer presentation and input polling, and boots directly into Doom in QEMU.

## Release Metadata

- Crate name: `ai4ose-tg-rcore-tutorial-ch8-doom`
- Crate version: `0.1.0-preview.1`
- Git repository: `https://github.com/Joshua912815/tg-rcore-tutorial`
- Recommended git tag: `ch8-doom-crate-v0.1.0-preview.1`
- Crate docs/report paths:
  - `docs/ch8-doom-report.md`
  - `docs/ch8-doom-ai-log.md`

## What Is Included

This crate package contains everything needed to reproduce the experiment:

- the Chapter 8 kernel source with Doom support
- the DoomGeneric source snapshot under `vendor/doomgeneric/`
- the Doom platform glue under `doom/`
- the shareware `doom1.wad` asset used for the demo
- a bundled minimal user-space source snapshot under `bundled-user/`, used to materialize and build `initproc`
- the experiment report and AI collaboration log under `docs/`

The package is designed to work independently of the original workspace layout. It does not require sibling source directories outside this crate tarball.

## Prerequisites

You need the following tools installed:

1. Rust stable toolchain
2. `riscv64gc-unknown-none-elf` target
3. `qemu-system-riscv64`
4. Docker

Install the Rust target if needed:

```bash
rustup target add riscv64gc-unknown-none-elf
```

## Reproduce From crates.io

Install `cargo-clone` if you do not already have it:

```bash
cargo install cargo-clone
```

Then reproduce the crate directly from crates.io:

```bash
cargo clone ai4ose-tg-rcore-tutorial-ch8-doom
cd ai4ose-tg-rcore-tutorial-ch8-doom
cargo run
```

Or:

```bash
make run
```

## Reproduce From Git Tag

```bash
git clone https://github.com/Joshua912815/tg-rcore-tutorial.git
cd tg-rcore-tutorial/tg-rcore-tutorial-ch8
git checkout ch8-doom-crate-v0.1.0-preview.1
cargo run
```

Or:

```bash
make run
```

## Runtime Notes

- QEMU opens a graphical window for framebuffer output.
- The current Doom input path reads from the terminal running `cargo run`, not directly from the QEMU window.
- Basic controls:
  - `w` / `s`: move forward or back
  - `a` / `d`: turn left or right
  - `space` or `f`: fire
  - `e`: use / open door
  - `q` or `Esc`: menu
  - `Enter`: confirm

## Build And Packaging Notes

- `build.rs` builds the bundled `initproc`, compiles the Doom port, and packs `fs.img`.
- Doom C code is compiled inside Docker using the included `doom/Dockerfile`.
- The crate includes the bundled user-space source snapshot under `bundled-user/`. `build.rs` materializes a local `tg-rcore-tutorial-user/` work directory during build, so packaging and independent extraction do not rely on a parent workspace.

## Experiment Documents

- Report: [`docs/ch8-doom-report.md`](docs/ch8-doom-report.md)
- AI collaboration log: [`docs/ch8-doom-ai-log.md`](docs/ch8-doom-ai-log.md)

## Repository Layout

```text
ai4ose-tg-rcore-tutorial-ch8-doom/
├── .cargo/config.toml
├── Cargo.toml
├── Makefile
├── README.md
├── build.rs
├── docs/
├── doom/
├── src/
├── bundled-user/
├── support-crates/
└── vendor/doomgeneric/
```

## License

GPL-3.0
