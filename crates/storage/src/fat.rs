use crate::error::StorageError;
use crate::filesystem::{FatBpb, FileSystem};
use std::io::Cursor;

pub struct FatVolume<'a> {
    data: &'a mut [u8],
    bpb: FatBpb,
}

impl<'a> FatVolume<'a> {
    pub fn new(data: &'a mut [u8]) -> Result<Self, StorageError> {
        let bpb = FatBpb::parse(data)?;
        if !bpb.is_fat32() {
            return Err(StorageError::Unsupported("only FAT32 volumes supported".into()));
        }
        Ok(Self { data, bpb })
    }

    fn bps(&self) -> usize { self.bpb.bytes_per_sector as usize }
    fn spc(&self) -> usize { self.bpb.sectors_per_cluster as usize }
    fn cluster_size(&self) -> usize { self.bps() * self.spc() }

    fn fat_offset(&self) -> usize { self.bpb.reserved_sectors as usize * self.bps() }
    fn data_start_sector(&self) -> usize {
        self.bpb.reserved_sectors as usize
            + self.bpb.num_fats as usize * self.bpb.fat_size() as usize
    }
    fn data_offset(&self) -> usize { self.data_start_sector() * self.bps() }

    fn cluster_offset(&self, cluster: u32) -> usize {
        self.data_offset() + (cluster as usize - 2) * self.cluster_size()
    }

    fn read_fat(&self, cluster: u32) -> u32 {
        let off = self.fat_offset() + cluster as usize * 4;
        if off + 4 > self.data.len() { return 0x0FFF_FFFF; }
        u32::from_le_bytes(self.data[off..off+4].try_into().unwrap()) & 0x0FFF_FFFF
    }

    fn write_fat(&mut self, cluster: u32, value: u32) {
        let off = self.fat_offset() + cluster as usize * 4;
        if off + 4 > self.data.len() { return; }
        self.data[off..off+4].copy_from_slice(&(value & 0x0FFF_FFFF).to_le_bytes());
    }

    fn chain(&self, start: u32) -> Vec<u32> {
        let mut out = Vec::new();
        let mut c = start;
        let mut guard = 0;
        while c >= 2 && c < 0x0FFF_FFF8 && guard < 1_000_000 {
            out.push(c);
            c = self.read_fat(c);
            guard += 1;
        }
        out
    }

    fn alloc_cluster(&mut self) -> Option<u32> {
        // find free cluster (value 0)
        let total_clusters = (self.data.len() - self.data_offset()) / self.cluster_size();
        for i in 2..(total_clusters as u32 + 2) {
            if self.read_fat(i) == 0 {
                self.write_fat(i, 0x0FFF_FFFF);
                // zero cluster
                let off = self.cluster_offset(i);
                let sz = self.cluster_size();
                if off + sz <= self.data.len() {
                    for b in &mut self.data[off..off+sz] { *b = 0; }
                }
                return Some(i);
            }
        }
        None
    }

    fn free_chain(&mut self, start: u32) {
        let mut c = start;
        let mut guard = 0;
        while c >= 2 && c < 0x0FFF_FFF8 && guard < 1_000_000 {
            let next = self.read_fat(c);
            self.write_fat(c, 0);
            c = next;
            guard += 1;
        }
    }

    // Directory entry helpers
    fn short_name_match(entry: &[u8], name: &str) -> bool {
        let mut base = [b' '; 8];
        let mut ext = [b' '; 3];
        let upper = name.to_uppercase();
        let parts: Vec<&str> = upper.split('.').collect();
        let b = parts[0].as_bytes();
        for i in 0..8.min(b.len()) { base[i] = b[i]; }
        if parts.len() > 1 {
            let e = parts[1].as_bytes();
            for i in 0..3.min(e.len()) { ext[i] = e[i]; }
        }
        // normalize entry bytes: treat 0 as space
        let mut ebase = [b' ';8];
        let mut eext = [b' ';3];
        for i in 0..8 { ebase[i] = if entry[i]==0 { b' ' } else { entry[i] }; }
        for i in 0..3 { eext[i] = if entry[8+i]==0 { b' ' } else { entry[8+i] }; }
        ebase == base && eext == ext
    }

