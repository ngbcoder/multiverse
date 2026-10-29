//! Chunk map model and image LBA <-> disk LBA translation (M5).

use crate::{CoreError, DiskExtent, ImgChunk, OverrideChunk, VirtChunk, ChainHead, RamdiskParam};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadSource {
    Disk,
    Override, // in-memory override data
    Virtual,  // synthesized
    Ramdisk,  // memdisk RAM
}

#[derive(Debug, Clone)]
pub struct ReadOp {
    pub source: ReadSource,
    /// For Disk: disk LBA (in disk sectors).
    pub disk_lba: u64,
    /// Number of disk sectors to read from this source.
    pub disk_sector_count: u64,
    /// For Override/Virtual: byte offset within the chunk's data (0-based) if applicable.
    pub data_offset_bytes: u64,
    /// For Override: in-memory data.
    pub override_data: Option<Vec<u8>>,
    /// Image sector index where this op begins (in image sectors).
    pub image_lba: u64,
    /// Number of image sectors covered.
    pub image_sector_count: u64,
}

#[derive(Debug, Clone)]
pub struct ChunkMap {
    pub chain_head: ChainHead,
    pub img_chunks: Vec<ImgChunk>,
    pub override_chunks: Vec<OverrideChunk>,
    pub virt_chunks: Vec<VirtChunk>,
    pub ramdisk: Option<RamdiskParam>,
    pub image_sector_size: u32,
    pub disk_sector_size: u32,
}

impl ChunkMap {
    pub fn new(
        chain_head: ChainHead,
        img_chunks: Vec<ImgChunk>,
        overrides: Vec<OverrideChunk>,
        virts: Vec<VirtChunk>,
        ramdisk: Option<RamdiskParam>,
        image_sector_size: u32,
        disk_sector_size: u32,
    ) -> Self {
        Self {
            chain_head,
            img_chunks,
            override_chunks: overrides,
            virt_chunks: virts,
            ramdisk,
            image_sector_size,
            disk_sector_size,
        }
    }

    /// Total image size in bytes.
    pub fn image_size_bytes(&self) -> u64 {
        self.chain_head.image_size_bytes
    }

    /// Total image sectors.
    pub fn image_sector_count(&self) -> u64 {
        (self.image_size_bytes() + self.image_sector_size as u64 - 1) / self.image_sector_size as u64
    }

    /// Find the img_chunk covering an image byte offset.
    fn chunk_for_byte(&self, byte_off: u64) -> Option<&ImgChunk> {
        // Need byte ranges; compute from img_chunks image_lba (image sectors).
        // We'll store derived: each chunk covers image sectors [image_lba, image_lba+span).
        // span in image sectors = extent.sector_count * disk_sector_size / image_sector_size
        for c in &self.img_chunks {
            let span_bytes = c.extent.sector_count * self.disk_sector_size as u64;
            let start_byte = c.image_lba * self.image_sector_size as u64;
            let end_byte = start_byte + span_bytes;
            if byte_off >= start_byte && byte_off < end_byte {
                return Some(c);
            }
        }
        None
    }

    fn override_for_image_lba(&self, image_lba: u64) -> Option<&OverrideChunk> {
        for o in &self.override_chunks {
            if image_lba >= o.image_lba && image_lba < o.image_lba + o.sector_count {
                return Some(o);
            }
        }
        None
    }

    fn virt_for_image_lba(&self, image_lba: u64) -> Option<&VirtChunk> {
        for v in &self.virt_chunks {
            if image_lba >= v.image_lba && image_lba < v.image_lba + v.sector_count {
                return Some(v);
            }
        }
        None
    }

