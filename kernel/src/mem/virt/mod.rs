//! Virtual memory and translation table management

use core::marker::PhantomData;
use core::ops::Deref;
use cortex_a::asm::barrier::{self, dsb, isb};
use cortex_a::registers::TTBR0_EL1;
use error::Errno;
use tock_registers::interfaces::Writeable;

pub mod table;
pub use table::{Entry, MapAttributes, Space, Table};
pub mod fixed;
pub use fixed::FixedTableGroup;

#[no_mangle]
static mut KERNEL_TTBR1: FixedTableGroup = FixedTableGroup::empty();

/// Structure representing a region of memory used for MMIO/device access
// TODO: this shouldn't be trivially-cloneable and should instead incorporate
//       refcount and properly implement Drop trait
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DeviceMemory {
    name: &'static str,
    base: usize,
    count: usize,
}

/// Structure implementing `Deref<T>` for convenient MMIO register access.
///
/// See [DeviceMemory].
pub struct DeviceMemoryIo<T> {
    mmio: DeviceMemory,
    _0: PhantomData<T>,
}
impl DeviceMemory {
    /// Returns base address of this MMIO region
    #[inline(always)]
    pub const fn base(&self) -> usize {
        self.base
    }

    /// Allocates a virtual memory region and maps it to contiguous region
    /// `phys`..`phys + count * PAGE_SIZE` for MMIO use.
    ///
    /// See [FixedTableGroup::map_region]
    pub fn map(name: &'static str, phys: usize, count: usize) -> Result<Self, Errno> {
        let base = unsafe { KERNEL_TTBR1.map_region(phys, count) }?;
        debugln!(
            "Mapping {:#x}..{:#x} -> {:#x} for {:?}",
            base,
            base + count * 0x1000,
            phys,
            name
        );
        Ok(Self { name, base, count })
    }
}

impl<T> DeviceMemoryIo<T> {
    /// Constructs a new [DeviceMemoryIo<T>] from existing `mmio` region
    pub const fn new(mmio: DeviceMemory) -> Self {
        Self {
            mmio,
            _0: PhantomData,
        }
    }

    /// Allocates and maps device MMIO memory.
    ///
    /// See [DeviceMemory::map]
    ///
    /// # Safety
    ///
    /// Unsafe: accepts arbitrary physical addresses
    pub unsafe fn map(name: &'static str, phys: usize, count: usize) -> Result<Self, Errno> {
        DeviceMemory::map(name, phys, count).map(Self::new)
    }
}

impl<T> Deref for DeviceMemoryIo<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        unsafe { &*(self.mmio.base as *const T) }
    }
}

/// Sets up device mapping tables and disable lower-half
/// identity-mapped translation
pub fn enable() -> Result<(), Errno> {
    unsafe {
        KERNEL_TTBR1.init_device_map();

        dsb(barrier::ISH);
        isb(barrier::SY);
    }

    // Disable lower-half translation
    TTBR0_EL1.set(0);
    //TCR_EL1.modify(TCR_EL1::EPD0::SET);

    Ok(())
}
