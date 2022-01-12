use crate::arch::x86_64::reg::{MSR_IA32_EFER, MSR_IA32_LSTAR, MSR_IA32_SFMASK, MSR_IA32_STAR};
use core::arch::global_asm;
use tock_registers::interfaces::{ReadWriteable, Writeable};

pub(super) fn init() {
    extern "C" {
        fn __x86_64_syscall_entry();
    }

    MSR_IA32_SFMASK.write(MSR_IA32_SFMASK::IF::SET);
    MSR_IA32_LSTAR.set(__x86_64_syscall_entry as u64);
    MSR_IA32_STAR
        .write(MSR_IA32_STAR::SYSRET_CS_SS.val(0x1B - 8) + MSR_IA32_STAR::SYSCALL_CS_SS.val(0x08));
    MSR_IA32_EFER.modify(MSR_IA32_EFER::SCE::SET);
}

global_asm!(include_str!("syscall.S"), options(att_syntax));
