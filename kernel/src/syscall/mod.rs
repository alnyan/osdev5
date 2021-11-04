//! System call implementation

use crate::arch::platform::exception::ExceptionFrame;
use crate::debug::Level;
use crate::proc::{wait, Pid, Process};
use core::mem::size_of;
use core::time::Duration;
use error::Errno;
use libcommon::{Read, Write};
use syscall::{abi, stat::AT_FDCWD};
use vfs::{FileMode, OpenFlags, Stat};

mod arg;
use arg::*;

/// Creates a "fork" process from current one using its register frame.
/// See [Process::fork()].
///
/// # Safety
///
/// Unsafe: accepts and clones process states. Only legal to call
/// from exception handlers.
pub unsafe fn sys_fork(regs: &mut ExceptionFrame) -> Result<Pid, Errno> {
    Process::current().fork(regs)
}

/// Main system call dispatcher function
pub fn syscall(num: usize, args: &[usize]) -> Result<usize, Errno> {
    match num {
        // Process management system calls
        abi::SYS_EXIT => {
            Process::current().exit(args[0] as i32);
            unreachable!();
        }

        // I/O system calls
        abi::SYS_OPENAT => {
            let at_fd = args[0];
            let path = validate_user_str(args[1], args[2])?;
            let mode = FileMode::from_bits(args[3] as u32).ok_or(Errno::InvalidArgument)?;
            let opts = OpenFlags::from_bits(args[4] as u32).ok_or(Errno::InvalidArgument)?;

            let at = if at_fd as i32 == AT_FDCWD {
                None
            } else {
                todo!();
            };

            let proc = Process::current();
            let mut io = proc.io.lock();

            let file = io.ioctx().open(at, path, mode, opts)?;
            io.place_file(file)
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
        abi::SYS_FSTATAT => {
            let proc = Process::current();
            let mut io = proc.io.lock();
            let fd = args[0];
            let filename = validate_user_str(args[1], args[2])?;
            let buf = validate_user_ptr_struct::<Stat>(args[3])?;

            // TODO "self" flag
            let at = if fd as i32 != AT_FDCWD {
                todo!();
            } else {
                None
            };
            let node = io.ioctx().find(at, filename, true)?;
            node.stat(buf)?;
            Ok(0)
        }
        abi::SYS_CLOSE => {
            let proc = Process::current();
            let mut io = proc.io.lock();
            let fd = args[0];

            io.close_file(fd)?;
            Ok(0)
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
                if rem_buf.is_some() {
                    todo!()
                }
            }
            res.map(|_| 0)
        }
        _ => panic!("Undefined system call: {}", num),
    }
}
