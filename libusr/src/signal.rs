use libsys::{calls::sys_ex_sigreturn, signal::Signal};
use crate::trace;

#[inline(never)]
pub(crate) extern "C" fn signal_handler(arg: Signal) -> ! {
    trace!("Entered signal handler: arg={:?}", arg);
    match arg {
        Signal::Interrupt | Signal::SegmentationFault =>
            loop {},
        _ => todo!()
    }
    sys_ex_sigreturn();
}
