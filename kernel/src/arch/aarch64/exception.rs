//! AArch64 exception handling

use crate::arch::machine;
use crate::dev::irq::{IntController, IrqContext};
use cortex_a::registers::{ESR_EL1, FAR_EL1};
use tock_registers::interfaces::Readable;
use crate::debug::Level;

/// Trapped SIMD/FP functionality
pub const EC_FP_TRAP: u64 = 0b000111;
/// Data Abort at current EL
pub const EC_DATA_ABORT_ELX: u64 = 0b100101;
/// SVC instruction in AA64 state
pub const EC_SVC_AA64: u64 = 0b010101;

#[derive(Debug)]
#[repr(C)]
struct ExceptionFrame {
    x: [u64; 32],
    spsr_el1: u64,
    elr_el1: u64,
    sp_el0: u64,
    ttbr0_el1: u64,
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
        EC_DATA_ABORT_ELX => {
            let far = FAR_EL1.get();
            dump_data_abort(Level::Error, esr, far);
        }
        EC_SVC_AA64 => {
            infoln!("{:#x} {:#x}", exc.x[0], exc.x[1]);
            exc.x[0] += 1;
            return;
        }
        _ => {}
    }

    errorln!(
        "Unhandled exception at ELR={:#018x}, ESR={:#010x}, exc ctx @ {:p}",
        exc.elr_el1,
        esr,
        exc
    );

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
