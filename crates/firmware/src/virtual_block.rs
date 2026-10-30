use multiverse_core::CoreError;
use multiverse_core::chunkmap::{ChunkMap, ReadSource};
use multiverse_core::constants as core_constants;

pub type DiskReaderFn = Box<dyn Fn(u64, u64, &mut [u8]) -> Result<(), CoreError> + Send + Sync>;

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
        self.disk_reader = Some(Box::new(reader));
        self
    }

    /// Read `count` blocks starting at `lba` (in disk sector units) into `buf`.
    /// Translates via chunk map (image sectors -> disk). For simplicity, callers
    /// should pass image LBA ranges already translated? Here we accept image LBA.
    pub fn read_blocks(&self, image_lba: u64, count: u64, buf: &mut [u8]) -> Result<(), CoreError> {
        let expected = count * self.block_size as u64;
        if buf.len() < expected as usize {
            return Err(CoreError::InvalidArg("buffer too small".into()));
        }
        let ops = self.chunk_map.translate(image_lba, count)?;
        let mut pos = 0usize;
        for op in ops {
            match op.source {
                ReadSource::Disk => {
                    let bytes = op.image_sector_count * self.chunk_map.image_sector_size as u64;
                    if let Some(reader) = &self.disk_reader {
                        reader(op.disk_lba, op.image_sector_count, &mut buf[pos..pos + bytes as usize])?;
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
}