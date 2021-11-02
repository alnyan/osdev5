use crate::debug::Level;
use crate::mem;
use crate::proc::{wait, Process};
use core::mem::size_of;
use core::time::Duration;
use error::Errno;
use libcommon::{Read, Write};
use syscall::abi;

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

fn validate_user_ptr_null<'a>(base: usize, len: usize) -> Result<Option<&'a mut [u8]>, Errno> {
    if base == 0 {
        Ok(None)
    } else {
        validate_user_ptr(base, len).map(|e| Some(e))
    }
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
            warnln!(
                "User string refers to unmapped memory: base={:#x}, off={:#x}",
                base,
                len
            );
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
        warnln!(
            "User string contains invalid UTF-8 characters: base={:#x}",
            base
        );
        Errno::InvalidArgument
    })
}

pub unsafe fn syscall(num: usize, args: &[usize]) -> Result<usize, Errno> {
    match num {
        // Process management system calls
        abi::SYS_EXIT => {
            Process::current().exit(args[0] as i32);
            unreachable!();
        }

        // I/O system calls
        abi::SYS_OPEN => {
            let path = validate_user_str(args[0], 256)?;

            let proc = Process::current();
            let mut io = proc.io.lock();

            let node = io.ioctx().find(None, path, true)?;
            // TODO check access
            io.place_file(node.open()?)
        }
        abi::SYS_READ => {
            let proc = Process::current();
            let mut io = proc.io.lock();
            let buf = validate_user_ptr(args[1], args[2])?;

            io.file(args[0])?.read(buf)
        }
        abi::SYS_WRITE => {
            let proc = Process::current();
            let mut io = proc.io.lock();
            let buf = validate_user_ptr(args[1], args[2])?;

            io.file(args[0])?.write(buf)
        }

        // Extra system calls
        abi::SYS_EX_DEBUG_TRACE => {
            let buf = validate_user_ptr(args[0], args[1])?;
            print!(Level::Debug, "[trace] ");
            for &byte in buf.iter() {
                print!(Level::Debug, "{}", byte as char);
            }
            println!(Level::Debug, "");
            Ok(args[1])
        }
        abi::SYS_EX_NANOSLEEP => {
            let rem_buf = validate_user_ptr_null(args[1], size_of::<u64>() * 2)?;
            let mut rem = Duration::new(0, 0);
            let res = wait::sleep(Duration::from_nanos(args[0] as u64), &mut rem);
            if res == Err(Errno::Interrupt) {
                warnln!("Sleep interrupted, {:?} remaining", rem);
                if let Some(_) = rem_buf {
                    todo!()
                }
            }
            res.map(|_| 0)
        }
        _ => panic!("Undefined system call: {}", num),
    }
}
