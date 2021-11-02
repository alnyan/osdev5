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

fn validate_user_ptr<'a>(base: usize, len: usize) -> Result<&'a mut [u8], Errno> {
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

    Ok(unsafe { core::slice::from_raw_parts_mut(base as *mut u8, len) })
}

fn validate_user_str<'a>(base: usize, limit: usize) -> Result<&'a str, Errno> {
    if base > mem::KERNEL_OFFSET {
        warnln!("User string refers to kernel memory: base={:#x}", base);
        return Err(Errno::InvalidArgument);
    }

    let base_ptr = base as *const u8;
    let mut len = 0;
    let mut page_valid = false;
    loop {
        if len == limit {
            warnln!("User string exceeded limit: base={:#x}", base);
            return Err(Errno::InvalidArgument);
        }

        if (base + len) % mem::PAGE_SIZE == 0 {
            page_valid = false;
        }

        if !page_valid && translate((base + len) & !0xFFF).is_none() {
            warnln!("User string refers to unmapped memory: base={:#x}, off={:#x}", base, len);
            return Err(Errno::InvalidArgument);
        }

        page_valid = true;

        let byte = unsafe { *base_ptr.add(len) };
        if byte == 0 {
            break;
        }

        len += 1;
    }

    let slice = unsafe { core::slice::from_raw_parts(base_ptr, len) };
    core::str::from_utf8(slice).map_err(|_| {
        warnln!("User string contains invalid UTF-8 characters: base={:#x}", base);
        Errno::InvalidArgument
    })
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
        // sys_ex_sleep
        121 => {
            use crate::proc::wait;
            use core::time::Duration;

            wait::sleep(Duration::from_nanos(args[0] as u64));

            Ok(0)
        }
        // sys_open
        2 => {
            use crate::proc::Process;
            let path = validate_user_str(args[0], 256)?;

            let proc = Process::current();
            let mut io = proc.io.lock();

            let node = io.ioctx.as_ref().unwrap().find(None, path, true)?;
            // TODO check access
            io.files.push(node.open()?);

            Ok(io.files.len() - 1)
        }
        // sys_read
        3 => {
            use crate::proc::Process;
            use libcommon::Read;
            let proc = Process::current();
            let mut io = proc.io.lock();
            let buf = validate_user_ptr(args[1], args[2])?;

            io.files[args[0]].read(buf)
        }
        // sys_write
        4 => {
            use crate::proc::Process;
            use libcommon::Write;
            let proc = Process::current();
            let mut io = proc.io.lock();
            let buf = validate_user_ptr(args[1], args[2])?;

            io.files[args[0]].write(buf)
        }
        _ => panic!("Undefined system call: {}", num),
    }
}
