use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields,
};
use crate::arch::x86_64::intrin::{rdmsr, wrmsr};

register_bitfields! {
    u64,
    pub MSR_IA32_EFER [
    ]
}

wrap_msr!(MSR_IA32_EFER, 0xC0000080);
