use crate::constants::{BOOT_PARTITION_SECTORS, DATA_START_SECTOR};
use crate::error::StorageError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionScheme {
    Mbr,
    Gpt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionType {
    Data,
    Esp,
    Other(u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub index: u32,
    pub start_sector: u64,
    pub sector_count: u64,
    pub kind: PartitionType,
    pub bootable: bool,
}

impl Partition {
    pub fn end_sector(&self) -> u64 {
        self.start_sector + self.sector_count
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionTable {
    pub scheme: PartitionScheme,
    pub disk_size_sectors: u64,
    pub partitions: Vec<Partition>,
}

impl PartitionTable {
    /// Build the canonical two-partition layout: data partition from sector 2048
    /// to end-reserved, boot/ESP partition as fixed-size tail.
    pub fn canonical(disk_size_sectors: u64) -> Result<Self, StorageError> {
        if disk_size_sectors <= DATA_START_SECTOR + BOOT_PARTITION_SECTORS {
            return Err(StorageError::InvalidArg("disk too small".into()));
        }
        let data_count = disk_size_sectors - DATA_START_SECTOR - BOOT_PARTITION_SECTORS;
        let data = Partition {
            index: 1,
            start_sector: DATA_START_SECTOR,
            sector_count: data_count,
            kind: PartitionType::Data,
            bootable: true,
        };
        let boot = Partition {
            index: 2,
            start_sector: DATA_START_SECTOR + data_count,
            sector_count: BOOT_PARTITION_SECTORS,
            kind: PartitionType::Esp,
            bootable: false,
        };
        Ok(Self {
            scheme: PartitionScheme::Mbr,
            disk_size_sectors,
            partitions: vec![data, boot],
        })
    }

    pub fn data_partition(&self) -> Option<&Partition> {
        self.partitions
            .iter()
            .find(|p| p.kind == PartitionType::Data)
    }

    pub fn boot_partition(&self) -> Option<&Partition> {
        self.partitions
            .iter()
            .find(|p| p.kind == PartitionType::Esp)
    }
}
