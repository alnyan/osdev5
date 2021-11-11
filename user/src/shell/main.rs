#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use libusr::sys::Signal;

fn readline(fd: i32, buf: &mut [u8]) -> Result<&str, ()> {
    let count = unsafe { libusr::sys::sys_read(fd, buf) };
    if count >= 0 {
        core::str::from_utf8(&buf[..count as usize]).map_err(|_| ())
    } else {
        Err(())
    }
}

#[no_mangle]
fn main() -> i32 {
    let mut buf = [0; 512];
    loop {
        print!("> ");
        let line = readline(libusr::sys::STDIN_FILENO, &mut buf).unwrap();
        if line.is_empty() {
            break;
        }
        let line = line.trim_end_matches('\n');

        println!(":: {:?}", line);

        if line == "test" {
            unsafe {
                libusr::sys::sys_ex_kill(0, Signal::Interrupt);
            }
            trace!("Returned from signal");
            continue;
        }

        if line == "quit" || line == "exit" {
            break;
        }
    }

    0
}
