use crate::constants::{PARTITION_TYPE_MBR_DATA, PARTITION_TYPE_MBR_ESP, PLATFORM_SIGNATURE};
use crate::error::StorageError;
use crate::partition::{Partition, PartitionScheme, PartitionTable, PartitionType};

const MBR_SIZE: usize = 512;

#[derive(Debug, Clone)]
pub struct MbrPartitionEntry {
    pub status: u8, // 0x80 bootable
    pub ptype: u8,
    pub start_lba: u32,
    pub sector_count: u32,
    pub start_head: u8,
    pub start_sector: u8,
    pub start_cylinder: u8,
    pub end_head: u8,
    pub end_sector: u8,
    pub end_cylinder: u8,
}

impl MbrPartitionEntry {
    pub fn from_partition(p: &Partition) -> Self {
        let ptype = match p.kind {
            PartitionType::Data => PARTITION_TYPE_MBR_DATA,
            PartitionType::Esp => PARTITION_TYPE_MBR_ESP,
            PartitionType::Other(v) => v,
        };
        Self {
            status: if p.bootable { 0x80 } else { 0x00 },
            ptype,
            start_lba: p.start_sector as u32,
            sector_count: p.sector_count as u32,
            start_head: 0,
            start_sector: 1,
            start_cylinder: 0,
            end_head: 0,
            end_sector: 0,
            end_cylinder: 0,
        }
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[0] = self.status;
        b[1] = self.start_head;
        b[2] = self.start_sector;
        b[3] = self.start_cylinder;
        b[4] = self.ptype;
        b[5] = self.end_head;
        b[6] = self.end_sector;
        b[7] = self.end_cylinder;
        b[8..12].copy_from_slice(&self.start_lba.to_le_bytes());
        b[12..16].copy_from_slice(&self.sector_count.to_le_bytes());
        b
    }
}

pub fn write_mbr(table: &PartitionTable) -> Result<Vec<u8>, StorageError> {
    if table.scheme != PartitionScheme::Mbr {
        return Err(StorageError::InvalidArg("not an MBR table".into()));
    }
    let mut buf = vec![0u8; MBR_SIZE];
    // Write up to 4 partition entries at offset 446.
    for (i, p) in table.partitions.iter().take(4).enumerate() {
        let entry = MbrPartitionEntry::from_partition(p);
        let off = 446 + i * 16;
        buf[off..off + 16].copy_from_slice(&entry.to_bytes());
    }
    // Platform signature at fixed offset (bytes 440..448 used by some; place at 440).
    buf[440..448].copy_from_slice(&PLATFORM_SIGNATURE);
    // Boot signature.
    buf[510] = 0x55;
    buf[511] = 0xAA;
    Ok(buf)
}

pub fn parse_mbr(buf: &[u8]) -> Result<PartitionTable, StorageError> {
    if buf.len() < MBR_SIZE {
        return Err(StorageError::InvalidPartitionTable("short MBR".into()));
    }
    if buf[510] != 0x55 || buf[511] != 0xAA {
        return Err(StorageError::InvalidPartitionTable(
            "bad boot signature".into(),
        ));
    }
    let mut partitions = Vec::new();
    for i in 0..4 {
        let off = 446 + i * 16;
        let ptype = buf[off + 4];
        if ptype == 0 {
            continue;
        }
        let start_lba = u32::from_le_bytes(buf[off + 8..off + 12].try_into().unwrap()) as u64;
        let sector_count = u32::from_le_bytes(buf[off + 12..off + 16].try_into().unwrap()) as u64;
        let kind = match ptype {
            PARTITION_TYPE_MBR_DATA => PartitionType::Data,
            PARTITION_TYPE_MBR_ESP => PartitionType::Esp,
            v => PartitionType::Other(v),
        };
        partitions.push(Partition {
            index: (i + 1) as u32,
            start_sector: start_lba,
            sector_count,
            kind,
            bootable: buf[off] == 0x80,
        });
    }
    Ok(PartitionTable {
        scheme: PartitionScheme::Mbr,
        disk_size_sectors: 0,
        partitions,
    })
}
