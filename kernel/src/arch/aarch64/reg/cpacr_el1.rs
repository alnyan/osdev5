//! CPACR_EL1 register
#![allow(missing_docs)]

use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields,
};

register_bitfields! {
    u64,
    /// EL1 Architectural Feature Access Control Register
    pub CPACR_EL1 [
        /// Enable EL0 and EL1 SIMD/FP accesses to EL1
        FPEN OFFSET(20) NUMBITS(2) [
            /// Trap both EL0 and EL1
            TrapAll = 0,
            /// Trap EL0
            TrapEl0 = 1,
            /// Trap EL1
            TrapEl1 = 2,
            /// Do not trap any SIMD/FP instructions
            TrapNone = 3
        ]
    ]
}

/// CPACR_EL1 register
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

/// CPACR_EL1 register
pub const CPACR_EL1: Reg = Reg;
