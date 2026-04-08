//! Chapter 2 multicore experiment: boot hart runs the batch kernel, secondary harts stay parked.

#![no_std]
#![no_main]
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code, unused_imports))]

#[macro_use]
extern crate tg_console;

use core::sync::atomic::{AtomicBool, Ordering};
use impls::{Console, SyscallContext};
use riscv::register::*;
use tg_console::log;
use tg_kernel_context::LocalContext;
use joshua912815_rcore_tutorial_t2l9_smp::{
    claim_boot_hart, detect_harts, mark_console_ready, start_secondary_harts, wait_for_console,
    HART_COUNT,
};
use tg_sbi;
use tg_syscall::{Caller, SyscallId};

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(include_str!(env!("APP_ASM")));

const STACK_SIZE: usize = 8 * 4096;
static CONSOLE_LOCK: AtomicBool = AtomicBool::new(false);

#[repr(align(4096))]
struct BootStacks(#[allow(dead_code)] [u8; STACK_SIZE * HART_COUNT]);

#[unsafe(link_section = ".boot.stack")]
static mut BOOT_STACKS: BootStacks = BootStacks([0; STACK_SIZE * HART_COUNT]);

fn with_console_lock(f: impl FnOnce()) {
    while CONSOLE_LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
    f();
    CONSOLE_LOCK.store(false, Ordering::Release);
}

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
    if !claim_boot_hart(hart_id) {
        wait_for_console();
        println!("[T2L9/ch2] secondary hart {hart_id} parked; boot hart keeps running apps");
        loop {
            unsafe { core::arch::asm!("wfi") };
        }
    }

    unsafe { tg_linker::KernelLayout::locate().zero_bss() };
    let harts = detect_harts();

    tg_console::init_console(&Console);
    tg_console::set_log_level(option_env!("LOG").or(Some("info")));
    tg_console::test_log();
    mark_console_ready();

    println!(
        "[T2L9/ch2] boot hart {hart_id} initializing batch kernel (detected {} harts)",
        harts.len()
    );
    if let Err(error) = start_secondary_harts(&harts, hart_id, _start as *const () as usize) {
        log::error!("failed to start secondary harts via SBI HSM: {error}");
        tg_sbi::shutdown(true);
    }

    tg_syscall::init_io(&SyscallContext);
    tg_syscall::init_process(&SyscallContext);

    for (i, app) in tg_linker::AppMeta::locate().iter().enumerate() {
        let app_base = app.as_ptr() as usize;
        log::info!("load app{i} to {app_base:#x}");

        let mut ctx = LocalContext::user(app_base);
        let mut user_stack: core::mem::MaybeUninit<[usize; 512]> = core::mem::MaybeUninit::uninit();
        let user_stack_ptr = user_stack.as_mut_ptr() as *mut usize;
        *ctx.sp_mut() = unsafe { user_stack_ptr.add(512) } as usize;

        loop {
            unsafe { ctx.execute() };

            use scause::{Exception, Trap};
            match scause::read().cause() {
                Trap::Exception(Exception::UserEnvCall) => {
                    use SyscallResult::*;
                    match handle_syscall(&mut ctx) {
                        Done => continue,
                        Exit(code) => log::info!("app{i} exit with code {code}"),
                        Error(id) => log::error!("app{i} call an unsupported syscall {}", id.0),
                    }
                }
                trap => log::error!("app{i} was killed because of {trap:?}"),
            }

            unsafe { core::arch::asm!("fence.i") };
            break;
        }
        let _ = core::hint::black_box(&user_stack);
        println!();
    }

    println!("[T2L9/ch2] boot hart completed all batch apps while secondary harts stayed parked");
    tg_sbi::shutdown(false)
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    tg_sbi::shutdown(true)
}

enum SyscallResult {
    Done,
    Exit(usize),
    Error(SyscallId),
}

fn handle_syscall(ctx: &mut LocalContext) -> SyscallResult {
    use tg_syscall::{SyscallId as Id, SyscallResult as Ret};

    let id = ctx.a(7).into();
    let args = [ctx.a(0), ctx.a(1), ctx.a(2), ctx.a(3), ctx.a(4), ctx.a(5)];

    match tg_syscall::handle(Caller { entity: 0, flow: 0 }, id, args) {
        Ret::Done(ret) => match id {
            Id::EXIT => SyscallResult::Exit(ctx.a(0)),
            _ => {
                *ctx.a_mut(0) = ret as _;
                ctx.move_next();
                SyscallResult::Done
            }
        },
        Ret::Unsupported(id) => SyscallResult::Error(id),
    }
}

mod impls {
    use super::with_console_lock;
    use tg_syscall::{STDDEBUG, STDOUT};

    pub struct Console;

    impl tg_console::Console for Console {
        #[inline]
        fn put_char(&self, c: u8) {
            with_console_lock(|| tg_sbi::console_putchar(c));
        }

        fn put_str(&self, s: &str) {
            with_console_lock(|| {
                for c in s.bytes() {
                    tg_sbi::console_putchar(c);
                }
            });
        }
    }

    pub struct SyscallContext;

    impl tg_syscall::IO for SyscallContext {
        fn write(
            &self,
            _caller: tg_syscall::Caller,
            fd: usize,
            buf: usize,
            count: usize,
        ) -> isize {
            match fd {
                STDOUT | STDDEBUG => {
                    print!("{}", unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            buf as *const u8,
                            count,
                        ))
                    });
                    count as _
                }
                _ => {
                    tg_console::log::error!("unsupported fd: {fd}");
                    -1
                }
            }
        }
    }

    impl tg_syscall::Process for SyscallContext {
        #[inline]
        fn exit(&self, _caller: tg_syscall::Caller, _status: usize) -> isize {
            0
        }
    }
}

#[cfg(not(target_arch = "riscv64"))]
mod stub {
    #[unsafe(no_mangle)]
    pub extern "C" fn main() -> i32 {
        0
    }
}
