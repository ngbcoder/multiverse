use multiverse_firmware::{UefiBootFlowManager, UefiVariableStore};
use multiverse_core::{PlatformId, ChainHead, DiskExtent, EnvParamBlock};
use multiverse_core::chunkmap::ChunkMapBuilder;

fn sample_chunk_map() -> multiverse_core::chunkmap::ChunkMap {
    let chain_head = ChainHead {
        partition_index: 1,
        start_sector: 2048,
        image_size_bytes: 4096,
        chunk_count: 1,
        flags: 0,
    };
    let ext = DiskExtent { start_lba: 100, sector_count: 8 };
    ChunkMapBuilder {
        chain_head,
        extents: vec![(0, ext)],
        overrides: vec![],
        virts: vec![],
        ramdisk: None,
        image_sector_size: 2048,
        disk_sector_size: 512,
    }
    .build()
}

#[test]
fn test_uefi_variable_store() {
    let mut store = UefiVariableStore::new();
    let env = EnvParamBlock {
        image_location: Some(2048),
        chunk_count: 1,
        platform_id: Some(PlatformId::X86_64Uefi),
        ramdisk: None,
        flags: 0,
    };
    store.set_env_param_block("MULTIVERSE_ENV", &env).unwrap();
    let fetched = store.get_env_param_block("MULTIVERSE_ENV").unwrap().unwrap();
    assert_eq!(fetched.image_location, Some(2048));
    assert_eq!(fetched.platform_id, Some(PlatformId::X86_64Uefi));
}

#[test]
fn test_uefi_boot_flow_success() {
    let chunk_map = sample_chunk_map();
    let mut manager = UefiBootFlowManager::new(PlatformId::X86_64Uefi, chunk_map);

    assert!(manager.virtual_device().is_some());
    assert_eq!(manager.env_block().platform_id, Some(PlatformId::X86_64Uefi));

    manager.prepare_boot().unwrap();
}

#[test]
fn test_execute_boot_sequence() {
    let chunk_map = sample_chunk_map();
    let mut manager = UefiBootFlowManager::new(PlatformId::X86_64Uefi, chunk_map);
    manager.execute_boot_sequence("/EFI/BOOT/BOOTX64.EFI").unwrap();
    assert!(manager.virtual_device().is_none()); // Cleaned up after successful boot
}

#[test]
fn test_uefi_boot_flow_fallback() {
    let chunk_map = sample_chunk_map();
    let mut manager = UefiBootFlowManager::new(PlatformId::X86_64Uefi, chunk_map);

    // First failure should trigger retry 1 (flag set)
    let can_retry_1 = manager.handle_failure().unwrap();
    assert!(can_retry_1);
    assert!(manager.env_block().flags & 0x80000000 != 0);

    // Second failure should trigger retry 2 (alternate sector size)
    let can_retry_2 = manager.handle_failure().unwrap();
    assert!(can_retry_2);

    // Third failure should return false (no more retries)
    let can_retry_3 = manager.handle_failure().unwrap();
    assert!(!can_retry_3);
}

#[test]
fn test_uefi_boot_flow_cleanup() {
    let chunk_map = sample_chunk_map();
    let mut manager = UefiBootFlowManager::new(PlatformId::X86_64Uefi, chunk_map);

    assert!(manager.virtual_device().is_some());
    manager.cleanup();
    assert!(manager.virtual_device().is_none());
}
