use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub clean: CleanConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    #[serde(default = "default_min_size")]
    pub min_size_bytes: u64,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default)]
    pub excluded_paths: Vec<String>,
    #[serde(default)]
    pub scan_paths: Vec<String>,
}

fn default_min_size() -> u64 {
    1024 * 1024
}

fn default_max_depth() -> usize {
    3
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            min_size_bytes: default_min_size(),
            max_depth: default_max_depth(),
            excluded_paths: Vec::new(),
            scan_paths: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanConfig {
    #[serde(default)]
    pub dry_run_by_default: bool,
    #[serde(default = "default_true")]
    pub log_history: bool,
    #[serde(default)]
    pub confirm_before_clean: bool,
}

fn default_true() -> bool {
    true
}

impl Default for CleanConfig {
    fn default() -> Self {
        Self {
            dry_run_by_default: true,
            log_history: true,
            confirm_before_clean: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default)]
    pub show_sizes_in_bytes: bool,
    #[serde(default = "default_true")]
    pub color_output: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_sizes_in_bytes: false,
            color_output: true,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cleanx")
            .join("config.toml")
    }

    pub fn add_excluded_path(&mut self, path: String) {
        if !self.scan.excluded_paths.contains(&path) {
            self.scan.excluded_paths.push(path);
        }
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cleanx")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan: ScanConfig::default(),
            clean: CleanConfig::default(),
            ui: UiConfig::default(),
        }
    }
}
