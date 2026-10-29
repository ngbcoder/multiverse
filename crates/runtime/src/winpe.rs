//! Windows PE helper (imdisk mount configuration).

pub struct WinPeMount;

impl WinPeMount {
    pub fn build_imdisk_command(drive_letter: char, image_path: &str) -> String {
        format!("imdisk -a -t file -f {} -m {}:", image_path, drive_letter.to_ascii_uppercase())
    }
}
