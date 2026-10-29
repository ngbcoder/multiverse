use multiverse_core::chunkmap::{ChunkMap, ChunkMapBuilder};
use multiverse_core::{ChainHead, DiskExtent, ImgChunk};

fn base_map() -> ChunkMap {
    let chain_head = ChainHead {
        partition_index: 1,
        start_sector: 2048,
        image_size_bytes: 4096, // 2 sectors of 2048
        chunk_count: 1,
        flags: 0,
    };
    let ext = DiskExtent { start_lba: 100, sector_count: 8 }; // 8*512=4096 bytes = 2*2048
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
fn translate_single_chunk() {
    let m = base_map();
    let ops = m.translate(0, 2).unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].disk_lba, 100);
    assert_eq!(ops[0].disk_sector_count, 8);
    assert_eq!(ops[0].image_sector_count, 2);
}

#[test]
fn read_only_rejects_write() {
    let m = base_map();
    assert!(m.write(0, 1).is_err());
}

#[test]
fn override_takes_priority() {
    use multiverse_core::OverrideChunk;
    let mut m = base_map();
    m.override_chunks.push(OverrideChunk {
        image_lba: 1,
        sector_count: 1,
        data: vec![0xAB; 2048],
    });
    let ops = m.translate(0, 2).unwrap();
    // first op disk (image sector 0), second override (image sector 1)
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0].source, multiverse_core::chunkmap::ReadSource::Disk);
    assert_eq!(ops[1].source, multiverse_core::chunkmap::ReadSource::Override);
}