# Chapter 8 Basic Experiment Report

## 1. Experiment Goal

This crate packages the completed Chapter 8 basic experiment of `tg-rcore-tutorial`.

The main goals are:

1. complete the Chapter 8 threading and synchronization kernel
2. implement `enable_deadlock_detect`
3. detect deadlock risks in `mutex_lock` and `semaphore_down`
4. keep Chapter 8 base tests and exercise tests passing

## 2. Main Implementation Points

### 2.1 Thread and process split

Chapter 8 separates the old process abstraction into:

- `Process`: shared resources such as address space, fd table, signal handlers, and synchronization primitives
- `Thread`: independent execution context and TID

This makes it possible for multiple threads to share one process resource container while still being scheduled independently.

### 2.2 Synchronization primitives

The kernel supports:

- blocking mutex
- semaphore
- condvar

Blocking on `mutex_lock`, `semaphore_down`, or `condvar_wait` moves the current thread out of the ready queue. Unlock or signal paths re-enqueue the waiting thread.

### 2.3 Deadlock detection

The exercise part adds a per-process deadlock detection state.

For mutex:

- the kernel tracks mutex owners
- the kernel tracks which mutex a thread is waiting for
- deadlock is detected by following the wait chain and checking for cycles

For semaphore:

- the kernel tracks total resources per semaphore
- the kernel tracks per-thread held resources
- the kernel tracks the current outstanding request
- the kernel checks safety using an `Available / Allocation / Need` style algorithm

If a request would make the system unsafe, the kernel rejects it and returns `-0xdead`.

## 3. Verification

The implementation was verified in `rcore-docker`.

Results:

- `cargo check --features exercise` passed
- `./test.sh base` passed with `22/22`
- `./test.sh exercise` passed with `25/25`

Important exercise outputs:

- `deadlock test mutex 1 OK!`
- `deadlock test semaphore 1 OK!`
- `deadlock test semaphore 2 OK!`

## 4. Reproducibility

This crate is intended to be reproducible from:

1. crates.io via `cargo clone`
2. the tagged git revision `ch8-basic-crate-v0.8.0-preview.1`

The included `tg-rcore-tutorial-user` snapshot and local build script make the crate runnable without requiring the original multi-crate workspace layout.

