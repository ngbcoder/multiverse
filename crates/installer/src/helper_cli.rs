//! Helper CLI tools for GPT header repair, FAT partition manipulation, and partition resizing.

use multiverse_storage::partition::PartitionTable;
use multiverse_storage::gpt;

pub struct HelperCli;

impl HelperCli {
    pub fn repair_gpt_header(disk_bytes: &mut [u8]) -> Result<(), String> {
        if disk_bytes.len() < 512 * 34 {
            return Err("Disk image too small for GPT header".to_string());
        }
        // Attempt GPT parse
        match gpt::parse_gpt(disk_bytes) {
            Ok(table) => {
                let repaired = gpt::write_gpt(&table, (disk_bytes.len() / 512) as u64)
                    .map_err(|e| e.to_string())?;
                disk_bytes[0..repaired.len()].copy_from_slice(&repaired);
                Ok(())
            }
            Err(e) => Err(format!("Failed to parse GPT for repair: {}", e)),
        }
    }

    pub fn validate_partition_table(table: &PartitionTable) -> bool {
        multiverse_storage::is_installed_device(table)
    }
}
