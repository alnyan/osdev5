#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;
#[macro_use]
extern crate lazy_static;

use libusr::io::{self, Read};
use libusr::sys::{Signal, SignalDestination};
use libusr::sync::Mutex;

static mut THREAD_STACK: [u8; 8192] = [0; 8192];
static mut THREAD_SIGNAL_STACK: [u8; 8192] = [0; 8192];
lazy_static! {
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

fn sleep(ns: u64) {
    let mut rem = [0; 2];
    libusr::sys::sys_ex_nanosleep(ns, &mut rem).unwrap();
}

fn fn0_signal(arg: Signal) {
    trace!("fn0_signal");
    unsafe {
        libusr::sys::sys_exit(libusr::sys::ExitCode::from(0));
    }
}

fn fn0(_arg: usize) {
    unsafe {
        libusr::sys::sys_ex_signal(fn0_signal as usize, THREAD_SIGNAL_STACK.as_mut_ptr().add(8192) as usize);
    }

    unsafe {
        core::ptr::read_volatile(0x1234 as *const u32);
    }
    loop {}
    //loop {
    //    sleep(100_000_000);
    //    println!("Tick from B");
    //    {
    //        let lock = MUTEX.lock();
    //        sleep(1_000_000_000);
    //    }
    //}
}

fn do_fault() {
    unsafe {
        core::ptr::read_volatile(0x1238 as *const u32);
    }
}

#[no_mangle]
fn main() -> i32 {
    unsafe {
        libusr::sys::sys_ex_clone(fn0 as usize, THREAD_STACK.as_mut_ptr().add(8192) as usize, 0);
    }

    sleep(1_000_000_000);

    do_fault();

    loop {}

    0
}
