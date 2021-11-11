use libsys::mem::{read_le16, read_le32};

#[derive(Debug)]
pub struct Bpb {
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fat_count: u8,
    sectors_per_fat: u32,
}

impl Bpb {
    pub fn from_sector(data: &[u8]) -> Self {
        Self {
            fat_count: data[16],
            reserved_sectors: read_le16(&data[14..]),
            sectors_per_cluster: data[13],
            sectors_per_fat: read_le32(&data[36..]),
        }
    }

    pub const fn cluster_base_sector(&self, cluster: u32) -> u32 {
        let first_data_sector =
            self.reserved_sectors as u32 + (self.fat_count as u32 * self.sectors_per_fat as u32);
        ((cluster - 2) * self.sectors_per_cluster as u32) + first_data_sector
    }

    pub const fn sectors_per_cluster(&self) -> u8 {
        self.sectors_per_cluster
    }
}
