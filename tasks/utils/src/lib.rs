use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

pub fn read_config<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: T = serde_yaml::from_str(&content)?;
    Ok(config)
}

