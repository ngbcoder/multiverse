// Minimal ISO 9660 reader for the CDROM BlockIO device.
use uefi::proto::media::block::BlockIO;
use alloc::vec::Vec;
use alloc::string::String;

pub fn read_cdrom_file(path: &str) -> Option<Vec<u8>> {
    let handles = uefi::boot::find_handles::<BlockIO>().ok()?;
    for h in handles {
        let bio = uefi::boot::open_protocol_exclusive::<BlockIO>(h).ok()?;
        if bio.media().block_size() != 2048 { continue; }
        let mid = bio.media().media_id();
        // read PVD at LBA 16
        let mut pvd = [0u8; 2048];
        if bio.read_blocks(mid, 16, &mut pvd).is_err() { continue; }
        if &pvd[1..6] != b"CD001" { continue; }
        // root dir record at offset 156
        let root_lba = u32::from_le_bytes(pvd[156+2..156+6].try_into().ok()?) as u64;
        let root_size = u32::from_le_bytes(pvd[156+10..156+14].try_into().ok()?) as u64;
        // traverse
        let parts: Vec<&str> = path.split(|c| c == '\\' || c == '/').filter(|s| !s.is_empty()).collect();
        let mut cur_lba = root_lba;
        let mut cur_size = root_size;
        let mut is_last = false;
        for (i, part) in parts.iter().enumerate() {
            is_last = i == parts.len() - 1;
            let (lba, size, is_dir) = find_entry(&bio, mid, cur_lba, cur_size, part)?;
            if is_last {
                // read file data
                if is_dir { return None; }
                return Some(read_extent(&bio, mid, lba, size));
            }
            if !is_dir { return None; }
            cur_lba = lba;
            cur_size = size;
        }
        let _ = is_last;
    }
    None
}

fn find_entry(bio: &BlockIO, mid: u32, dir_lba: u64, dir_size: u64, name: &str) -> Option<(u64, u64, bool)> {
    let blocks = (dir_size + 2047) / 2048;
    let mut buf = alloc::vec![0u8; (blocks as usize) * 2048];
    for b in 0..blocks {
        if bio.read_blocks(mid, dir_lba + b, &mut buf[b as usize * 2048..(b as usize + 1) * 2048]).is_err() {
            return None;
        }
    }
    let mut off = 0usize;
    while off < dir_size as usize {
        let rec_len = buf[off] as usize;
        if rec_len == 0 {
            // skip to next sector boundary
            let rem = off % 2048;
            if rem == 0 { break; }
            off += 2048 - rem;
            continue;
        }
        if off + rec_len > buf.len() { break; }
        let rec = &buf[off..off + rec_len];
        // LBA at offset 2 (8 bytes both-endian) -> use first 4
        let lba = u32::from_le_bytes(rec[2..6].try_into().ok()?) as u64;
        let size = u32::from_le_bytes(rec[10..14].try_into().ok()?) as u64;
        let flags = rec[25];
        let is_dir = flags & 0x02 != 0;
        let name_len = rec[30] as usize;
        if 31 + name_len <= rec.len() {
            let raw_name = &rec[31..31 + name_len];
            // ISO names may have ";1" version suffix
            let name_str = String::from_utf8_lossy(raw_name);
            let name_clean = name_str.split(';').next().unwrap_or(&name_str);
            if name_clean.eq_ignore_ascii_case(name) {
                return Some((lba, size, is_dir));
            }
        }
        off += rec_len;
    }
    None
}

fn read_extent(bio: &BlockIO, mid: u32, lba: u64, size: u64) -> Vec<u8> {
    let blocks = (size + 2047) / 2048;
    let mut buf = alloc::vec![0u8; (blocks as usize) * 2048];
    for b in 0..blocks {
        let _ = bio.read_blocks(mid, lba + b, &mut buf[b as usize * 2048..(b as usize + 1) * 2048]);
    }
    buf.truncate(size as usize);
    buf
}