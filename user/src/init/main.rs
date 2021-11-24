#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

#[no_mangle]
fn main() -> i32 {
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
        libusr::sys::sys_execve("/bin/shell", &["/bin/shell"]).unwrap();
        loop {}
    }
}
