//! Contiguous/defragged loop device path manager.

pub struct LoopDeviceManager;

impl LoopDeviceManager {
    pub fn build_losetup_command(loop_dev: &str, image_file: &str, offset_bytes: u64) -> String {
        format!("losetup -o {} {} {}", offset_bytes, loop_dev, image_file)
    }
}
