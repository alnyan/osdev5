extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use quote::ToTokens;
use std::collections::HashSet;
use syn::{parse_macro_input, ImplItem, ItemImpl, Ident};

fn impl_inode_fn<T: ToTokens>(name: &str, behavior: T) -> ImplItem {
    // TODO somehow know if current crate is vfs or not?
    ImplItem::Verbatim(match name {
        "create" => quote! {
            fn create(&mut self, _at: VnodeRef, _name: &str, kind: VnodeKind) ->
                Result<VnodeRef, libsys::error::Errno>
            {
                #behavior
            }
        },
        "remove" => quote! {
            fn remove(&mut self, _at: VnodeRef, _name: &str) -> Result<(), libsys::error::Errno> {
                #behavior
            }
        },
        "lookup" => quote! {
            fn lookup(&mut self, _at: VnodeRef, _name: &str) ->
                Result<VnodeRef, libsys::error::Errno>
            {
                #behavior
            }
        },
        "stat" => quote! {
            fn stat(&mut self, _at: VnodeRef, _stat: &mut libsys::stat::Stat) ->
                Result<(), libsys::error::Errno>
            {
                #behavior
            }
        },
        "truncate" => quote! {
            fn truncate(&mut self, _node: VnodeRef, _size: usize) ->
                Result<(), libsys::error::Errno>
            {
                #behavior
            }
        },
        "size" => quote! {
            fn size(&mut self, _node: VnodeRef) -> Result<usize, libsys::error::Errno> {
                #behavior
            }
        },
        "read" => quote! {
            fn read(&mut self, _node: VnodeRef, _pos: usize, _data: &mut [u8]) ->
                Result<usize, libsys::error::Errno>
            {
                #behavior
            }
        },
        "write" => quote! {
            fn write(&mut self, _node: VnodeRef, _pos: usize, _data: &[u8]) ->
                Result<usize, libsys::error::Errno>
            {
                #behavior
            }
        },
        "open" => quote! {
            fn open(&mut self, _node: VnodeRef, _flags: libsys::stat::OpenFlags) ->
                Result<usize, libsys::error::Errno>
            {
                #behavior
            }
        },
        "close" => quote! {
            fn close(&mut self, _node: VnodeRef) -> Result<(), libsys::error::Errno> {
                #behavior
            }
        },
        "ioctl" => quote! {
            fn ioctl(
                &mut self,
                _node: VnodeRef,
                _cmd: libsys::ioctl::IoctlCmd,
                _ptr: usize,
                _len: usize) ->
                Result<usize, libsys::error::Errno>
            {
                #behavior
            }
        },
        _ => panic!("TODO implement {:?}", name),
    })
}

#[proc_macro_attribute]
pub fn auto_inode(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut impl_item = parse_macro_input!(input as ItemImpl);
    let mut missing = HashSet::<String>::new();
    let behavior = if attr.is_empty() {
        "unimplemented".to_string()
    } else {
        parse_macro_input!(attr as Ident).to_string()
    };
    let behavior = match behavior.as_str() {
        "unimplemented" => quote! { unimplemented!() },
        "panic" => quote! { panic!() },
        "error" => quote! { Err(libsys::error::Errno::NotImplemented) },
        _ => panic!("Unknown #[auto_inode] behavior: {:?}", behavior)
    };

    missing.insert("create".to_string());
    missing.insert("remove".to_string());
    missing.insert("lookup".to_string());
    missing.insert("open".to_string());
    missing.insert("close".to_string());
    missing.insert("truncate".to_string());
    missing.insert("read".to_string());
    missing.insert("write".to_string());
    missing.insert("stat".to_string());
    missing.insert("size".to_string());
    missing.insert("ioctl".to_string());

    for item in &impl_item.items {
        match item {
            ImplItem::Method(method) => {
                let name = &method.sig.ident.to_string();
                if missing.contains(name) {
                    missing.remove(name);
                }
            }
            _ => panic!("Unexpected impl item"),
        }
    }

    for item in &missing {
        impl_item
            .items
            .push(impl_inode_fn(item, behavior.clone()));
    }

    impl_item.to_token_stream().into()
}
