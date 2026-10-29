use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControlConfig {
    pub menu_mode: Option<String>,
    pub auto_boot_delay: Option<u32>,
    pub timeout: Option<u32>,
    pub scroll_speed: Option<u32>,
    pub display: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub colors: Option<BTreeMap<String, String>>,
    pub fonts: Option<Vec<String>>,
    pub background: Option<String>,
    pub layout: Option<String>,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub control: Option<ControlConfig>,
    pub theme: Option<ThemeConfig>,
    pub menu_alias: Option<BTreeMap<String, String>>,
    pub menu_tip: Option<BTreeMap<String, String>>,
    pub menu_class: Option<BTreeMap<String, String>>,
    pub menu_extra: Option<Vec<String>>,
    pub image_list: Option<ImageListConfig>,
    pub auto_install: Option<Vec<String>>,
    pub injection: Option<Vec<String>>,
    pub persistence: Option<Vec<String>>,
    pub conf_replace: Option<BTreeMap<String, String>>,
    pub dud: Option<Vec<String>>,
    pub password: Option<PasswordConfig>,
    pub vhdboot: Option<serde_json::Value>,
    pub vdisk_boot: Option<serde_json::Value>,
    pub memdisk: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageListConfig {
    pub whitelist: Option<Vec<String>>,
    pub blacklist: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PasswordConfig {
    pub global: Option<String>,
    pub per_image: Option<BTreeMap<String, String>>,
}

impl Config {
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}