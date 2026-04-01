//! # 第一章：静态显示 Tangram 图案
//!
//! 本章在最小裸机执行环境上继续扩展，直接驱动 QEMU 的 VirtIO-GPU，
//! 通过 framebuffer 将七巧板 “OS” 图案渲染到屏幕。
//!
//! ## 关键概念
//!
//! - `#![no_std]` / `#![no_main]`：依旧运行在无标准库、无运行时的裸机环境
//! - VirtIO-GPU：通过 MMIO 暴露的图形设备，支持 2D framebuffer
//! - DMA：驱动使用物理连续内存与设备共享队列和帧缓冲
//! - Framebuffer 渲染：将代码中的多边形数组转换为屏幕上的像素

#![no_std]
#![no_main]
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

#[cfg(target_arch = "riscv64")]
mod framebuffer;
#[cfg(target_arch = "riscv64")]
mod tangram;

#[cfg(target_arch = "riscv64")]
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};
#[cfg(target_arch = "riscv64")]
use framebuffer::FrameBuffer;
use tg_sbi::shutdown;
#[cfg(target_arch = "riscv64")]
use tg_sbi::{console_getchar, console_putchar};
#[cfg(target_arch = "riscv64")]
use virtio_drivers::{Hal, MmioTransport, VirtIOGpu, VirtIOHeader};

/// VirtIO-GPU 的 MMIO 基地址。
#[cfg(target_arch = "riscv64")]
const VIRTIO0: usize = 0x1000_1000;
/// 驱动内部使用的页大小。
#[cfg(target_arch = "riscv64")]
const PAGE_SIZE: usize = 4096;
/// 为 VirtIO 队列和 framebuffer 预留的 DMA 内存池大小。
#[cfg(target_arch = "riscv64")]
const DMA_ARENA_SIZE: usize = 16 * 1024 * 1024;
/// 最小 bump allocator 使用的堆大小。
#[cfg(target_arch = "riscv64")]
const HEAP_SIZE: usize = 1024 * 1024;

/// 页对齐的 DMA 内存池。
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
#[repr(align(4096))]
struct DmaArena([u8; DMA_ARENA_SIZE]);

/// 字节对齐的静态堆。
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
#[repr(align(16))]
struct KernelHeap([u8; HEAP_SIZE]);

/// 供 VirtIO 驱动分配 DMA 页的静态内存池。
#[cfg(target_arch = "riscv64")]
#[unsafe(link_section = ".bss.uninit")]
static mut DMA_ARENA: DmaArena = DmaArena([0; DMA_ARENA_SIZE]);
/// 供 `alloc` crate 使用的最小静态堆。
#[cfg(target_arch = "riscv64")]
#[unsafe(link_section = ".bss.uninit")]
static mut KERNEL_HEAP: KernelHeap = KernelHeap([0; HEAP_SIZE]);
/// DMA bump allocator 的当前偏移。
#[cfg(target_arch = "riscv64")]
static DMA_OFFSET: AtomicUsize = AtomicUsize::new(0);
/// 全局堆分配器的当前偏移。
#[cfg(target_arch = "riscv64")]
static HEAP_OFFSET: AtomicUsize = AtomicUsize::new(0);

/// ch1 中用于 VirtIO-GPU 的最小 HAL。
#[cfg(target_arch = "riscv64")]
struct VirtioHal;

/// ch1 的最小全局分配器。
#[cfg(target_arch = "riscv64")]
struct BumpAllocator;

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

/// S 态程序入口点。
#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 64 * 1024;

    #[unsafe(link_section = ".bss.uninit")]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    core::arch::naked_asm!(
        "la sp, {stack} + {stack_size}",
        "j  {main}",
        stack_size = const STACK_SIZE,
        stack      =   sym STACK,
        main       =   sym rust_main,
    )
}

/// 内核主函数：初始化 VirtIO-GPU，绘制 Tangram 图案，并等待用户退出。
#[cfg(target_arch = "riscv64")]
extern "C" fn rust_main() -> ! {
    puts("Booting ch1 tangram...\n");
    let mut gpu = unsafe {
        VirtIOGpu::<VirtioHal, MmioTransport>::new(
            MmioTransport::new(NonNull::new(VIRTIO0 as *mut VirtIOHeader).unwrap())
                .expect("failed to create VirtIO MMIO transport"),
        )
        .expect("failed to create VirtIO-GPU driver")
    };
    let (width, height) = gpu.resolution().expect("failed to query resolution");
    puts("VirtIO-GPU ready at ");
    put_u32(width);
    puts("x");
    put_u32(height);
    puts(".\n");

    let pixels = gpu
        .setup_framebuffer()
        .expect("failed to setup framebuffer");
    let mut framebuffer = FrameBuffer::new(pixels, width as usize, height as usize);
    tangram::render(&mut framebuffer);
    gpu.flush().expect("failed to flush framebuffer");

    puts("Tangram rendered. Press q to quit.\n");
    wait_for_exit()
}

/// 等待用户在串口按下 `q` 或 `Q`，随后关机。
#[cfg(target_arch = "riscv64")]
fn wait_for_exit() -> ! {
    loop {
        match console_getchar() {
            c if c == b'q' as usize || c == b'Q' as usize => shutdown(false),
            usize::MAX => core::hint::spin_loop(),
            _ => {}
        }
    }
}

/// 向串口输出一个字符串。
#[cfg(target_arch = "riscv64")]
fn puts(message: &str) {
    for byte in message.bytes() {
        console_putchar(byte);
    }
}

/// 向串口输出一个十进制无符号整数。
#[cfg(target_arch = "riscv64")]
fn put_u32(mut value: u32) {
    let mut digits = [0u8; 10];
    let mut len = 0usize;
    if value == 0 {
        console_putchar(b'0');
        return;
    }
    while value != 0 {
        digits[len] = (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len != 0 {
        len -= 1;
        console_putchar(b'0' + digits[len]);
    }
}

/// 将值按 `align` 向上对齐。
#[cfg(target_arch = "riscv64")]
const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}


/// panic 处理函数。
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    #[cfg(target_arch = "riscv64")]
    {
        puts("panic: ");
        if let Some(message) = info.message().as_str() {
            puts(message);
        } else {
            puts("unknown panic");
        }
        puts("\n");
    }
    shutdown(true)
}

/// 非 RISC-V64 架构的占位模块。
#[cfg(not(target_arch = "riscv64"))]
mod stub {
    #[unsafe(no_mangle)]
    pub extern "C" fn main() -> i32 {
        0
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn __libc_start_main() -> i32 {
        0
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_eh_personality() {}
}
