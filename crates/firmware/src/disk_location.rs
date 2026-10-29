use multiverse_core::DiskHandle;
use multiverse_core::CoreError;

#[derive(Debug, Clone)]
pub struct CandidateDisk {
    pub handle: DiskHandle,
    /// Expected size range (min,max) in bytes.
    pub expected_min_bytes: u64,
    pub expected_max_bytes: u64,
}

/// Find a disk among candidates by size and stored signature/GUID.
pub fn locate_disk(
    candidates: &[CandidateDisk],
    stored_signature: Option<u32>,
    stored_guid: Option<uuid::Uuid>,
) -> Result<DiskHandle, CoreError> {
    for c in candidates {
        let size_ok = c.handle.size_bytes >= c.expected_min_bytes
            && c.handle.size_bytes <= c.expected_max_bytes;
        if !size_ok {
            continue;
        }
        let sig_ok = match (stored_signature, c.handle.signature) {
            (Some(s), Some(h)) => s == h,
            (None, _) => true,
            (Some(_), None) => false,
        };
        let guid_ok = match (stored_guid, c.handle.guid) {
            (Some(g), Some(h)) => g == h,
            (None, _) => true,
            (Some(_), None) => false,
        };
        if sig_ok && guid_ok {
            return Ok(c.handle.clone());
        }
    }
    Err(CoreError::NotFound("no matching disk".into()))
}