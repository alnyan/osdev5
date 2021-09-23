use cortex_a::asm;

#[no_mangle]
fn __aa64_bsp_main() {
    debugln!("Test");
    loop {
        asm::wfe();
    }
}

global_asm!(include_str!("entry.S"));
