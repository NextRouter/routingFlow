use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NicConfig {
    pub lan: String,
    pub wan0: String,
    pub wan1: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StatusResponse {
    pub config: NicConfig,
    pub mappings: HashMap<String, String>,
}

impl StatusResponse {
    /// Get WAN interface for a given IP address
    /// If IP is not in mappings, return wan0 as default
    pub fn get_wan_for_ip(&self, ip: &str) -> String {
        self.mappings
            .get(ip)
            .cloned()
            .unwrap_or_else(|| "wan0".to_string())
    }

    /// Get actual NIC name for a WAN identifier (e.g., "wan0" -> "eth0")
    pub fn get_nic_for_wan(&self, wan: &str) -> Option<String> {
        match wan {
            "wan0" => Some(self.config.wan0.clone()),
            "wan1" => Some(self.config.wan1.clone()),
            _ => None,
        }
    }
}

pub struct Config {
    pub prometheus_url: String,
    pub status_url: String,
    pub nic_config: NicConfig,
}

impl Config {
    pub fn load() -> Result<Self> {
        let nic_config_str = fs::read_to_string("nic.json").context("Failed to read nic.json")?;
        let nic_config: NicConfig =
            serde_json::from_str(&nic_config_str).context("Failed to parse nic.json")?;

        Ok(Config {
            prometheus_url: "http://localhost:9090".to_string(),
            status_url: "http://localhost:32599/status".to_string(),
            nic_config,
        })
    }

    /// Get list of WAN interfaces
    pub fn get_wan_list(&self) -> Vec<String> {
        vec!["wan0".to_string(), "wan1".to_string()]
    }

    /// Get NIC name for a WAN identifier
    pub fn get_nic_for_wan(&self, wan: &str) -> Option<String> {
        match wan {
            "wan0" => Some(self.nic_config.wan0.clone()),
            "wan1" => Some(self.nic_config.wan1.clone()),
            _ => None,
        }
    }
}
