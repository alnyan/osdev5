//! x86_64 model-specific and control register interfaces

macro_rules! wrap_msr {
    ($struct_name:ident, $name:ident, $address:expr, $fields:tt) => {
        register_bitfields! {
            u64,
            #[allow(missing_docs)]
            pub $name $fields
        }

        #[allow(missing_docs)]
        pub struct $struct_name;

        impl Readable for $struct_name {
            type T = u64;
            type R = $name::Register;

            #[inline(always)]
            fn get(&self) -> Self::T {
                unsafe {
                    rdmsr($address)
                }
            }
        }

        impl Writeable for $struct_name {
            type T = u64;
            type R = $name::Register;

            #[inline(always)]
            fn set(&self, value: Self::T) {
                unsafe {
                    wrmsr($address, value);
                }
            }
        }

        #[allow(missing_docs)]
        pub const $name: $struct_name = $struct_name;
    }
}

use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields,
};
use core::arch::asm;
use crate::arch::x86_64::intrin::{rdmsr, wrmsr};

// CRn registers
register_bitfields! {
    u64,
    /// Control register CR4 fields
    #[allow(missing_docs)]
    pub CR4 [
        /// Indicates OS support for FXSR/FXRSTOR instructions
        OSFXSR OFFSET(9) NUMBITS(1) [],
        /// Indicates OS support for unmasked SIMD exceptions
        OSXMMEXCPT OFFSET(10) NUMBITS(1) []
    ]
}

register_bitfields! {
    u64,
    /// Control register CR0 fields
    #[allow(missing_docs)]
    pub CR0 [
        /// Indicates requirement for x87 emulation
        EM OFFSET(2) NUMBITS(1) [],
        /// Controls x87 exception handling
        MP OFFSET(1) NUMBITS(1) []
    ]
}

#[allow(missing_docs)]
pub struct Cr4;
#[allow(missing_docs)]
pub struct Cr0;

impl Readable for Cr4 {
    type T = u64;
    type R = CR4::Register;

    #[inline(always)]
    fn get(&self) -> Self::T {
        let mut res: u64;
        unsafe {
            asm!("mov %cr4, {}", out(reg) res, options(att_syntax))
        }
        res
    }
}

impl Writeable for Cr4 {
    type T = u64;
    type R = CR4::Register;

    #[inline(always)]
    fn set(&self, value: Self::T) {
        unsafe {
            asm!("mov {}, %cr4", in(reg) value, options(att_syntax));
        }
    }
}

impl Readable for Cr0 {
    type T = u64;
    type R = CR0::Register;

    #[inline(always)]
    fn get(&self) -> Self::T {
        let mut res: u64;
        unsafe {
            asm!("mov %cr0, {}", out(reg) res, options(att_syntax))
        }
        res
    }
}

impl Writeable for Cr0 {
    type T = u64;
    type R = CR0::Register;

    #[inline(always)]
    fn set(&self, value: Self::T) {
        unsafe {
            asm!("mov {}, %cr0", in(reg) value, options(att_syntax));
        }
    }
}

/// Control register CR4
pub const CR4: Cr4 = Cr4;
/// Control register CR0
pub const CR0: Cr0 = Cr0;

wrap_msr!(MsrIa32Efer, MSR_IA32_EFER, 0xC0000080, [
    SCE OFFSET(0) NUMBITS(1) [],
    LME OFFSET(8) NUMBITS(1) [],
    LMA OFFSET(10) NUMBITS(1) [],
    NXE OFFSET(11) NUMBITS(1) []
]);
wrap_msr!(MsrIa32Lstar, MSR_IA32_LSTAR, 0xC0000082, [
    VALUE OFFSET(0) NUMBITS(64) []
]);
wrap_msr!(MsrIa32Star, MSR_IA32_STAR, 0xC0000081, [
    SYSCALL_CS_SS OFFSET(32) NUMBITS(8) [],
    SYSRET_CS_SS OFFSET(48) NUMBITS(8) []
]);
wrap_msr!(MsrIa32Sfmask, MSR_IA32_SFMASK, 0xC0000084, [
    IF OFFSET(9) NUMBITS(1) []
]);
