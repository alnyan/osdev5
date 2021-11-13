#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use libusr::sys::stat::{FdSet, FileDescriptor};
use libusr::sys::{Signal, SignalDestination};

fn readline(fd: FileDescriptor, buf: &mut [u8]) -> Result<&str, ()> {
    // select() just for test
    loop {
        let mut rfds = FdSet::empty();
        rfds.set(fd);
        let res = unsafe {
            libusr::sys::sys_select(Some(&mut rfds), None, 1_000_000_000)
        };
        if res < 0 {
            return Err(());
        }
        if res == 0 {
            continue;
        }
        if !rfds.is_set(fd) {
            panic!();
        }

        let count = unsafe { libusr::sys::sys_read(fd, buf) };
        if count >= 0 {
            return core::str::from_utf8(&buf[..count as usize]).map_err(|_| ());
        } else {
            return Err(());
        }
    }
}

#[no_mangle]
fn main() -> i32 {
    let mut buf = [0; 512];

    loop {
        print!("> ");
        let line = readline(FileDescriptor::STDIN, &mut buf).unwrap();
        if line.is_empty() {
            break;
        }
        let line = line.trim_end_matches('\n');

        println!(":: {:?}", line);

        if line == "test" {
            unsafe {
                libusr::sys::sys_ex_kill(SignalDestination::This, Signal::Interrupt);
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
