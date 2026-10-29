//! FUSE ISO mounting helper.

pub struct FuseIsoMount;

impl FuseIsoMount {
    pub fn build_mount_command(iso_path: &str, mount_point: &str) -> String {
        format!("fuseiso -p {} {}", iso_path, mount_point)
    }
}
