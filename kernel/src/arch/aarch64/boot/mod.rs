//! aarch64 common boot logic

#[no_mangle]
fn __aa64_bsp_main() {
    debugln!("Test");
    use crate::arch::machine;
    use crate::dev::{Device, serial::SerialDevice};

    unsafe {
        machine::console().lock().enable().unwrap();
    }

    loop {
        let ch = machine::console().lock().recv(true).unwrap();
        debugln!("{:#04x} = '{}'!", ch, ch as char);
    }
}

global_asm!(include_str!("entry.S"));
