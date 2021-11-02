#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

#[no_mangle]
fn main() -> i32 {
    loop {
        println!("Hello to stdout");
        trace!("Hello from userspace");

        unsafe {
            libusr::sys::sys_ex_nanosleep(1_000_000_000, core::ptr::null_mut());
        }
    }
}
