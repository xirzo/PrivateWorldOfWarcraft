use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::realmlist::EditKind;
use crate::error::{Error, Result};

pub const CONFIG_WTF: &str = "Config.wtf";

/// One entry in the embedded locale registry (`locales.json`).
///
/// A locale with `url == None` ships with the base client (e.g. enUS) and
/// needs no patch. Others are downloaded, checksum-verified and merged into
/// `Data/<id>/` before the in-game locale is switched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleSpec {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    pub set_locale: String,
}

pub const EMBEDDED_LOCALES_JSON: &str = include_str!("../../locales.json");

/// Load the locale registry embedded in the binary.
pub fn registry() -> Result<BTreeMap<String, LocaleSpec>> {
    let map: BTreeMap<String, LocaleSpec> = serde_json::from_str(EMBEDDED_LOCALES_JSON)
        .map_err(|e| Error::Msg(format!("invalid embedded locales.json: {e}")))?;
    Ok(map)
}

/// Locale spec, or an error naming what's wrong.
#[allow(dead_code)]
pub fn spec(id: &str) -> Result<LocaleSpec> {
    let reg = registry()?;
    reg.get(id)
        .cloned()
        .ok_or_else(|| Error::InvalidLocale(id.to_string()))
}

/// Locales that have a downloadable patch (used to populate the UI).
pub fn downloadable_locales() -> Result<Vec<(String, LocaleSpec)>> {
    Ok(registry()?
        .into_iter()
        .filter(|(_, spec)| spec.url.is_some())
        .collect())
}

pub fn config_path(install_dir: &Path) -> PathBuf {
    install_dir.join("WTF").join(CONFIG_WTF)
}

/// Set `SET locale "<locale>"` in a Config.wtf file, idempotently.
/// Other settings are preserved.
pub fn set_locale(config_path: &Path, locale: &str) -> Result<EditKind> {
    let target = format!("SET locale \"{locale}\"");
    let mut found = None;

    let mut lines = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        let lines: Vec<String> = content.lines().map(str::to_string).collect();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("set locale") || lower.starts_with("locale") {
                found = Some(i);
                break;
            }
        }
        lines
    } else {
        Vec::new()
    };

    match found {
        Some(i) => {
            if lines[i].trim() == target {
                return Ok(EditKind::Unchanged);
            }
            lines[i] = target.clone();
        }
        None => lines.push(target.clone()),
    }

    let mut content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(config_path, content)?;
    Ok(if found.is_some() {
        EditKind::Replaced
    } else {
        EditKind::Appended
    })
}

/// Read the current in-game locale from a Config.wtf file.
pub fn get_locale(config_path: &Path) -> Result<Option<String>> {
    if !config_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(config_path)?;
    for line in content.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        let body = if lower.starts_with("set locale") {
            &trimmed["set locale".len()..]
        } else if lower.starts_with("locale") {
            &trimmed["locale".len()..]
        } else {
            continue;
        };
        let body = body.trim().trim_matches('"');
        if !body.is_empty() {
            return Ok(Some(body.to_string()));
        }
    }
    Ok(None)
}

/// Set the in-game locale for an install directory (`<dir>/WTF/Config.wtf`).
pub fn set_locale_for_install(install_dir: &Path, locale: &str) -> Result<EditKind> {
    set_locale(&config_path(install_dir), locale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "wow_installer_locale_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn replaces_locale_line() {
        let dir = tmpdir();
        let path = dir.join("Config.wtf");
        fs::write(&path, "SET locale \"enUS\"\nSET foo \"bar\"\n").unwrap();
        assert_eq!(set_locale(&path, "ruRU").unwrap(), EditKind::Replaced);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("SET locale \"ruRU\""));
        assert!(!content.contains("enUS"));
        assert!(content.contains("SET foo"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn appends_when_missing() {
        let dir = tmpdir();
        let path = dir.join("Config.wtf");
        fs::write(&path, "SET foo \"bar\"\n").unwrap();
        assert_eq!(set_locale(&path, "ruRU").unwrap(), EditKind::Appended);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("SET locale \"ruRU\""));
        assert!(content.contains("SET foo"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn creates_file_when_absent() {
        let dir = tmpdir();
        let path = dir.join("WTF/Config.wtf");
        assert_eq!(set_locale(&path, "ruRU").unwrap(), EditKind::Appended);
        assert_eq!(fs::read_to_string(&path).unwrap(), "SET locale \"ruRU\"\n");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn idempotent() {
        let dir = tmpdir();
        let path = dir.join("Config.wtf");
        fs::write(&path, "SET locale \"ruRU\"\n").unwrap();
        assert_eq!(set_locale(&path, "ruRU").unwrap(), EditKind::Unchanged);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn get_and_set_roundtrip() {
        let dir = tmpdir();
        let path = dir.join("Config.wtf");
        assert_eq!(get_locale(&path).unwrap(), None);
        fs::write(&path, "SET locale \"enUS\"\n").unwrap();
        assert_eq!(get_locale(&path).unwrap(), Some("enUS".to_string()));
        set_locale(&path, "ruRU").unwrap();
        assert_eq!(get_locale(&path).unwrap(), Some("ruRU".to_string()));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn embedded_registry_valid() {
        let reg = registry().unwrap();
        assert!(reg.contains_key("enUS"));
        assert!(reg.contains_key("ruRU"));
        let en = spec("enUS").unwrap();
        assert_eq!(en.set_locale, "enUS");
        assert!(en.url.is_none());
    }
}
