#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

#[no_mangle]
fn main() -> i32 {
    let pid = unsafe { libusr::sys::sys_fork() };
    if pid < 0 {
        eprintln!("fork() failed");
        return -1;
    } else if pid == 0 {
        return unsafe { libusr::sys::sys_execve("/bin/shell") };
    } else {
        let mut status = 0;
        let res = unsafe { libusr::sys::sys_waitpid(pid as u32, &mut status) };
        if res == 0 {
            println!("Process {} exited with status {}", pid, status);
        } else {
            eprintln!("waitpid() failed");
        }
    }
    loop {}
}