    fn read_dir_entries(&self, cluster: u32) -> Vec<[u8;32]> {
        // returns (cluster of dir, entries) flattened? We'll just collect entries across chain.
        let mut entries = Vec::new();
        for c in self.chain(cluster) {
            let off = self.cluster_offset(c);
            let sz = self.cluster_size();
            let mut i = 0;
            while i + 32 <= sz && off + i + 32 <= self.data.len() {
                let e = &self.data[off+i..off+i+32];
                if e[0] == 0 { break; }
                if e[0] != 0xE5 {
                    let mut arr = [0u8;32];
                    arr.copy_from_slice(e);
                    entries.push(arr);
                }
                i += 32;
            }
        }
        entries
    }

    fn resolve_path(&self, path: &str) -> Result<(u32, bool, u32, usize), StorageError> {
        // returns (parent_cluster, is_dir, entry_cluster, file_size)
        // We'll implement lookup returning (cluster_of_entry, is_dir, size)
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        let mut cur_cluster = self.bpb.root_cluster;
        if parts.is_empty() {
            return Ok((cur_cluster, true, cur_cluster, 0));
        }
        for (idx, part) in parts.iter().enumerate() {
            let entries = self.read_dir_entries(cur_cluster);
            let mut found: Option<[u8;32]> = None;
            for e in entries {
                if e[0] == 0 { continue; }
                if Self::short_name_match(&e, part) {
                    found = Some(e);
                    break;
                }
            }
            let e = found.ok_or_else(|| StorageError::NotFound(path.into()))?;
            let is_dir = e[11] & 0x10 != 0;
            let cluster = (u16::from_le_bytes([e[26], e[27]]) as u32)
                | ((u16::from_le_bytes([e[20], e[21]]) as u32) << 16);
            let size = u32::from_le_bytes([e[28], e[29], e[30], e[31]]) as usize;
            if idx == parts.len() - 1 {
                return Ok((cur_cluster, is_dir, cluster, size));
            }
            if !is_dir {
                return Err(StorageError::InvalidArg("not a directory".into()));
            }
            cur_cluster = cluster;
        }
        Ok((cur_cluster, true, cur_cluster, 0))
    }

    fn read_file_bytes(&self, cluster: u32, size: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(size);
        for c in self.chain(cluster) {
            let off = self.cluster_offset(c);
            let sz = self.cluster_size();
            let take = sz.min(size - out.len());
            if off + take <= self.data.len() {
                out.extend_from_slice(&self.data[off..off+take]);
            }
            if out.len() >= size { break; }
        }
        out.truncate(size);
        out
    }

