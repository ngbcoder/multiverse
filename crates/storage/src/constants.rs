pub use multiverse_core::constants::*;

pub const MBR_SECTOR_SIZE: u64 = 512;
pub const GPT_HEADER_LBA: u64 = 1;
pub const GPT_FIRST_USABLE_LBA: u64 = 34;
pub const GPT_LAST_USABLE_LBA_OFFSET: u64 = 33; // from end

pub const PARTITION_TYPE_MBR_DATA: u8 = 0x07;
pub const PARTITION_TYPE_MBR_ESP: u8 = 0xEF;

// GPT type GUIDs (as bytes, little-endian fields).
pub const GPT_TYPE_ESP: [u8; 16] = [
    0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
];
pub const GPT_TYPE_BASIC_DATA: [u8; 16] = [
    0xA2, 0xA0, 0xD0, 0xEB, 0xE5, 0xB9, 0x33, 0x44, 0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26, 0x99, 0xC7,
];

/// Platform signature stored in MBR at fixed offset (placeholder bytes).
pub const PLATFORM_SIGNATURE: [u8; 8] = *b"MULTVRSE";
