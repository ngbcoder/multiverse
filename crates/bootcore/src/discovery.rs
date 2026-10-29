use multiverse_storage::filesystem::FileSystem;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ImageMeta {
    pub path: String,
    pub size: u64,
    pub fragments: u32,
    pub extension: String,
    pub os_hint: Option<String>,
}

const IMAGE_EXTS: &[&str] = &["iso", "wim", "img", "vhd", "vhdx", "efi"];

pub fn is_image(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn ext_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// Walk a filesystem (via FileSystem trait) collecting image files.
/// `root` is the starting directory path. `ignore` patterns are simple suffix/prefix? We'll do substring ignore.
pub fn discover_images<F: FileSystem>(
    fs: &F,
    root: &str,
    ignore: &[String],
) -> Vec<ImageMeta> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_string()];
    while let Some(dir) = stack.pop() {
        let entries = match fs.read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries {
            // entry may be "path" or "path/" for dirs; our InMemoryFs returns files only in read_dir.
            // We'll treat entries without trailing slash as files.
            let full = if entry.starts_with('/') {
                entry.clone()
            } else {
                format!("{}/{}", dir.trim_end_matches('/'), entry)
            };
            if ignore.iter().any(|ig| full.contains(ig)) {
                continue;
            }
            // Determine if dir: our read_dir returns file names; dirs unknown. Try open.
            if fs.exists(&format!("{}/", full.trim_end_matches('/'))) && !is_image(&full) {
                // heuristic: if path has no extension, treat as dir
                if Path::new(&full).extension().is_none() {
                    stack.push(full.clone());
                    continue;
                }
            }
            if is_image(&full) {
                let size = fs.file_size(&full).unwrap_or(0);
                out.push(ImageMeta {
                    path: full.clone(),
                    size,
                    fragments: 0,
                    extension: ext_of(&full),
                    os_hint: None,
                });
            }
        }
    }
    out
}

/// Filter images by whitelist/blacklist globs (simple substring match).
pub fn apply_image_list(images: &mut Vec<ImageMeta>, cfg: &crate::config::ImageListConfig) {
    if let Some(wl) = &cfg.whitelist {
        images.retain(|img| wl.iter().any(|p| img.path.contains(p)));
    }
    if let Some(bl) = &cfg.blacklist {
        images.retain(|img| !bl.iter().any(|p| img.path.contains(p)));
    }
}