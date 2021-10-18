//! Fixed-size table group for device MMIO mappings

use crate::mem::{
    self,
    virt::{Entry, MapAttributes, Table},
};
use cortex_a::asm::barrier::{self, dsb, isb};
use error::Errno;

const DEVICE_MAP_OFFSET: usize = mem::KERNEL_OFFSET + (256usize << 30);

/// Fixed-layout group of tables describing device MMIO and kernel identity
/// mappings
#[repr(C, align(0x1000))]
pub struct FixedTableGroup {
    l0: Table,
    l1: Table,
    l2: Table,

    pages_4k: usize,
    pages_2m: usize,
    pages_1g: usize,
}

impl FixedTableGroup {
    /// Constructs a new instance of [Self], initialized with non-present mapping
    /// entries
    pub const fn empty() -> Self {
        Self {
            l0: Table::empty(),
            l1: Table::empty(),
            l2: Table::empty(),

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
        let attrs = MapAttributes::SH_OUTER | MapAttributes::DEVICE | MapAttributes::ACCESS;

        match count {
            262144 => {
                let count = self.pages_1g;
                if count == 512 {
                    return Err(Errno::OutOfMemory);
                }
                self.pages_1g += 1;

                self.l0[count + 256] = Entry::block(phys_page, attrs);
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

                self.l1[count] = Entry::block(phys_page, attrs);
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

                self.l2[count] = Entry::table(phys_page, attrs);
                unsafe {
                    dsb(barrier::SY);
                    isb(barrier::SY);
                }

                Ok(DEVICE_MAP_OFFSET + (count << 12) + (phys & 0xFFF))
            }
            _ => unimplemented!(),
        }
    }

    /// Sets up initial mappings for 4K, 2M and 1G device memory page translation
    pub fn init_device_map(&mut self) {
        let l1_phys = (&self.l1 as *const _) as usize - mem::KERNEL_OFFSET;
        let l2_phys = (&self.l2 as *const _) as usize - mem::KERNEL_OFFSET;

        self.l0[256] = Entry::table(l1_phys, MapAttributes::empty());
        self.l1[0] = Entry::table(l2_phys, MapAttributes::empty());
    }
}
