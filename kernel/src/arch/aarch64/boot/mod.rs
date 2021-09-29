//! aarch64 common boot logic
use crate::arch::aarch64::asm::CPACR_EL1;
use cortex_a::registers::VBAR_EL1;
use tock_registers::interfaces::Writeable;

#[no_mangle]
fn __aa64_bsp_main() {
    // Disable FP instruction trapping
    CPACR_EL1.write(CPACR_EL1::FPEN::TrapNone);

    extern "C" {
        static aa64_el1_vectors: u8;
    }
    unsafe {
        VBAR_EL1.set(&aa64_el1_vectors as *const _ as u64);
    }

    debugln!("Test");

    use crate::arch::machine;
    use crate::dev::{serial::SerialDevice, timer::TimestampSource, Device};

    unsafe {
        machine::console().lock().enable().unwrap();
        machine::local_timer().lock().enable().unwrap();
    }

    let base = machine::local_timer().lock().timestamp().unwrap();

    loop {
        let count = machine::local_timer().lock().timestamp().unwrap();
        let ch = machine::console().lock().recv(true).unwrap();
        debugln!("[{:?}] {:#04x} = '{}'!", count - base, ch, ch as char);
    }
}

global_asm!(include_str!("entry.S"));
