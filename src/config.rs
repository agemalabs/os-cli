//! CLI configuration — stored at ~/.config/os/config.toml

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// CLI configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_url: String,
    pub token: String,
    pub default_org: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:8080".to_string(),
            token: String::new(),
            default_org: "agema-labs".to_string(),
        }
    }
}

/// Path to the config file.
pub fn config_path() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("os");
    dir.join("config.toml")
}

/// Load config from disk, or create a default one.
pub fn load_or_init() -> Result<Config> {
    let path = config_path();

    if path.exists() {
        let contents = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    } else {
        let config = Config::default();
        save(&config)?;
        Ok(config)
    }
}

/// Save config to disk.
pub fn save(config: &Config) -> Result<()> {
    let path = config_path();

    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let contents = toml::to_string_pretty(config)?;
    fs::write(&path, contents)?;

    // Set permissions to 0600 (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_localhost_url() {
        let config = Config::default();
        assert_eq!(config.api_url, "http://localhost:8080");
        assert_eq!(config.default_org, "agema-labs");
    }

    #[test]
    fn config_serializes_to_toml() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("api_url"));
        assert!(toml_str.contains("localhost:8080"));
    }

    #[test]
    fn config_roundtrips_through_toml() {
        let config = Config {
            api_url: "https://api.os.agemalabs.com".into(),
            token: "test-token".into(),
            default_org: "agema-labs".into(),
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.api_url, config.api_url);
        assert_eq!(parsed.token, config.token);
    }
}
