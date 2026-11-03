//! Firmware abstraction layer: command line, virtual block I/O, disk location, memdisk, boot flow, bios, boot_fs, secure_boot.

pub mod bios;
pub mod block_device;
pub mod block_io;
pub mod blockio;
pub mod boot_flow;
pub mod boot_fs;
pub mod cmdline;
pub mod disk_location;
pub mod fs_driver;
pub mod iso9660;
pub mod iso_filesystem;
pub mod memdisk;
pub mod platform;
pub mod secure_boot;
pub mod uefi_block_io;
pub mod uefi_blockio;
pub mod uefi_fs_driver;
pub mod virtual_block;
pub mod virtual_block_io;

pub use bios::GrubBiosConfig;
pub use boot_flow::{UefiBootFlowConfig, UefiBootFlowManager, UefiVariableStore};
pub use boot_fs::BootPartitionFs;
pub use cmdline::CommandLine;
pub use secure_boot::{SecureBootManager, SecureBootState};
pub use uefi_blockio::*;
pub use uefi_fs_driver::*;
pub use virtual_block::VirtualBlockDevice;
pub use virtual_block::BlockIoMedia;
pub use virtual_block::BlockIoMedia as BlockMedia;
pub use virtual_block::BlockIoMedia as VirtualBlockMedia;