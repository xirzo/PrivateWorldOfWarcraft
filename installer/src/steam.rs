//! Steam detection.
//!
//! Integration is deliberately guided: we only detect Steam so the wizard
//! can show instructions. We never hand-edit Steam's `shortcuts.vdf` — it is
//! brittle and locked while Steam runs.

use std::path::{Path, PathBuf};

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn has_steam_markers(root: &Path) -> bool {
    root.join("steam.sh").exists() || root.join("steamapps").is_dir()
}

/// Locate the Steam install root, if any.
///
/// Linux: `~/.local/share/Steam`, `~/.steam/steam` and the Flatpak install.
/// Windows: the registry (`HKCU\Software\Valve\Steam` SteamPath /
/// `HKLM\SOFTWARE\WOW6432Node\Valve\Steam` InstallPath) plus the default
/// `C:\Program Files (x86)\Steam` fallback.
pub fn detect_steam_root() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let mut candidates = Vec::new();
        if let Some(home) = home() {
            candidates.push(home.join(".local/share/Steam"));
            candidates.push(home.join(".steam/steam"));
            candidates.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
        }
        for c in candidates {
            if has_steam_markers(&c) {
                return Some(c);
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    {
        let mut candidates = Vec::new();
        for (key, value) in [
            ("HKCU\\Software\\Valve\\Steam", "SteamPath"),
            ("HKLM\\SOFTWARE\\WOW6432Node\\Valve\\Steam", "InstallPath"),
        ] {
            if let Ok(out) = std::process::Command::new("reg")
                .args(["query", key, "/v", value])
                .output()
            {
                if let Ok(text) = String::from_utf8(out.stdout) {
                    if let Some(dir) = text.lines().filter_map(parse_reg_value).next() {
                        candidates.push(PathBuf::from(dir.trim_matches('"')));
                    }
                }
            }
        }
        candidates.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
        for c in candidates {
            if c.is_dir() {
                return Some(c);
            }
        }
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = home;
        None
    }
}

/// Extract the value after the `REG_*` type token from a `reg query` line.
///
/// `    SteamPath    REG_SZ    C:\Program Files (x86)\Steam` →
/// `Some("C:\\Program Files (x86)\\Steam")`.
// The function is used on Windows only, but its unit test runs everywhere.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn parse_reg_value(line: &str) -> Option<String> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let idx = tokens.iter().position(|t| t.starts_with("REG_"))?;
    let value = tokens[idx + 1..].join(" ");
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_registry_parse_is_robust() {
        let sample = "HKEY_CURRENT_USER\\Software\\Valve\\Steam\n    SteamPath    REG_SZ    C:\\Program Files (x86)\\Steam\n";
        assert_eq!(
            parse_reg_value(&sample).as_deref(),
            Some("C:\\Program Files (x86)\\Steam")
        );
        let multi = "    InstallPath    REG_SZ    D:\\Steam Library\n";
        assert_eq!(parse_reg_value(multi).as_deref(), Some("D:\\Steam Library"));
        assert_eq!(parse_reg_value("    empty    REG_SZ    \n"), None);
    }
}
