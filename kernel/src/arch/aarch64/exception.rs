//! AArch64 exception handling

use crate::arch::machine;
use crate::dev::irq::{IntController, IrqContext};

/// Trapped SIMD/FP functionality
pub const EC_FP_TRAP: u64 = 0b000111;
/// Data Abort at current EL
pub const EC_DATA_ABORT_ELX: u64 = 0b100101;

#[derive(Debug)]
#[repr(C)]
struct ExceptionFrame {
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
    x8: u64,
    x9: u64,
    x10: u64,
    x11: u64,
    x12: u64,
    x13: u64,
    x14: u64,
    x15: u64,
    x16: u64,
    x17: u64,
    x18: u64,
    x29: u64,
    x30: u64,
    elr: u64,
    esr: u64,
    far: u64,
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
extern "C" fn __aa64_exc_irq_handler() {
    unsafe {
        let ic = IrqContext::new();
        machine::intc().handle_pending_irqs(&ic);
    }
}

#[no_mangle]
extern "C" fn __aa64_exc_sync_handler(exc: &mut ExceptionFrame) {
    loop {}
    let err_code = exc.esr >> 26;
    let iss = exc.esr & 0x1FFFFFF;

    debugln!("Unhandled exception at ELR={:#018x}", exc.elr);

    #[allow(clippy::single_match)]
    match err_code {
        EC_DATA_ABORT_ELX => {
            debugln!("Data Abort:");

            debug!("  Illegal {}", data_abort_access_type(iss),);
            if iss & (1 << 24) != 0 {
                debug!(" of a {}", data_abort_access_size(iss));
            }
            if iss & (1 << 10) == 0 {
                debug!(" at {:#018x}", exc.far);
            } else {
                debug!(" at UNKNOWN");
            }
            debugln!("");
        }
        _ => {}
    }

    debugln!("{:#018x?}", exc);

    panic!("Unhandled exception");
}

global_asm!(include_str!("vectors.S"));
