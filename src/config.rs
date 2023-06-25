use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Symlink {
    pub path: String,
    pub link: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub metrics_file_path: String,
    pub metrics_dump_interval_ms: u64,
    pub tmpfs_volume_path: String,
    pub tmpfs_min_space_left_mb: u64,
    pub grace_period_seconds: u64,
    pub runner_binary_path: String,
    pub symlinks: Vec<Symlink>,
}

pub fn load_json(path: &str) -> Result<Config, String> {
    let content = match fs::read_to_string(path) {
        Ok(val) => val,
        Err(e) => {
            return Err(format!(
                "Failed to read config file. Reason - {}",
                e.to_string()
            ))
        }
    };

    let config: Config = match serde_json::from_str(&content) {
        Ok(val) => val,
        Err(e) => {
            return Err(format!(
                "Failed to parse config file. Reason - {}",
                e.to_string()
            ))
        }
    };

    Ok(config)
}