    /// Write or overwrite a file (root or subdir supported). Creates parent dirs? Assumes exist.
    pub fn write_file_path(&mut self, path: &str, content: &[u8]) -> Result<(), StorageError> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err(StorageError::InvalidArg("empty path".into()));
        }
        // resolve parent
        let (parent_cluster, _fname) = if parts.len() == 1 {
            (self.bpb.root_cluster, parts[0])
        } else {
            let parent_path = format!("/{}", parts[..parts.len()-1].join("/"));
            let (_, is_dir, cluster, _) = self.resolve_path(&parent_path)?;
            if !is_dir { return Err(StorageError::InvalidArg("parent not dir".into())); }
            (cluster, parts[parts.len()-1])
        };
        let fname = *parts.last().unwrap();

        // find existing entry
        let mut found_offset: Option<usize> = None; // byte offset in data
        for c in self.chain(parent_cluster) {
            let off = self.cluster_offset(c);
            let sz = self.cluster_size();
            let mut i = 0;
            while i + 32 <= sz && off + i + 32 <= self.data.len() {
                let e = &self.data[off+i..off+i+32];
                if e[0] != 0 && e[0] != 0xE5 && Self::short_name_match(e, fname) {
                    found_offset = Some(off + i);
                }
                i += 32;
            }
        }

        // allocate clusters for content
        let cluster_size = self.cluster_size();
        let needed = (content.len() + cluster_size - 1) / cluster_size;
        let mut new_clusters: Vec<u32> = Vec::new();
        for _ in 0..needed.max(1) {
            match self.alloc_cluster() {
                Some(c) => new_clusters.push(c),
                None => return Err(StorageError::Unsupported("no free clusters".into())),
            }
        }
        // link clusters
        for i in 0..new_clusters.len() {
            if i + 1 < new_clusters.len() {
                self.write_fat(new_clusters[i], new_clusters[i+1]);
            } else {
                self.write_fat(new_clusters[i], 0x0FFF_FFFF);
            }
        }
        // write content
        for (i, c) in new_clusters.iter().enumerate() {
            let off = self.cluster_offset(*c);
            let start = i * cluster_size;
            let end = (start + cluster_size).min(content.len());
            if start < content.len() {
                let len = end - start;
                self.data[off..off+len].copy_from_slice(&content[start..end]);
                if len < cluster_size {
                    for b in &mut self.data[off+len..off+cluster_size] { *b = 0; }
                }
            } else {
                for b in &mut self.data[off..off+cluster_size] { *b = 0; }
            }
        }

        let first_cluster = new_clusters.first().copied().unwrap_or(0);

        if let Some(off) = found_offset {
            // free old chain
            let old_low = u16::from_le_bytes([self.data[off+26], self.data[off+27]]);
            let old_high = u16::from_le_bytes([self.data[off+20], self.data[off+21]]);
            let old_cluster = (old_high as u32) << 16 | old_low as u32;
            if old_cluster >= 2 { self.free_chain(old_cluster); }
            // update entry
            self.data[off+28..off+32].copy_from_slice(&(content.len() as u32).to_le_bytes());
            self.data[off+26..off+28].copy_from_slice(&(first_cluster as u16).to_le_bytes());
            self.data[off+20..off+22].copy_from_slice(&((first_cluster >> 16) as u16).to_le_bytes());
        } else {
            // find free slot in parent dir
            let mut slot: Option<usize> = None;
            'outer: for c in self.chain(parent_cluster) {
                let off = self.cluster_offset(c);
                let sz = self.cluster_size();
                let mut i = 0;
                while i + 32 <= sz && off + i + 32 <= self.data.len() {
                    if self.data[off+i] == 0 || self.data[off+i] == 0xE5 {
                        slot = Some(off + i);
                        break 'outer;
                    }
                    i += 32;
                }
            }
            let slot = slot.ok_or_else(|| StorageError::Unsupported("directory full".into()))?;
            // build 8.3 entry
            let mut entry = [0u8; 32];
            let upper = fname.to_uppercase();
            let parts: Vec<&str> = upper.split('.').collect();
            let mut base = [b' '; 8];
            let mut ext = [b' '; 3];
            let b = parts[0].as_bytes();
            for i in 0..8.min(b.len()) { base[i] = b[i]; }
            if parts.len() > 1 {
                let e = parts[1].as_bytes();
                for i in 0..3.min(e.len()) { ext[i] = e[i]; }
            }
            entry[0..8].copy_from_slice(&base);
            entry[8..11].copy_from_slice(&ext);
            entry[11] = 0x20; // archive
            entry[26..28].copy_from_slice(&(first_cluster as u16).to_le_bytes());
            entry[20..22].copy_from_slice(&((first_cluster >> 16) as u16).to_le_bytes());
            entry[28..32].copy_from_slice(&(content.len() as u32).to_le_bytes());
            self.data[slot..slot+32].copy_from_slice(&entry);
        }

        Ok(())
    }

    pub fn create_dir_path(&mut self, path: &str) -> Result<(), StorageError> {
        // create directory entry with cluster allocated
        let content: [u8;0] = [];
        let _ = content;
        // Reuse write? Directories need '.' and '..' entries. Minimal: allocate cluster and add entry with dir attr.
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() { return Ok(()); }
        let parent_cluster = if parts.len() == 1 {
            self.bpb.root_cluster
        } else {
            let parent_path = format!("/{}", parts[..parts.len()-1].join("/"));
            let (_, is_dir, cluster, _) = self.resolve_path(&parent_path)?;
            if !is_dir { return Err(StorageError::InvalidArg("parent not dir".into())); }
            cluster
        };
        let fname = *parts.last().unwrap();

        // allocate cluster
        let c = self.alloc_cluster().ok_or_else(|| StorageError::Unsupported("no free clusters".into()))?;
        // write . and .. entries
        let off = self.cluster_offset(c);
        let mut dot = [0u8;32];
        dot[0] = b'.';
        for i in 1..8 { dot[i] = b' '; }
        dot[11] = 0x10;
        dot[26..28].copy_from_slice(&(c as u16).to_le_bytes());
        dot[20..22].copy_from_slice(&((c >> 16) as u16).to_le_bytes());
        self.data[off..off+32].copy_from_slice(&dot);
        let mut dotdot = [0u8;32];
        dotdot[0] = b'.';
        dotdot[1] = b'.';
        for i in 2..8 { dotdot[i] = b' '; }
        dotdot[11] = 0x10;
        dotdot[26..28].copy_from_slice(&(parent_cluster as u16).to_le_bytes());
        dotdot[20..22].copy_from_slice(&((parent_cluster >> 16) as u16).to_le_bytes());
        self.data[off+32..off+64].copy_from_slice(&dotdot);

        // add entry in parent
        let mut slot: Option<usize> = None;
        'outer: for cc in self.chain(parent_cluster) {
            let base = self.cluster_offset(cc);
            let sz = self.cluster_size();
            let mut i = 0;
            while i + 32 <= sz && base + i + 32 <= self.data.len() {
                if self.data[base+i] == 0 || self.data[base+i] == 0xE5 {
                    slot = Some(base + i);
                    break 'outer;
                }
                i += 32;
            }
        }
        let slot = slot.ok_or_else(|| StorageError::Unsupported("directory full".into()))?;
        let mut entry = [0u8;32];
        let upper = fname.to_uppercase();
        let mut base = [b' ';8];
        let b = upper.as_bytes();
        for i in 0..8.min(b.len()) { base[i] = b[i]; }
        entry[0..8].copy_from_slice(&base);
        entry[8] = b' ';
        entry[9] = b' ';
        entry[10] = b' ';
        entry[11] = 0x10;
        entry[26..28].copy_from_slice(&(c as u16).to_le_bytes());
        entry[20..22].copy_from_slice(&((c >> 16) as u16).to_le_bytes());
        self.data[slot..slot+32].copy_from_slice(&entry);
        Ok(())
    }
}

