//! Firmware abstraction layer: command line, virtual block I/O, disk location, memdisk, boot flow, bios, boot_fs, secure_boot.

pub mod bios;
pub mod boot_flow;
pub mod boot_fs;
pub mod cmdline;
pub mod disk_location;
pub mod memdisk;
pub mod platform;
pub mod secure_boot;
pub mod virtual_block;

pub use bios::GrubBiosConfig;
pub use boot_flow::{UefiBootFlowConfig, UefiBootFlowManager, UefiVariableStore};
pub use boot_fs::BootPartitionFs;
pub use cmdline::CommandLine;
pub use secure_boot::{SecureBootManager, SecureBootState};
pub use virtual_block::VirtualBlockDevice;