    /// Translate a contiguous range of image sectors into read operations.
    /// image_lba: starting image sector (image sector size).
    /// count: number of image sectors.
    pub fn translate(&self, image_lba: u64, count: u64) -> Result<Vec<ReadOp>, CoreError> {
        let mut ops = Vec::new();
        let mut cur = image_lba;
        let end = image_lba + count;
        // work in image sectors, but split at chunk/override/virt boundaries in bytes.
        // We'll iterate image sector by image sector? Could be large. Instead iterate by
        // determining next boundary in image sectors.
        while cur < end {
            let remaining = end - cur;
            // check override/virt at this image LBA (sector granularity)
            if let Some(o) = self.override_for_image_lba(cur) {
                let avail = (o.image_lba + o.sector_count) - cur;
                let n = avail.min(remaining);
                // override data offset in bytes: (cur - o.image_lba) * image_sector_size
                let data_off = (cur - o.image_lba) * self.image_sector_size as u64;
                ops.push(ReadOp {
                    source: ReadSource::Override,
                    disk_lba: 0,
                    disk_sector_count: 0,
                    data_offset_bytes: data_off,
                    override_data: Some(o.data.clone()),
                    image_lba: cur,
                    image_sector_count: n,
                });
                cur += n;
                continue;
            }
            if let Some(v) = self.virt_for_image_lba(cur) {
                let avail = (v.image_lba + v.sector_count) - cur;
                let n = avail.min(remaining);
                ops.push(ReadOp {
                    source: ReadSource::Virtual,
                    disk_lba: 0,
                    disk_sector_count: 0,
                    data_offset_bytes: 0,
                    override_data: None,
                    image_lba: cur,
                    image_sector_count: n,
                });
                cur += n;
                continue;
            }
            // disk chunk
            let byte_off = cur * self.image_sector_size as u64;
            let chunk = self.chunk_for_byte(byte_off).ok_or_else(|| CoreError::NotFound("no chunk for image LBA".into()))?;
            // compute position within chunk
            let chunk_start_byte = chunk.image_lba * self.image_sector_size as u64;
            let pos_in_chunk = byte_off - chunk_start_byte;
            let chunk_bytes = chunk.extent.sector_count * self.disk_sector_size as u64;
            let bytes_left_in_chunk = chunk_bytes - pos_in_chunk;
            // convert to image sectors available
            let img_sectors_avail = bytes_left_in_chunk / self.image_sector_size as u64;
            let mut max_n = img_sectors_avail.min(remaining);
            for o in &self.override_chunks {
                if o.image_lba > cur && o.image_lba < cur + max_n {
                    max_n = o.image_lba - cur;
                }
            }
            for v in &self.virt_chunks {
                if v.image_lba > cur && v.image_lba < cur + max_n {
                    max_n = v.image_lba - cur;
                }
            }
            if max_n == 0 { max_n = 1; }
            let n = max_n;
            // disk LBA
            let disk_byte = chunk.extent.start_lba * self.disk_sector_size as u64 + pos_in_chunk;
            let disk_lba = disk_byte / self.disk_sector_size as u64;
            // number of disk sectors covered by n image sectors
            let bytes_n = n * self.image_sector_size as u64;
            let disk_sector_count = (bytes_n + self.disk_sector_size as u64 - 1) / self.disk_sector_size as u64;
            ops.push(ReadOp {
                source: ReadSource::Disk,
                disk_lba,
                disk_sector_count,
                data_offset_bytes: pos_in_chunk % self.disk_sector_size as u64,
                override_data: None,
                image_lba: cur,
                image_sector_count: n,
            });
            cur += n;
        }
        Ok(ops)
    }

    /// Reject writes (read-only media semantics).
    pub fn write(&self, _image_lba: u64, _count: u64) -> Result<(), CoreError> {
        Err(CoreError::Unsupported("virtual device is read-only".into()))
    }
}

/// Builder helper: construct img_chunks from a list of (image_byte_offset, disk_extent).
pub struct ChunkMapBuilder {
    pub chain_head: ChainHead,
    pub extents: Vec<(u64, DiskExtent)>, // image byte offset -> disk extent
    pub overrides: Vec<OverrideChunk>,
    pub virts: Vec<VirtChunk>,
    pub ramdisk: Option<RamdiskParam>,
    pub image_sector_size: u32,
    pub disk_sector_size: u32,
}

impl ChunkMapBuilder {
    pub fn build(self) -> ChunkMap {
        let mut img_chunks = Vec::new();
        for (img_off, ext) in self.extents {
            let image_lba = img_off / self.image_sector_size as u64;
            img_chunks.push(ImgChunk { image_lba, extent: ext });
        }
        // sort by image_lba
        img_chunks.sort_by_key(|c| c.image_lba);
        ChunkMap::new(
            self.chain_head,
            img_chunks,
            self.overrides,
            self.virts,
            self.ramdisk,
            self.image_sector_size,
            self.disk_sector_size,
        )
    }
}

impl fmt::Display for ReadSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}