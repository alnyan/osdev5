use crate::arch::x86_64;
use crate::debug::Level;
use crate::dev::irq::{IntController, IrqContext};
use core::arch::{asm, global_asm};
use libsys::{error::Errno, signal::Signal};
use crate::mem::{self, virt::table::Space};
use crate::proc::{Thread, sched};

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

fn pfault_dump(level: Level, frame: &ExceptionFrame, cr2: usize) {
    println!(level, "\x1B[41;1mPage fault:");
    println!(
        level,
        "  Illegal {} at {:#018x}\x1B[0m",
        pfault_access_type(frame.err_code),
        cr2
    );
}

#[no_mangle]
extern "C" fn __x86_64_exception_handler(frame: &mut ExceptionFrame) {
    if frame.err_no == 14 {
        let cr2 = pfault_read_cr2() as usize;

        if cr2 < mem::KERNEL_OFFSET && sched::is_ready() {
            let thread = Thread::current();
            let proc = thread.owner().unwrap();

            let res = proc.manipulate_space(|space| {
                space.try_cow_copy(cr2)?;
                // unsafe {
                //     intrin::flush_tlb_asid(asid);
                // }
                Result::<(), Errno>::Ok(())
            });

            if res.is_err() {
                errorln!("Page fault at {:#x} in user {:?}", frame.rip, thread.owner_id());
                pfault_dump(Level::Error, frame, cr2);
                proc.enter_fault_signal(thread, Signal::SegmentationFault);
            }

            return;
        }

        errorln!("Unresolved page fault:");
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

#[no_mangle]
extern "C" fn __x86_64_irq_handler(frame: &mut ExceptionFrame) {
    unsafe {
        let ic = IrqContext::new(frame.err_no as usize);
        x86_64::intc().handle_pending_irqs(&ic);
    }
}
