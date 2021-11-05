#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use libusr::sys::{Stat, AT_FDCWD, OpenFlags, FileMode, AT_EMPTY_PATH};

#[no_mangle]
fn main() -> i32 {
    let mut stat = Stat::default();
    let fd = unsafe {
        libusr::sys::sys_openat(AT_FDCWD, "/test.txt", FileMode::empty(), OpenFlags::O_RDONLY)
    };
    println!("fd = {}", fd);
    let ret = unsafe {
        libusr::sys::sys_fstatat(fd, "", &mut stat, AT_EMPTY_PATH)
    };
    println!("{}, {:?}", ret, stat);

    -1
}
