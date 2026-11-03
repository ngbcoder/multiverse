//! UEFI Block I/O protocol abstraction for the virtual block device (M6).

use multiverse_core::CoreError;
use crate::virtual_block::{VirtualBlockDevice, BlockIoMedia};

/// A UEFI-style Block I/O protocol instance wrapping a VirtualBlockDevice.
#[derive(Debug)]
pub struct BlockIoProtocol {
    pub media: BlockIoMedia,
    device: VirtualBlockDevice,
}

impl BlockIoProtocol {
    pub fn new(device: VirtualBlockDevice) -> Self {
        let media = device.media();
        Self { media, device }
    }

    pub fn from_device(device: VirtualBlockDevice) -> Self {
        Self::new(device)
    }

    pub fn from(device: VirtualBlockDevice) -> Self {
        Self::new(device)
    }

    pub fn device(&self) -> &VirtualBlockDevice {
        &self.device
    }

    pub fn device_mut(&mut self) -> &mut VirtualBlockDevice {
        &mut self.device
    }

    pub fn into_device(self) -> VirtualBlockDevice {
        self.device
    }

    pub fn virtual_device(&self) -> &VirtualBlockDevice {
        &self.device
    }

    pub fn reset(&mut self, _extended_verify: bool) -> Result<(), CoreError> {
        self.device.reset(false)
    }

    /// UEFI-style ReadBlocks: validates media id, reads `buf.len()/block_size` blocks.
    pub fn read_blocks(&self, media_id: u32, lba: u64, buf: &mut [u8]) -> Result<(), CoreError> {
        if media_id != self.media.media_id {
            return Err(CoreError::InvalidArg("media id mismatch".into()));
        }
        let bs = self.media.block_size as u64;
        let count = if bs > 0 { (buf.len() as u64) / bs } else { 0 };
        if count == 0 {
            return Ok(());
        }
        self.device.read_blocks(lba, count, buf)
    }

    /// Convenience read by LBA/count.
    pub fn read_blocks_range(&self, lba: u64, count: u64, buf: &mut [u8]) -> Result<(), CoreError> {
        self.device.read_blocks(lba, count, buf)
    }

    pub fn write_blocks(&self, media_id: u32, _lba: u64, _buf: &[u8]) -> Result<(), CoreError> {
        if media_id != self.media.media_id {
            return Err(CoreError::InvalidArg("media id mismatch".into()));
        }
        if self.media.read_only {
            return Err(CoreError::Unsupported("virtual block device is read-only".into()));
        }
        Err(CoreError::Unsupported("writes not supported".into()))
    }

    pub fn flush_blocks(&self, media_id: u32) -> Result<(), CoreError> {
        if media_id != self.media.media_id {
            return Err(CoreError::InvalidArg("media id mismatch".into()));
        }
        self.device.flush_blocks()
    }

    pub fn last_block(&self) -> u64 {
        self.media.last_block
    }

    pub fn block_size(&self) -> u32 {
        self.media.block_size
    }

    pub fn is_read_only(&self) -> bool {
        self.media.read_only
    }
}

// Aliases
pub type BlockIo = BlockIoProtocol;
pub type BlockIO = BlockIoProtocol;
pub type UefiBlockIo = BlockIoProtocol;
pub type EfiBlockIo = BlockIoProtocol;
pub type BlockIoDevice = BlockIoProtocol;
pub type VirtualBlockIo = BlockIoProtocol;
pub type BlockIoProtocolType = BlockIoProtocol;
pub type BlockIOProtocol = BlockIoProtocol;
pub type EfiBlockIoProtocol = BlockIoProtocol;
pub type UefiBlockIoProtocol = BlockIoProtocol;

/// Install a block I/O protocol (host-side: returns the wrapped protocol).
pub fn install_block_io_protocol(device: VirtualBlockDevice) -> BlockIoProtocol {
    BlockIoProtocol::new(device)
}

pub fn install_block_io(device: VirtualBlockDevice) -> BlockIoProtocol {
    BlockIoProtocol::new(device)
}

pub fn create_block_io(device: VirtualBlockDevice) -> BlockIoProtocol {
    BlockIoProtocol::new(device)
}