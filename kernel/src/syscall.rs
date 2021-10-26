use crate::mem;
use error::Errno;

fn translate(virt: usize) -> Option<usize> {
    let mut res: usize;
    unsafe {
        asm!("at s1e1r, {}; mrs {}, par_el1", in(reg) virt, out(reg) res);
    }
    if res & 1 == 0 {
        Some(res & !(0xFFF | (0xFF << 56)))
    } else {
        None
    }
}

fn validate_user_ptr(base: usize, len: usize) -> Result<(), Errno> {
    if base > mem::KERNEL_OFFSET || base + len > mem::KERNEL_OFFSET {
        warnln!(
            "User region refers to kernel memory: base={:#x}, len={:#x}",
            base,
            len
        );
        return Err(Errno::InvalidArgument);
    }

    for i in (base / mem::PAGE_SIZE)..((base + len + mem::PAGE_SIZE - 1) / mem::PAGE_SIZE) {
        if translate(i * mem::PAGE_SIZE).is_none() {
            warnln!(
                "User region refers to unmapped memory: base={:#x}, len={:#x} (page {:#x})",
                base,
                len,
                i * mem::PAGE_SIZE
            );
            return Err(Errno::InvalidArgument);
        }
    }

    Ok(())
}

pub unsafe fn syscall(num: usize, args: &[usize]) -> Result<usize, Errno> {
    match num {
        // sys_exit
        1 => {
            use crate::proc::Process;
            Process::current().exit(args[0] as i32);
            unreachable!();
        }
        // sys_ex_debug_trace
        120 => {
            use crate::debug::Level;
            validate_user_ptr(args[0], args[1])?;

            let buf = core::slice::from_raw_parts(args[0] as *const u8, args[1]);
            print!(Level::Debug, "[trace] ");
            for &byte in buf.iter() {
                print!(Level::Debug, "{}", byte as char);
            }
            println!(Level::Debug, "");
            Ok(args[1])
        }
        _ => panic!("Undefined system call: {}", num),
    }
}
