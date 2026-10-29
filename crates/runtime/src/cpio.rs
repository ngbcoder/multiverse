//! Initramfs CPIO extraction, compression format detection, and layer injection.

use crate::RuntimeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    Gzip,
    Bzip2,
    Xz,
    Lzma,
    Lz4,
    Zstd,
    Uncompressed,
}

impl CompressionFormat {
    pub fn detect(header: &[u8]) -> Self {
        if header.len() >= 2 && header[0] == 0x1F && header[1] == 0x8B {
            CompressionFormat::Gzip
        } else if header.len() >= 3 && &header[0..3] == b"BZh" {
            CompressionFormat::Bzip2
        } else if header.len() >= 6 && &header[0..6] == b"\xFD7zXZ\x00" {
            CompressionFormat::Xz
        } else if header.len() >= 4 && &header[0..4] == b"\x04\x22\x4D\x18" {
            CompressionFormat::Lz4
        } else if header.len() >= 4 && &header[0..4] == b"\x28\xB5\x2F\xFD" {
            CompressionFormat::Zstd
        } else {
            CompressionFormat::Uncompressed
        }
    }
}

pub struct CpioInjector;

impl CpioInjector {
    pub fn prepare_injection_layer(archive_name: &str, target_path: &str) -> Result<String, RuntimeError> {
        if archive_name.is_empty() || target_path.is_empty() {
            return Err(RuntimeError::Init("invalid injection path".into()));
        }
        Ok(format!("cpio -idmv < {} -D {}", archive_name, target_path))
    }
}
