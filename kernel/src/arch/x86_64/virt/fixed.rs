use super::table::TableImpl;

#[repr(C, align(0x1000))]
pub struct FixedTableGroup {
    pub pml4: TableImpl,
    pub pdpt: TableImpl,
    pub pd: [TableImpl; 16],
}

// Upper mappings
#[no_mangle]
pub(super) static mut KERNEL_FIXED: FixedTableGroup = FixedTableGroup {
    pml4: TableImpl::empty(),
    pdpt: TableImpl::empty(),
    pd: [TableImpl::empty(); 16],
};
