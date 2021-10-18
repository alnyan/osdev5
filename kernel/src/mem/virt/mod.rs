#![allow(missing_docs)]

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

#[derive(Debug)]
#[allow(dead_code)]
pub struct DeviceMemory {
    name: &'static str,
    base: usize,
    count: usize,
}

pub struct DeviceMemoryIo<T> {
    mmio: DeviceMemory,
    _0: PhantomData<T>,
}
impl DeviceMemory {
    #[inline(always)]
    pub const fn base(&self) -> usize {
        self.base
    }

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

    pub unsafe fn clone(&self) -> Self {
        // TODO maybe add refcount and remove "unsafe"?
        Self {
            name: self.name,
            base: self.base,
            count: self.count,
        }
    }
}

impl<T> DeviceMemoryIo<T> {
    pub const fn new(mmio: DeviceMemory) -> Self {
        Self {
            mmio,
            _0: PhantomData,
        }
    }

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
