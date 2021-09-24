//! aarch64 common boot logic

#[no_mangle]
fn __aa64_bsp_main() {
    debugln!("Test");
    use crate::arch::machine;
    use crate::dev::{Device, timer::TimestampSource, serial::SerialDevice};

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
