use multiverse_firmware::{VirtualBlockDevice, BlockIoProtocol, is_iso9660, UefiBootFlowManager};
use multiverse_core::{PlatformId, ChainHead, DiskExtent};
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
fn vbd_read_disk() {
    let cm = sample_chunk_map();
    let mut disk = vec![0u8; 512 * 200];
    for i in 0..disk.len() {
        disk[i] = (i % 251) as u8;
    }
    let disk2 = disk.clone();
    let vdev = VirtualBlockDevice::new(cm).with_disk_reader(move |lba, count, buf| {
        let start = lba as usize * 512;
        let bytes = count as usize * 512;
        buf.copy_from_slice(&disk2[start..start + bytes]);
        Ok(())
    });
    let mut buf = vec![0u8; 4096];
    vdev.read_blocks(0, 2, &mut buf).unwrap();
    assert_eq!(buf, &disk[100 * 512..100 * 512 + 4096]);
}

#[test]
fn blockio_protocol_basic() {
    let cm = sample_chunk_map();
    let vdev = VirtualBlockDevice::new(cm);
    let mut bio = BlockIoProtocol::new(vdev);
    assert_eq!(bio.media.media_id, 1);
    assert!(bio.is_read_only());
    assert!(bio.reset(false).is_ok() || bio.reset(true).is_ok());
    // write rejected
    assert!(bio.write_blocks(1, 0, &[1, 2, 3]).is_err());
    // flush ok
    assert!(bio.flush_blocks(1).is_ok());
}

#[test]
fn iso9660_detect() {
    let mut buf = vec![0u8; 17 * 2048 + 6];
    buf[16 * 2048 + 1..16 * 2048 + 6].copy_from_slice(b"CD001");
    assert!(is_iso9660(&buf));
}

#[test]
fn boot_flow_env_chunk_count() {
    let cm = sample_chunk_map();
    let manager = UefiBootFlowManager::new(PlatformId::X86_64Uefi, cm);
    assert_eq!(manager.env_block().chunk_count, 1);
    assert_eq!(manager.env_block().image_location, Some(2048));
}

#[test]
fn boot_flow_install_blockio() {
    let cm = sample_chunk_map();
    let manager = UefiBootFlowManager::new(PlatformId::X86_64Uefi, cm);
    let bio = manager.install_block_io_protocol().unwrap();
    assert!(bio.is_read_only());
}