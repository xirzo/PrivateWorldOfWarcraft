//! Steam detection and desktop integration.
//!
//! Integration is deliberately guided: we detect Steam, help add the game as
//! a non-Steam game and create desktop shortcuts. We never hand-edit Steam's
//! `shortcuts.vdf` — it is brittle and locked while Steam runs.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub const GAME_NAME: &str = "WoW 3.3.5a";

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

/// Newest installed Proton runtime under a Steam root, if any.
#[cfg(target_os = "linux")]
fn find_proton(steam_root: &Path) -> Option<PathBuf> {
    let common = steam_root.join("steamapps/common");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&common)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with("Proton"))
                .unwrap_or(false)
                && p.join("proton").exists()
        })
        .collect();
    dirs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    dirs.into_iter().next()
}

/// Create a desktop shortcut that launches WoW through the given executable.
///
/// Windows: a `.lnk` on the Desktop pointing at `WoW.exe`.
/// Linux: a `.desktop` entry on the Desktop that runs the game via Steam's
/// newest Proton (or falls back to opening the install folder / Steam).
///
/// Returns the path of the created shortcut.
pub fn create_desktop_shortcut(install_dir: &Path, exe_path: &Path) -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let user_profile = std::env::var("USERPROFILE")
            .map_err(|_| Error::Msg("USERPROFILE is not set".to_string()))?;
        let desktop = PathBuf::from(user_profile).join("Desktop");
        let lnk = desktop.join(format!("{GAME_NAME}.lnk"));
        if !desktop.is_dir() {
            return Err(Error::Msg(format!(
                "desktop directory does not exist: {}",
                desktop.display()
            )));
        }

        let target = exe_path.to_string_lossy();
        let workdir = install_dir.to_string_lossy();
        let ps = format!(
            "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}'); \
             $s.TargetPath='{}'; $s.WorkingDirectory='{}'; \
             $s.Description='{}'; $s.Save()",
            esc_ps(&lnk.to_string_lossy()),
            esc_ps(&target),
            esc_ps(&workdir),
            esc_ps(GAME_NAME),
        );
        run_quiet(std::process::Command::new("powershell").args(["-NoProfile", "-Command", &ps]))?;
        Ok(lnk)
    }

    #[cfg(target_os = "linux")]
    {
        let desktop_dir = linux_desktop_dir();
        if !desktop_dir.is_dir() {
            std::fs::create_dir_all(&desktop_dir)?;
        }
        let desktop_file = desktop_dir.join(format!("{}.desktop", GAME_NAME.replace(' ', "-")));

        let exec = match detect_steam_root() {
            Some(steam_root) => match find_proton(&steam_root) {
                Some(proton) => {
                    let prefix = install_dir.join("SteamCompat");
                    format!(
                        "env STEAM_COMPAT_CLIENT_INSTALL_PATH={} STEAM_COMPAT_DATA_PATH={} {} run {}",
                        quote_desktop(&steam_root.to_string_lossy()),
                        quote_desktop(&prefix.to_string_lossy()),
                        quote_desktop(&proton.to_string_lossy()),
                        quote_desktop(&exe_path.to_string_lossy()),
                    )
                }
                None => "steam steam://open/games".to_string(),
            },
            None => format!("xdg-open {}", quote_desktop(&install_dir.to_string_lossy())),
        };

        std::fs::write(&desktop_file, desktop_entry_contents(&exec))?;
        set_executable(&desktop_file);
        Ok(desktop_file)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (install_dir, exe_path);
        Err(Error::Msg(
            "desktop shortcuts are not supported on this platform".to_string(),
        ))
    }
}

#[cfg(target_os = "linux")]
fn linux_desktop_dir() -> PathBuf {
    if let Ok(out) = std::process::Command::new("xdg-user-dir")
        .arg("DESKTOP")
        .output()
        && let Ok(text) = String::from_utf8(out.stdout)
        && let dir = text.trim()
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }
    home().map(|h| h.join("Desktop")).unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(mut perms) = std::fs::metadata(path).map(|m| m.permissions()) {
        perms.set_mode(0o755);
        let _ = std::fs::set_permissions(path, perms);
    }
}

/// Contents of a Linux `.desktop` launcher (pure function, unit-tested).
fn desktop_entry_contents(exec: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={GAME_NAME}\n\
         Comment=World of Warcraft 3.3.5a (WotLK) private server\n\
         Exec={exec}\n\
         Terminal=false\n\
         Categories=Game;\n"
    )
}

#[cfg(target_os = "windows")]
fn esc_ps(s: &str) -> String {
    s.replace('\'', "''")
}

fn quote_desktop(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$");
    format!("\"{escaped}\"")
}

#[cfg(target_os = "windows")]
fn run_quiet(cmd: &mut std::process::Command) -> Result<()> {
    let status = cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| {
            Error::Msg(format!(
                "failed to run {}: {e}",
                cmd.get_program().to_string_lossy()
            ))
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Msg(format!(
            "{} exited with {status}",
            cmd.get_program().to_string_lossy()
        )))
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

/// Open the Steam Library page where non-Steam games are added. Fire and
/// forget — returns immediately once the command was spawned.
pub fn open_steam_games() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let _ = run_quiet(std::process::Command::new("cmd").args([
            "/C",
            "start",
            "",
            "steam://open/games",
        ]));
    }

    #[cfg(target_os = "linux")]
    {
        let root = detect_steam_root();
        let flatpak = root
            .as_deref()
            .map(|r| r.to_string_lossy().contains("com.valvesoftware.Steam"))
            .unwrap_or(false);
        let mut cmd = if flatpak {
            let mut c = std::process::Command::new("flatpak");
            c.args(["run", "com.valvesoftware.Steam"]);
            c
        } else {
            std::process::Command::new("steam")
        };
        cmd.arg("steam://open/games");
        cmd.spawn()
            .map_err(|e| Error::Msg(format!("failed to launch Steam: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_entry_contains_exec_and_name() {
        let entry = desktop_entry_contents("env FOO=\"a b\" \"proton\" run \"/x/WoW.exe\"");
        assert!(entry.starts_with("[Desktop Entry]\n"));
        assert!(entry.contains("Name=WoW 3.3.5a"));
        assert!(entry.contains("Exec=env FOO=\"a b\" \"proton\" run \"/x/WoW.exe\"\n"));
        assert!(entry.contains("Categories=Game;"));
    }

    #[test]
    fn desktop_quotes_paths() {
        assert_eq!(
            quote_desktop(r#"C:\Games\My WoW" dir"#),
            "\"C:\\\\Games\\\\My WoW\\\" dir\""
        );
        assert_eq!(quote_desktop("/home/u/My WoW"), "\"/home/u/My WoW\"");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn ps_escaping() {
        assert_eq!(esc_ps("it's here"), "it''s here");
    }

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
