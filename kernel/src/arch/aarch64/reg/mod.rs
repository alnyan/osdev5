//! AArch64 architectural registers

use core::arch::asm;
use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields,
};

macro_rules! wrap_msr {
    ($struct_name:ident, $name:ident, $reg:literal, $fields:tt) => {
        #[allow(missing_docs)]
        pub struct $struct_name;

        register_bitfields! {
            u64,
            #[allow(missing_docs)]
            pub $name $fields
        }

        impl Readable for $struct_name {
            type T = u64;
            type R = $name::Register;

            #[inline(always)]
            fn get(&self) -> Self::T {
                let mut value;
                unsafe {
                    asm!(concat!("mrs {}, ", $reg), out(reg) value)
                }
                value
            }
        }

        impl Writeable for $struct_name {
            type T = u64;
            type R = $name::Register;

            #[inline(always)]
            fn set(&self, value: Self::T) {
                unsafe {
                    asm!(concat!("msr ", $reg, ", {}"), in(reg) value);
                }
            }
        }

        #[allow(missing_docs)]
        pub const $name: $struct_name = $struct_name;
    };
}

wrap_msr!(CpacrEl1, CPACR_EL1, "cpacr_el1", [
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
]);

wrap_msr!(CntkctlEl1, CNTKCTL_EL1, "cntkctl_el1", [
    /// If set, disables CNTPCT and CNTFRQ trapping from EL0
    EL0PCTEN OFFSET(0) NUMBITS(1) []
]);
