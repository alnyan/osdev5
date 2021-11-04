#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use libusr::io;
use libusr::sys::{OpenFlags, AT_FDCWD};

#[no_mangle]
fn main() -> i32 {
    loop {
        let pid = unsafe { libusr::sys::sys_fork() };

        if pid == 0 {
            trace!("Hello!");
            unsafe {
                libusr::sys::sys_ex_nanosleep(3_000_000_000, core::ptr::null_mut());
            }
            trace!("Exiting");
            return 0;
        } else {
            trace!("Spawned {}", pid);
            unsafe {
                libusr::sys::sys_ex_nanosleep(5_000_000_000, core::ptr::null_mut());
            }
        }
    }
    //let mut buf = [0; 128];

    // print!("\x1B[2J\x1B[1;1H");
    // println!("Hello!");

    // loop {
    //     print!("> ");

    //     let count = unsafe {
    //         libusr::sys::sys_read(0, buf.as_mut_ptr(), buf.len())
    //     };
    //     if count < 0 {
    //         trace!("Read from stdio failed");
    //         break;
    //     }
    //     let count = count as usize;

    //     if let Ok(s) = core::str::from_utf8(&buf[..count]) {
    //         println!("Got string {:?}", s);

    //         if s == "quit" {
    //             break;
    //         }
    //     } else {
    //         println!("Got string (non-utf8) {:?}", &buf[..count]);
    //     }
    // }
    // -1
}