impl<'a> FileSystem for FatVolume<'a> {
    type File = Cursor<Vec<u8>>;

    fn open(&self, path: &str) -> Result<Self::File, StorageError> {
        let (_, is_dir, cluster, size) = self.resolve_path(path)?;
        if is_dir {
            return Err(StorageError::InvalidArg("is directory".into()));
        }
        let data = self.read_file_bytes(cluster, size);
        Ok(Cursor::new(data))
    }

    fn exists(&self, path: &str) -> bool {
        self.resolve_path(path).is_ok()
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, StorageError> {
        let (_, is_dir, cluster, _) = self.resolve_path(path)?;
        if !is_dir {
            return Err(StorageError::InvalidArg("not a directory".into()));
        }
        let entries = self.read_dir_entries(cluster);
        let mut out = Vec::new();
        for e in entries {
            if e[0] == 0 || e[0] == 0xE5 { continue; }
            if e[11] & 0x08 != 0 { continue; } // volume label
            // build name
            let mut name = String::from_utf8_lossy(&e[0..8]).trim_end_matches(|c| c == ' ' || c == '\0').to_string();
            let ext = String::from_utf8_lossy(&e[8..11]).trim_end_matches(|c| c == ' ' || c == '\0').to_string();
            if !ext.is_empty() {
                name.push('.');
                name.push_str(&ext);
            }
            let is_dir = e[11] & 0x10 != 0;
            if is_dir { name.push('/'); }
            out.push(name);
        }
        Ok(out)
    }

    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), StorageError> {
        self.write_file_path(path, data)
    }

    fn create_dir(&mut self, path: &str) -> Result<(), StorageError> {
        self.create_dir_path(path)
    }
}