//! # 第二章：Moving Tangram 批处理系统
//!
//! 本章在批处理系统基础上扩展了 VirtIO-GPU 支持，
//! 通过多个用户程序依次触发自定义绘图 syscall，
//! 将 Tangram “OS” 图案逐块渲染到 framebuffer。

#![no_std]
#![no_main]
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

#[cfg(target_arch = "riscv64")]
mod framebuffer;
#[cfg(target_arch = "riscv64")]
mod tangram;

#[macro_use]
extern crate tg_console;

use impls::{Console, SyscallContext};
use riscv::register::*;
use tg_console::log;
use tg_kernel_context::LocalContext;
use tg_syscall::{Caller, SyscallId};

#[cfg(target_arch = "riscv64")]
use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};
#[cfg(target_arch = "riscv64")]
use framebuffer::FrameBuffer;
#[cfg(target_arch = "riscv64")]
use tg_sbi::console_getchar;
#[cfg(target_arch = "riscv64")]
use virtio_drivers::{Hal, MmioTransport, VirtIOGpu, VirtIOHeader};

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(include_str!(env!("APP_ASM")));

const SYSCALL_DRAW_TANGRAM_PIECE: usize = 0x1000;

#[cfg(target_arch = "riscv64")]
const VIRTIO0: usize = 0x1000_1000;
#[cfg(target_arch = "riscv64")]
const PAGE_SIZE: usize = 4096;
#[cfg(target_arch = "riscv64")]
const DMA_ARENA_SIZE: usize = 16 * 1024 * 1024;
#[cfg(target_arch = "riscv64")]
const HEAP_SIZE: usize = 1024 * 1024;
#[cfg(target_arch = "riscv64")]
const FRAME_DELAY_LOOPS: usize = 12_000_000;

#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
#[repr(align(4096))]
struct DmaArena([u8; DMA_ARENA_SIZE]);

#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
#[repr(align(16))]
struct KernelHeap([u8; HEAP_SIZE]);

#[cfg(target_arch = "riscv64")]
#[unsafe(link_section = ".bss.uninit")]
static mut DMA_ARENA: DmaArena = DmaArena([0; DMA_ARENA_SIZE]);
#[cfg(target_arch = "riscv64")]
#[unsafe(link_section = ".bss.uninit")]
static mut KERNEL_HEAP: KernelHeap = KernelHeap([0; HEAP_SIZE]);
#[cfg(target_arch = "riscv64")]
static DMA_OFFSET: AtomicUsize = AtomicUsize::new(0);
#[cfg(target_arch = "riscv64")]
static HEAP_OFFSET: AtomicUsize = AtomicUsize::new(0);

#[cfg(target_arch = "riscv64")]
struct DisplayCell(UnsafeCell<Option<GameDisplay>>);

#[cfg(target_arch = "riscv64")]
unsafe impl Sync for DisplayCell {}

#[cfg(target_arch = "riscv64")]
static DISPLAY: DisplayCell = DisplayCell(UnsafeCell::new(None));

#[cfg(target_arch = "riscv64")]
struct VirtioHal;

#[cfg(target_arch = "riscv64")]
struct BumpAllocator;

#[cfg(target_arch = "riscv64")]
struct GameDisplay {
    gpu: VirtIOGpu<'static, VirtioHal, MmioTransport>,
    pixels_ptr: *mut u8,
    pixels_len: usize,
    width: usize,
    height: usize,
    drawn_mask: u64,
}

#[cfg(target_arch = "riscv64")]
#[global_allocator]
static GLOBAL_ALLOCATOR: BumpAllocator = BumpAllocator;

#[cfg(target_arch = "riscv64")]
unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();
        loop {
            let start = HEAP_OFFSET.load(Ordering::Relaxed);
            let aligned = align_up(start, align);
            let end = aligned.saturating_add(size);
            if end > HEAP_SIZE {
                return core::ptr::null_mut();
            }
            if HEAP_OFFSET
                .compare_exchange(start, end, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                let base = core::ptr::addr_of_mut!(KERNEL_HEAP).cast::<u8>() as usize;
                return (base + aligned) as *mut u8;
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[cfg(target_arch = "riscv64")]
impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> usize {
        let size = pages.saturating_mul(PAGE_SIZE);
        loop {
            let start = DMA_OFFSET.load(Ordering::Relaxed);
            let end = start.saturating_add(size);
            if end > DMA_ARENA_SIZE {
                return 0;
            }
            if DMA_OFFSET
                .compare_exchange(start, end, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                let base = core::ptr::addr_of_mut!(DMA_ARENA).cast::<u8>() as usize;
                unsafe {
                    core::ptr::write_bytes((base + start) as *mut u8, 0, size);
                }
                return base + start;
            }
        }
    }

    fn dma_dealloc(_paddr: usize, _pages: usize) -> i32 {
        0
    }

    fn phys_to_virt(paddr: usize) -> usize {
        paddr
    }

    fn virt_to_phys(vaddr: usize) -> usize {
        vaddr
    }
}

#[cfg(target_arch = "riscv64")]
impl GameDisplay {
    fn new() -> Self {
        let mut gpu = unsafe {
            VirtIOGpu::<VirtioHal, MmioTransport>::new(
                MmioTransport::new(NonNull::new(VIRTIO0 as *mut VirtIOHeader).unwrap())
                    .expect("failed to create VirtIO MMIO transport"),
            )
            .expect("failed to create VirtIO-GPU driver")
        };
        let (width, height) = gpu.resolution().expect("failed to query resolution");
        let pixels = gpu
            .setup_framebuffer()
            .expect("failed to setup framebuffer");
        let pixels_ptr = pixels.as_mut_ptr();
        let pixels_len = pixels.len();
        Self {
            gpu,
            pixels_ptr,
            pixels_len,
            width: width as usize,
            height: height as usize,
            drawn_mask: 0,
        }
    }

    fn clear(&mut self) {
        let mut framebuffer = self.framebuffer();
        tangram::clear_scene(&mut framebuffer);
        self.flush();
    }

    fn draw_piece(&mut self, piece_id: usize) -> isize {
        if piece_id >= tangram::piece_count() {
            return -1;
        }
        let bit = 1u64 << piece_id;
        if self.drawn_mask & bit == 0 {
            let mut framebuffer = self.framebuffer();
            tangram::draw_piece(&mut framebuffer, piece_id);
            self.drawn_mask |= bit;
            self.flush();
            busy_delay(FRAME_DELAY_LOOPS);
        }
        0
    }

    fn framebuffer(&mut self) -> FrameBuffer<'_> {
        let pixels = unsafe { core::slice::from_raw_parts_mut(self.pixels_ptr, self.pixels_len) };
        FrameBuffer::new(pixels, self.width, self.height)
    }

    fn flush(&mut self) {
        self.gpu.flush().expect("failed to flush framebuffer");
    }
}

