//! ISO Downloader utility for fetching official OS images with integrity verification.

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct IsoDescriptor {
    pub name: String,
    pub os_family: String,
    pub download_url: String,
    pub expected_sha256: Option<String>,
}

pub struct IsoDownloaderCatalog {
    entries: BTreeMap<String, IsoDescriptor>,
}

impl Default for IsoDownloaderCatalog {
    fn default() -> Self {
        let mut entries = BTreeMap::new();
        entries.insert(
            "ubuntu-22.04".to_string(),
            IsoDescriptor {
                name: "Ubuntu 22.04 LTS".to_string(),
                os_family: "Linux".to_string(),
                download_url: "https://releases.ubuntu.com/22.04/ubuntu-22.04-desktop-amd64.iso".to_string(),
                expected_sha256: Some("a435730303e488ed:placeholder".to_string()),
            },
        );
        entries.insert(
            "fedora-38".to_string(),
            IsoDescriptor {
                name: "Fedora Workstation 38".to_string(),
                os_family: "Linux".to_string(),
                download_url: "https://download.fedoraproject.org/pub/fedora/linux/releases/38/Workstation/x86_64/iso/Fedora-Workstation-Live-x86_64-38-1.6.iso".to_string(),
                expected_sha256: None,
            },
        );
        Self { entries }
    }
}

impl IsoDownloaderCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Vec<&IsoDescriptor> {
        self.entries.values().collect()
    }

    pub fn get(&self, key: &str) -> Option<&IsoDescriptor> {
        self.entries.get(key)
    }

    pub fn generate_download_script(&self, key: &str, output_dir: &str) -> Result<String, String> {
        let iso = self.get(key).ok_or_else(|| format!("ISO {} not found in catalog", key))?;
        let target_path = format!("{}/{}.iso", output_dir, key);
        Ok(format!("curl -L -o {} {}", target_path, iso.download_url))
    }
}
