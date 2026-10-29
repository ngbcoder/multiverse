use multiverse_firmware::{GrubBiosConfig, BootPartitionFs, SecureBootManager, SecureBootState};

#[test]
fn test_grub_bios_config() {
    let config = GrubBiosConfig::default();
    let script = config.generate_script();
    assert!(script.contains("insmod memdisk"));
    assert!(script.contains("insmod fat"));
}

#[test]
fn test_secure_boot_manager() {
    let mut sb = SecureBootManager::new(SecureBootState::Disabled);
    assert!(!sb.is_active());
    sb.set_state(SecureBootState::Enabled).unwrap();
    assert!(sb.is_active());
    assert_eq!(sb.state(), SecureBootState::Enabled);
}

#[test]
fn test_boot_partition_fs_opening() {
    // Construct minimal FAT32 image buffer for testing
    let mut img = vec![0u8; 32768];
    img[11..13].copy_from_slice(&512u16.to_le_bytes());
    img[13] = 8;
    img[14..16].copy_from_slice(&32u16.to_le_bytes());
    img[16] = 2;
    img[32..36].copy_from_slice(&65536u32.to_le_bytes());
    img[36..40].copy_from_slice(&256u32.to_le_bytes());
    img[44..48].copy_from_slice(&2u32.to_le_bytes());
    img[510] = 0x55;
    img[511] = 0xAA;

    let boot_fs = BootPartitionFs::new(&mut img);
    assert!(boot_fs.is_ok());
}
