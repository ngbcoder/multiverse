use crate::error::StorageError;
use std::io::{Read, Seek, SeekFrom};

/// Abstract filesystem interface used by boot partition and data partition.
pub trait FileSystem {
    type File: Read + Seek;

    fn open(&self, path: &str) -> Result<Self::File, StorageError>;
    fn file_size(&self, path: &str) -> Result<u64, StorageError> {
        let mut f = self.open(path)?;
        Ok(f.seek(SeekFrom::End(0))?)
    }
    fn exists(&self, path: &str) -> bool;
    fn read_dir(&self, path: &str) -> Result<Vec<String>, StorageError>;
    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), StorageError>;
    fn create_dir(&mut self, path: &str) -> Result<(), StorageError>;
}

/// FAT BIOS Parameter Block (common to FAT12/16/32).
#[derive(Debug, Clone, Copy)]
pub struct FatBpb {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media: u8,
    pub fat_size_16: u16,
    pub total_sectors_32: u32,
    pub fat_size_32: u32,
    pub root_cluster: u32,
}

impl FatBpb {
    pub fn parse(buf: &[u8]) -> Result<Self, StorageError> {
        if buf.len() < 512 {
            return Err(StorageError::InvalidArg("short BPB".into()));
        }
        // FAT signature at offset 510/511
        if buf[510] != 0x55 || buf[511] != 0xAA {
            return Err(StorageError::InvalidArg("bad boot signature".into()));
        }
        let bytes_per_sector = u16::from_le_bytes([buf[11], buf[12]]);
        let sectors_per_cluster = buf[13];
        let reserved_sectors = u16::from_le_bytes([buf[14], buf[15]]);
        let num_fats = buf[16];
        let root_entry_count = u16::from_le_bytes([buf[17], buf[18]]);
        let total_sectors_16 = u16::from_le_bytes([buf[19], buf[20]]);
        let media = buf[21];
        let fat_size_16 = u16::from_le_bytes([buf[22], buf[23]]);
        let total_sectors_32 = u32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]);
        let fat_size_32 = u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
        let root_cluster = u32::from_le_bytes([buf[44], buf[45], buf[46], buf[47]]);
        Ok(Self {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            num_fats,
            root_entry_count,
            total_sectors_16,
            media,
            fat_size_16,
            total_sectors_32,
            fat_size_32,
            root_cluster,
        })
    }

    pub fn fat_size(&self) -> u32 {
        if self.fat_size_32 != 0 {
            self.fat_size_32
        } else {
            self.fat_size_16 as u32
        }
    }

    pub fn total_sectors(&self) -> u32 {
        if self.total_sectors_32 != 0 {
            self.total_sectors_32
        } else {
            self.total_sectors_16 as u32
        }
    }

    pub fn is_fat32(&self) -> bool {
        self.fat_size_32 != 0
    }
}

/// Simple in-memory filesystem for testing and configurator staging.
#[derive(Debug, Default)]
pub struct InMemoryFs {
    files: std::collections::BTreeMap<String, Vec<u8>>,
    dirs: std::collections::BTreeSet<String>,
}

impl InMemoryFs {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.dirs.insert("/".to_string());
        s
    }
}

impl FileSystem for InMemoryFs {
    type File = std::io::Cursor<Vec<u8>>;

    fn open(&self, path: &str) -> Result<Self::File, StorageError> {
        let data = self
            .files
            .get(path)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(path.into()))?;
        Ok(std::io::Cursor::new(data))
    }

    fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path) || self.dirs.contains(path)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, StorageError> {
        if !self.dirs.contains(path) {
            return Err(StorageError::NotFound(path.into()));
        }
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        };
        let mut out = Vec::new();
        for k in self.files.keys() {
            if k.starts_with(&prefix) && !k[prefix.len()..].contains('/') {
                out.push(k.clone());
            }
        }
        Ok(out)
    }

    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), StorageError> {
        self.files.insert(path.to_string(), data.to_vec());
        Ok(())
    }

    fn create_dir(&mut self, path: &str) -> Result<(), StorageError> {
        self.dirs.insert(path.to_string());
        Ok(())
    }
}
