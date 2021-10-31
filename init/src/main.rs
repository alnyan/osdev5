#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

#[no_mangle]
fn main() -> i32 {
    loop {
        trace!("Hello from userspace");
        unsafe {
            asm!("svc #0", in("x8") 121, in("x0") 1000000000);
        }
    }
    123
}
