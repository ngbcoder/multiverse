//! Linux runtime integration layer: init dispatcher, cpio extraction/injection, device-mapper, chain script, fuse, loop, winpe.

pub mod chain_script;
pub mod cpio;
pub mod fuse_iso;
pub mod loop_dev;
pub mod winpe;

pub use chain_script::ChainScriptContext;
pub use cpio::{CompressionFormat, CpioInjector};
pub use fuse_iso::FuseIsoMount;
pub use loop_dev::LoopDeviceManager;
pub use winpe::WinPeMount;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("init error: {0}")]
    Init(String),
    #[error("device-mapper error: {0}")]
    DeviceMapper(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitMode {
    Chain,
    Loop,
}

pub struct InitDispatcher;

impl InitDispatcher {
    pub fn dispatch(args: &[String]) -> InitMode {
        if args.iter().any(|arg| arg == "multiverse_loop=1" || arg == "init_loop") {
            InitMode::Loop
        } else {
            InitMode::Chain
        }
    }
}

/// Device-mapper table builder for exposing multi-sector image chunks as a block device.
#[derive(Debug, Clone)]
pub struct DeviceMapperTable {
    target_name: String,
    sectors: u64,
    target_type: String, // e.g. "linear" or "striped"
    params: String,
}

impl DeviceMapperTable {
    pub fn linear(name: &str, sectors: u64, underlying_dev: &str, offset: u64) -> Self {
        Self {
            target_name: name.to_string(),
            sectors,
            target_type: "linear".to_string(),
            params: format!("{} {}", underlying_dev, offset),
        }
    }

    pub fn striped(name: &str, sectors: u64, chunk_size: u32, devs: &[(&str, u64)]) -> Self {
        let mut params = format!("{} {}", devs.len(), chunk_size);
        for (dev, offset) in devs {
            params.push_str(&format!(" {} {}", dev, offset));
        }
        Self {
            target_name: name.to_string(),
            sectors,
            target_type: "striped".to_string(),
            params,
        }
    }

    pub fn to_table_string(&self) -> String {
        format!("0 {} {} {}", self.sectors, self.target_type, self.params)
    }
}
