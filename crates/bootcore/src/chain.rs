//! Chain Data Builder for assembling boot payloads and descriptors.

use multiverse_core::{ChainHead, ImgChunk, OverrideChunk, VirtChunk, RamdiskParam};

#[derive(Debug, Clone, Default)]
pub struct ChainDataBuilder {
    partition_index: u32,
    start_sector: u64,
    image_size_bytes: u64,
    chunks: Vec<ImgChunk>,
    overrides: Vec<OverrideChunk>,
    virtual_chunks: Vec<VirtChunk>,
    ramdisk: Option<RamdiskParam>,
    flags: u32,
}

impl ChainDataBuilder {
    pub fn new(partition_index: u32, start_sector: u64, image_size_bytes: u64) -> Self {
        Self {
            partition_index,
            start_sector,
            image_size_bytes,
            chunks: Vec::new(),
            overrides: Vec::new(),
            virtual_chunks: Vec::new(),
            ramdisk: None,
            flags: 0,
        }
    }

    pub fn with_chunks(mut self, chunks: Vec<ImgChunk>) -> Self {
        self.chunks = chunks;
        self
    }

    pub fn with_overrides(mut self, overrides: Vec<OverrideChunk>) -> Self {
        self.overrides = overrides;
        self
    }

    pub fn with_virtual_chunks(mut self, virtual_chunks: Vec<VirtChunk>) -> Self {
        self.virtual_chunks = virtual_chunks;
        self
    }

    pub fn with_ramdisk(mut self, ramdisk: RamdiskParam) -> Self {
        self.ramdisk = Some(ramdisk);
        self
    }

    pub fn with_flags(mut self, flags: u32) -> Self {
        self.flags = flags;
        self
    }

    pub fn build(self) -> (ChainHead, Vec<ImgChunk>, Vec<OverrideChunk>, Vec<VirtChunk>, Option<RamdiskParam>) {
        let chunk_count = self.chunks.len() as u32;
        let head = ChainHead {
            partition_index: self.partition_index,
            start_sector: self.start_sector,
            image_size_bytes: self.image_size_bytes,
            chunk_count,
            flags: self.flags,
        };
        (head, self.chunks, self.overrides, self.virtual_chunks, self.ramdisk)
    }
}
