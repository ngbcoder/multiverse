//! Multiverse CLI Installer for deploying the multi-image bootable platform.

use multiverse_installer::{IsoDownloaderCatalog, TranslationMap};
use multiverse_storage::constants::{BOOT_PARTITION_SECTORS, DATA_START_SECTOR};
use multiverse_storage::partition::PartitionTable;
use multiverse_storage::mbr;
use multiverse_storage::{FatVolume, FileSystem};
use std::fs;
use std::path::Path;

fn main() {
    let tr = TranslationMap::english();
    println!("{}", tr.translate("welcome"));

    let args: Vec<String> = std::env::args().collect();
    let is_dry_run = args.iter().any(|a| a == "--dry-run");
    let is_force = args.iter().any(|a| a == "--force");

    if args.iter().any(|a| a == "--list-downloadable") {
        println!("Available ISO Downloads:");
        let catalog = IsoDownloaderCatalog::new();
        for iso in catalog.list() {
            println!(" - {} ({}) -> {}", iso.name, iso.os_family, iso.download_url);
        }
        return;
    }

    let target_path = args.get(1).map(|s| s.as_str()).unwrap_or("multiverse_target.img");

    if is_dry_run {
        println!("[DRY RUN] Would install Multiverse platform to: {}", target_path);
        return;
    }

    println!("Target installation image: {} (force={})", target_path, is_force);

    let total_sectors = DATA_START_SECTOR + BOOT_PARTITION_SECTORS + 20480;
    let disk_size = total_sectors * 512;
    let mut disk_buf = vec![0u8; disk_size as usize];

    // 1. Validate and create canonical partition table
    let table = PartitionTable::canonical(disk_size).expect("Failed to create canonical layout");
    let mbr_bytes = mbr::write_mbr(&table).expect("Failed to write MBR");
    disk_buf[0..512].copy_from_slice(&mbr_bytes);

    // 2. Initialize FAT32 boot partition
    let boot_start_byte = (DATA_START_SECTOR as usize) * 512;
    let boot_size_byte = (BOOT_PARTITION_SECTORS as usize) * 512;
    let bpb_slice = &mut disk_buf[boot_start_byte..boot_start_byte + boot_size_byte];

    let bps: u16 = 512;
    let spc: u8 = 8;
    let reserved: u16 = 32;
    let nf: u8 = 2;
    let fat_size: u32 = 256;

    bpb_slice[11..13].copy_from_slice(&bps.to_le_bytes());
    bpb_slice[13] = spc;
    bpb_slice[14..16].copy_from_slice(&reserved.to_le_bytes());
    bpb_slice[16] = nf;
    bpb_slice[19..21].copy_from_slice(&0u16.to_le_bytes());
    bpb_slice[21] = 0xF8;
    bpb_slice[22..24].copy_from_slice(&0u16.to_le_bytes());
    bpb_slice[32..36].copy_from_slice(&(BOOT_PARTITION_SECTORS as u32).to_le_bytes());
    bpb_slice[36..40].copy_from_slice(&fat_size.to_le_bytes());
    bpb_slice[44..48].copy_from_slice(&2u32.to_le_bytes());
    bpb_slice[82..90].copy_from_slice(b"FAT32   ");
    bpb_slice[510] = 0x55;
    bpb_slice[511] = 0xAA;

    let fat_off = reserved as usize * bps as usize;
    for f in 0..nf as usize {
        let off = fat_off + f * fat_size as usize * bps as usize;
        if off + 12 <= bpb_slice.len() {
            bpb_slice[off..off + 4].copy_from_slice(&0x0FFF_FFF8u32.to_le_bytes());
            bpb_slice[off + 4..off + 8].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes());
            bpb_slice[off + 8..off + 12].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes());
        }
    }

    // 3. Deploy INSTALL/ payload
    {
        let mut vol = FatVolume::new(bpb_slice).expect("Failed to initialize FAT volume");
        vol.create_dir("/EFI").unwrap();
        vol.create_dir("/EFI/BOOT").unwrap();
        vol.create_dir("/CONFIG").unwrap();

        let config_src = Path::new("INSTALL/config/config.json");
        if config_src.exists() {
            let cfg = fs::read(config_src).unwrap();
            vol.write_file("/CONFIG/CONFIG.JSON", &cfg).unwrap();
        } else {
            vol.write_file("/CONFIG/CONFIG.JSON", b"{\"control\":{\"timeout\":30}}").unwrap();
        }

        let efi_src = Path::new("crates/uefi-test/target/x86_64-unknown-uefi/debug/uefi-test.efi");
        if efi_src.exists() {
            let efi = fs::read(efi_src).unwrap();
            vol.write_file("/EFI/BOOT/BOOTX64.EFI", &efi).unwrap();
        }
    }

    // 4. Write image file
    fs::write(target_path, &disk_buf).expect("Failed to write target disk image");
    println!("{}", tr.translate("done"));
}
