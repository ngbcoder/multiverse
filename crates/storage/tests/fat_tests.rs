use multiverse_storage::fat::FatVolume;
use multiverse_storage::filesystem::FileSystem;
use std::io::Read;

fn make_image() -> Vec<u8> {
    let total_sectors: u32 = 2144;
    let bps: u16 = 512;
    let spc: u8 = 8;
    let reserved: u16 = 32;
    let nf: u8 = 2;
    let fat_size: u32 = 256;
    let size = total_sectors as usize * bps as usize;
    let mut buf = vec![0u8; size];
    // BPB
    buf[11..13].copy_from_slice(&bps.to_le_bytes());
    buf[13] = spc;
    buf[14..16].copy_from_slice(&reserved.to_le_bytes());
    buf[16] = nf;
    buf[19..21].copy_from_slice(&0u16.to_le_bytes());
    buf[21] = 0xF8;
    buf[22..24].copy_from_slice(&0u16.to_le_bytes());
    buf[32..36].copy_from_slice(&total_sectors.to_le_bytes());
    buf[36..40].copy_from_slice(&fat_size.to_le_bytes());
    buf[44..48].copy_from_slice(&2u32.to_le_bytes()); // root cluster
    buf[510] = 0x55;
    buf[511] = 0xAA;
    // mark root cluster as end of chain in both FATs
    let fat_off = reserved as usize * bps as usize;
    for f in 0..nf as usize {
        let off = fat_off + f * fat_size as usize * bps as usize + 2 * 4;
        buf[off..off+4].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes());
    }
    buf
}

#[test]
fn fat_roundtrip() {
    let mut img = make_image();
    {
        let mut vol = FatVolume::new(&mut img).unwrap();
        vol.write_file("/HELLO.TXT", b"Hello, FAT!").unwrap();
        vol.create_dir("/SUB").unwrap();
        vol.write_file("/SUB/X.TXT", b"nested").unwrap();
    }
    let mut vol = FatVolume::new(&mut img).unwrap();
    assert!(vol.exists("/HELLO.TXT"));
    let mut f = vol.open("/HELLO.TXT").unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, b"Hello, FAT!");
    let entries = vol.read_dir("/").unwrap();
    // names are uppercase 8.3
    assert!(entries.iter().any(|e| e.starts_with("HELLO")));
    assert!(entries.iter().any(|e| e.starts_with("SUB")));
    let mut f2 = vol.open("/SUB/X.TXT").unwrap();
    let mut buf2 = Vec::new();
    f2.read_to_end(&mut buf2).unwrap();
    assert_eq!(buf2, b"nested");
}