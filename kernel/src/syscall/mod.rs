//! System call implementation

use crate::arch::platform::exception::ExceptionFrame;
use crate::debug::Level;
use crate::proc::{self, elf, wait, Process, ProcessIo, Thread};
use core::mem::size_of;
use core::ops::DerefMut;
use core::time::Duration;
use libsys::{
    abi,
    error::Errno,
    ioctl::IoctlCmd,
    proc::{ExitCode, Pid},
    signal::{Signal, SignalDestination},
    stat::{FdSet, AccessMode, FileDescriptor, FileMode, OpenFlags, Stat, AT_EMPTY_PATH},
    traits::{Read, Write},
};
use vfs::VnodeRef;

pub mod arg;
pub use arg::*;

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

fn find_at_node<T: DerefMut<Target = ProcessIo>>(
    io: &mut T,
    at_fd: Option<FileDescriptor>,
    filename: &str,
    empty_path: bool,
) -> Result<VnodeRef, Errno> {
    let at = if let Some(at_fd) = at_fd {
        io.file(at_fd)?.borrow().node()
    } else {
        None
    };

    if empty_path && filename.is_empty() {
        at.ok_or(Errno::InvalidArgument)
    } else {
        io.ioctx().find(at, filename, true)
    }
}

/// Main system call dispatcher function
pub fn syscall(num: usize, args: &[usize]) -> Result<usize, Errno> {
    match num {
        // Process management system calls
        abi::SYS_EXIT => {
            Process::exit(ExitCode::from(args[0] as i32));
            unreachable!();
        }
        abi::SYS_EX_THREAD_EXIT => {
            Process::exit_thread(Thread::current(), ExitCode::from(args[0] as i32));
            unreachable!();
        },
        abi::SYS_EX_GETTID => {
            Ok(Thread::current().id() as usize)
        },

        // I/O system calls
        abi::SYS_OPENAT => {
            let at_fd = FileDescriptor::from_i32(args[0] as i32)?;
            let path = validate_user_str(args[1], args[2])?;
            let mode = FileMode::from_bits(args[3] as u32).ok_or(Errno::InvalidArgument)?;
            let opts = OpenFlags::from_bits(args[4] as u32).ok_or(Errno::InvalidArgument)?;

            let proc = Process::current();
            let mut io = proc.io.lock();

            let at = if let Some(fd) = at_fd {
                io.file(fd)?.borrow().node()
            } else {
                None
            };

            let file = io.ioctx().open(at, path, mode, opts)?;
            Ok(u32::from(io.place_file(file)?) as usize)
        }
        abi::SYS_READ => {
            let proc = Process::current();
            let fd = FileDescriptor::from(args[0] as u32);
            let mut io = proc.io.lock();
            let buf = validate_user_ptr(args[1], args[2])?;

            io.file(fd)?.borrow_mut().read(buf)
        }
        abi::SYS_WRITE => {
            let proc = Process::current();
            let fd = FileDescriptor::from(args[0] as u32);
            let mut io = proc.io.lock();
            let buf = validate_user_ptr(args[1], args[2])?;

            io.file(fd)?.borrow_mut().write(buf)
        }
        abi::SYS_FSTATAT => {
            let at_fd = FileDescriptor::from_i32(args[0] as i32)?;
            let filename = validate_user_str(args[1], args[2])?;
            let buf = validate_user_ptr_struct::<Stat>(args[3])?;
            let flags = args[4] as u32;

            let proc = Process::current();
            let mut io = proc.io.lock();
            find_at_node(&mut io, at_fd, filename, flags & AT_EMPTY_PATH != 0)?.stat(buf)?;
            Ok(0)
        }
        abi::SYS_CLOSE => {
            let proc = Process::current();
            let mut io = proc.io.lock();
            let fd = FileDescriptor::from(args[0] as u32);

            io.close_file(fd)?;
            Ok(0)
        }
        abi::SYS_EXECVE => {
            let node = {
                let proc = Process::current();
                let mut io = proc.io.lock();
                let filename = validate_user_str(args[0], args[1])?;
                // TODO argv, envp array passing ABI?
                let node = io.ioctx().find(None, filename, true)?;
                drop(io);
                node
            };
            let file = node.open(OpenFlags::O_RDONLY)?;
            Process::execve(|space| elf::load_elf(space, file), 0).unwrap();
            panic!();
        }
        abi::SYS_WAITPID => {
            // TODO special "pid" values
            let pid = unsafe { Pid::from_raw(args[0] as u32) };
            let status = validate_user_ptr_struct::<i32>(args[1])?;

            match Process::waitpid(pid) {
                Ok(exit) => {
                    *status = i32::from(exit);
                    Ok(0)
                }
                _ => todo!(),
            }
        }
        abi::SYS_EX_THREAD_WAIT => {
            let tid = args[0] as u32;

            match Thread::waittid(tid) {
                Ok(_) => {
                    Ok(0)
                },
                _ => todo!(),
            }
        },
        abi::SYS_IOCTL => {
            let fd = FileDescriptor::from(args[0] as u32);
            let cmd = IoctlCmd::try_from(args[1] as u32)?;

            let proc = Process::current();
            let mut io = proc.io.lock();

            let node = io.file(fd)?.borrow().node().ok_or(Errno::InvalidFile)?;
            node.ioctl(cmd, args[2], args[3])
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
        abi::SYS_EX_SIGNAL => {
            Thread::current().set_signal_entry(args[0], args[1]);
            Ok(0)
        }
        abi::SYS_EX_SIGRETURN => {
            Thread::current().return_from_signal();
            panic!("This code won't run");
        }
        abi::SYS_EX_KILL => {
            let target = SignalDestination::from(args[0] as isize);
            let signal = Signal::try_from(args[1] as u32)?;

            match target {
                SignalDestination::This => Process::current().set_signal(signal),
                SignalDestination::Process(pid) => Process::get(pid)
                    .ok_or(Errno::DoesNotExist)?
                    .set_signal(signal),
                _ => todo!(),
            };
            Ok(0)
        }
        abi::SYS_EX_CLONE => {
            let entry = args[0];
            let stack = args[1];
            let arg = args[2];

            Process::current()
                .new_user_thread(entry, stack, arg)
                .map(|e| e as usize)
        }
        abi::SYS_EX_YIELD => {
            proc::switch();
            Ok(0)
        },

        abi::SYS_SELECT => {
            let rfds = validate_user_ptr_struct_option::<FdSet>(args[0])?;
            let wfds = validate_user_ptr_struct_option::<FdSet>(args[1])?;
            let timeout = if args[2] == 0 {
                None
            } else {
                Some(Duration::from_nanos(args[2] as u64))
            };

            wait::select(Thread::current(), rfds, wfds, timeout)
        }
        abi::SYS_FACCESSAT => {
            let at_fd = FileDescriptor::from_i32(args[0] as i32)?;
            let path = validate_user_str(args[1], args[2])?;
            let mode = AccessMode::from_bits(args[3] as u32).ok_or(Errno::InvalidArgument)?;
            let flags = args[4] as u32;

            let proc = Process::current();
            let mut io = proc.io.lock();

            find_at_node(&mut io, at_fd, path, flags & AT_EMPTY_PATH != 0)?.check_access(io.ioctx(), mode)?;
            Ok(0)
        },

        _ => {
            let thread = Thread::current();
            let proc = thread.owner().unwrap();
            errorln!("Undefined system call: {}", num);
            proc.enter_fault_signal(thread, Signal::InvalidSystemCall);
            Err(Errno::InvalidArgument)
        }
    }
}
