//! AArch64 exception handling

use crate::arch::machine;
use crate::debug::Level;
use crate::dev::irq::{IntController, IrqContext};
use crate::mem;
use crate::proc::{sched, Thread};
use crate::syscall;
use cortex_a::registers::{ESR_EL1, FAR_EL1};
use libsys::{abi::SystemCall, signal::Signal};
use tock_registers::interfaces::Readable;

/// Trapped SIMD/FP functionality
pub const EC_FP_TRAP: u64 = 0b000111;
/// Data Abort at current EL
pub const EC_DATA_ABORT_ELX: u64 = 0b100101;
/// Data Abort at lower EL
pub const EC_DATA_ABORT_EL0: u64 = 0b100100;
/// SVC instruction in AA64 state
pub const EC_SVC_AA64: u64 = 0b010101;

/// Storage for saving context interrupted by exception
#[derive(Debug)]
#[repr(C)]
pub struct ExceptionFrame {
    /// General-purpose registers x0-x31
    pub x: [usize; 32],
    /// Saved processor status
    pub spsr_el1: u64,
    /// Exception return address
    pub elr_el1: u64,
    /// User-space stack pointer
    pub sp_el0: u64,
    /// Translation table base register for user-space
    pub ttbr0_el1: u64,
}

#[inline(always)]
const fn data_abort_access_type(iss: u64) -> &'static str {
    if iss & (1 << 6) != 0 {
        "WRITE"
    } else {
        "READ"
    }
}

#[inline(always)]
const fn data_abort_access_size(iss: u64) -> &'static str {
    match (iss >> 22) & 0x3 {
        0 => "BYTE",
        1 => "HALFWORD",
        2 => "WORD",
        3 => "DOUBLEWORD",
        _ => unreachable!(),
    }
}

#[no_mangle]
extern "C" fn __aa64_exc_irq_handler(_exc: &mut ExceptionFrame) {
    unsafe {
        let ic = IrqContext::new();
        machine::intc().handle_pending_irqs(&ic);
    }
}

fn dump_data_abort(level: Level, esr: u64, far: u64) {
    let iss = esr & 0x1FFFFFF;
    println!(level, "\x1B[41;1mData Abort:");

    print!(level, "  Illegal {}", data_abort_access_type(iss),);
    if iss & (1 << 24) != 0 {
        print!(level, " of a {}", data_abort_access_size(iss));
    }
    if iss & (1 << 10) == 0 {
        print!(level, " at {:#018x}", far);
    } else {
        print!(level, " at UNKNOWN");
    }
    println!(level, "\x1B[0m");
}

#[no_mangle]
extern "C" fn __aa64_exc_sync_handler(exc: &mut ExceptionFrame) {
    let esr = ESR_EL1.get();
    let err_code = esr >> 26;

    #[allow(clippy::single_match)]
    match err_code {
        EC_DATA_ABORT_EL0 | EC_DATA_ABORT_ELX => {
            let far = FAR_EL1.get() as usize;
            let iss = esr & 0x1FFFFFF;

            if iss & (1 << 6) != 0 && far < mem::KERNEL_OFFSET && sched::is_ready() {
                let thread = Thread::current();
                let proc = thread.owner().unwrap();

                if proc
                    .manipulate_space(|space| space.try_cow_copy(far))
                    .is_err()
                {
                    // Kill program
                    dump_data_abort(Level::Error, esr, far as u64);
                    proc.enter_fault_signal(thread, Signal::SegmentationFault);
                }

                unsafe {
                    use cortex_a::registers::TTBR0_EL1;
                    let ttbr = TTBR0_EL1.get() as usize;
                    let asid = (ttbr >> 48) & 0xFF;
                    asm!("tlbi aside1, {}", in(reg) (asid << 48));
                }

                return;
            }

            errorln!("Unresolved data abort");
            dump_data_abort(Level::Error, esr, far as u64);
        }
        EC_SVC_AA64 => {
            let num = SystemCall::from_repr(exc.x[8]);
            if num.is_none() {
                todo!();
            }
            let num = num.unwrap();

            if num == SystemCall::Fork {
                match unsafe { syscall::sys_fork(exc) } {
                    Ok(pid) => exc.x[0] = pid.value() as usize,
                    Err(err) => {
                        exc.x[0] = err.to_negative_isize() as usize;
                    }
                }
                return;
            }

            match syscall::syscall(num, &exc.x[..6]) {
                Ok(val) => exc.x[0] = val,
                Err(err) => {
                    exc.x[0] = err.to_negative_isize() as usize;
                }
            }

            return;
        }
        _ => {}
    }

    if sched::is_ready() {
        let thread = Thread::current();
        errorln!("Unhandled exception in thread {}, {:?}", thread.id(), thread.owner().map(|e| e.id()));
    }

    errorln!(
        "Unhandled exception at ELR={:#018x}, ESR={:#010x}",
        exc.elr_el1,
        esr,
    );
    errorln!("Error code: {:#08b}", err_code);

    panic!("Unhandled exception");
}

#[no_mangle]
extern "C" fn __aa64_exc_fiq_handler(_exc: &mut ExceptionFrame) {
    todo!();
}

#[no_mangle]
extern "C" fn __aa64_exc_serror_handler(_exc: &mut ExceptionFrame) {
    todo!();
}

global_asm!(include_str!("vectors.S"));
