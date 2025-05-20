use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;
use anyhow::Context;

pub fn read_config<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T, anyhow::Error> {
    let content = fs::read_to_string(&path).context("Failed to read config file")?;
    let config: T = serde_yaml::from_str(&content).context("Failed to parse config file")?;
    Ok(config)
}

