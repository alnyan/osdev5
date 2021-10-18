//
//#[repr(C, align(0x1000))]
//pub struct OldTable([u64; 512]);
//
//#[no_mangle]
//static mut KERNEL_TTBR1: OldTable = OldTable([0; 512]);
//// 1GiB
//static mut KERNEL_L1: OldTable = OldTable([0; 512]);
//// 2MiB
//static mut KERNEL_L2: OldTable = OldTable([0; 512]);
//static mut COUNT: usize = 0;
//static mut BIG_COUNT: usize = 1;
//static mut HUGE_COUNT: usize = 1;
//

use crate::mem::{
    self,
    virt::{Entry, MapAttributes, Table},
};
use cortex_a::asm::barrier::{self, dsb, isb};
use error::Errno;

const DEVICE_MAP_OFFSET: usize = mem::KERNEL_OFFSET + (256usize << 30);

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

    pub fn init_device_map(&mut self) {
        let l1_phys = (&self.l1 as *const _) as usize - mem::KERNEL_OFFSET;
        let l2_phys = (&self.l2 as *const _) as usize - mem::KERNEL_OFFSET;

        self.l0[256] = Entry::table(l1_phys, MapAttributes::empty());
        self.l1[0] = Entry::table(l2_phys, MapAttributes::empty());
    }
}
