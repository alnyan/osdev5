#![allow(missing_docs)]

use cortex_a::asm::barrier::{self, dsb, isb};
use cortex_a::registers::{ID_AA64MMFR0_EL1, MAIR_EL1, SCTLR_EL1, TCR_EL1, TTBR0_EL1, TTBR1_EL1};
use error::Errno;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

const PTE_BLOCK_AF: u64 = 1 << 10;
const PTE_BLOCK_ISH: u64 = 3 << 8;
const PTE_BLOCK_OSH: u64 = 2 << 8;
const PTE_PRESENT: u64 = 1 << 0;

#[no_mangle]
static mut KERNEL_TTBR0: [u64; 512] = {
    let mut table = [0; 512];
    // TODO fine-grained mapping
    table[0] = (0 << 30) | PTE_BLOCK_AF | PTE_BLOCK_ISH | PTE_PRESENT;
    table[1] = (1 << 30) | PTE_BLOCK_AF | PTE_BLOCK_ISH | PTE_PRESENT;

    table
};

pub struct DeviceMemory {
    base: usize,
    count: usize,
}

impl DeviceMemory {
    pub fn map(phys: usize, count: usize) -> Result<Self, Errno> {
        todo!()
    }
}

impl Drop for DeviceMemory {
    fn drop(&mut self) {
        todo!()
    }
}

pub fn enable() -> Result<(), Errno> {
    MAIR_EL1.write(
        MAIR_EL1::Attr0_Normal_Outer::NonCacheable + MAIR_EL1::Attr0_Normal_Inner::NonCacheable,
    );
    TTBR0_EL1.set(unsafe { &mut KERNEL_TTBR0 as *mut _ as u64 });

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
            + TCR_EL1::EPD1::SET
    );

    SCTLR_EL1.modify(SCTLR_EL1::M::SET);

    Ok(())
}
