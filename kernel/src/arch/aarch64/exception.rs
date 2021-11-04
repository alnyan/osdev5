//! AArch64 exception handling

use crate::arch::machine;
use crate::debug::Level;
use crate::dev::irq::{IntController, IrqContext};
use crate::syscall;
use cortex_a::registers::{ESR_EL1, FAR_EL1};
use tock_registers::interfaces::Readable;
use ::syscall::abi;

/// Trapped SIMD/FP functionality
pub const EC_FP_TRAP: u64 = 0b000111;
/// Data Abort at current EL
pub const EC_DATA_ABORT_ELX: u64 = 0b100101;
/// Data Abort at lower EL
pub const EC_DATA_ABORT_EL0: u64 = 0b100100;
/// SVC instruction in AA64 state
pub const EC_SVC_AA64: u64 = 0b010101;

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionFrame {
    pub x: [usize; 32],
    pub spsr_el1: u64,
    pub elr_el1: u64,
    pub sp_el0: u64,
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
    println!(level, "Data Abort:");

    print!(level, "  Illegal {}", data_abort_access_type(iss),);
    if iss & (1 << 24) != 0 {
        print!(level, " of a {}", data_abort_access_size(iss));
    }
    if iss & (1 << 10) == 0 {
        print!(level, " at {:#018x}", far);
    } else {
        print!(level, " at UNKNOWN");
    }
    println!(level, "");
}

#[no_mangle]
extern "C" fn __aa64_exc_sync_handler(exc: &mut ExceptionFrame) {
    let esr = ESR_EL1.get();
    let err_code = esr >> 26;

    #[allow(clippy::single_match)]
    match err_code {
        EC_DATA_ABORT_ELX | EC_DATA_ABORT_EL0 => {
            let far = FAR_EL1.get();
            dump_data_abort(Level::Error, esr, far);
        }
        EC_SVC_AA64 => {
            unsafe {
                if exc.x[8] == abi::SYS_FORK {
                    match syscall::sys_fork(exc) {
                        Ok(pid) => exc.x[0] = pid.value() as usize,
                        Err(err) => {
                            warnln!("fork() syscall failed: {:?}", err);
                            exc.x[0] = usize::MAX;
                        },
                    }
                    return;
                }

                match syscall::syscall(exc.x[8], &exc.x[..6]) {
                    Ok(val) => exc.x[0] = val,
                    Err(err) => {
                        warnln!("syscall {} failed: {:?}", exc.x[8], err);
                        exc.x[0] = usize::MAX
                    },
                }
            }
            return;
        }
        _ => {}
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
