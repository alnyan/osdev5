#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use libusr::io::{self, Read};

#[no_mangle]
fn main() -> i32 {
    let mut buf = [0; 512];
    let mut stdin = io::stdin();

    eprintln!("stderr test");

    loop {
        let count = stdin.read(&mut buf).unwrap();
        if count == 0 {
            break;
        }
        let line = core::str::from_utf8(&buf[..count]).unwrap();
        println!("{:?}", line);
    }

    0
}
