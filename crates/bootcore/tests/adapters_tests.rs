use multiverse_bootcore::{get_adapter, ChainDataBuilder, OsType};
use multiverse_core::{DiskExtent, ImgChunk, RamdiskParam};

#[test]
fn test_linux_adapter() {
    let adapter = get_adapter("/iso/ubuntu-22.04.iso");
    assert_eq!(adapter.os_type(), OsType::Linux);
    let params = adapter.prepare_boot_params("/iso/ubuntu-22.04.iso").unwrap();
    assert_eq!(params.os_type, OsType::Linux);
    assert!(params.kernel_path.is_some());
    assert!(params.cmdline.contains("rdinit=/init"));
}

#[test]
fn test_windows_adapter() {
    let adapter = get_adapter("/iso/win11.iso");
    assert_eq!(adapter.os_type(), OsType::Windows);
    let params = adapter.prepare_boot_params("/iso/win11.iso").unwrap();
    assert_eq!(params.os_type, OsType::Windows);
    assert_eq!(params.kernel_path.as_deref(), Some("/bootmgr"));
    assert_eq!(params.cmdline, "wimboot");
}

#[test]
fn test_chain_data_builder() {
    let extent = DiskExtent { start_lba: 2048, sector_count: 1024 };
    let chunk = ImgChunk { image_lba: 0, extent };
    let ramdisk = RamdiskParam { location: 0x10000, size_bytes: 4096 };

    let (head, chunks, overrides, vchunks, rdisk) = ChainDataBuilder::new(1, 2048, 1024 * 512)
        .with_chunks(vec![chunk])
        .with_ramdisk(ramdisk)
        .with_flags(0x1)
        .build();

    assert_eq!(head.partition_index, 1);
    assert_eq!(head.start_sector, 2048);
    assert_eq!(head.chunk_count, 1);
    assert_eq!(head.flags, 0x1);
    assert_eq!(chunks.len(), 1);
    assert!(overrides.is_empty());
    assert!(vchunks.is_empty());
    assert!(rdisk.is_some());
}
