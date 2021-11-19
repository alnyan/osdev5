#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;
#[macro_use]
extern crate lazy_static;

use libusr::thread;
use libusr::io::{self, Read};
use libusr::sys::{Signal, SignalDestination};
use libusr::sync::Mutex;

fn sleep(ns: u64) {
    let mut rem = [0; 2];
    libusr::sys::sys_ex_nanosleep(ns, &mut rem).unwrap();
}

#[no_mangle]
fn main() -> i32 {
    let value = 1234;
    let thread = thread::spawn(move || {
        trace!("Closure is alive: {}", value);
        sleep(2_000_000_000);
        trace!("Closure will now exit");

        value - 100
    });
    sleep(1_000_000_000);

    trace!("???");

    trace!("Thread joined: {:?}", thread.join());

    0
}
