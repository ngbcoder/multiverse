//! Shared core types for the multiverse platform.

use std::fmt;
use thiserror::Error;

/// Supported firmware/platform identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PlatformId {
    LegacyBios,
    Ia32Uefi,
    X86_64Uefi,
    AArch64Uefi,
    Mips64ElUefi,
}

impl PlatformId {
    pub fn is_uefi(self) -> bool {
        !matches!(self, PlatformId::LegacyBios)
    }
}

pub mod chunkmap;

/// Fixed on-disk constants (in sectors of 512 bytes unless noted).
pub mod constants {
    /// Data partition always starts at this sector.
    pub const DATA_START_SECTOR: u64 = 2048;
    /// Boot/ESP partition fixed size in sectors.
    pub const BOOT_PARTITION_SECTORS: u64 = 65536;
    /// Boot partition size in bytes (32 MiB).
    pub const BOOT_PARTITION_BYTES: u64 = BOOT_PARTITION_SECTORS * 512;
    /// Sector sizes supported for virtual block I/O.
    pub const SECTOR_SIZE_512: u32 = 512;
    pub const SECTOR_SIZE_2048: u32 = 2048;
    pub const SECTOR_SIZE_4096: u32 = 4096;
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid argument: {0}")]
    InvalidArg(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// A handle to a physical disk (opaque; firmware/OS specific).
#[derive(Debug, Clone)]
pub struct DiskHandle {
    pub index: u32,
    pub size_bytes: u64,
    pub signature: Option<u32>,
    pub guid: Option<uuid::Uuid>,
}

/// Filesystem kind for the data partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DataFilesystem {
    Fat32,
    ExFat,
    Ntfs,
    Udf,
    Xfs,
    Btrfs,
    Ext2,
    Ext3,
    Ext4,
}

impl DataFilesystem {
    pub fn is_supported(self) -> bool {
        true
    }
}

/// Chunk map primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiskExtent {
    /// Starting LBA on the physical disk.
    pub start_lba: u64,
    /// Number of sectors.
    pub sector_count: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImgChunk {
    /// Image LBA offset (in image sectors).
    pub image_lba: u64,
    /// Disk extent backing this run.
    pub extent: DiskExtent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OverrideChunk {
    pub image_lba: u64,
    pub sector_count: u64,
    /// Data held in memory.
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VirtChunk {
    pub image_lba: u64,
    pub sector_count: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainHead {
    pub partition_index: u32,
    pub start_sector: u64,
    pub image_size_bytes: u64,
    pub chunk_count: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RamdiskParam {
    pub location: u64,
    pub size_bytes: u64,
}

/// Environment parameter block persisted across firmware/guest boundary.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EnvParamBlock {
    pub image_location: Option<u64>,
    pub chunk_count: u32,
    pub platform_id: Option<PlatformId>,
    pub ramdisk: Option<RamdiskParam>,
    pub flags: u32,
}

impl fmt::Display for PlatformId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
