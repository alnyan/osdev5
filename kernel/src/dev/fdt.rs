use error::Errno;
use fdt_rs::prelude::*;
use fdt_rs::{
    base::DevTree,
    index::{DevTreeIndex, DevTreeIndexNode},
};

#[repr(align(16))]
struct Wrap {
    data: [u8; 65536],
}

static mut INDEX_BUFFER: Wrap = Wrap { data: [0; 65536] };

type INode<'a> = DevTreeIndexNode<'a, 'a, 'a>;

#[allow(dead_code)]
pub struct DeviceTree {
    tree: DevTree<'static>,
    index: DevTreeIndex<'static, 'static>,
}

fn tab(depth: usize) {
    for _ in 0..depth {
        debug!("\t");
    }
}

fn dump_node(node: &INode, depth: usize) {
    if node.name().unwrap().starts_with("virtio_mmio@") {
        return;
    }

    tab(depth);
    debugln!("{:?} {{", node.name().unwrap());

    for prop in node.props() {
        tab(depth + 1);
        let name = prop.name().unwrap();
        debug!("{:?} = ", name);

        match name {
            "compatible" => debug!("{:?}", prop.str().unwrap()),
            "#address-cells" | "#size-cells" => debug!("{}", prop.u32(0).unwrap()),
            "reg" => {
                debug!("<");
                let len = prop.length() / 4;
                for i in 0..len {
                    debug!("{:#010x}", prop.u32(i).unwrap());
                    if i < len - 1 {
                        debug!(", ");
                    }
                }
                debug!(">");
            }
            _ => debug!("..."),
        }
        debugln!(";");
    }

    if node.children().next().is_some() {
        debugln!("");
    }

    for child in node.children() {
        dump_node(&child, depth + 1);
    }

    tab(depth);
    debugln!("}}");
}

impl DeviceTree {
    pub fn dump(&self) {
        dump_node(&self.index.root(), 0);
    }

    pub fn from_phys(base: usize) -> Result<DeviceTree, Errno> {
        // TODO virtualize address
        let tree = unsafe { DevTree::from_raw_pointer(base as *const _) }
            .map_err(|_| Errno::InvalidArgument)?;
        let layout = DevTreeIndex::get_layout(&tree).unwrap();
        if layout.size() + layout.align() >= unsafe { INDEX_BUFFER.data.len() } {
            return Err(Errno::OutOfMemory);
        }
        let index = DevTreeIndex::new(tree, unsafe {
            &mut INDEX_BUFFER.data[0..layout.size() + layout.align()]
        })
        .unwrap();

        Ok(DeviceTree { tree, index })
    }
}
