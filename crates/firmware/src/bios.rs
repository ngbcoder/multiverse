//! Legacy BIOS path: GRUB i386-pc integration, module loading, memdisk support.

#[derive(Debug, Clone)]
pub struct GrubBiosConfig {
    pub modules: Vec<String>,
    pub memdisk_enabled: bool,
    pub prefix: String,
}

impl Default for GrubBiosConfig {
    fn default() -> Self {
        Self {
            modules: vec![
                "part_msdos".to_string(),
                "fat".to_string(),
                "ext2".to_string(),
                "iso9660".to_string(),
                "memdisk".to_string(),
                "search".to_string(),
            ],
            memdisk_enabled: true,
            prefix: "(hd0,msdos1)/boot/grub".to_string(),
        }
    }
}

impl GrubBiosConfig {
    pub fn generate_script(&self) -> String {
        let mut script = String::new();
        script.push_str(&format!("set prefix={}\n", self.prefix));
        if self.memdisk_enabled {
            script.push_str("insmod memdisk\n");
        }
        for m in &self.modules {
            script.push_str(&format!("insmod {}\n", m));
        }
        script
    }
}
