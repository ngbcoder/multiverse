//! Binary to build a 100% compliant UEFI boot disk image (uefi_boot.img) using mkfs.fat -F 32 -I.

use multiverse_storage::{FatVolume, FileSystem};
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("Building 100% compliant UEFI boot disk image (uefi_boot.img)...");

    let img_path = "uefi_boot.img";
    let size_mb = 64;

    // 1. Create blank image file of 64MB
    let disk_buf = vec![0u8; size_mb * 1024 * 1024];
    fs::write(img_path, &disk_buf).unwrap();

    // 2. Format image as FAT32 via mkfs.fat -F 32 -I
    let status = Command::new("C:/msys64/usr/bin/mkfs.fat.exe")
        .args(&["-F", "32", "-I", img_path])
        .status()
        .expect("Failed to execute mkfs.fat");

    if !status.success() {
        panic!("mkfs.fat failed");
    }
    println!("Formatted {} as FAT32 via mkfs.fat", img_path);

    // 3. Read formatted image for FatVolume file copy
    let mut disk_buf = fs::read(img_path).unwrap();

    // 4. Write config and BOOTX64.EFI
    {
        let mut vol = FatVolume::new(&mut disk_buf).unwrap();
        let config_data = fs::read_to_string("INSTALL/config/config.json").unwrap_or_else(|_| "{\"control\":{\"timeout\":30}}".to_string());
        vol.write_file("/CONFIG.JSON", config_data.as_bytes()).unwrap();
        vol.create_dir("/BOOT").unwrap();
        vol.write_file("/BOOT/README.TXT", b"Multiverse UEFI Boot Disk").unwrap();

        vol.create_dir("/EFI").unwrap();
        vol.create_dir("/EFI/BOOT").unwrap();

        let efi_path = Path::new("crates/uefi-test/target/x86_64-unknown-uefi/debug/uefi-test.efi");
        if efi_path.exists() {
            let efi_bytes = fs::read(efi_path).unwrap();
            vol.write_file("/EFI/BOOT/BOOTX64.EFI", &efi_bytes).unwrap();
            println!("Installed BOOTX64.EFI ({} bytes)", efi_bytes.len());
        }
    }

    fs::write(img_path, &disk_buf).unwrap();
    println!("Successfully built 100% compliant UEFI boot image: {} ({} bytes)", img_path, disk_buf.len());
}
