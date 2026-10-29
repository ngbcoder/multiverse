use multiverse_core::RamdiskParam;

#[derive(Debug, Clone)]
pub struct MemdiskImage {
    pub data: Vec<u8>,
    pub location: u64,
}

impl MemdiskImage {
    pub fn new(data: Vec<u8>, location: u64) -> Self {
        Self { data, location }
    }

    pub fn ramdisk_param(&self) -> RamdiskParam {
        RamdiskParam {
            location: self.location,
            size_bytes: self.data.len() as u64,
        }
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}