#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

#[no_mangle]
fn main() -> i32 {
    let mut buf = [0; 128];

    print!("\x1B[2J\x1B[1;1H");
    println!("Hello!");

    loop {
        print!("> ");

        let count = unsafe {
            libusr::sys::sys_read(0, buf.as_mut_ptr(), buf.len())
        };
        if count < 0 {
            trace!("Read from stdio failed");
            break;
        }
        let count = count as usize;

        if let Ok(s) = core::str::from_utf8(&buf[..count]) {
            println!("Got string {:?}", s);

            if s == "quit" {
                break;
            }
        } else {
            println!("Got string (non-utf8) {:?}", &buf[..count]);
        }
    }
    -1
}
