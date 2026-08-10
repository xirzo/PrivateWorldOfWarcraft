use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Result;

pub const REALMLIST_FILENAME: &str = "realmlist.wtf";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    /// An existing `set realmlist` line was replaced.
    Replaced,
    /// No line existed; one was appended.
    Appended,
    /// The value was already correct; nothing changed.
    Unchanged,
}

/// Replace or append `set realmlist <addr>` in a realmlist file, idempotently.
///
/// Other lines (comments, `set realmname`, ...) are preserved.
pub fn set_realmlist(path: &Path, addr: &str) -> Result<EditKind> {
    let target = format!("set realmlist {addr}");
    let mut found = None;

    let mut lines = if path.exists() {
        let content = fs::read_to_string(path)?;
        let lines: Vec<String> = content.lines().map(str::to_string).collect();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.to_ascii_lowercase().starts_with("set realmlist") {
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
            write_lines(path, &lines)?;
            Ok(EditKind::Replaced)
        }
        None => {
            lines.push(target.clone());
            write_lines(path, &lines)?;
            Ok(EditKind::Appended)
        }
    }
}

fn write_lines(path: &Path, lines: &[String]) -> Result<()> {
    let mut content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Read the current address from a realmlist file, if present.
pub fn read_realmlist(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("set realmlist") {
            let addr = &trimmed["set realmlist".len()..];
            let addr = addr.trim();
            if !addr.is_empty() {
                return Ok(Some(addr.to_string()));
            }
        }
    }
    Ok(None)
}

/// Collect every `realmlist.wtf` in the client: `Data/<locale>/realmlist.wtf`
/// for each installed locale, plus a root-level one if it exists.
pub fn realmlist_paths(install_dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();

    let data_dir = install_dir.join("Data");
    if let Ok(entries) = fs::read_dir(&data_dir) {
        let mut locales: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        locales.sort();
        for locale_dir in locales {
            let candidate = locale_dir.join(REALMLIST_FILENAME);
            if candidate.exists() {
                paths.push(candidate);
            }
        }
    }

    let root = install_dir.join(REALMLIST_FILENAME);
    if root.exists() && !paths.contains(&root) {
        paths.push(root);
    }

    paths
}

/// Write the given server into every known realmlist file.
pub fn set_realmlist_all(install_dir: &Path, addr: &str) -> Result<Vec<(PathBuf, EditKind)>> {
    let paths = realmlist_paths(install_dir);
    let mut results = Vec::new();
    for path in paths {
        let kind = set_realmlist(&path, addr)?;
        results.push((path, kind));
    }
    Ok(results)
}

/// Find locale directories present under `install_dir/Data`.
pub fn installed_locales(install_dir: &Path) -> Vec<String> {
    let data_dir = install_dir.join("Data");
    let Ok(entries) = fs::read_dir(&data_dir) else {
        return Vec::new();
    };
    let mut locales: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    locales.sort();
    locales
}

pub fn ensure_realmlist_file(install_dir: &Path, locale: &str, addr: &str) -> Result<PathBuf> {
    let path = install_dir
        .join("Data")
        .join(locale)
        .join(REALMLIST_FILENAME);
    set_realmlist(&path, addr)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "wow_installer_test_{}_{}",
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
    fn appends_when_missing() {
        let dir = tmpdir();
        let path = dir.join("realmlist.wtf");
        assert_eq!(
            set_realmlist(&path, "127.0.0.1").unwrap(),
            EditKind::Appended
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "set realmlist 127.0.0.1\n"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replaces_existing() {
        let dir = tmpdir();
        let path = dir.join("realmlist.wtf");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "# my settings").unwrap();
        writeln!(f, "set realmlist old.server.com").unwrap();
        writeln!(f, "set realmname \"Test\"").unwrap();

        assert_eq!(
            set_realmlist(&path, "127.0.0.1").unwrap(),
            EditKind::Replaced
        );
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("set realmlist 127.0.0.1"));
        assert!(!content.contains("old.server.com"));
        assert!(content.contains("# my settings"));
        assert!(content.contains("set realmname"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn idempotent_unchanged() {
        let dir = tmpdir();
        let path = dir.join("realmlist.wtf");
        fs::write(&path, "set realmlist 1.2.3.4\n").unwrap();
        assert_eq!(
            set_realmlist(&path, "1.2.3.4").unwrap(),
            EditKind::Unchanged
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn case_insensitive_match() {
        let dir = tmpdir();
        let path = dir.join("realmlist.wtf");
        fs::write(&path, "SET Realmlist old.com\n").unwrap();
        set_realmlist(&path, "new.com").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("set realmlist new.com"));
        assert!(!content.contains("old.com"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn read_returns_addr() {
        let dir = tmpdir();
        let path = dir.join("realmlist.wtf");
        assert_eq!(read_realmlist(&path).unwrap(), None);
        fs::write(&path, "set realmlist 5.6.7.8\n").unwrap();
        assert_eq!(read_realmlist(&path).unwrap(), Some("5.6.7.8".to_string()));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn collects_per_locale() {
        let dir = tmpdir();
        let en = dir.join("Data/enUS");
        let ru = dir.join("Data/ruRU");
        fs::create_dir_all(&en).unwrap();
        fs::create_dir_all(&ru).unwrap();
        fs::write(en.join("realmlist.wtf"), "set realmlist a\n").unwrap();
        fs::write(ru.join("realmlist.wtf"), "set realmlist b\n").unwrap();

        let paths = realmlist_paths(&dir);
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|p| p.ends_with("enUS/realmlist.wtf")));
        assert!(paths.iter().any(|p| p.ends_with("ruRU/realmlist.wtf")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn installed_locales_scans_data() {
        let dir = tmpdir();
        fs::create_dir_all(dir.join("Data/enUS")).unwrap();
        fs::create_dir_all(dir.join("Data/ruRU")).unwrap();
        fs::create_dir_all(dir.join("Data")).unwrap();
        assert_eq!(
            installed_locales(&dir),
            vec!["enUS".to_string(), "ruRU".to_string()]
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
