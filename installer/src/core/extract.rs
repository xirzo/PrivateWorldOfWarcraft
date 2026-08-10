use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use zip::read::ZipArchive;

use crate::error::{Error, Result};

/// Progress reported during extraction.
#[derive(Debug, Clone)]
pub struct ExtractProgress {
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub current_file: String,
}

impl ExtractProgress {
    pub fn fraction(&self) -> f64 {
        if self.bytes_total == 0 {
            1.0
        } else {
            (self.bytes_done as f64) / (self.bytes_total as f64)
        }
    }
}

/// Extract a zip archive into `dest`.
///
/// - If every entry shares a single top-level folder (e.g. `ChromieCraft/...`),
///   that folder is stripped so the game files land directly in `dest`.
/// - `cancel` is checked before each entry; when set, `Error::Cancelled` is
///   returned and partial output remains for the caller to clean up.
/// - Zip-slip is prevented: absolute paths and `..` segments are rejected.
pub fn extract_zip(
    zip_path: &Path,
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(&ExtractProgress),
) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    let entries: Vec<usize> = (0..archive.len()).collect();
    let bytes_total: u64 = entries
        .iter()
        .map(|&i| {
            archive
                .by_index(i)
                .map(|e| if e.is_dir() { 0 } else { e.size() })
                .unwrap_or(0)
        })
        .sum();

    let strip = detect_top_level_folder(&archive)?;

    let mut files_done = 0u64;
    let mut bytes_done = 0u64;

    for &i in &entries {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }

        let mut entry = archive.by_index(i)?;
        let Some(out_path) = safe_out_path(entry.name(), dest, strip)? else {
            files_done += 1;
            continue;
        };

        on_progress(&ExtractProgress {
            files_done,
            files_total: entries.len() as u64,
            bytes_done,
            bytes_total,
            current_file: entry.name().to_string(),
        });

        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            files_done += 1;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out = fs::File::create(&out_path)?;
        let mut buf = [0u8; 128 * 1024];
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            bytes_done += n as u64;
        }
        files_done += 1;
    }

    on_progress(&ExtractProgress {
        files_done,
        files_total: entries.len() as u64,
        bytes_done,
        bytes_total,
        current_file: String::new(),
    });
    Ok(())
}

/// Merge only entries under `prefix/` of a patch zip into `dest/<prefix>/`.
///
/// Used to apply localization patches: `Data/ruRU/...` is merged into the
/// client's `Data/ruRU/...` (overwriting existing files).
pub fn merge_zip_prefix(
    zip_path: &Path,
    dest: &Path,
    prefix: &str,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(&ExtractProgress),
) -> Result<u64> {
    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    let entries: Vec<usize> = (0..archive.len())
        .filter(|&i| {
            archive
                .by_index(i)
                .map(|e| e.is_file() && e.name().starts_with(prefix.trim_end_matches('/')))
                .unwrap_or(false)
        })
        .collect();

    let bytes_total: u64 = entries
        .iter()
        .map(|&i| archive.by_index(i).map(|e| e.size()).unwrap_or(0))
        .sum();

    let mut bytes_done = 0u64;
    let mut written = 0u64;

    for (files_done, &i) in entries.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let mut entry = archive.by_index(i)?;
        let rel = relative_to_prefix(entry.name(), prefix);
        let out_path = dest.join(&rel);

        on_progress(&ExtractProgress {
            files_done: files_done as u64,
            files_total: entries.len() as u64,
            bytes_done,
            bytes_total,
            current_file: rel.to_string_lossy().to_string(),
        });

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&out_path)?;
        let mut buf = [0u8; 128 * 1024];
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            bytes_done += n as u64;
        }
        written += 1;
    }
    Ok(written)
}

/// Detect a single shared top-level folder across all entries.
fn detect_top_level_folder<R: Read + std::io::Seek>(archive: &ZipArchive<R>) -> Result<bool> {
    let mut top: Option<&str> = None;
    for name in archive.file_names() {
        let name = name.trim_matches('/');
        if name.is_empty() {
            continue;
        }
        let first = name.split('/').next().unwrap_or("");
        if first.is_empty() {
            continue;
        }
        match top {
            None => top = Some(first),
            Some(t) if t != first => return Ok(false),
            _ => {}
        }
    }
    Ok(top.is_some())
}

/// Map an archive entry name to a safe output path under `dest`.
///
/// Returns `None` for entries that should be skipped (e.g. `..` escapes).
fn safe_out_path(name: &str, dest: &Path, strip_top: bool) -> Result<Option<PathBuf>> {
    let mut parts: Vec<&str> = name
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();

    for p in &parts {
        if *p == ".." || p.contains('\\') {
            return Err(Error::Msg(format!("unsafe zip entry: {name}")));
        }
    }

    if strip_top && !parts.is_empty() {
        parts.remove(0);
    }

    let mut out = PathBuf::from(dest);
    for p in parts {
        if p == ".." {
            return Err(Error::Msg(format!("unsafe zip entry: {name}")));
        }
        // Ensure we never escape `dest`, even via odd components.
        out.push(p);
        let normalized = out.components().any(|c| c == Component::ParentDir);
        if normalized {
            return Err(Error::Msg(format!("unsafe zip entry: {name}")));
        }
    }
    Ok(Some(out))
}

