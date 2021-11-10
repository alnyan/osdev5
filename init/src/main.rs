#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

#[no_mangle]
fn main() -> i32 {
    loop {
        trace!("Hello from userspace");
        for _ in 0..100000 {
            unsafe { asm!("nop"); }
        }
    }
    123
}
