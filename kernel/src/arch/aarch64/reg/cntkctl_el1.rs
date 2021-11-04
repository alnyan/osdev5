//! CNTKCTL_EL1 register

use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields,
};

register_bitfields! {
    u64,
    /// Counter-timer Kernel Control Register
    pub CNTKCTL_EL1 [
        /// If set, disables CNTPCT and CNTFRQ trapping from EL0
        EL0PCTEN OFFSET(0) NUMBITS(1) []
    ]
}

/// CNTKCTL_EL1 register
pub struct Reg;

impl Readable for Reg {
    type T = u64;
    type R = CNTKCTL_EL1::Register;

    #[inline(always)]
    fn get(&self) -> Self::T {
        let mut tmp;
        unsafe {
            asm!("mrs {}, cntkctl_el1", out(reg) tmp);
        }
        tmp
    }
}

impl Writeable for Reg {
    type T = u64;
    type R = CNTKCTL_EL1::Register;

    #[inline(always)]
    fn set(&self, value: Self::T) {
        unsafe {
            asm!("msr cntkctl_el1, {}", in(reg) value);
        }
    }
}

/// CNTKCTL_EL1 register
pub const CNTKCTL_EL1: Reg = Reg;
