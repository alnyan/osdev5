#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use libusr::sys::{stat::MountOptions, sys_execve, sys_fork, sys_mount, sys_waitpid};

#[no_mangle]
fn main() -> i32 {
    sys_mount(
        "/dev",
        &MountOptions {
            device: None,
            fs: Some("devfs"),
        },
    )
    .expect("Failed to mount devfs");

    let pid = unsafe { libusr::sys::sys_fork().unwrap() };

    if let Some(pid) = pid {
        let mut status = 0;
        libusr::sys::sys_waitpid(pid, &mut status).unwrap();
        println!("Process {:?} exited with status {}", pid, status);

        loop {
            unsafe {
                asm!("nop");
            }
        }
    } else {
        libusr::sys::sys_execve("/sbin/login", &["/sbin/login", "/dev/ttyS0"]).unwrap();
        loop {}
    }
}
