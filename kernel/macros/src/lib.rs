extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(TtyCharDevice)]
pub fn derive_tty_char_device(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    if !ast.generics.params.is_empty() {
        panic!(
            "Derived TtyCharDevice cannot have generic parameters: {:?}",
            ast.ident
        );
    }
    let ident = ast.ident;

    quote! {
        impl vfs::CharDevice for #ident {
            fn read(&self, blocking: bool, data: &mut [u8]) -> Result<usize, libsys::error::Errno> {
                assert!(blocking);
                crate::dev::tty::TtyDevice::line_read(self, data)
            }
            fn write(&self, blocking: bool, data: &[u8]) -> Result<usize, libsys::error::Errno> {
                assert!(blocking);
                crate::dev::tty::TtyDevice::line_write(self, data)
            }
            fn ioctl(&self, cmd: libsys::ioctl::IoctlCmd, ptr: usize, len: usize) ->
                Result<usize, libsys::error::Errno>
            {
                crate::dev::tty::TtyDevice::tty_ioctl(self, cmd, ptr, len)
            }
            fn is_ready(&self, write: bool) -> Result<bool, libsys::error::Errno> {
                crate::dev::tty::TtyDevice::is_ready(self, write)
            }
        }
    }
    .into()
}
