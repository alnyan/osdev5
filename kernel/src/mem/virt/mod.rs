//! Virtual memory and translation table management

use core::marker::PhantomData;
use core::ops::Deref;
use libsys::{mem::memcpy, error::Errno};
use crate::mem::{self, phys::{self, PageUsage}};

pub mod table;
use crate::arch::platform::virt as virt_impl;

use table::{Space, SpaceImpl, MapAttributes};

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
        let base = virt_impl::map_device_memory(phys, count)?;
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
        virt_impl::enable();
    }

    Ok(())
}

/// Writes a [Copy]able object to `dst` address in `space`. Will allocate any affected pages.
///
/// # Safety
///
/// Unsafe: arbitrary memory write.
pub unsafe fn write_paged<T: Clone + Copy>(space: &mut SpaceImpl, dst: usize, src: T) -> Result<(), Errno> {
    write_paged_bytes(space, dst, core::slice::from_raw_parts(&src as *const _ as *const u8, core::mem::size_of::<T>()))
}

/// Writes a byte slice to `dst` address in `space`. Will allocate any affected pages.
///
/// # Safety
///
/// Unsafe: arbitrary memory write.
pub unsafe fn write_paged_bytes(space: &mut SpaceImpl, dst: usize, src: &[u8]) -> Result<(), Errno> {
    if (src.len() + (dst % 4096)) > 4096 {
        todo!("Object crossed page boundary");
    }
    let page_virt = dst & !4095;
    let page_phys = if let Ok(phys) = space.translate(dst) {
        phys
    } else {
        let page = phys::alloc_page(PageUsage::UserPrivate)?;
        let flags = MapAttributes::SHARE_OUTER | MapAttributes::USER_READ;
        space.map(page_virt, page, flags)?;
        page
    };

    memcpy(
        (mem::virtualize(page_phys) + (dst % 4096)) as *mut u8,
        src.as_ptr(),
        src.len(),
    );
    Ok(())
}
