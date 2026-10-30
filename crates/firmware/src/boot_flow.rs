//! UEFI Boot Flow management, environment persistence, and virtual device dispatch.

use multiverse_core::{EnvParamBlock, PlatformId, RamdiskParam, CoreError};
use multiverse_core::chunkmap::ChunkMap;
use crate::virtual_block::VirtualBlockDevice;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct UefiVariableStore {
    vars: BTreeMap<String, Vec<u8>>,
}

impl UefiVariableStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_env_param_block(&mut self, name: &str, block: &EnvParamBlock) -> Result<(), CoreError> {
        let json = serde_json::to_vec(block).map_err(|e| CoreError::InvalidArg(e.to_string()))?;
        self.vars.insert(name.to_string(), json);
        Ok(())
    }

    pub fn get_env_param_block(&self, name: &str) -> Result<Option<EnvParamBlock>, CoreError> {
        if let Some(bytes) = self.vars.get(name) {
            let block: EnvParamBlock = serde_json::from_slice(bytes).map_err(|e| CoreError::InvalidArg(e.to_string()))?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone)]
pub struct UefiBootFlowConfig {
    pub platform_id: PlatformId,
    pub fallback_enabled: bool,
    pub alternate_sector_size: Option<u32>,
}

pub type BootDispatchFn = Box<dyn Fn(&str) -> Result<(), CoreError> + Send + Sync>;

pub struct UefiBootFlowManager {
    config: UefiBootFlowConfig,
    env_block: EnvParamBlock,
    var_store: UefiVariableStore,
    virtual_device: Option<VirtualBlockDevice>,
    retries: u32,
    dispatch_hook: Option<BootDispatchFn>,
}

impl std::fmt::Debug for UefiBootFlowManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UefiBootFlowManager")
            .field("config", &self.config)
            .field("env_block", &self.env_block)
            .field("var_store", &self.var_store)
            .field("virtual_device", &self.virtual_device)
            .field("retries", &self.retries)
            .field("dispatch_hook", &self.dispatch_hook.is_some())
            .finish()
    }
}

impl UefiBootFlowManager {
    pub fn new(platform_id: PlatformId, chunk_map: ChunkMap) -> Self {
        let vdev = VirtualBlockDevice::new(chunk_map);
        let env_block = EnvParamBlock {
            image_location: Some(2048),
            chunk_count: 1,
            platform_id: Some(platform_id),
            ramdisk: None,
            flags: 0,
        };
        Self {
            config: UefiBootFlowConfig {
                platform_id,
                fallback_enabled: true,
                alternate_sector_size: None,
            },
            env_block,
            var_store: UefiVariableStore::new(),
            virtual_device: Some(vdev),
            retries: 0,
            dispatch_hook: None,
        }
    }

    pub fn with_dispatch_hook<F>(mut self, hook: F) -> Self
    where
        F: Fn(&str) -> Result<(), CoreError> + Send + Sync + 'static,
    {
        self.dispatch_hook = Some(Box::new(hook));
        self
    }

    pub fn with_ramdisk(mut self, ramdisk: RamdiskParam) -> Self {
        self.env_block.ramdisk = Some(ramdisk);
        self
    }

    pub fn env_block(&self) -> &EnvParamBlock {
        &self.env_block
    }

    pub fn var_store(&self) -> &UefiVariableStore {
        &self.var_store
    }

    pub fn virtual_device(&self) -> Option<&VirtualBlockDevice> {
        self.virtual_device.as_ref()
    }

    pub fn prepare_boot(&mut self) -> Result<(), CoreError> {
        if let Some(vdev) = &self.virtual_device {
            if vdev.block_size == 0 {
                return Err(CoreError::InvalidArg("invalid block size".into()));
            }
        }
        self.var_store.set_env_param_block("MULTIVERSE_ENV", &self.env_block)?;
        Ok(())
    }

    /// Simulate/Execute UEFI LoadImage and StartImage dispatch from virtual block device
    pub fn load_and_start_image(&self, bootloader_path: &str) -> Result<(), CoreError> {
        if self.virtual_device.is_none() {
            return Err(CoreError::NotFound("virtual block device not initialized".into()));
        }
        if bootloader_path.is_empty() {
            return Err(CoreError::InvalidArg("empty bootloader path".into()));
        }
        if let Some(hook) = &self.dispatch_hook {
            hook(bootloader_path)?;
        }
        Ok(())
    }

    /// Execute complete boot sequence with fallback retries and cleanup on return
    pub fn execute_boot_sequence(&mut self, bootloader_path: &str) -> Result<(), CoreError> {
        self.prepare_boot()?;

        loop {
            match self.load_and_start_image(bootloader_path) {
                Ok(()) => {
                    self.cleanup();
                    return Ok(());
                }
                Err(e) => {
                    if !self.handle_failure()? {
                        self.cleanup();
                        return Err(e);
                    }
                }
            }
        }
    }

    pub fn handle_failure(&mut self) -> Result<bool, CoreError> {
        if !self.config.fallback_enabled || self.retries >= 2 {
            return Ok(false);
        }
        self.retries += 1;
        if self.retries == 1 {
            // Retry 1: disable virtual device opts / flags
            self.env_block.flags |= 0x80000000;
        } else if self.retries == 2 {
            // Retry 2: alternate sector size
            self.config.alternate_sector_size = Some(2048);
        }
        self.var_store.set_env_param_block("MULTIVERSE_ENV", &self.env_block)?;
        Ok(true)
    }

    pub fn cleanup(&mut self) {
        self.virtual_device = None;
    }
}
