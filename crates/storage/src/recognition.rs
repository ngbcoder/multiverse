use crate::constants::{BOOT_PARTITION_SECTORS, DATA_START_SECTOR};
use crate::error::StorageError;
use crate::partition::PartitionTable;

#[derive(Debug, Clone)]
pub struct InstalledDiskInfo {
    pub data_start_sector: u64,
    pub boot_sector_count: u64,
    pub is_installed: bool,
}

/// A disk is recognized as an installed device when:
/// (a) partition 1 starts at sector 2048, and
/// (b) partition 2 has exactly the fixed sector count (65536).
pub fn is_installed_device(table: &PartitionTable) -> bool {
    let data = match table
        .partitions
        .iter()
        .find(|p| p.start_sector == DATA_START_SECTOR)
    {
        Some(p) => p,
        None => return false,
    };
    // data partition should be first
    if table.partitions.first().map(|p| p.start_sector) != Some(DATA_START_SECTOR) {
        return false;
    }
    let boot = match table
        .partitions
        .iter()
        .find(|p| p.kind == crate::partition::PartitionType::Esp)
    {
        Some(p) => p,
        None => {
            // fallback: second partition
            if table.partitions.len() < 2 {
                return false;
            }
            &table.partitions[1]
        }
    };
    let _ = data;
    boot.sector_count == BOOT_PARTITION_SECTORS
}

pub fn inspect(table: &PartitionTable) -> InstalledDiskInfo {
    let data_start = table
        .partitions
        .first()
        .map(|p| p.start_sector)
        .unwrap_or(0);
    let boot_count = table.partitions.get(1).map(|p| p.sector_count).unwrap_or(0);
    InstalledDiskInfo {
        data_start_sector: data_start,
        boot_sector_count: boot_count,
        is_installed: is_installed_device(table),
    }
}

/// Helper to validate a candidate table for upgrade (non-destructive).
pub fn validate_for_upgrade(table: &PartitionTable) -> Result<(), StorageError> {
    if !is_installed_device(table) {
        return Err(StorageError::InvalidPartitionTable(
            "not an installed device".into(),
        ));
    }
    Ok(())
}
