//! System call implementation

use crate::arch::{machine, platform::exception::ExceptionFrame};
use crate::debug::Level;
use crate::proc::{self, elf, wait, Process, ProcessIo, Thread};
use crate::dev::timer::TimestampSource;
use core::mem::size_of;
use core::ops::DerefMut;
use core::time::Duration;
use libsys::{
    abi::SystemCall,
    error::Errno,
    ioctl::IoctlCmd,
    proc::{ExitCode, Pid},
    signal::{Signal, SignalDestination},
    stat::{FdSet, AccessMode, FileDescriptor, FileMode, OpenFlags, Stat, AT_EMPTY_PATH},
    traits::{Read, Write},
};
use vfs::VnodeRef;

pub mod arg;

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
pub fn syscall(num: SystemCall, args: &[usize]) -> Result<usize, Errno> {
    match num {
        // I/O
        SystemCall::Read => {
            let proc = Process::current();
            let fd = FileDescriptor::from(args[0] as u32);
            let mut io = proc.io.lock();
            let buf = arg::buf_mut(args[1], args[2])?;

            io.file(fd)?.borrow_mut().read(buf)
        },
        SystemCall::Write => {
            let proc = Process::current();
            let fd = FileDescriptor::from(args[0] as u32);
            let mut io = proc.io.lock();
            let buf = arg::buf_ref(args[1], args[2])?;

            io.file(fd)?.borrow_mut().write(buf)
        },
        SystemCall::Open => {
            let at_fd = FileDescriptor::from_i32(args[0] as i32)?;
            let path = arg::str_ref(args[1], args[2])?;
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
        },
        SystemCall::Close => {
            let proc = Process::current();
            let mut io = proc.io.lock();
            let fd = FileDescriptor::from(args[0] as u32);

            io.close_file(fd)?;
            Ok(0)
        },
        SystemCall::FileStatus => {
            let at_fd = FileDescriptor::from_i32(args[0] as i32)?;
            let filename = arg::str_ref(args[1], args[2])?;
            let buf = arg::struct_mut::<Stat>(args[3])?;
            let flags = args[4] as u32;

            let proc = Process::current();
            let mut io = proc.io.lock();
            find_at_node(&mut io, at_fd, filename, flags & AT_EMPTY_PATH != 0)?.stat(buf)?;
            Ok(0)
        },
        SystemCall::Ioctl => {
            let fd = FileDescriptor::from(args[0] as u32);
            let cmd = IoctlCmd::try_from(args[1] as u32)?;

            let proc = Process::current();
            let mut io = proc.io.lock();

            let node = io.file(fd)?.borrow().node().ok_or(Errno::InvalidFile)?;
            node.ioctl(cmd, args[2], args[3])
        },
        SystemCall::Select => {
            let rfds = arg::option_struct_mut::<FdSet>(args[0])?;
            let wfds = arg::option_struct_mut::<FdSet>(args[1])?;
            let timeout = if args[2] == 0 {
                None
            } else {
                Some(Duration::from_nanos(args[2] as u64))
            };

            wait::select(Thread::current(), rfds, wfds, timeout)
        },
        SystemCall::Access => {
            let at_fd = FileDescriptor::from_i32(args[0] as i32)?;
            let path = arg::str_ref(args[1], args[2])?;
            let mode = AccessMode::from_bits(args[3] as u32).ok_or(Errno::InvalidArgument)?;
            let flags = args[4] as u32;

            let proc = Process::current();
            let mut io = proc.io.lock();

            find_at_node(&mut io, at_fd, path, flags & AT_EMPTY_PATH != 0)?.check_access(io.ioctx(), mode)?;
            Ok(0)
        },

        // Process
        SystemCall::Clone => {
            let entry = args[0];
            let stack = args[1];
            let arg = args[2];

            Process::current()
                .new_user_thread(entry, stack, arg)
                .map(|e| e as usize)
        },
        SystemCall::Exec => {
            let node = {
                let proc = Process::current();
                let mut io = proc.io.lock();
                let filename = arg::str_ref(args[0], args[1])?;
                // TODO argv, envp array passing ABI?
                let node = io.ioctx().find(None, filename, true)?;
                drop(io);
                node
            };
            let file = node.open(OpenFlags::O_RDONLY)?;
            Process::execve(move |space| elf::load_elf(space, file), 0).unwrap();
            panic!();
        },
        SystemCall::Exit => {
            let status = ExitCode::from(args[0] as i32);
            let flags = args[1];

            if flags & (1 << 0) != 0 {
                Process::exit_thread(Thread::current(), status);
            } else {
                Process::exit(status);
            }

            unreachable!();
        },
        SystemCall::WaitPid => {
            // TODO special "pid" values
            let pid = unsafe { Pid::from_raw(args[0] as u32) };
            let status = arg::struct_mut::<i32>(args[1])?;

            match Process::waitpid(pid) {
                Ok(exit) => {
                    *status = i32::from(exit);
                    Ok(0)
                }
                _ => todo!(),
            }
        },
        SystemCall::WaitTid => {
            let tid = args[0] as u32;

            match Thread::waittid(tid) {
                Ok(_) => {
                    Ok(0)
                },
                _ => todo!(),
            }
        },
        SystemCall::GetPid => todo!(),
        SystemCall::GetTid => Ok(Thread::current().id() as usize),
        SystemCall::Sleep => {
            let rem_buf = arg::option_buf_ref(args[1], size_of::<u64>() * 2)?;
            let mut rem = Duration::new(0, 0);
            let res = wait::sleep(Duration::from_nanos(args[0] as u64), &mut rem);
            if res == Err(Errno::Interrupt) {
                warnln!("Sleep interrupted, {:?} remaining", rem);
                if rem_buf.is_some() {
                    todo!()
                }
            }
            res.map(|_| 0)
        },
        SystemCall::SetSignalEntry => {
            Thread::current().set_signal_entry(args[0], args[1]);
            Ok(0)
        },
        SystemCall::SignalReturn => {
            Thread::current().return_from_signal();
            unreachable!();
        },
        SystemCall::SendSignal => {
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
        },
        SystemCall::Yield => {
            proc::switch();
            Ok(0)
        },

        // System
        SystemCall::GetCpuTime => {
            let time = machine::local_timer().timestamp()?;
            Ok(time.as_nanos() as usize)
        },

        // Debugging
        SystemCall::DebugTrace => {
            let buf = arg::str_ref(args[0], args[1])?;
            print!(Level::Debug, "[trace] ");
            print!(Level::Debug, "{}", buf);
            println!(Level::Debug, "");
            Ok(args[1])
        },

        // Handled elsewhere
        SystemCall::Fork => unreachable!()
    }
}
