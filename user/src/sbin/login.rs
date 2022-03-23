#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use libsys::{
    error::Errno,
    ioctl::IoctlCmd,
    stat::{FileDescriptor, FileMode, GroupId, OpenFlags, UserId},
    termios::{Termios, TermiosLflag},
};
use libusr::sys::{
    sys_close, sys_dup, sys_execve, sys_fork, sys_getgid, sys_getpgid, sys_getuid, sys_ioctl,
    sys_openat, sys_read, sys_setgid, sys_setpgid, sys_setsid, sys_setuid, sys_waitpid,
};
use libusr::{
    env::{self, UserInfo, UserShadow},
    io,
};

struct HiddenInput {
    fd: FileDescriptor,
    termios: Termios,
}

impl HiddenInput {
    fn open(fd: FileDescriptor) -> Result<Self, Errno> {
        use core::mem::{size_of, MaybeUninit};
        let mut termios: MaybeUninit<Termios> = MaybeUninit::uninit();
        sys_ioctl(
            fd,
            IoctlCmd::TtyGetAttributes,
            termios.as_mut_ptr() as usize,
            size_of::<Termios>(),
        )?;
        let termios = unsafe { termios.assume_init() };

        let mut new_termios = termios.clone();
        new_termios.lflag &= !(TermiosLflag::ECHO | TermiosLflag::ECHOK | TermiosLflag::ECHOE);
        sys_ioctl(
            fd,
            IoctlCmd::TtySetAttributes,
            &new_termios as *const _ as usize,
            size_of::<Termios>(),
        )?;

        Ok(Self { fd, termios })
    }

    fn readline<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a str, Errno> {
        readline(self.fd, buf)
    }
}

impl Drop for HiddenInput {
    fn drop(&mut self) {
        use core::mem::size_of;
        sys_ioctl(
            self.fd,
            IoctlCmd::TtySetAttributes,
            &self.termios as *const _ as usize,
            size_of::<Termios>(),
        )
        .ok();
    }
}

fn readline(fd: FileDescriptor, buf: &mut [u8]) -> Result<&str, Errno> {
    let len = sys_read(fd, buf)?;

    if len == 0 {
        Ok("")
    } else {
        Ok(core::str::from_utf8(&buf[..len - 1]).unwrap())
    }
}

fn login(uid: UserId, gid: GroupId, shell: &str) -> Result<(), Errno> {
    if let Some(pid) = unsafe { sys_fork() }? {
        let mut status = 0;
        sys_waitpid(pid, &mut status).ok();
        let pgid = sys_getpgid(None).unwrap();
        io::tcsetpgrp(FileDescriptor::STDIN, pgid).unwrap();
        Ok(())
    } else {
        sys_setuid(uid).expect("setuid failed");
        sys_setgid(gid).expect("setgid failed");
        let pgid = sys_setpgid(None, None).unwrap();
        io::tcsetpgrp(FileDescriptor::STDIN, pgid).unwrap();
        sys_execve(shell, &[shell]).expect("execve() failed");
        panic!();
    }
}

fn login_as(name: &str) -> Result<(), Errno> {
    let ent = UserInfo::by_name(name).map_err(|_| Errno::DoesNotExist)?;
    login(ent.uid(), ent.gid(), ent.shell())
}

// TODO baud rate and misc port settings
#[no_mangle]
fn main() -> i32 {
    if !sys_getuid().is_root() || !sys_getgid().is_root() {
        panic!("This program must be run as root");
    }

    let args = env::args();
    if args.len() != 2 {
        panic!("Usage: {} TTY", args[0]);
    }

    sys_setsid().expect("setsid() failed");

    // Close controlling terminal
    // NOTE this will invalidate rust-side Stdin, Stdout, Stderr
    //      until replacement is re-opened using the specified TTY
    sys_close(FileDescriptor::STDERR).unwrap();
    sys_close(FileDescriptor::STDOUT).unwrap();
    sys_close(FileDescriptor::STDIN).unwrap();

    sys_openat(
        None,
        args[1],
        FileMode::default_reg(),
        OpenFlags::O_RDONLY | OpenFlags::O_CTTY,
    )
    .expect("Failed to open stdin");
    sys_openat(
        None,
        args[1],
        FileMode::default_reg(),
        OpenFlags::O_WRONLY | OpenFlags::O_CTTY,
    )
    .expect("Failed to open stdout");
    sys_dup(FileDescriptor::STDOUT, Some(FileDescriptor::STDERR)).expect("Failed to open stderr");

    let mut user_buf = [0; 128];
    let mut password_buf = [0; 128];
    loop {
        print!("login: ");
        let username = readline(FileDescriptor::STDIN, &mut user_buf).expect("Login read failed");

        let shadow = match UserShadow::by_name(username) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !shadow.password().is_empty() {
            print!("password: ");
            let password = {
                let mut input = HiddenInput::open(FileDescriptor::STDIN).unwrap();
                input.readline(&mut password_buf)
            }
            .expect("Password read failed");

            if password != shadow.password() {
                eprintln!("Incorrect password");
                continue;
            }
        }

        login_as(username).ok();
    }
}
