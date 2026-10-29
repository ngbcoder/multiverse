//! Open boot partition filesystem from firmware abstraction.

use multiverse_storage::{FatVolume, FileSystem, StorageError};

pub struct BootPartitionFs<'a> {
    volume: FatVolume<'a>,
}

impl<'a> BootPartitionFs<'a> {
    pub fn new(partition_data: &'a mut [u8]) -> Result<Self, StorageError> {
        let volume = FatVolume::new(partition_data)?;
        Ok(Self { volume })
    }

    pub fn volume(&self) -> &FatVolume<'a> {
        &self.volume
    }

    pub fn volume_mut(&mut self) -> &mut FatVolume<'a> {
        &mut self.volume
    }
}
