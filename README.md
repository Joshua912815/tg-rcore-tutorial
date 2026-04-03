# joshua912815-tg-rcore-tutorial-ch8-basic

`joshua912815-tg-rcore-tutorial-ch8-basic` is a publishable Chapter 8 basic experiment crate for AI4OSE.

It packages the completed `tg-rcore-tutorial` Chapter 8 kernel experiment as a standalone crate root, including:

- thread support based on the `Process` + `Thread` split
- mutex, semaphore, and condvar system calls
- the Chapter 8 deadlock detection exercise
- a local snapshot of `tg-rcore-tutorial-user` for reproducible builds
- experiment documents that describe the implementation, verification, and AI collaboration process

## Release Metadata

- Crate name: `joshua912815-tg-rcore-tutorial-ch8-basic`
- Crate version: `0.8.0-preview.4`
- Git repository: `https://github.com/Joshua912815/tg-rcore-tutorial`
- Recommended git tag: `ch8-basic-crate-v0.8.0-preview.4`
- Documentation on docs.rs: `https://docs.rs/joshua912815-tg-rcore-tutorial-ch8-basic`
- Included report paths:
  - `docs/ch8-basic-report.md`
  - `docs/ch8-basic-ai-log.md`

## What This Crate Solves

This crate corresponds to the Chapter 8 basic experiment, not the Doom extension version.

The key completed tasks are:

1. thread creation, waiting, and scheduling
2. mutex / semaphore / condvar synchronization
3. process-level deadlock detection enable switch
4. deadlock rejection in `mutex_lock` and `semaphore_down`, returning `-0xdead`
5. preservation of Chapter 8 base functionality and exercise tests

## Reproduce From crates.io

Install `cargo-clone` if needed:

```bash
cargo install cargo-clone
```

Then:

```bash
cargo clone joshua912815-tg-rcore-tutorial-ch8-basic
cd joshua912815-tg-rcore-tutorial-ch8-basic
cargo run
```

Or:

```bash
make run
```

Exercise mode:

```bash
cargo run --features exercise
```

## Reproduce From Git Tag

```bash
git clone --branch ch8-basic-crate-v0.8.0-preview.4 --depth 1 https://github.com/Joshua912815/tg-rcore-tutorial.git
cd tg-rcore-tutorial
cargo run
```

Or:

```bash
make run
```

## Prerequisites

You need:

1. Rust stable
2. `riscv64gc-unknown-none-elf` target
3. `qemu-system-riscv64`

Install the target if needed:

```bash
rustup target add riscv64gc-unknown-none-elf
```

## Build Notes

- `.cargo/config.toml` already sets the RISC-V target and QEMU runner.
- `build.rs` builds the user programs and packs them into `fs.img`.
- This crate includes a local `tg-rcore-tutorial-user` snapshot, so `cargo run` does not depend on a parent workspace layout.

## Verification Summary

The completed Chapter 8 experiment was verified with:

- `cargo check --features exercise`
- `./test.sh base` -> `Test PASSED: 22/22`
- `./test.sh exercise` -> `Test PASSED: 25/25`

Key exercise outputs include:

- `deadlock test mutex 1 OK!`
- `deadlock test semaphore 1 OK!`
- `deadlock test semaphore 2 OK!`

## Included Documents

- Report: [`docs/ch8-basic-report.md`](docs/ch8-basic-report.md)
- AI collaboration notes: [`docs/ch8-basic-ai-log.md`](docs/ch8-basic-ai-log.md)

## License

GPL-3.0
