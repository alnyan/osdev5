#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use core::arch::asm;

use libusr::sys::{stat::MountOptions, sys_execve, sys_fork, sys_mount, sys_waitpid};

#[no_mangle]
fn main() -> i32 {
    println!("Test");
    trace_debug!("test!");

    sys_mount(
        "/dev",
        &MountOptions {
            device: None,
            fs: Some("devfs"),
        },
    )
    .expect("Failed to mount devfs");
    sys_mount(
        "/sys",
        &MountOptions {
            device: None,
            fs: Some("sysfs"),
        },
    )
    .expect("Failed to mount sysfs");

    let pid = unsafe { sys_fork().unwrap() };

    trace_debug!("fork returned {:?}", pid);

    loop {}

    // loop {}

    // if let Some(pid) = unsafe { sys_fork().unwrap() } {
    //     let mut status = 0;
    //     sys_waitpid(pid, &mut status).unwrap();
    //     println!("Process {:?} exited with status {}", pid, status);

    //     loop {
    //     }
    // } else {
    //     sys_execve("/sbin/login", &["/sbin/login", "/dev/ttyS0"]).unwrap();
    //     unreachable!();
    // }
}