fn relative_to_prefix(name: &str, prefix: &str) -> PathBuf {
    let prefix = prefix.trim_end_matches('/');
    let mut rel = PathBuf::new();
    for p in name.split('/') {
        if p == prefix {
            continue;
        }
        rel.push(p);
    }
    rel
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn make_zip(files: &[(&str, &[u8])]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "wow_installer_zip_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.zip");

        let file = fs::File::create(&path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in files {
            writer.start_file(*name, opts).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
        path
    }

    #[test]
    fn strips_single_top_folder() {
        let zip = make_zip(&[
            ("ChromieCraft/WoW.exe", b"exe"),
            ("ChromieCraft/Data/realmlist", b"rl"),
        ]);
        let dest = zip.parent().unwrap().join("dest");
        let cancel = AtomicBool::new(false);
        extract_zip(&zip, &dest, &cancel, &mut |_| {}).unwrap();
        assert!(dest.join("WoW.exe").exists());
        assert!(dest.join("Data/realmlist").exists());
        assert!(!dest.join("ChromieCraft").exists());
        fs::remove_dir_all(zip.parent().unwrap()).unwrap();
    }

    #[test]
    fn flat_zip_no_strip() {
        let zip = make_zip(&[("WoW.exe", b"exe"), ("Data/enUS/realmlist.wtf", b"rl")]);
        let dest = zip.parent().unwrap().join("dest2");
        let cancel = AtomicBool::new(false);
        extract_zip(&zip, &dest, &cancel, &mut |_| {}).unwrap();
        assert!(dest.join("WoW.exe").exists());
        assert!(dest.join("Data/enUS/realmlist.wtf").exists());
        fs::remove_dir_all(zip.parent().unwrap()).unwrap();
    }

    #[test]
    fn zip_slip_rejected() {
        let zip = make_zip(&[("../evil.txt", b"x")]);
        let dest = zip.parent().unwrap().join("dest3");
        let cancel = AtomicBool::new(false);
        assert!(extract_zip(&zip, &dest, &cancel, &mut |_| {}).is_err());
        assert!(!dest.exists() || !dest.join("evil.txt").exists());
        fs::remove_dir_all(zip.parent().unwrap()).unwrap();
    }

    #[test]
    fn progress_reaches_100_percent() {
        let zip = make_zip(&[("a.bin", &vec![0u8; 1000]), ("b.bin", &vec![1u8; 2000])]);
        let dest = zip.parent().unwrap().join("dest4");
        let cancel = AtomicBool::new(false);
        let mut last = 0.0f64;
        extract_zip(&zip, &dest, &cancel, &mut |p| last = p.fraction()).unwrap();
        assert_eq!(last, 1.0);
        fs::remove_dir_all(zip.parent().unwrap()).unwrap();
    }

    #[test]
    fn cancel_aborts() {
        let zip = make_zip(&[("a.bin", &vec![0u8; 1000])]);
        let dest = zip.parent().unwrap().join("dest5");
        let cancel = AtomicBool::new(true);
        let err = extract_zip(&zip, &dest, &cancel, &mut |_| {});
        assert!(matches!(err, Err(Error::Cancelled)));
        fs::remove_dir_all(zip.parent().unwrap()).unwrap();
    }

    #[test]
    fn merge_prefix_only_copies_matching() {
        let zip = make_zip(&[
            ("Data/ruRU/ruRU.MPQ", b"mpq"),
            ("Data/ruRU/realmlist.wtf", b"rl"),
            ("Data/enUS/enUS.MPQ", b"other"),
            ("WoW.exe", b"exe"),
        ]);
        let dest = zip.parent().unwrap().join("dest6");
        fs::create_dir_all(&dest).unwrap();
        let cancel = AtomicBool::new(false);
        let n = merge_zip_prefix(&zip, &dest, "Data/ruRU", &cancel, &mut |_| {}).unwrap();
        assert_eq!(n, 2);
        assert!(dest.join("Data/ruRU/ruRU.MPQ").exists());
        assert!(dest.join("Data/ruRU/realmlist.wtf").exists());
        assert!(!dest.join("Data/enUS/enUS.MPQ").exists());
        fs::remove_dir_all(zip.parent().unwrap()).unwrap();
    }

    #[test]
    fn safe_out_path_rejects_escape() {
        assert!(safe_out_path("../x", Path::new("/tmp"), false).is_err());
        assert!(safe_out_path("a/../../x", Path::new("/tmp"), false).is_err());
        assert!(safe_out_path("a/b.txt", Path::new("/tmp"), false).is_ok());
    }
}
