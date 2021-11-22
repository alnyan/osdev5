use crate::trace;
use libsys::{
    calls::{sys_ex_sigreturn, sys_exit},
    proc::ExitCode,
    signal::Signal,
};

#[derive(Clone, Copy)]
pub enum SignalHandler {
    Func(fn(Signal) -> ()),
    Ignore,
    Terminate,
}

// TODO per-thread signal handler table
static mut SIGNAL_HANDLERS: [SignalHandler; 32] = [SignalHandler::Terminate; 32];

pub fn set_handler(sig: Signal, handler: SignalHandler) -> SignalHandler {
    unsafe {
        let old = SIGNAL_HANDLERS[sig as usize];
        SIGNAL_HANDLERS[sig as usize] = handler;
        old
    }
}

#[inline(never)]
pub(crate) extern "C" fn signal_handler(arg: Signal) -> ! {
    // TODO tpidr_el0 is invalidated when entering signal context
    trace!("Entered signal handler: arg={:?}", arg);
    let no = arg as usize;
    if no >= 32 {
        panic!("Undefined signal number: {}", no);
    }
    match unsafe { SIGNAL_HANDLERS[no] } {
        SignalHandler::Func(f) => f(arg),
        SignalHandler::Ignore => (),
        SignalHandler::Terminate => sys_exit(ExitCode::from(-1)),
    }

    sys_ex_sigreturn();
}
