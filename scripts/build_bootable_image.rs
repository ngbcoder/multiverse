//! Script to build a bootable multi-image storage raw disk image (IMG) using multiverse-storage.

use multiverse_storage::constants::{BOOT_PARTITION_SECTORS, DATA_START_SECTOR};
use multiverse_storage::partition::PartitionTable;
use multiverse_storage::mbr;
use multiverse_storage::FatVolume;
use std::fs;

fn main() {
    println!("Building bootable multi-image storage disk image...");

    let total_sectors = DATA_START_SECTOR + BOOT_PARTITION_SECTORS + 10240; // ~35MB disk
    let disk_size = total_sectors * 512;
    let mut disk_buf = vec![0u8; disk_size as usize];

    // 1. Create canonical partition table & write MBR
    let table = PartitionTable::canonical(disk_size).unwrap();
    let mbr_bytes = mbr::write_mbr(&table).unwrap();
    disk_buf[0..512].copy_from_slice(&mbr_bytes);

    // 2. Initialize FAT32 boot partition starting at DATA_START_SECTOR
    let boot_start_byte = (DATA_START_SECTOR as usize) * 512;
    let boot_size_byte = (BOOT_PARTITION_SECTORS as usize) * 512;

    let bpb_slice = &mut disk_buf[boot_start_byte..boot_start_byte + boot_size_byte];
    let bps: u16 = 512;
    let spc: u8 = 8;
    let reserved: u16 = 32;
    let nf: u8 = 2;
    let fat_size: u32 = 512;

    bpb_slice[11..13].copy_from_slice(&bps.to_le_bytes());
    bpb_slice[13] = spc;
    bpb_slice[14..16].copy_from_slice(&reserved.to_le_bytes());
    bpb_slice[16] = nf;
    bpb_slice[19..21].copy_from_slice(&0u16.to_le_bytes());
    bpb_slice[21] = 0xF8;
    bpb_slice[22..24].copy_from_slice(&0u16.to_le_bytes());
    bpb_slice[32..36].copy_from_slice(&BOOT_PARTITION_SECTORS.to_le_bytes());
    bpb_slice[36..40].copy_from_slice(&fat_size.to_le_bytes());
    bpb_slice[44..48].copy_from_slice(&2u32.to_le_bytes()); // root cluster
    bpb_slice[510] = 0x55;
    bpb_slice[511] = 0xAA;

    let fat_off = reserved as usize * bps as usize;
    for f in 0..nf as usize {
        let off = fat_off + f * fat_size as usize * bps as usize + 2 * 4;
        if off + 4 <= bpb_slice.len() {
            bpb_slice[off..off+4].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes());
        }
    }

    // 3. Write default config.json into boot partition FAT volume
    {
        let mut vol = FatVolume::new(bpb_slice).unwrap();
        let config_data = fs::read_to_string("INSTALL/config/config.json").unwrap_or_else(|_| "{\"control\":{\"timeout\":30}}".to_string());
        vol.write_file("/CONFIG.JSON", config_data.as_bytes()).unwrap();
        vol.create_dir("/BOOT").unwrap();
        vol.write_file("/BOOT/README.TXT", b"Multiverse Multi-Image Bootable Platform").unwrap();
    }

    // 4. Output image file
    let output_path = "multiverse_disk.img";
    fs::write(output_path, &disk_buf).unwrap();
    println!("Successfully built bootable storage image: {} ({} bytes)", output_path, disk_buf.len());
}
