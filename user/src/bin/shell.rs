#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;
extern crate alloc;

use alloc::{borrow::ToOwned, vec::Vec};
use libusr::io::{self, Read};
use libusr::signal::{self, SignalHandler};
use libusr::sys::{
    sys_chdir, sys_execve, sys_exit, sys_faccessat, sys_fork, sys_getpgid, sys_setpgid,
    sys_waitpid, AccessMode, Errno, ExitCode, FileDescriptor, Signal,
};

struct Builtin {
    func: fn(&[&str]) -> ExitCode,
    name: &'static str,
}

fn cmd_cd(args: &[&str]) -> ExitCode {
    if args.len() != 2 {
        eprintln!("Usage: cd DIR");
        ExitCode::from(-1)
    } else if let Err(err) = sys_chdir(args[1]) {
        eprintln!("{}: {:?}", args[1], err);
        ExitCode::from(-1)
    } else {
        ExitCode::from(0)
    }
}

static BUILTINS: [Builtin; 1] = [Builtin {
    name: "cd",
    func: cmd_cd,
}];

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

    for item in BUILTINS.iter() {
        if item.name == cmd {
            return Ok((item.func)(&args));
        }
    }

    let filename = "/bin/".to_owned() + cmd;
    sys_faccessat(None, &filename, AccessMode::X_OK, 0)?;

    if let Some(pid) = unsafe { sys_fork()? } {
        let mut status = 0;
        sys_waitpid(pid, &mut status)?;
        let pgid = sys_getpgid(None).unwrap();
        io::tcsetpgrp(FileDescriptor::STDIN, pgid).unwrap();
        Ok(ExitCode::from(status))
    } else {
        let pgid = sys_setpgid(None, None).unwrap();
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
    let pgid = sys_setpgid(None, None).unwrap();
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
            }
            Err(_) => {
                println!("Interrupt!");
                continue;
            }
        }
    }
    0
}
