//! Device tree facilities
use crate::debug::Level;
use fdt_rs::prelude::*;
use fdt_rs::{
    base::DevTree,
    index::{
        iters::DevTreeIndexCompatibleNodeIter, DevTreeIndex, DevTreeIndexNode, DevTreeIndexProp,
    },
};
use libsys::{error::Errno, path::path_component_left};

#[repr(align(16))]
struct Wrap {
    data: [u8; 65536],
}

static mut INDEX_BUFFER: Wrap = Wrap { data: [0; 65536] };

type INode<'a> = DevTreeIndexNode<'a, 'a, 'a>;
type IProp<'a> = DevTreeIndexProp<'a, 'a, 'a>;

/// Device tree manager structure
#[allow(dead_code)]
pub struct DeviceTree {
    tree: DevTree<'static>,
    index: DevTreeIndex<'static, 'static>,
}

fn tab(level: Level, depth: usize) {
    for _ in 0..depth {
        print!(level, "\t");
    }
}

fn dump_node(level: Level, node: &INode, depth: usize) {
    if node.name().unwrap().starts_with("virtio_mmio@") {
        return;
    }

    tab(level, depth);
    println!(level, "{:?} {{", node.name().unwrap());

    for prop in node.props() {
        tab(level, depth + 1);
        let name = prop.name().unwrap();
        print!(level, "{:?} = ", name);

        match name {
            "compatible" | "enable-method" => print!(level, "{:?}", prop.str().unwrap()),
            "#address-cells" | "#size-cells" => print!(level, "{}", prop.u32(0).unwrap()),
            "reg" => {
                print!(level, "<");
                let len = prop.length() / 4;
                for i in 0..len {
                    print!(level, "{:#010x}", prop.u32(i).unwrap());
                    if i < len - 1 {
                        print!(level, ", ");
                    }
                }
                print!(level, ">");
            }
            _ => print!(level, "..."),
        }
        println!(level, ";");
    }

    if node.children().next().is_some() {
        println!(level, "");
    }

    for child in node.children() {
        dump_node(level, &child, depth + 1);
    }

    tab(level, depth);
    println!(level, "}}");
}

fn find_node<'a>(at: INode<'a>, path: &str) -> Option<INode<'a>> {
    let (item, path) = path_component_left(path);
    if item.is_empty() {
        assert_eq!(path, "");
        Some(at)
    } else {
        let child = at.children().find(|c| c.name().unwrap() == item)?;
        if path.is_empty() {
            Some(child)
        } else {
            find_node(child, path)
        }
    }
}

/// Looks up a node's property by its name
pub fn find_prop<'a>(at: INode<'a>, name: &str) -> Option<IProp<'a>> {
    at.props().find(|p| p.name().unwrap() == name)
}

// fn read_cells(prop: &IProp, off: usize, cells: u32) -> Option<u64> {
//     Some(match cells {
//         1 => prop.u32(off).ok()? as u64,
//         2 => (prop.u32(off).ok()? as u64) | ((prop.u32(off + 1).ok()? as u64) << 32),
//         _ => todo!(),
//     })
// }

impl DeviceTree {
    /// Dumps contents of the device tree
    pub fn dump(&self, level: Level) {
        dump_node(level, &self.index.root(), 0);
    }

    /// Looks up given `path` in the tree
    pub fn node_by_path(&self, path: &str) -> Option<INode> {
        find_node(self.index.root(), path.trim_start_matches('/'))
    }

    /// Loads a device tree from physical `base` address and
    /// creates an index for it
    pub fn compatible<'a, 's>(&'a self, compat: &'s str) -> DevTreeIndexCompatibleNodeIter<'s, 'a, 'a, 'a> {
        self.index.compatible_nodes(compat)
    }

    pub fn initrd(&self) -> Option<(usize, usize)> {
        let chosen = self.node_by_path("/chosen")?;
        let initrd_start = find_prop(chosen.clone(), "linux,initrd-start")?
            .u32(0)
            .ok()?;
        let initrd_end = find_prop(chosen, "linux,initrd-end")?.u32(0).ok()?;

        Some((initrd_start as usize, initrd_end as usize))
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
