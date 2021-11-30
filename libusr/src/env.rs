use libsys::{debug::TraceLevel, ProgramArgs};
use alloc::vec::Vec;
use crate::trace;

static mut PROGRAM_ARGS: Vec<&'static str> = Vec::new();

pub fn args() -> &'static [&'static str] {
    unsafe { &PROGRAM_ARGS }
}

pub(crate) unsafe fn setup_env(arg: &ProgramArgs) {
    for i in 0..arg.argc {
        let base = core::ptr::read((arg.argv + i * 16) as *const *const u8);
        let len = core::ptr::read((arg.argv + i * 16 + 8) as *const usize);

        let string = core::str::from_utf8(core::slice::from_raw_parts(base, len)).unwrap();
        PROGRAM_ARGS.push(string);
    }

    trace!(TraceLevel::Debug, "args = {:?}", PROGRAM_ARGS);
}
