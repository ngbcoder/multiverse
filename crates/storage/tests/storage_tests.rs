use multiverse_storage::constants::{BOOT_PARTITION_SECTORS, DATA_START_SECTOR};
use multiverse_storage::partition::PartitionTable;
use multiverse_storage::recognition::is_installed_device;

#[test]
fn canonical_layout_recognized() {
    let disk_size = DATA_START_SECTOR + BOOT_PARTITION_SECTORS + 1000;
    let table = PartitionTable::canonical(disk_size).unwrap();
    assert_eq!(table.partitions.len(), 2);
    assert_eq!(table.partitions[0].start_sector, DATA_START_SECTOR);
    assert_eq!(table.partitions[1].sector_count, BOOT_PARTITION_SECTORS);
    assert!(is_installed_device(&table));
}

#[test]
fn too_small_disk_rejected() {
    let disk_size = DATA_START_SECTOR + BOOT_PARTITION_SECTORS;
    assert!(PartitionTable::canonical(disk_size).is_err());
}

#[test]
fn mbr_roundtrip() {
    use multiverse_storage::mbr;
    let disk_size = DATA_START_SECTOR + BOOT_PARTITION_SECTORS + 1000;
    let table = PartitionTable::canonical(disk_size).unwrap();
    let bytes = mbr::write_mbr(&table).unwrap();
    assert_eq!(bytes.len(), 512);
    assert_eq!(bytes[510], 0x55);
    assert_eq!(bytes[511], 0xAA);
    let parsed = mbr::parse_mbr(&bytes).unwrap();
    assert_eq!(parsed.partitions.len(), 2);
    assert_eq!(parsed.partitions[0].start_sector, DATA_START_SECTOR);
}
