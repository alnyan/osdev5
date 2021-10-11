#![allow(missing_docs)]

use crate::mem::KERNEL_OFFSET;
use core::marker::PhantomData;
use core::ops::Deref;
use cortex_a::asm::barrier::{self, dsb, isb};
use cortex_a::registers::{TCR_EL1, TTBR0_EL1};
use error::Errno;
use tock_registers::interfaces::{ReadWriteable, Writeable};

const PTE_BLOCK_AF: u64 = 1 << 10;
const PTE_BLOCK_OSH: u64 = 2 << 8;
const PTE_TABLE: u64 = 1 << 1;
const PTE_PRESENT: u64 = 1 << 0;
const PTE_ATTR1: u64 = 1 << 2;

#[repr(C, align(0x1000))]
struct Table([u64; 512]);

#[no_mangle]
static mut KERNEL_TTBR1: Table = Table([0; 512]);
// 1GiB
static mut KERNEL_L1: Table = Table([0; 512]);
// 2MiB
static mut KERNEL_L2: Table = Table([0; 512]);
static mut COUNT: usize = 0;
static mut BIG_COUNT: usize = 1;
static mut HUGE_COUNT: usize = 1;

const DEVICE_MAP_OFFSET: usize = KERNEL_OFFSET + (256 << 30);

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
        // TODO generalize this
        let phys_page = phys & !0xFFF;

        let base = unsafe {
            match count {
                262144 => {
                    let count = HUGE_COUNT;
                    if count == 512 {
                        return Err(Errno::OutOfMemory);
                    }
                    HUGE_COUNT += 1;

                    KERNEL_TTBR1.0[count + 256] =
                        (phys_page as u64) | PTE_PRESENT | PTE_BLOCK_OSH | PTE_BLOCK_AF | PTE_ATTR1;
                    asm!("dsb ish; isb");

                    DEVICE_MAP_OFFSET + (count << 30) + (phys & 0xFFF)
                }
                512 => {
                    let count = BIG_COUNT;
                    if count == 512 {
                        return Err(Errno::OutOfMemory);
                    }
                    BIG_COUNT += 1;

                    KERNEL_L1.0[count] =
                        (phys_page as u64) | PTE_PRESENT | PTE_BLOCK_OSH | PTE_BLOCK_AF | PTE_ATTR1;
                    asm!("dsb ish; isb");

                    DEVICE_MAP_OFFSET + (count << 21) + (phys & 0xFFF)
                }
                1 => {
                    let count = COUNT;
                    if count == 512 {
                        return Err(Errno::OutOfMemory);
                    }
                    COUNT += 1;

                    KERNEL_L2.0[count] = (phys_page as u64)
                        | PTE_TABLE
                        | PTE_BLOCK_OSH
                        | PTE_PRESENT
                        | PTE_BLOCK_AF
                        | PTE_ATTR1;
                    asm!("dsb ish; isb");

                    DEVICE_MAP_OFFSET + (count << 12) + (phys & 0xFFF)
                }
                _ => unimplemented!(),
            }
        };

        debugln!(
            "Mapping {:#x}..{:#x} -> {:#x} ({:?})",
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
            count: self.count
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
        // TODO function to translate kernel addresses to physical ones
        let l1_base = (&KERNEL_L1 as *const _ as u64) - KERNEL_OFFSET as u64;
        let l2_base = (&KERNEL_L2 as *const _ as u64) - KERNEL_OFFSET as u64;

        KERNEL_L1.0[0] = l2_base | PTE_TABLE | PTE_PRESENT;
        KERNEL_TTBR1.0[256] = l1_base | PTE_TABLE | PTE_PRESENT;

        // NOTE don't think tlb needs to be invalidated when new entries are created
    }

    unsafe {
        dsb(barrier::ISH);
        isb(barrier::SY);
    }

    // Disable lower-half translation
    TTBR0_EL1.set(0);
    TCR_EL1.modify(TCR_EL1::EPD0::SET);

    Ok(())
}
