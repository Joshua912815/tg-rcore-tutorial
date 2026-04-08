//! Chapter 1 multicore experiment: all harts enter S-mode and print their identity.

#![no_std]
#![no_main]
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code, unused_imports))]

#[macro_use]
extern crate tg_console;

use core::sync::atomic::{AtomicUsize, Ordering};
use tg_console::log;
use joshua912815_rcore_tutorial_t2l9_smp::{
    claim_boot_hart, mark_console_ready, start_secondary_harts, wait_for_console, HART_COUNT,
};
use tg_sbi::shutdown;

const STACK_SIZE: usize = 4096;
static ONLINE: AtomicUsize = AtomicUsize::new(0);

#[repr(align(4096))]
struct BootStacks(#[allow(dead_code)] [u8; STACK_SIZE * HART_COUNT]);

#[unsafe(link_section = ".boot.stack")]
static mut BOOT_STACKS: BootStacks = BootStacks([0; STACK_SIZE * HART_COUNT]);

#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "li t0, {stack_size}",
        "la t1, {stacks}",
        "addi t2, a0, 1",
        "mul t2, t2, t0",
        "add sp, t1, t2",
        "j {main}",
        stack_size = const STACK_SIZE,
        stacks = sym BOOT_STACKS,
        main = sym rust_main,
    )
}

extern "C" fn rust_main(hart_id: usize, _opaque: usize) -> ! {
    if claim_boot_hart(hart_id) {
        tg_console::init_console(&Console);
        tg_console::set_log_level(option_env!("LOG").or(Some("info")));
        mark_console_ready();

        let order = ONLINE.fetch_add(1, Ordering::SeqCst) + 1;
        println!("[T2L9/ch1] hart {hart_id} online ({order}/{HART_COUNT})");

        if let Err(error) = start_secondary_harts(hart_id, _start as *const () as usize) {
            log::error!("failed to start secondary harts via SBI HSM: {error}");
            shutdown(true);
        }

        while ONLINE.load(Ordering::Acquire) < HART_COUNT {
            core::hint::spin_loop();
        }
        println!("[T2L9/ch1] all {HART_COUNT} harts reached S-mode");
        shutdown(false)
    } else {
        wait_for_console();
        let order = ONLINE.fetch_add(1, Ordering::SeqCst) + 1;
        println!("[T2L9/ch1] hart {hart_id} online ({order}/{HART_COUNT})");
        loop {
            unsafe { core::arch::asm!("wfi") };
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    shutdown(true)
}

struct Console;

impl tg_console::Console for Console {
    #[inline]
    fn put_char(&self, c: u8) {
        tg_sbi::console_putchar(c);
    }
}

#[cfg(not(target_arch = "riscv64"))]
mod stub {
    #[unsafe(no_mangle)]
    pub extern "C" fn main() -> i32 {
        0
    }
}
