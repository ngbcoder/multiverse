//! Storage / on-disk layer: partition tables, recognition, filesystem traits.

pub mod constants;
pub mod error;
pub mod fat;
pub mod filesystem;
pub mod gpt;
pub mod mbr;
pub mod partition;
pub mod recognition;

pub use error::StorageError;
pub use fat::FatVolume;
pub use filesystem::{FatBpb, FileSystem, InMemoryFs};
pub use partition::{Partition, PartitionTable, PartitionType};
pub use recognition::{is_installed_device, InstalledDiskInfo};
