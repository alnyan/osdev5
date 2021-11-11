#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use core::cmp::Ordering;

#[no_mangle]
fn main() -> i32 {
    let pid = unsafe { libusr::sys::sys_fork() };
    match pid.cmp(&0) {
        Ordering::Less => {
            eprintln!("fork() failed");
            -1
        }
        Ordering::Equal => unsafe { libusr::sys::sys_execve("/bin/shell") },
        Ordering::Greater => {
            let mut status = 0;
            let res = unsafe { libusr::sys::sys_waitpid(pid as u32, &mut status) };
            if res == 0 {
                println!("Process {} exited with status {}", pid, status);
            } else {
                eprintln!("waitpid() failed");
            }

            loop {
                unsafe {
                    asm!("nop");
                }
            }
        }
    }
}
