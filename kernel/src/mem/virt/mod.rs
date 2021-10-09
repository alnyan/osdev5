#![allow(missing_docs)]

use core::marker::PhantomData;
use core::ops::Deref;
use core::sync::atomic::{AtomicBool, Ordering};
use cortex_a::asm::barrier::{self, dsb, isb};
use cortex_a::registers::{ID_AA64MMFR0_EL1, MAIR_EL1, SCTLR_EL1, TCR_EL1, TTBR0_EL1, TTBR1_EL1};
use error::Errno;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

const PTE_BLOCK_AF: u64 = 1 << 10;
const PTE_BLOCK_ISH: u64 = 3 << 8;
const PTE_BLOCK_OSH: u64 = 2 << 8;
const PTE_TABLE: u64 = 1 << 1;
const PTE_PRESENT: u64 = 1 << 0;

#[repr(C, align(0x1000))]
struct Table([u64; 512]);

#[no_mangle]
static mut KERNEL_TTBR0: Table = {
    let mut table = [0; 512];
    // TODO fine-grained mapping
    table[0] = (0 << 30) | PTE_BLOCK_AF | PTE_BLOCK_ISH | PTE_PRESENT;
    table[1] = (1 << 30) | PTE_BLOCK_AF | PTE_BLOCK_ISH | PTE_PRESENT;

    Table(table)
};

static mut KERNEL_TTBR1: Table = Table([0; 512]);
// 1GiB
static mut KERNEL_L1: Table = Table([0; 512]);
// 2MiB
static mut KERNEL_L2: Table = Table([0; 512]);
static mut COUNT: usize = 0;
static mut BIG_COUNT: usize = 1;
static mut HUGE_COUNT: usize = 1;

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
        let base = unsafe {
            match count {
                262144 => {
                    let count = HUGE_COUNT;
                    if count == 512 {
                        return Err(Errno::OutOfMemory);
                    }
                    HUGE_COUNT += 1;

                    KERNEL_TTBR1.0[count + 256] = (phys as u64) | PTE_PRESENT | PTE_BLOCK_OSH | PTE_BLOCK_AF;

                    0xFFFFFFC000000000 + (count << 30)
                },
                512 => {
                    let count = BIG_COUNT;
                    if count == 512 {
                        return Err(Errno::OutOfMemory);
                    }
                    BIG_COUNT += 1;

                    KERNEL_L1.0[count] = (phys as u64) | PTE_PRESENT | PTE_BLOCK_OSH | PTE_BLOCK_AF;

                    0xFFFFFFC000000000 + (count << 21)
                },
                1 => {
                    let count = COUNT;
                    if count == 512 {
                        return Err(Errno::OutOfMemory);
                    }
                    COUNT += 1;

                    KERNEL_L2.0[count] =
                        (phys as u64) | PTE_TABLE | PTE_PRESENT | PTE_BLOCK_OSH | PTE_BLOCK_AF;

                    0xFFFFFFC000000000 + count * 0x1000
                },
                _ => unimplemented!()
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
    MAIR_EL1.write(
        MAIR_EL1::Attr0_Normal_Outer::NonCacheable + MAIR_EL1::Attr0_Normal_Inner::NonCacheable,
    );

    unsafe {
        KERNEL_L1.0[0] = (&KERNEL_L2 as *const _ as u64) | PTE_TABLE | PTE_PRESENT;
        KERNEL_TTBR1.0[256] = (&KERNEL_L1 as *const _ as u64) | PTE_TABLE | PTE_PRESENT;
    }

    TTBR0_EL1.set(unsafe { &mut KERNEL_TTBR0 as *mut _ as u64 });
    TTBR1_EL1.set(unsafe { &mut KERNEL_TTBR1 as *mut _ as u64 });

    if ID_AA64MMFR0_EL1.matches_all(ID_AA64MMFR0_EL1::TGran4::NotSupported) {
        return Err(Errno::InvalidArgument);
    }
    let parange = ID_AA64MMFR0_EL1.read(ID_AA64MMFR0_EL1::PARange);

    unsafe {
        dsb(barrier::ISH);
        isb(barrier::SY);
    }

    TCR_EL1.write(
        TCR_EL1::IPS.val(parange)
            + TCR_EL1::T0SZ.val(25)
            + TCR_EL1::TG0::KiB_4
            + TCR_EL1::SH0::Outer
            + TCR_EL1::IRGN0::NonCacheable
            + TCR_EL1::ORGN0::NonCacheable
            + TCR_EL1::T1SZ.val(25)
            + TCR_EL1::TG1::KiB_4
            + TCR_EL1::SH1::Outer
            + TCR_EL1::IRGN1::NonCacheable
            + TCR_EL1::ORGN1::NonCacheable,
    );

    SCTLR_EL1.modify(SCTLR_EL1::M::SET);

    Ok(())
}