#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 8 * 4096;
    #[unsafe(link_section = ".boot.stack")]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    core::arch::naked_asm!(
        "la sp, {stack} + {stack_size}",
        "j  {main}",
        stack = sym STACK,
        stack_size = const STACK_SIZE,
        main = sym rust_main,
    )
}

extern "C" fn rust_main() -> ! {
    unsafe { tg_linker::KernelLayout::locate().zero_bss() };

    tg_console::init_console(&Console);
    tg_console::set_log_level(option_env!("LOG"));
    tg_console::test_log();

    tg_syscall::init_io(&SyscallContext);
    tg_syscall::init_process(&SyscallContext);

    #[cfg(target_arch = "riscv64")]
    init_display();

    for (i, app) in tg_linker::AppMeta::locate().iter().enumerate() {
        let app_base = app.as_ptr() as usize;
        log::info!("load app{i} to {app_base:#x}");

        let mut ctx = LocalContext::user(app_base);
        let mut user_stack: core::mem::MaybeUninit<[usize; 512]> =
            core::mem::MaybeUninit::uninit();
        let user_stack_ptr = user_stack.as_mut_ptr() as *mut usize;
        *ctx.sp_mut() = unsafe { user_stack_ptr.add(512) } as usize;

        loop {
            unsafe { ctx.execute() };

            use scause::{Exception, Trap};
            match scause::read().cause() {
                Trap::Exception(Exception::UserEnvCall) => match handle_syscall(&mut ctx) {
                    SyscallResult::Done => continue,
                    SyscallResult::Exit(code) => log::info!("app{i} exit with code {code}"),
                    SyscallResult::Error(id) => {
                        log::error!("app{i} call an unsupported syscall {}", id.0)
                    }
                },
                trap => log::error!("app{i} was killed because of {trap:?}"),
            }
            unsafe { core::arch::asm!("fence.i") };
            break;
        }
        let _ = core::hint::black_box(&user_stack);
        println!();
    }

    #[cfg(target_arch = "riscv64")]
    {
        println!("Tangram completed. Press q to quit.");
        wait_for_exit();
    }

    #[cfg(not(target_arch = "riscv64"))]
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

    let id: SyscallId = ctx.a(7).into();
    let args = [ctx.a(0), ctx.a(1), ctx.a(2), ctx.a(3), ctx.a(4), ctx.a(5)];

    if id.0 == SYSCALL_DRAW_TANGRAM_PIECE {
        let ret = draw_tangram_piece(args[0]);
        *ctx.a_mut(0) = ret as usize;
        ctx.move_next();
        return SyscallResult::Done;
    }

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

#[cfg(target_arch = "riscv64")]
fn init_display() {
    let mut display = GameDisplay::new();
    log::info!(
        "moving tangram display ready at {}x{} with {} pieces",
        display.width,
        display.height,
        tangram::piece_count()
    );
    display.clear();
    unsafe {
        *DISPLAY.0.get() = Some(display);
    }
}

fn draw_tangram_piece(piece_id: usize) -> isize {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        if let Some(display) = (*DISPLAY.0.get()).as_mut() {
            return display.draw_piece(piece_id);
        }
        -1
    }

    #[cfg(not(target_arch = "riscv64"))]
    {
        let _ = piece_id;
        -1
    }
}

#[cfg(target_arch = "riscv64")]
fn wait_for_exit() -> ! {
    loop {
        match console_getchar() {
            c if c == b'q' as usize || c == b'Q' as usize => tg_sbi::shutdown(false),
            usize::MAX => core::hint::spin_loop(),
            _ => {}
        }
    }
}

#[cfg(target_arch = "riscv64")]
fn busy_delay(iterations: usize) {
    for _ in 0..iterations {
        core::hint::spin_loop();
    }
}

#[cfg(target_arch = "riscv64")]
const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

mod impls {
    use tg_syscall::{STDDEBUG, STDOUT};

    pub struct Console;

    impl tg_console::Console for Console {
        #[inline]
        fn put_char(&self, c: u8) {
            tg_sbi::console_putchar(c);
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
