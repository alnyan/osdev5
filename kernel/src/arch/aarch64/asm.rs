//! Assembly intrinsics for AArch64 platform
#![allow(missing_docs)]

use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields,
};

register_bitfields! {
    u64,
    pub CPACR_EL1 [
        FPEN OFFSET(20) NUMBITS(2) [
            TrapAll = 0,
            TrapEl0 = 1,
            TrapNone = 3
        ]
    ]
}

pub struct Reg;

impl Readable for Reg {
    type T = u64;
    type R = CPACR_EL1::Register;

    #[inline(always)]
    fn get(&self) -> Self::T {
        let mut tmp;
        unsafe {
            asm!("mrs {}, cpacr_el1", out(reg) tmp);
        }
        tmp
    }
}

impl Writeable for Reg {
    type T = u64;
    type R = CPACR_EL1::Register;

    #[inline(always)]
    fn set(&self, value: Self::T) {
        unsafe {
            asm!("msr cpacr_el1, {}", in(reg) value);
        }
    }
}

pub const CPACR_EL1: Reg = Reg;
