#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;
extern crate alloc;

use alloc::borrow::ToOwned;
use libusr::sys::{sys_faccessat, sys_exit, sys_execve, sys_waitpid, sys_fork, ExitCode, Errno, AccessMode};
use libusr::io::{self, Read};

fn readline<'a, F: Read>(f: &mut F, bytes: &'a mut [u8]) -> Result<Option<&'a str>, io::Error> {
    let size = f.read(bytes)?;
    Ok(if size == 0 {
        None
    } else {
        Some(core::str::from_utf8(&bytes[..size]).unwrap().trim_end_matches('\n'))
    })
}

fn execvp(cmd: &str) -> ! {
    sys_execve(&("/bin/".to_owned() + cmd));
    sys_exit(ExitCode::from(-1));
}

fn execute(line: &str) -> Result<ExitCode, Errno> {
    let mut words = line.split(' ');
    let cmd = words.next().unwrap();

    if let Some(pid) = unsafe { sys_fork()? } {
        let mut status = 0;
        sys_waitpid(pid, &mut status)?;
        Ok(ExitCode::from(status))
    } else {
        execvp(cmd);
    }
}

#[no_mangle]
fn main() -> i32 {
    let mut buf = [0; 256];
    let mut stdin = io::stdin();

    loop {
        print!("> ");
        let line = readline(&mut stdin, &mut buf).unwrap();
        if line.is_none() {
            break;
        }
        let line = line.unwrap().trim_start_matches(' ');
        if line.is_empty() {
            continue;
        }

        execute(line);
    }
    0
}
