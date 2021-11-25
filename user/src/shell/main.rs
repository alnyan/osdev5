#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;
extern crate alloc;

use alloc::{borrow::ToOwned, vec::Vec};
use libusr::io::{self, Read};
use libusr::signal::{self, SignalHandler};
use libusr::sys::{
    proc::Pid, sys_execve, sys_setpgid, sys_exit, sys_fork, sys_getpgid, sys_waitpid, Errno, ExitCode,
    sys_faccessat, AccessMode,
    FileDescriptor, Signal,
};

fn readline<'a, F: Read>(f: &mut F, bytes: &'a mut [u8]) -> Result<Option<&'a str>, io::Error> {
    let size = f.read(bytes)?;
    Ok(if size == 0 {
        None
    } else {
        Some(
            core::str::from_utf8(&bytes[..size])
                .unwrap()
                .trim_end_matches('\n'),
        )
    })
}

fn execute(line: &str) -> Result<ExitCode, Errno> {
    // TODO proper arg handling
    let args: Vec<&str> = line.split(' ').collect();
    let cmd = args[0];

    let filename = "/bin/".to_owned() + cmd;
    sys_faccessat(None, &filename, AccessMode::X_OK, 0)?;

    if let Some(pid) = unsafe { sys_fork()? } {
        let mut status = 0;
        sys_waitpid(pid, &mut status)?;
        let pgid = sys_getpgid(unsafe { Pid::from_raw(0) }).unwrap();
        io::tcsetpgrp(FileDescriptor::STDIN, pgid).unwrap();
        Ok(ExitCode::from(status))
    } else {
        let pgid = sys_setpgid(unsafe { Pid::from_raw(0) }, unsafe { Pid::from_raw(0) }).unwrap();
        io::tcsetpgrp(FileDescriptor::STDIN, pgid).unwrap();
        sys_execve(&filename, &args).unwrap();
        sys_exit(ExitCode::from(-1));
    }
}

#[no_mangle]
fn main() -> i32 {
    let mut buf = [0; 256];
    let mut stdin = io::stdin();

    signal::set_handler(Signal::Interrupt, SignalHandler::Ignore);
    let pgid = sys_setpgid(unsafe { Pid::from_raw(0) }, unsafe { Pid::from_raw(0) }).unwrap();
    io::tcsetpgrp(FileDescriptor::STDIN, pgid).unwrap();

    loop {
        print!("> ");
        match readline(&mut stdin, &mut buf) {
            Ok(line) => {
                if line.is_none() {
                    break;
                }
                let line = line.unwrap().trim_start_matches(' ');
                if line.is_empty() {
                    continue;
                }

                if let Err(e) = execute(line) {
                    eprintln!("{}: {:?}", line.split(' ').next().unwrap(), e);
                }
            },
            Err(_) => {
                println!("Interrupt!");
                continue;
            },
            _ => panic!()
        }
    }
    0
}
