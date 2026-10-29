//! OS boot adapters for different operating systems and image formats.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OsType {
    Linux,
    Windows,
    Unix,
    ChromeOS,
    VmwareXen,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootParams {
    pub os_type: OsType,
    pub kernel_path: Option<String>,
    pub initrd_path: Option<String>,
    pub initrds: Vec<String>,
    pub cmdline: String,
    pub extra_args: Vec<String>,
    pub persistence: bool,
    pub dud_disks: Vec<String>,
    pub auto_install_file: Option<String>,
}

pub trait OsAdapter {
    fn os_type(&self) -> OsType;
    fn prepare_boot_params(&self, image_path: &str) -> Result<BootParams, String>;
}

#[derive(Debug, Clone, Default)]
pub struct LinuxAdapter;

impl OsAdapter for LinuxAdapter {
    fn os_type(&self) -> OsType {
        OsType::Linux
    }

    fn prepare_boot_params(&self, image_path: &str) -> Result<BootParams, String> {
        let kernel = if image_path.contains("ubuntu") || image_path.contains("debian") || image_path.contains("fedora") {
            Some("/boot/vmlinuz".to_string())
        } else {
            Some("/kernel".to_string())
        };
        let initrd = Some("/boot/initrd.img".to_string());
        let cmdline = "quiet splash rdinit=/init multiverse_chain=1".to_string();
        Ok(BootParams {
            os_type: OsType::Linux,
            kernel_path: kernel,
            initrd_path: initrd.clone(),
            initrds: vec![initrd.unwrap_or_default()],
            cmdline,
            extra_args: vec![],
            persistence: true,
            dud_disks: vec![],
            auto_install_file: None,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct WindowsAdapter;

impl OsAdapter for WindowsAdapter {
    fn os_type(&self) -> OsType {
        OsType::Windows
    }

    fn prepare_boot_params(&self, image_path: &str) -> Result<BootParams, String> {
        Ok(BootParams {
            os_type: OsType::Windows,
            kernel_path: Some("/bootmgr".to_string()),
            initrd_path: Some("/sources/boot.wim".to_string()),
            initrds: vec!["/sources/boot.wim".to_string()],
            cmdline: "wimboot".to_string(),
            extra_args: vec![format!("image={}", image_path)],
            persistence: false,
            dud_disks: vec![],
            auto_install_file: Some("/autounattend.xml".to_string()),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct UnixAdapter;

impl OsAdapter for UnixAdapter {
    fn os_type(&self) -> OsType {
        OsType::Unix
    }

    fn prepare_boot_params(&self, image_path: &str) -> Result<BootParams, String> {
        Ok(BootParams {
            os_type: OsType::Unix,
            kernel_path: Some("/boot/kernel/kernel".to_string()),
            initrd_path: None,
            initrds: vec![],
            cmdline: "autoboot".to_string(),
            extra_args: vec![format!("image={}", image_path)],
            persistence: false,
            dud_disks: vec![],
            auto_install_file: None,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChromeOSAdapter;

impl OsAdapter for ChromeOSAdapter {
    fn os_type(&self) -> OsType {
        OsType::ChromeOS
    }

    fn prepare_boot_params(&self, image_path: &str) -> Result<BootParams, String> {
        Ok(BootParams {
            os_type: OsType::ChromeOS,
            kernel_path: Some("/syslinux/vmlinuz.A".to_string()),
            initrd_path: Some("/syslinux/initrd.isb".to_string()),
            initrds: vec!["/syslinux/initrd.isb".to_string()],
            cmdline: "cros_secure console=tty2 init=/sbin/init".to_string(),
            extra_args: vec![format!("image={}", image_path)],
            persistence: false,
            dud_disks: vec![],
            auto_install_file: None,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct VmwareXenAdapter;

impl OsAdapter for VmwareXenAdapter {
    fn os_type(&self) -> OsType {
        OsType::VmwareXen
    }

    fn prepare_boot_params(&self, image_path: &str) -> Result<BootParams, String> {
        Ok(BootParams {
            os_type: OsType::VmwareXen,
            kernel_path: Some("/mboot.c32".to_string()),
            initrd_path: None,
            initrds: vec![],
            cmdline: "hypervisor".to_string(),
            extra_args: vec![format!("image={}", image_path)],
            persistence: false,
            dud_disks: vec![],
            auto_install_file: None,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct GenericAdapter;

impl OsAdapter for GenericAdapter {
    fn os_type(&self) -> OsType {
        OsType::Generic
    }

    fn prepare_boot_params(&self, image_path: &str) -> Result<BootParams, String> {
        Ok(BootParams {
            os_type: OsType::Generic,
            kernel_path: None,
            initrd_path: None,
            initrds: vec![],
            cmdline: "memdisk".to_string(),
            extra_args: vec![format!("image={}", image_path)],
            persistence: false,
            dud_disks: vec![],
            auto_install_file: None,
        })
    }
}

pub fn get_adapter(image_path: &str) -> Box<dyn OsAdapter> {
    let lower = image_path.to_lowercase();
    if lower.contains("win") || lower.ends_with(".wim") || lower.ends_with(".vhd") {
        Box::new(WindowsAdapter)
    } else if lower.contains("bsd") || lower.contains("solaris") {
        Box::new(UnixAdapter)
    } else if lower.contains("chrome") || lower.contains("chromium") {
        Box::new(ChromeOSAdapter)
    } else if lower.contains("vmware") || lower.contains("esxi") || lower.contains("xen") {
        Box::new(VmwareXenAdapter)
    } else if lower.contains("ubuntu") || lower.contains("debian") || lower.contains("fedora") || lower.contains("arch") || lower.ends_with(".iso") {
        Box::new(LinuxAdapter)
    } else {
        Box::new(GenericAdapter)
    }
}
