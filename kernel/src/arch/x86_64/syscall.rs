use crate::arch::x86_64::reg::{MSR_IA32_EFER, MSR_IA32_LSTAR, MSR_IA32_SFMASK, MSR_IA32_STAR};
use core::arch::global_asm;
use tock_registers::interfaces::{ReadWriteable, Writeable};
use libsys::abi::SystemCall;
use crate::syscall;

#[derive(Clone, Debug)]
pub struct SyscallFrame {
    x: [usize; 13],

    saved_rsp: usize,
    saved_rflags: usize,
    saved_rip: usize,
}

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

#[no_mangle]
extern "C" fn __x86_64_syscall(frame: &mut SyscallFrame) {
    let num = SystemCall::from_repr(frame.x[6]);
    if num.is_none() {
        todo!();
    }
    let num = num.unwrap();
    if num == SystemCall::Fork {
        match unsafe { syscall::sys_fork(frame) } {
            Ok(pid) => frame.x[6] = u32::from(pid) as usize,
            Err(err) => {
                frame.x[6] = err.to_negative_isize() as usize;
            }
        }
        return;
    }

    match syscall::syscall(num, &frame.x[..6]) {
        Ok(val) => frame.x[6] = val,
        Err(err) => {
            frame.x[6] = err.to_negative_isize() as usize;
        }
    }
}

global_asm!(include_str!("syscall.S"), options(att_syntax));
