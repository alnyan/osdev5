use super::{EntryImpl, TableImpl};
use crate::mem;
use crate::mem::virt::table::{Entry, MapAttributes};
use cortex_a::asm::barrier::{self, dsb, isb};
use libsys::error::Errno;

/// Fixed-layout group of tables describing device MMIO and kernel identity
/// mappings
#[repr(C, align(0x1000))]
pub struct FixedTableGroup {
    l0: TableImpl,
    l1: TableImpl,
    l2: TableImpl,

    pages_4k: usize,
    pages_2m: usize,
    pages_1g: usize,
}

impl FixedTableGroup {
    /// Constructs a new instance of [Self], initialized with non-present mapping
    /// entries
    pub const fn empty() -> Self {
        Self {
            l0: TableImpl::empty(),
            l1: TableImpl::empty(),
            l2: TableImpl::empty(),

            pages_4k: 0,
            pages_2m: 1,
            pages_1g: 1,
        }
    }

    /// Allocates a virtual memory range from this table group for requested
    /// `phys`..`phys + count * PAGE_SIZE` physical memory region and maps it.
    ///
    /// TODO: only allows 4K, 2M and 1G mappings.
    pub fn map_region(&mut self, phys: usize, count: usize) -> Result<usize, Errno> {
        // TODO generalize region allocation
        let phys_page = phys & !0xFFF;
        let attrs = MapAttributes::SHARE_OUTER | MapAttributes::DEVICE_MEMORY;

        match count {
            262144 => {
                let count = self.pages_1g;
                if count == 512 {
                    return Err(Errno::OutOfMemory);
                }
                self.pages_1g += 1;

                self.l0[count + 256] = EntryImpl::block(phys_page, attrs | MapAttributes::ACCESS);
                unsafe {
                    dsb(barrier::SY);
                    isb(barrier::SY);
                }

                Ok(DEVICE_MAP_OFFSET + (count << 30) + (phys & 0xFFF))
            }
            512 => {
                let count = self.pages_2m;
                if count == 512 {
                    return Err(Errno::OutOfMemory);
                }
                self.pages_2m += 1;

                self.l1[count] = EntryImpl::block(phys_page, attrs | MapAttributes::ACCESS);
                unsafe {
                    dsb(barrier::SY);
                    isb(barrier::SY);
                }

                Ok(DEVICE_MAP_OFFSET + (count << 21) + (phys & 0xFFF))
            }
            1 => {
                let count = self.pages_4k;
                if count == 512 {
                    return Err(Errno::OutOfMemory);
                }
                self.pages_4k += 1;

                self.l2[count] = EntryImpl::normal(phys_page, attrs | MapAttributes::ACCESS);
                unsafe {
                    dsb(barrier::SY);
                    isb(barrier::SY);
                }

                Ok(DEVICE_MAP_OFFSET + (count << 12) + (phys & 0xFFF))
            }
            _ => unimplemented!(),
        }
    }
}

const DEVICE_MAP_OFFSET: usize = mem::KERNEL_OFFSET + (256usize << 30);

#[no_mangle]
static mut KERNEL_TTBR1: FixedTableGroup = FixedTableGroup::empty();

/// Allocates a range of virtual memory of requested size and maps
/// it to specified device memory
pub fn map_device_memory(phys: usize, count: usize) -> Result<usize, Errno> {
    unsafe { KERNEL_TTBR1.map_region(phys, count) }
}

/// Sets up initial mappings for device-memory virtual tables.
///
/// # Safety
///
/// Only safe to be called once during virtual memory init.
pub unsafe fn init_device_map() {
    let l1_phys = (&KERNEL_TTBR1.l1 as *const _) as usize - mem::KERNEL_OFFSET;
    let l2_phys = (&KERNEL_TTBR1.l2 as *const _) as usize - mem::KERNEL_OFFSET;

    KERNEL_TTBR1.l0[256] = Entry::normal(l1_phys, MapAttributes::empty());
    KERNEL_TTBR1.l1[0] = Entry::normal(l2_phys, MapAttributes::empty());
}
