//! Chain script execution: OS detection, OS-specific hooks, persistence, dud disks, and switch-root execution.

use crate::RuntimeError;

#[derive(Debug, Clone)]
pub struct ChainScriptContext {
    pub os_name: String,
    pub hook_path: String,
    pub persistence_dev: Option<String>,
    pub dud_disks: Vec<String>,
}

impl ChainScriptContext {
    pub fn new(os_name: &str) -> Self {
        Self {
            os_name: os_name.to_string(),
            hook_path: format!("hook/{}/os-hook.sh", os_name),
            persistence_dev: None,
            dud_disks: Vec::new(),
        }
    }

    pub fn with_persistence(mut self, dev: &str) -> Self {
        self.persistence_dev = Some(dev.to_string());
        self
    }

    pub fn with_dud_disks(mut self, dud_disks: Vec<String>) -> Self {
        self.dud_disks = dud_disks;
        self
    }

    pub fn generate_script(&self) -> String {
        let mut script = String::new();
        script.push_str("#!/bin/sh\n");
        script.push_str(&format!("# Multiverse Chain Script for {}\n", self.os_name));
        script.push_str(&format!("if [ -f {} ]; then\n  . {}\nfi\n", self.hook_path, self.hook_path));
        if let Some(pdev) = &self.persistence_dev {
            script.push_str(&format!("export PERSISTENCE_DEV={}\n", pdev));
        }
        for (idx, dud) in self.dud_disks.iter().enumerate() {
            script.push_str(&format!("export DUD_DISK_{}={}\n", idx, dud));
        }
        script.push_str("exec switch_root /newroot /sbin/init\n");
        script
    }
}
