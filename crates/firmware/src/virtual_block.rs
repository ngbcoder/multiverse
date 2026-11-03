use multiverse_core::CoreError;
use multiverse_core::chunkmap::{ChunkMap, ReadSource};
use multiverse_core::constants as core_constants;

pub type DiskReaderFn = std::sync::Arc<dyn Fn(u64, u64, &mut [u8]) -> Result<(), CoreError> + Send + Sync>;

/// Media descriptor for a (virtual) block I/O device (UEFI BlockIo media).
#[derive(Debug, Clone)]
pub struct BlockIoMedia {
    pub media_id: u32,
    pub removable_media: bool,
    pub media_present: bool,
    pub logical_partition: bool,
    pub read_only: bool,
    pub block_size: u32,
    pub last_block: u64,
    pub io_align: u32,
}

/// Alias used by some callers.
pub type BlockMedia = BlockIoMedia;
pub type VirtualBlockMedia = BlockIoMedia;
pub type EfiBlockIoMedia = BlockIoMedia;
pub type BlockIoMediaInfo = BlockIoMedia;

#[derive(Clone)]
pub struct VirtualBlockDevice {
    pub chunk_map: ChunkMap,
    pub media_id: u32,
    pub read_only: bool,
    pub removable: bool,
    pub block_size: u32, // disk sector size
    pub disk_reader: Option<DiskReaderFn>,
}

impl std::fmt::Debug for VirtualBlockDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualBlockDevice")
            .field("chunk_map", &self.chunk_map)
            .field("media_id", &self.media_id)
            .field("read_only", &self.read_only)
            .field("removable", &self.removable)
            .field("block_size", &self.block_size)
            .field("disk_reader", &self.disk_reader.is_some())
            .finish()
    }
}

impl VirtualBlockDevice {
    pub fn new(chunk_map: ChunkMap) -> Self {
        let bs = chunk_map.disk_sector_size;
        Self {
            chunk_map,
            media_id: 1,
            read_only: true,
            removable: true,
            block_size: bs,
            disk_reader: None,
        }
    }

    pub fn with_disk_reader<F>(mut self, reader: F) -> Self
    where
        F: Fn(u64, u64, &mut [u8]) -> Result<(), CoreError> + Send + Sync + 'static,
    {
        self.disk_reader = Some(std::sync::Arc::new(reader));
        self
    }

    /// Connect a disk reader after construction.
    pub fn set_disk_reader<F>(&mut self, reader: F)
    where
        F: Fn(u64, u64, &mut [u8]) -> Result<(), CoreError> + Send + Sync + 'static,
    {
        self.disk_reader = Some(std::sync::Arc::new(reader));
    }

    /// Read `count` image sectors starting at `image_lba` into `buf`.
    pub fn read_blocks(&self, image_lba: u64, count: u64, buf: &mut [u8]) -> Result<(), CoreError> {
        let expected = count * self.chunk_map.image_sector_size as u64;
        if buf.len() < expected as usize {
            return Err(CoreError::InvalidArg("buffer too small".into()));
        }
        let ops = self.chunk_map.translate(image_lba, count)?;
        let mut pos = 0usize;
        for op in ops {
            let bytes = op.image_sector_count * self.chunk_map.image_sector_size as u64;
            match op.source {
                ReadSource::Disk => {
                    let disk_sector_size = self.chunk_map.disk_sector_size as usize;
                    let tmp_size = (op.disk_sector_count as usize) * disk_sector_size;
                    if let Some(reader) = &self.disk_reader {
                        let mut tmp = vec![0u8; tmp_size.max(1)];
                        reader(op.disk_lba, op.disk_sector_count, &mut tmp)?;
                        let off = op.data_offset_bytes as usize;
                        let end = off + bytes as usize;
                        if end <= tmp.len() {
                            buf[pos..pos + bytes as usize].copy_from_slice(&tmp[off..end]);
                        } else {
                            // partial copy then zero
                            let avail = tmp.len().saturating_sub(off);
                            if avail > 0 {
                                buf[pos..pos + avail].copy_from_slice(&tmp[off..off + avail]);
                            }
                            for b in &mut buf[pos + avail..pos + bytes as usize] {
                                *b = 0;
                            }
                        }
                    } else {
                        for b in &mut buf[pos..pos + bytes as usize] {
                            *b = 0;
                        }
                    }
                    pos += bytes as usize;
                }
                ReadSource::Override => {
                    let bytes = op.image_sector_count * self.chunk_map.image_sector_size as u64;
                    if let Some(data) = &op.override_data {
                        let start = op.data_offset_bytes as usize;
                        let end = (start + bytes as usize).min(data.len());
                        let n = end - start;
                        buf[pos..pos + n].copy_from_slice(&data[start..end]);
                        if n < bytes as usize {
                            for b in &mut buf[pos + n..pos + bytes as usize] {
                                *b = 0;
                            }
                        }
                    } else {
                        for b in &mut buf[pos..pos + bytes as usize] {
                            *b = 0;
                        }
                    }
                    pos += bytes as usize;
                }
                ReadSource::Virtual => {
                    let bytes = op.image_sector_count * self.chunk_map.image_sector_size as u64;
                    for b in &mut buf[pos..pos + bytes as usize] {
                        *b = 0;
                    }
                    pos += bytes as usize;
                }
                ReadSource::Ramdisk => {
                    let bytes = op.image_sector_count * self.chunk_map.image_sector_size as u64;
                    for b in &mut buf[pos..pos + bytes as usize] {
                        *b = 0;
                    }
                    pos += bytes as usize;
                }
            }
        }
        // zero any trailing bytes in the requested range
        if pos < expected as usize {
            for b in &mut buf[pos..expected as usize] {
                *b = 0;
            }
        }
        let _ = core_constants::SECTOR_SIZE_512;
        Ok(())
    }

    pub fn write_blocks(&self, _image_lba: u64, _count: u64, _buf: &[u8]) -> Result<(), CoreError> {
        if self.read_only {
            return Err(CoreError::Unsupported("virtual block device is read-only".into()));
        }
        Err(CoreError::Unsupported("writes not supported".into()))
    }

    pub fn flush(&self) -> Result<(), CoreError> {
        Ok(())
    }

    pub fn flush_blocks(&self) -> Result<(), CoreError> {
        Ok(())
    }

    pub fn reset(&self, _extended: bool) -> Result<(), CoreError> {
        Ok(())
    }

    pub fn block_count(&self) -> u64 {
        self.chunk_map.image_sector_count()
    }

    pub fn last_block(&self) -> u64 {
        self.block_count().saturating_sub(1)
    }

    pub fn media(&self) -> BlockIoMedia {
        BlockIoMedia {
            media_id: self.media_id,
            removable_media: self.removable,
            media_present: true,
            logical_partition: false,
            read_only: self.read_only,
            block_size: self.chunk_map.image_sector_size,
            last_block: self.last_block(),
            io_align: 0,
        }
    }

    // Alias for media()
    pub fn media_info(&self) -> BlockIoMedia {
        self.media()
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }
}