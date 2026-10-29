use crate::constants::{GPT_TYPE_BASIC_DATA, GPT_TYPE_ESP};
use crate::error::StorageError;
use crate::partition::{Partition, PartitionScheme, PartitionTable, PartitionType};
use uuid::Uuid;

const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";

#[derive(Debug, Clone)]
pub struct GptHeader {
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
    pub current_lba: u64,
    pub backup_lba: u64,
    pub first_usable_lba: u64,
    pub last_usable_lba: u64,
    pub disk_guid: Uuid,
    pub partition_entries_lba: u64,
    pub num_partition_entries: u32,
    pub partition_entry_size: u32,
    pub partition_entries_crc32: u32,
}

impl GptHeader {
    pub fn to_bytes(&self) -> [u8; 92] {
        let mut b = [0u8; 92];
        b[0..8].copy_from_slice(GPT_SIGNATURE);
        b[8..12].copy_from_slice(&self.revision.to_le_bytes());
        b[12..16].copy_from_slice(&self.header_size.to_le_bytes());
        b[16..20].copy_from_slice(&self.crc32.to_le_bytes());
        b[20..24].copy_from_slice(&self.reserved.to_le_bytes());
        b[24..32].copy_from_slice(&self.current_lba.to_le_bytes());
        b[32..40].copy_from_slice(&self.backup_lba.to_le_bytes());
        b[40..48].copy_from_slice(&self.first_usable_lba.to_le_bytes());
        b[48..56].copy_from_slice(&self.last_usable_lba.to_le_bytes());
        b[56..72].copy_from_slice(self.disk_guid.as_bytes());
        b[72..80].copy_from_slice(&self.partition_entries_lba.to_le_bytes());
        b[80..84].copy_from_slice(&self.num_partition_entries.to_le_bytes());
        b[84..88].copy_from_slice(&self.partition_entry_size.to_le_bytes());
        b[88..92].copy_from_slice(&self.partition_entries_crc32.to_le_bytes());
        b
    }
}

#[derive(Debug, Clone)]
pub struct GptPartitionEntry {
    pub type_guid: [u8; 16],
    pub unique_guid: Uuid,
    pub start_lba: u64,
    pub end_lba: u64,
    pub attributes: u64,
    pub name: [u16; 36],
}

impl GptPartitionEntry {
    pub fn from_partition(p: &Partition) -> Self {
        let type_guid = match p.kind {
            PartitionType::Esp => GPT_TYPE_ESP,
            PartitionType::Data => GPT_TYPE_BASIC_DATA,
            _ => [0u8; 16],
        };
        let mut name = [0u16; 36];
        let label: &[u16] = match p.kind {
            PartitionType::Data => &[0x44, 0x61, 0x74, 0x61], // "Data"
            PartitionType::Esp => &[0x42, 0x6F, 0x6F, 0x74],  // "Boot"
            _ => &[],
        };
        for (i, c) in label.iter().enumerate() {
            name[i] = *c;
        }
        Self {
            type_guid,
            unique_guid: Uuid::new_v4(),
            start_lba: p.start_sector,
            end_lba: p.start_sector + p.sector_count - 1,
            attributes: 0,
            name,
        }
    }

    pub fn to_bytes(&self) -> [u8; 128] {
        let mut b = [0u8; 128];
        b[0..16].copy_from_slice(&self.type_guid);
        b[16..32].copy_from_slice(self.unique_guid.as_bytes());
        b[32..40].copy_from_slice(&self.start_lba.to_le_bytes());
        b[40..48].copy_from_slice(&self.end_lba.to_le_bytes());
        b[48..56].copy_from_slice(&self.attributes.to_le_bytes());
        // name as UTF-16LE
        for (i, c) in self.name.iter().enumerate() {
            let off = 56 + i * 2;
            b[off..off + 2].copy_from_slice(&c.to_le_bytes());
        }
        b
    }
}

