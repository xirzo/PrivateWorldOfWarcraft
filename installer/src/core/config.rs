use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::server::Server;
use crate::error::{Error, Result};

pub const CONFIG_FILE_NAME: &str = "wow_installer.toml";

/// Settings persisted between runs (last install dir, chosen server, ...).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub install_dir: Option<PathBuf>,
    pub server: Server,
    #[serde(default)]
    pub locales: Vec<String>,
}

impl AppConfig {
    pub fn path() -> Option<PathBuf> {
        let proj = directories::ProjectDirs::from("com", "dadsmmo", "wow-installer")?;
        Some(proj.config_dir().join(CONFIG_FILE_NAME))
    }

    pub fn load() -> Self {
        match Self::path() {
            Some(p) if p.exists() => match fs::read_to_string(&p) {
                Ok(s) => toml::from_str(&s).unwrap_or_default(),
                Err(_) => AppConfig::default(),
            },
            _ => AppConfig::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        let Some(dir) = path.parent() else {
            return Ok(());
        };
        fs::create_dir_all(dir)?;
        let s = toml::to_string_pretty(self)
            .map_err(|e| Error::Msg(format!("failed to serialize config: {e}")))?;
        fs::write(path, s)?;
        Ok(())
    }

    /// Install directory to use, falling back to the per-OS default.
    pub fn resolved_install_dir(&self) -> PathBuf {
        self.install_dir
            .clone()
            .unwrap_or_else(crate::core::check::default_install_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut cfg = AppConfig::default();
        cfg.install_dir = Some(PathBuf::from("/tmp/wow"));
        cfg.server = Server::parse("play.server.com:8085").unwrap();
        cfg.locales = vec!["enUS".to_string(), "ruRU".to_string()];
        let s = toml::to_string_pretty(&cfg).unwrap();
        let back: AppConfig = toml::from_str(&s).unwrap();
        assert_eq!(back.install_dir, cfg.install_dir);
        assert_eq!(back.server, cfg.server);
        assert_eq!(back.locales, cfg.locales);
    }

    #[test]
    fn default_is_local_server() {
        let cfg = AppConfig::default();
        assert!(cfg.server.is_local());
        assert_eq!(cfg.server.realmlist_value(), "127.0.0.1");
    }
}
