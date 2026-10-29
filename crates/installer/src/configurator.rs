//! Local HTTP Configurator Server for live config.json editing and backup.

use multiverse_bootcore::config::Config;

pub struct ConfiguratorServer {
    pub port: u16,
    pub current_config: Config,
}

impl ConfiguratorServer {
    pub fn new(port: u16, current_config: Config) -> Self {
        Self { port, current_config }
    }

    pub fn handle_update_json(&mut self, json_str: &str) -> Result<String, String> {
        let updated = Config::from_json(json_str).map_err(|e| format!("Invalid JSON config: {}", e))?;
        self.current_config = updated;
        self.current_config.to_json().map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn generate_backup_filename(&self) -> String {
        format!("config.json.bak.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
    }
}
