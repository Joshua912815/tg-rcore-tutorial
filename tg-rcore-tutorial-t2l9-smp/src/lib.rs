//! Shared helpers for the T2L9 multicore chapter 1/2 experiments.

#![no_std]
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code, unused_imports))]

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Number of harts expected by the lab runner.
pub const HART_COUNT: usize = 4;
const MAX_HART_SCAN: usize = 8;

const SBI_EXT_HSM: usize = 0x48534D;
const HSM_HART_START: usize = 0;
const HSM_HART_STATUS: usize = 2;

static CONSOLE_READY: AtomicBool = AtomicBool::new(false);
static BOOT_HART: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Discovered hart set.
#[derive(Clone, Copy)]
pub struct HartSet {
    ids: [usize; MAX_HART_SCAN],
    len: usize,
}

impl HartSet {
    /// Number of discovered harts.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// All discovered hart IDs as a slice.
    #[inline]
    pub fn ids(&self) -> &[usize] {
        &self.ids[..self.len]
    }
}

/// Claim the boot-hart role for the first hart that reaches the kernel.
#[inline]
pub fn claim_boot_hart(hart_id: usize) -> bool {
    BOOT_HART
        .compare_exchange(usize::MAX, hart_id, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

/// Return the hart selected as the boot hart.
#[inline]
pub fn boot_hart() -> Option<usize> {
    let hart_id = BOOT_HART.load(Ordering::Acquire);
    (hart_id != usize::MAX).then_some(hart_id)
}

/// Mark the shared console as initialized by the boot hart.
#[inline]
pub fn mark_console_ready() {
    CONSOLE_READY.store(true, Ordering::Release);
}

/// Wait until the boot hart finishes console initialization.
#[inline]
pub fn wait_for_console() {
    while !CONSOLE_READY.load(Ordering::Acquire) {
        core::hint::spin_loop();
    }
}

/// Discover harts visible to the SBI HSM extension.
pub fn detect_harts() -> HartSet {
    let mut set = HartSet {
        ids: [0; MAX_HART_SCAN],
        len: 0,
    };
    for hart_id in 0..MAX_HART_SCAN {
        if hart_get_status(hart_id).is_ok() {
            set.ids[set.len] = hart_id;
            set.len += 1;
        }
    }
    set
}

/// Start all other harts from the current boot hart.
pub fn start_secondary_harts(harts: &HartSet, boot_hart_id: usize, entry: usize) -> Result<(), isize> {
    for &hart_id in harts.ids() {
        if hart_id == boot_hart_id {
            continue;
        }
        hart_start(hart_id, entry, 0)?;
    }
    Ok(())
}

/// Start one hart through the SBI HSM extension.
pub fn hart_start(hart_id: usize, entry: usize, opaque: usize) -> Result<usize, isize> {
    let (error, value) = sbi_call(SBI_EXT_HSM, HSM_HART_START, hart_id, entry, opaque);
    if error == 0 {
        Ok(value)
    } else {
        Err(error)
    }
}

/// Query a hart status through SBI HSM.
pub fn hart_get_status(hart_id: usize) -> Result<usize, isize> {
    let (error, value) = sbi_call(SBI_EXT_HSM, HSM_HART_STATUS, hart_id, 0, 0);
    if error == 0 {
        Ok(value)
    } else {
        Err(error)
    }
}

#[cfg(target_arch = "riscv64")]
#[inline(always)]
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> (isize, usize) {
    let error: isize;
    let value: usize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") arg0 => error,
            inlateout("x11") arg1 => value,
            in("x12") arg2,
            in("x16") fid,
            in("x17") eid,
        );
    }
    (error, value)
}

#[cfg(not(target_arch = "riscv64"))]
#[inline(always)]
fn sbi_call(_eid: usize, _fid: usize, _arg0: usize, _arg1: usize, _arg2: usize) -> (isize, usize) {
    unimplemented!("SBI calls are only supported on riscv64")
}
