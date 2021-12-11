use core::arch::asm;
use crate::debug::Level;

#[derive(Debug)]
struct ExceptionFrame {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rdi: u64,
    rsi: u64,
    rbp: u64,
    rbx: u64,
    rdx: u64,
    rcx: u64,
    rax: u64,

    err_no: u64,
    err_code: u64,
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

fn pfault_read_cr2() -> u64 {
    let mut res;
    unsafe {
        asm!("mov %cr2, {}", out(reg) res, options(att_syntax));
    }
    res
}

fn pfault_access_type(code: u64) -> &'static str {
    if code & (1 << 4) != 0 {
        "INSTRUCTION FETCH"
    } else if code & (1 << 1) != 0 {
        "WRITE"
    } else {
        "READ"
    }
}

fn pfault_dump(level: Level, frame: &ExceptionFrame, cr2: u64) {
    println!(level, "\x1B[41;1mPage fault:");
    println!(level, "  Illegal {} at {:#018x}\x1B[0m", pfault_access_type(frame.err_code), cr2);
}

#[no_mangle]
extern "C" fn __x86_64_exception_handler(frame: &mut ExceptionFrame) {
    if frame.err_no == 14 {
        // TODO userspace page faults
        let cr2 = pfault_read_cr2();
        pfault_dump(Level::Error, frame, cr2);
    }

    errorln!(
        "Exception occurred: err_no={}, err_code={:#x}",
        frame.err_no,
        frame.err_code,
    );
    errorln!("cs:rip = {:02x}:{:#x}", frame.cs, frame.rip);
    errorln!("ss:rsp = {:02x}:{:#x}", frame.ss, frame.rsp);

    panic!("Unhandled exception");
}
