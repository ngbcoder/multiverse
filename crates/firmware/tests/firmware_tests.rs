use multiverse_firmware::cmdline::CommandLine;
use multiverse_firmware::disk_location::{locate_disk, CandidateDisk};
use multiverse_core::DiskHandle;

#[test]
fn parse_cmdline() {
    let cl = CommandLine::parse("debug fallback memdisk sector_size=2048 foo=bar");
    assert!(cl.debug);
    assert!(cl.fallback);
    assert!(cl.memdisk);
    assert_eq!(cl.sector_size, Some(2048));
    assert_eq!(cl.get("foo"), Some("bar"));
}

#[test]
fn locate_disk_by_signature() {
    let h = DiskHandle { index: 0, size_bytes: 1000, signature: Some(42), guid: None };
    let c = CandidateDisk { handle: h.clone(), expected_min_bytes: 500, expected_max_bytes: 2000 };
    let found = locate_disk(&[c], Some(42), None).unwrap();
    assert_eq!(found.signature, Some(42));
}

#[test]
fn locate_disk_not_found() {
    let h = DiskHandle { index: 0, size_bytes: 1000, signature: Some(7), guid: None };
    let c = CandidateDisk { handle: h, expected_min_bytes: 500, expected_max_bytes: 2000 };
    assert!(locate_disk(&[c], Some(42), None).is_err());
}