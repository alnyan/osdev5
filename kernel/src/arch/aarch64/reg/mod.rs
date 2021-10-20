//! AArch64 architectural registers

pub mod cpacr_el1;
pub use cpacr_el1::CPACR_EL1;

pub mod cntkctl_el1;
pub use cntkctl_el1::CNTKCTL_EL1;
