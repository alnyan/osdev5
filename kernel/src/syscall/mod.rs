//! System call implementation

use crate::arch::{machine, platform::exception::ExceptionFrame};
use crate::mem::{virt::MapAttributes, phys::PageUsage};
use crate::debug::Level;
use crate::dev::timer::TimestampSource;
use crate::fs::create_filesystem;
use crate::proc::{self, elf, wait, Process, ProcessIo, Thread};
use core::mem::size_of;
use core::ops::DerefMut;
use core::time::Duration;
use libsys::{
    abi::SystemCall,
    debug::TraceLevel,
    error::Errno,
    ioctl::IoctlCmd,
    proc::{ExitCode, Pid, MemoryAccess},
    signal::{Signal, SignalDestination},
    stat::{
        AccessMode, DirectoryEntry, FdSet, FileDescriptor, FileMode, GroupId, MountOptions,
        OpenFlags, Stat, UserId, AT_EMPTY_PATH,
    },
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
        }
        SystemCall::Write => {
            let proc = Process::current();
            let fd = FileDescriptor::from(args[0] as u32);
            let mut io = proc.io.lock();
            let buf = arg::buf_ref(args[1], args[2])?;

            io.file(fd)?.borrow_mut().write(buf)
        }
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
        }
        SystemCall::Close => {
            let proc = Process::current();
            let mut io = proc.io.lock();
            let fd = FileDescriptor::from(args[0] as u32);

            io.close_file(fd)?;
            Ok(0)
        }
        SystemCall::FileStatus => {
            let at_fd = FileDescriptor::from_i32(args[0] as i32)?;
            let filename = arg::str_ref(args[1], args[2])?;
            let buf = arg::struct_mut::<Stat>(args[3])?;
            let flags = args[4] as u32;

            let proc = Process::current();
            let mut io = proc.io.lock();
            let stat =
                find_at_node(&mut io, at_fd, filename, flags & AT_EMPTY_PATH != 0)?.stat()?;
            *buf = stat;
            Ok(0)
        }
        SystemCall::Ioctl => {
            let fd = FileDescriptor::from(args[0] as u32);
            let cmd = IoctlCmd::try_from(args[1] as u32)?;

            let proc = Process::current();
            let mut io = proc.io.lock();

            let node = io.file(fd)?.borrow().node().ok_or(Errno::InvalidFile)?;
            node.ioctl(cmd, args[2], args[3])
        }
        SystemCall::Select => {
            let rfds = arg::option_struct_mut::<FdSet>(args[0])?;
            let wfds = arg::option_struct_mut::<FdSet>(args[1])?;
            let timeout = if args[2] == 0 {
                None
            } else {
                Some(Duration::from_nanos(args[2] as u64))
            };

            wait::select(Thread::current(), rfds, wfds, timeout)
        }
        SystemCall::Access => {
            let at_fd = FileDescriptor::from_i32(args[0] as i32)?;
            let path = arg::str_ref(args[1], args[2])?;
            let mode = AccessMode::from_bits(args[3] as u32).ok_or(Errno::InvalidArgument)?;
            let flags = args[4] as u32;

            let proc = Process::current();
            let mut io = proc.io.lock();

            find_at_node(&mut io, at_fd, path, flags & AT_EMPTY_PATH != 0)?
                .check_access(io.ioctx(), mode)?;
            Ok(0)
        }
        SystemCall::ReadDirectory => {
            let proc = Process::current();
            let fd = FileDescriptor::from(args[0] as u32);
            let mut io = proc.io.lock();
            let buf = arg::struct_buf_mut::<DirectoryEntry>(args[1], args[2])?;

            io.file(fd)?.borrow_mut().readdir(buf)
        }
        SystemCall::GetUserId => {
            let proc = Process::current();
            let uid = proc.io.lock().uid();
            Ok(u32::from(uid) as usize)
        }
        SystemCall::GetGroupId => {
            let proc = Process::current();
            let gid = proc.io.lock().gid();
            Ok(u32::from(gid) as usize)
        }
        SystemCall::DuplicateFd => {
            let src = FileDescriptor::from(args[0] as u32);
            let dst = FileDescriptor::from_i32(args[1] as i32)?;

            let proc = Process::current();
            let mut io = proc.io.lock();

            let res = io.duplicate_file(src, dst)?;

            Ok(u32::from(res) as usize)
        }
        SystemCall::SetUserId => {
            let uid = UserId::from(args[0] as u32);
            let proc = Process::current();
            proc.io.lock().set_uid(uid)?;
            Ok(0)
        }
        SystemCall::SetGroupId => {
            let gid = GroupId::from(args[0] as u32);
            let proc = Process::current();
            proc.io.lock().set_gid(gid)?;
            Ok(0)
        }
        SystemCall::SetCurrentDirectory => {
            let path = arg::str_ref(args[0], args[1])?;
            let proc = Process::current();
            proc.io.lock().ioctx().chdir(path)?;
            Ok(0)
        }
        SystemCall::GetCurrentDirectory => {
            todo!()
        }
        SystemCall::Seek => {
            todo!()
        }
        SystemCall::MapMemory => {
            let len = args[1];
            if len == 0 || (len & 0xFFF) != 0 {
                return Err(Errno::InvalidArgument);
            }
            let acc = MemoryAccess::from_bits(args[2] as u32).ok_or(Errno::InvalidArgument)?;
            let _flags = MemoryAccess::from_bits(args[3] as u32).ok_or(Errno::InvalidArgument)?;

            let mut attrs = MapAttributes::NOT_GLOBAL | MapAttributes::SH_OUTER | MapAttributes::PXN;
            if !acc.contains(MemoryAccess::READ) {
                return Err(Errno::NotImplemented);
            }
            if acc.contains(MemoryAccess::WRITE) {
                if acc.contains(MemoryAccess::EXEC) {
                    return Err(Errno::PermissionDenied);
                }
                attrs |= MapAttributes::AP_BOTH_READWRITE;
            } else {
                attrs |= MapAttributes::AP_BOTH_READONLY;
            }
            if !acc.contains(MemoryAccess::EXEC) {
                attrs |= MapAttributes::UXN;
            }

            // TODO don't ignore flags
            let usage = PageUsage::UserPrivate;

            let proc = Process::current();

            proc.manipulate_space(move |space| {
                space.allocate(0x100000000, 0xF00000000, len / 4096, attrs, usage)
            })
        }
        SystemCall::UnmapMemory => {
            let addr = args[0];
            let len = args[1];

            if addr == 0 || len == 0 || addr & 0xFFF != 0 || len & 0xFFF != 0 {
                return Err(Errno::InvalidArgument);
            }

            let proc = Process::current();
            proc.manipulate_space(move |space| {
                space.free(addr, len / 4096)
            })?;
            Ok(0)
        }

        // Process
        SystemCall::Clone => {
            let entry = args[0];
            let stack = args[1];
            let arg = args[2];

            Process::current()
                .new_user_thread(entry, stack, arg)
                .map(|e| e as usize)
        }
        SystemCall::Exec => {
            let filename = arg::str_ref(args[0], args[1])?;
            let argv = arg::struct_buf_ref::<&str>(args[2], args[3])?;
            // Validate each argument as well
            for item in argv.iter() {
                arg::validate_ptr(item.as_ptr() as usize, item.len(), false)?;
            }
            let node = {
                let proc = Process::current();
                let mut io = proc.io.lock();
                // TODO argv, envp array passing ABI?
                let node = io.ioctx().find(None, filename, true)?;
                drop(io);
                node
            };
            let file = node.open(OpenFlags::O_RDONLY)?;
            Process::execve(move |space| elf::load_elf(space, file), argv).unwrap();
            panic!();
        }
        SystemCall::Exit => {
            let status = ExitCode::from(args[0] as i32);
            let flags = args[1];

            if flags & (1 << 0) != 0 {
                Process::exit_thread(Thread::current(), status);
            } else {
                Process::current().exit(status);
            }

            unreachable!();
        }
        SystemCall::WaitPid => {
            // TODO special "pid" values
            let pid = Pid::try_from(args[0] as u32)?;
            let status = arg::struct_mut::<i32>(args[1])?;

            match Process::waitpid(pid) {
                Ok(exit) => {
                    *status = i32::from(exit);
                    Ok(0)
                }
                e => e.map(|e| i32::from(e) as usize),
            }
        }
        SystemCall::WaitTid => {
            let tid = args[0] as u32;

            match Thread::waittid(tid) {
                Ok(_) => Ok(0),
                _ => todo!(),
            }
        }
        SystemCall::GetPid => Ok(u32::from(Process::current().id()) as usize),
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
        }
        SystemCall::SetSignalEntry => {
            Thread::current().set_signal_entry(args[0], args[1]);
            Ok(0)
        }
        SystemCall::SignalReturn => {
            Thread::current().return_from_signal();
            unreachable!();
        }
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
        }
        SystemCall::Yield => {
            proc::switch();
            Ok(0)
        }
        SystemCall::GetSid => {
            // TODO handle kernel processes here?
            let pid = args[0] as u32;
            let current = Process::current();
            let proc = if pid == 0 {
                current
            } else {
                let pid = Pid::try_from(pid)?;
                let proc = Process::get(pid).ok_or(Errno::DoesNotExist)?;
                if proc.sid() != current.sid() {
                    return Err(Errno::PermissionDenied);
                }
                proc
            };

            Ok(u32::from(proc.sid()) as usize)
        }
        SystemCall::GetPgid => {
            // TODO handle kernel processes here?
            let pid = args[0] as u32;
            let current = Process::current();
            let proc = if pid == 0 {
                current
            } else {
                let pid = Pid::try_from(pid)?;
                Process::get(pid).ok_or(Errno::DoesNotExist)?
            };

            Ok(u32::from(proc.pgid()) as usize)
        }
        SystemCall::GetPpid => Ok(u32::from(Process::current().ppid().unwrap()) as usize),
        SystemCall::SetSid => {
            let proc = Process::current();
            let mut io = proc.io.lock();

            if let Some(_ctty) = io.ctty() {
                todo!();
            }

            let id = proc.id();
            proc.set_sid(id);
            Ok(u32::from(id) as usize)
        }
        SystemCall::SetPgid => {
            let pid = args[0] as u32;
            let pgid = args[1] as u32;

            let current = Process::current();
            let proc = if pid == 0 { current } else { todo!() };

            if pgid == 0 {
                proc.set_pgid(proc.id());
            } else {
                todo!();
            }

            Ok(u32::from(proc.pgid()) as usize)
        }

        // System
        SystemCall::GetCpuTime => {
            let time = machine::local_timer().timestamp()?;
            Ok(time.as_nanos() as usize)
        }
        SystemCall::Mount => {
            let target = arg::str_ref(args[0], args[1])?;
            let options = arg::struct_ref::<MountOptions>(args[2])?;

            let proc = Process::current();
            let mut io = proc.io.lock();

            debugln!("mount(target={:?}, options={:#x?})", target, options);

            let target_node = io.ioctx().find(None, target, true)?;
            let root = create_filesystem(options)?;

            target_node.mount(root)?;

            Ok(0)
        }

        // Debugging
        SystemCall::DebugTrace => {
            let level = TraceLevel::from_repr(args[0])
                .map(Level::from)
                .ok_or(Errno::InvalidArgument)?;
            let buf = arg::str_ref(args[1], args[2])?;
            let thread = Thread::current();
            let proc = thread.owner().unwrap();
            println!(level, "[trace {:?}:{}] {}", proc.id(), thread.id(), buf);
            Ok(args[1])
        }

        // Handled elsewhere
        SystemCall::Fork => unreachable!(),
    }
}
