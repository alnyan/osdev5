//! aarch64 common boot logic

use crate::arch::{aarch64::asm::CPACR_EL1, machine};
use crate::dev::{Device, serial::SerialDevice, timer::TimestampSource};
use cortex_a::asm::barrier::{self, dsb, isb};
use cortex_a::registers::{SCTLR_EL1, VBAR_EL1};
use tock_registers::interfaces::{ReadWriteable, Writeable};

#[no_mangle]
fn __aa64_bsp_main() {
    // Disable FP instruction trapping
    CPACR_EL1.modify(CPACR_EL1::FPEN::TrapNone);

    extern "C" {
        static aa64_el1_vectors: u8;
    }
    unsafe {
        VBAR_EL1.set(&aa64_el1_vectors as *const _ as u64);

        // Setup caching in SCTLR_EL1
        dsb(barrier::SY);
        isb(barrier::SY);

        SCTLR_EL1
            .modify(SCTLR_EL1::I::SET + SCTLR_EL1::SA::SET + SCTLR_EL1::C::SET + SCTLR_EL1::A::SET);

        dsb(barrier::SY);
        isb(barrier::SY);
    }

    machine::init_board().unwrap();

    unsafe {
        machine::local_timer().lock().enable().unwrap();
    }

    let base = machine::local_timer().lock().timestamp().unwrap();

    loop {
        let count = machine::local_timer().lock().timestamp().unwrap();
        let ch = machine::console().lock().recv(true).unwrap();
        debugln!("[{:?}] {:#04x} = '{}'!", count - base, ch, ch as char);
    }
}

cfg_if! {
    if #[cfg(feature = "mach_orangepi3")] {
        global_asm!(include_str!("uboot.S"));
    } else {
        global_asm!(include_str!("entry.S"));
    }
}
