//! T2L8 engineering-quality teaching bundle for tg-rcore-tutorial.
//!
//! This crate packages the full set of sub-crates needed to reproduce the
//! T2L8 experiment, including:
//!
//! - unified `[EVENT]` / `[METRIC]` tracing support,
//! - user workloads for `ch3`, `ch4`, and `ch8`,
//! - regression scripts,
//! - design report and debugging documents.
//!
//! # Usage
//!
//! After cloning this crate via cargo:
//!
//! ```bash
//! cargo clone joshua912815-tg-rcore-tutorial-t2l8
//! cd joshua912815-tg-rcore-tutorial-t2l8
//! make run
//! ```
//!
//! Or extract the bundled sub-crates manually:
//!
//! ```bash
//! bash scripts/extract_submodules.sh
//! ```

/// Crate identifier for the T2L8 workspace bundle.
pub const BUNDLE_NAME: &str = "joshua912815-tg-rcore-tutorial-t2l8";

/// Version of the bundle.
pub const BUNDLE_VERSION: &str = "0.8.0-preview.1";

/// List of all included submodule crates.
pub const SUBMODULE_CRATES: &[&str] = &[
    "tg-rcore-tutorial-ch1",
    "tg-rcore-tutorial-ch2",
    "tg-rcore-tutorial-ch3",
    "tg-rcore-tutorial-ch4",
    "tg-rcore-tutorial-ch5",
    "tg-rcore-tutorial-ch6",
    "tg-rcore-tutorial-ch7",
    "tg-rcore-tutorial-ch8",
    "tg-rcore-tutorial-checker",
    "tg-rcore-tutorial-console",
    "tg-rcore-tutorial-easy-fs",
    "tg-rcore-tutorial-kernel-alloc",
    "tg-rcore-tutorial-kernel-context",
    "tg-rcore-tutorial-kernel-vm",
    "tg-rcore-tutorial-linker",
    "tg-rcore-tutorial-sbi",
    "tg-rcore-tutorial-signal",
    "tg-rcore-tutorial-signal-defs",
    "tg-rcore-tutorial-signal-impl",
    "tg-rcore-tutorial-sync",
    "tg-rcore-tutorial-syscall",
    "tg-rcore-tutorial-task-manage",
    "tg-rcore-tutorial-user",
];