/// Write a protective MBR + GPT header + partition entries (minimal, no CRC computation beyond placeholders).
pub fn write_gpt(table: &PartitionTable, disk_size_sectors: u64) -> Result<Vec<u8>, StorageError> {
    if table.scheme != PartitionScheme::Gpt {
        // allow canonical MBR tables to be converted? We'll just require Gpt.
    }
    // We'll generate a full disk image header region of first 34 sectors (17408 bytes) + entries.
    // For simplicity, produce protective MBR (sector 0) + GPT header (sector 1) + entries (sector 2..).
    let mut buf = vec![0u8; 512 * 34];
    // Protective MBR
    buf[510] = 0x55;
    buf[511] = 0xAA;
    // protective partition entry at offset 446: type 0xEE covering whole disk
    buf[446 + 4] = 0xEE;
    buf[446 + 8..446 + 12].copy_from_slice(&1u32.to_le_bytes());
    let size = (disk_size_sectors - 1).min(u32::MAX as u64) as u32;
    buf[446 + 12..446 + 16].copy_from_slice(&size.to_le_bytes());

    let header = GptHeader {
        revision: 0x00010000,
        header_size: 92,
        crc32: 0,
        reserved: 0,
        current_lba: 1,
        backup_lba: disk_size_sectors - 1,
        first_usable_lba: 34,
        last_usable_lba: disk_size_sectors - 34,
        disk_guid: Uuid::new_v4(),
        partition_entries_lba: 2,
        num_partition_entries: 128,
        partition_entry_size: 128,
        partition_entries_crc32: 0,
    };
    buf[512..512 + 92].copy_from_slice(&header.to_bytes());

    // partition entries at sector 2
    for (i, p) in table.partitions.iter().enumerate() {
        let entry = GptPartitionEntry::from_partition(p);
        let off = 512 * 2 + i * 128;
        if off + 128 <= buf.len() {
            buf[off..off + 128].copy_from_slice(&entry.to_bytes());
        }
    }
    Ok(buf)
}

pub fn parse_gpt(buf: &[u8]) -> Result<PartitionTable, StorageError> {
    if buf.len() < 512 * 2 {
        return Err(StorageError::InvalidPartitionTable("short GPT".into()));
    }
    if &buf[0..8] != GPT_SIGNATURE {
        // header at sector 1
        if buf.len() < 512 * 2 || &buf[512..520] != GPT_SIGNATURE {
            return Err(StorageError::InvalidPartitionTable(
                "bad GPT signature".into(),
            ));
        }
    }
    let hdr_off = if &buf[0..8] == GPT_SIGNATURE { 0 } else { 512 };
    let num = u32::from_le_bytes(buf[hdr_off + 80..hdr_off + 84].try_into().unwrap());
    let entry_size =
        u32::from_le_bytes(buf[hdr_off + 84..hdr_off + 88].try_into().unwrap()) as usize;
    let entries_lba = u64::from_le_bytes(buf[hdr_off + 72..hdr_off + 80].try_into().unwrap());
    let entries_off = (entries_lba as usize) * 512;

    let mut partitions = Vec::new();
    for i in 0..num as usize {
        let off = entries_off + i * entry_size;
        if off + entry_size > buf.len() {
            break;
        }
        // type guid all zero => empty
        if buf[off..off + 16].iter().all(|&b| b == 0) {
            continue;
        }
        let start = u64::from_le_bytes(buf[off + 32..off + 40].try_into().unwrap());
        let end = u64::from_le_bytes(buf[off + 40..off + 48].try_into().unwrap());
        let type_guid = &buf[off..off + 16];
        let kind = if type_guid == GPT_TYPE_ESP {
            PartitionType::Esp
        } else if type_guid == GPT_TYPE_BASIC_DATA {
            PartitionType::Data
        } else {
            PartitionType::Other(0)
        };
        partitions.push(Partition {
            index: (i + 1) as u32,
            start_sector: start,
            sector_count: end - start + 1,
            kind,
            bootable: false,
        });
    }
    Ok(PartitionTable {
        scheme: PartitionScheme::Gpt,
        disk_size_sectors: 0,
        partitions,
    })
}
