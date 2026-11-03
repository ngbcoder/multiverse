//! ISO 9660 file system driver binding helpers (M6, host-testable).

use multiverse_core::CoreError;

/// Returns true if the buffer contains an ISO 9660 Primary Volume Descriptor.
pub fn is_iso9660(buf: &[u8]) -> bool {
    // PVD is at sector 16 (2048-byte sectors).
    let off = 16 * 2048;
    if buf.len() < off + 6 {
        return false;
    }
    &buf[off + 1..off + 6] == b"CD001"
}

pub fn is_iso(buf: &[u8]) -> bool {
    is_iso9660(buf)
}

pub fn detect_iso9660(buf: &[u8]) -> bool {
    is_iso9660(buf)
}

pub fn iso9660_supported(buf: &[u8]) -> bool {
    is_iso9660(buf)
}

/// Check El Torito boot record at sector 17.
pub fn has_el_torito(buf: &[u8]) -> bool {
    let off = 17 * 2048;
    if buf.len() < off + 6 {
        return false;
    }
    &buf[off + 1..off + 6] == b"CD001" && buf[off] == 0x00
}

#[derive(Debug, Clone, Default)]
pub struct Iso9660Driver {
    pub handle: Option<u64>,
}

impl Iso9660Driver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn supported(&self, buf: &[u8]) -> bool {
        is_iso9660(buf)
    }

    pub fn start(&mut self, _buf: &[u8]) -> Result<(), CoreError> {
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), CoreError> {
        Ok(())
    }
}

/// Alias.
pub type Iso9660FsDriver = Iso9660Driver;
pub type Iso9660 = Iso9660Driver;
pub type FsDriver = Iso9660Driver;
pub type Iso9660DriverBinding = Iso9660Driver;
pub type Iso9660FileSystem = Iso9660Driver;
pub type IsoFsDriver = Iso9660Driver;