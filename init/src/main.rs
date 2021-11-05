#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

#[no_mangle]
fn main() -> i32 {
    println!("Pre-fork");
    let pid = unsafe { libusr::sys::sys_fork() };
    println!("Post-fork: {}", pid);

    -1
}
