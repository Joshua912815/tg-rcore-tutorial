#![no_std]
//! Chapter 4 basic lab crate for `tg-rcore-tutorial`.
//!
//! This package publishes a standalone, reproducible Chapter 4 kernel lab
//! that can be fetched directly from crates.io or reproduced from the tagged
//! Git repository.
//!
//! The runnable kernel is implemented in [`main.rs`](main.rs), while the
//! report materials are stored under `docs/ch4-report.md`.

/// Published crate name.
pub const CRATE_NAME: &str = "joshua912815-tg-rcore-tutorial-ch4-basic";

/// Published version.
pub const CRATE_VERSION: &str = "0.4.0-preview.1";

/// Git tag for this release.
pub const GIT_TAG: &str = "ch4-basic-crate-v0.4.0-preview.1";
