//! Memory management and functions module

pub mod heap;
pub mod phys;
pub mod virt;

/// Virtual offset applied to kernel address space
pub const KERNEL_OFFSET: usize = 0xFFFFFF8000000000;

/// Default page size used by the kernel
pub const PAGE_SIZE: usize = 4096;

/// Returns input `addr` with [KERNEL_OFFSET] applied.
///
/// Will panic if `addr` is not mapped by kernel's
/// direct translation tables.
pub fn virtualize(addr: usize) -> usize {
    assert!(addr < (256 << 30));
    addr + KERNEL_OFFSET
}

/// Returns the physical address of kernel's end in memory.
pub fn kernel_end_phys() -> usize {
    extern "C" {
        static __kernel_end: u8;
    }
    unsafe { &__kernel_end as *const _ as usize - KERNEL_OFFSET }
}
