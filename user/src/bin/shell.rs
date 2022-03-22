#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;
extern crate alloc;

use alloc::{borrow::ToOwned, vec::Vec};
use libusr::io::{self, Read};
use libusr::signal::{self, SignalHandler};
use libusr::sys::{
    sys_chdir, sys_ex_nanosleep, sys_execve, sys_exit, sys_faccessat, sys_fork, sys_getpgid,
    sys_setpgid, sys_waitpid, AccessMode, Errno, ExitCode, FileDescriptor, Signal,
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

fn cmd_sleep(args: &[&str]) -> ExitCode {
    if args.len() == 2 {
        match args[1].parse::<u32>() {
            Err(e) => {
                eprintln!("{}: {:?}", args[1], e);
                ExitCode::from(-1)
            }
            Ok(count) => {
                let mut rem = [0; 2];
                if let Err(err) = sys_ex_nanosleep((count as u64) * 1000000000, &mut rem) {
                    eprintln!("Sleep failed (rem. {:?}): {:?}", rem, err);
                    ExitCode::from(-1)
                } else {
                    ExitCode::from(0)
                }
            }
        }
    } else {
        eprintln!("Usage: sleep SECS");
        ExitCode::from(-1)
    }
}

static BUILTINS: [Builtin; 2] = [
    Builtin {
        name: "cd",
        func: cmd_cd,
    },
    Builtin {
        name: "sleep",
        func: cmd_sleep,
    },
];

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
