use super::{table::TableImpl, SpaceImpl};

// Upper mappings
#[no_mangle]
static KERNEL_PDPT: TableImpl = TableImpl::empty();
#[no_mangle]
static KERNEL_PD0: TableImpl = TableImpl::empty();
#[no_mangle]
static KERNEL_PD1: TableImpl = TableImpl::empty();

#[no_mangle]
static KERNEL_PML4: TableImpl = TableImpl::empty();
