//! VirtIO GPU / framebuffer support for ch8-doom.

use crate::{build_flags, KERNEL_SPACE, Sv39};
use core::{ptr::NonNull, slice};
use spin::{Lazy, Mutex};
use tg_kernel_vm::page_table::{MmuMeta, VAddr, VmFlags};
use tg_syscall::FrameBufferInfo;
use virtio_drivers::{Hal, MmioTransport, VirtIOGpu, VirtIOHeader};

/// VirtIO GPU MMIO base on the second virtio-mmio slot.
const VIRTIO_GPU0: usize = 0x1000_2000;

/// Global GPU device.
pub static GPU_DEVICE: Lazy<VirtIOGpuDevice> = Lazy::new(|| {
    let mut gpu = unsafe {
        VirtIOGpu::<VirtioHal, MmioTransport>::new(
            MmioTransport::new(NonNull::new(VIRTIO_GPU0 as *mut VirtIOHeader).unwrap())
                .expect("failed to create VirtIO GPU transport"),
        )
        .expect("failed to create VirtIO GPU driver")
    };
    let (width, height) = gpu.resolution().expect("failed to query GPU resolution");
    let (fb_ptr, fb_len) = {
        let framebuffer = gpu
            .setup_framebuffer()
            .expect("failed to setup VirtIO framebuffer");
        framebuffer.fill(0);
        (framebuffer.as_mut_ptr() as usize, framebuffer.len())
    };
    gpu.flush().expect("failed to flush initial framebuffer");
    let gpu = unsafe {
        core::mem::transmute::<
            VirtIOGpu<'_, VirtioHal, MmioTransport>,
            VirtIOGpu<'static, VirtioHal, MmioTransport>,
        >(gpu)
    };
    VirtIOGpuDevice {
        gpu: Mutex::new(gpu),
        width: width as usize,
        height: height as usize,
        fb_ptr,
        fb_len,
    }
});

/// Initialize GPU eagerly.
pub fn init() -> &'static VirtIOGpuDevice {
    &GPU_DEVICE
}

/// Simple GPU device wrapper.
pub struct VirtIOGpuDevice {
    gpu: Mutex<VirtIOGpu<'static, VirtioHal, MmioTransport>>,
    width: usize,
    height: usize,
    fb_ptr: usize,
    fb_len: usize,
}

// Safety: all GPU access goes through the internal Mutex and the framebuffer
// DMA memory is managed for the whole kernel lifetime after initialization.
unsafe impl Send for VirtIOGpuDevice {}
unsafe impl Sync for VirtIOGpuDevice {}

impl VirtIOGpuDevice {
    /// Export framebuffer metadata to user space.
    pub fn info(&self) -> FrameBufferInfo {
        FrameBufferInfo {
            width: self.width as u32,
            height: self.height as u32,
            stride: (self.width * 4) as u32,
            bytes_per_pixel: 4,
        }
    }

    /// Blit a BGRA8888 source frame into the host framebuffer and flush it.
    pub fn present(&self, src: &[u8], src_width: usize, src_height: usize) -> isize {
        if src_width == 0 || src_height == 0 {
            return -1;
        }
        let expected = src_width.saturating_mul(src_height).saturating_mul(4);
        if src.len() < expected {
            return -1;
        }
        let dst = unsafe { slice::from_raw_parts_mut(self.fb_ptr as *mut u8, self.fb_len) };
        dst.fill(0);

        let copy_width = src_width.min(self.width);
        let copy_height = src_height.min(self.height);
        let x_offset = (self.width - copy_width) / 2;
        let y_offset = (self.height - copy_height) / 2;
        let dst_stride = self.width * 4;
        let src_stride = src_width * 4;

        for row in 0..copy_height {
            let src_row = row * src_stride;
            let dst_row = (row + y_offset) * dst_stride + x_offset * 4;
            dst[dst_row..dst_row + copy_width * 4]
                .copy_from_slice(&src[src_row..src_row + copy_width * 4]);
        }

        self.gpu.lock().flush().expect("failed to flush framebuffer");
        0
    }
}

struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> usize {
        unsafe {
            alloc::alloc::alloc_zeroed(core::alloc::Layout::from_size_align_unchecked(
                pages << Sv39::PAGE_BITS,
                1 << Sv39::PAGE_BITS,
            )) as _
        }
    }

    fn dma_dealloc(paddr: usize, pages: usize) -> i32 {
        unsafe {
            alloc::alloc::dealloc(
                paddr as _,
                core::alloc::Layout::from_size_align_unchecked(
                    pages << Sv39::PAGE_BITS,
                    1 << Sv39::PAGE_BITS,
                ),
            )
        }
        0
    }

    fn phys_to_virt(paddr: usize) -> usize {
        paddr
    }

    fn virt_to_phys(vaddr: usize) -> usize {
        const VALID: VmFlags<Sv39> = build_flags("__V");
        let ptr: NonNull<u8> = unsafe {
            KERNEL_SPACE
                .assume_init_ref()
                .translate(VAddr::new(vaddr), VALID)
                .unwrap()
        };
        ptr.as_ptr() as usize
    }
}
