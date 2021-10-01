//! aarch64 common boot logic

use crate::arch::aarch64::asm::{CPACR_EL1};
use cortex_a::registers::{VBAR_EL1, SCTLR_EL1};
use tock_registers::interfaces::{Writeable, ReadWriteable};

use cortex_a::asm::barrier::{self, dsb, isb};

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

        SCTLR_EL1.modify(SCTLR_EL1::I::SET +
                         SCTLR_EL1::SA::SET +
                         SCTLR_EL1::C::SET +
                         SCTLR_EL1::A::SET);

        dsb(barrier::SY);
        isb(barrier::SY);
    }

    debugln!("Test");

    let mut el: u64;
    let mut sctlr_el1: u64;
    unsafe {
        asm!("mrs {}, currentel", out(reg) el);
        asm!("mrs {}, sctlr_el1", out(reg) sctlr_el1);
    }
    el >>= 2;
    el &= 0x3;

    debugln!("Current EL = {}", el);
    debugln!("SCTLR_EL1 = {:#x}", sctlr_el1);

    //use crate::arch::machine;
    //use crate::dev::{serial::SerialDevice, timer::TimestampSource, Device};

    //unsafe {
        //machine::console().lock().enable().unwrap();
        //machine::local_timer().lock().enable().unwrap();
    //}

    //let base = machine::local_timer().lock().timestamp().unwrap();

    loop {
        cortex_a::asm::wfe();
        //let count = machine::local_timer().lock().timestamp().unwrap();
        //let ch = machine::console().lock().recv(true).unwrap();
        //debugln!("[{:?}] {:#04x} = '{}'!", count - base, ch, ch as char);
    }
}

cfg_if! {
    if #[cfg(feature = "mach_orangepi3")] {
        global_asm!(include_str!("uboot.S"));
    } else {
        global_asm!(include_str!("entry.S"));
    }
}
