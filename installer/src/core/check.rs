use std::fs;
use std::path::Path;

use crate::core::realmlist::installed_locales;
use crate::error::Result;

/// Rough uncompressed size of the full 3.3.5a client, used for
/// disk-space warnings when the exact figure is unknown.
pub const CLIENT_SIZE_GUESS_BYTES: u64 = 25 * 1024 * 1024 * 1024;

pub const WOW_EXE_NAME: &str = "WoW.exe";

/// True if a WoW executable already exists in `dir`.
pub fn has_wow_executable(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| {
        entries.filter_map(|e| e.ok()).any(|e| {
            e.file_name().to_string_lossy().to_ascii_lowercase()
                == WOW_EXE_NAME.to_ascii_lowercase()
        })
    })
}

/// Free bytes on the filesystem containing `path`.
///
/// Returns `Ok(None)` when the value can't be determined on this platform.
#[cfg(unix)]
pub fn free_space(path: &Path) -> Result<Option<u64>> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    #[repr(C)]
    struct Statvfs {
        f_bsize: u64,
        f_frsize: u64,
        f_blocks: u64,
        f_bfree: u64,
        f_bavail: u64,
        f_files: u64,
        f_ffree: u64,
        f_favail: u64,
        f_fsid: u64,
        f_flag: u64,
        f_namemax: u64,
    }
    unsafe extern "C" {
        fn statvfs(path: *const i8, buf: *mut Statvfs) -> i32;
    }
    let path = path.to_string_lossy();
    let c_path = CString::new(path.as_ref())
        .map_err(|_| crate::error::Error::Msg("path contains NUL byte".into()))?;
    let mut buf = MaybeUninit::<Statvfs>::uninit();
    // SAFETY: statvfs writes into a valid struct; path is a valid C string.
    let rc = unsafe { statvfs(c_path.as_ptr(), buf.as_mut_ptr()) };
    if rc != 0 {
        return Ok(None);
    }
    // SAFETY: rc == 0 means buf is initialized.
    let info = unsafe { buf.assume_init() };
    Ok(Some(info.f_bavail * info.f_frsize))
}

/// Free bytes on Windows.
#[cfg(windows)]
pub fn free_space(path: &Path) -> Result<Option<u64>> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    unsafe extern "system" {
        fn GetDiskFreeSpaceExW(
            lp_directory_name: *const u16,
            lp_free_bytes_available: *mut u64,
            _lp_total_number_of_bytes: *mut u64,
            _lp_total_number_of_free_bytes: *mut u64,
        ) -> i32;
    }

    let wide: Vec<u16> = OsStr::new(path.as_os_str())
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut avail = 0u64;
    let mut total = 0u64;
    let mut free = 0u64;
    // SAFETY: wide is NUL-terminated; pointers point to writable u64s.
    let rc = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, &mut total, &mut free) };
    if rc == 0 {
        return Ok(None);
    }
    Ok(Some(avail))
}

/// Non-unix/non-windows fallback.
#[cfg(not(any(unix, windows)))]
pub fn free_space(_path: &Path) -> Result<Option<u64>> {
    Ok(None)
}

/// Whether there is (probably) enough space for a fresh install.
pub fn enough_space_for_client(dir: &Path) -> Result<bool> {
    match free_space(dir)? {
        Some(free) => Ok(free >= CLIENT_SIZE_GUESS_BYTES),
        None => Ok(true),
    }
}

/// A snapshot of the state of a target install directory.
#[derive(Debug, Default)]
pub struct InstallState {
    pub has_client: bool,
    pub locales: Vec<String>,
    pub has_config: bool,
}

pub fn inspect(dir: &Path) -> InstallState {
    InstallState {
        has_client: has_wow_executable(dir),
        locales: installed_locales(dir),
        has_config: dir.join("WTF").join("Config.wtf").exists(),
    }
}

/// Suggested default install directory for the current OS.
pub fn default_install_dir() -> std::path::PathBuf {
    let base = directories::BaseDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("Games").join("WoW")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wow_exe_detection() {
        let dir = std::env::temp_dir().join(format!(
            "wow_installer_check_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        assert!(!has_wow_executable(&dir));
        fs::write(dir.join("WoW.exe"), b"").unwrap();
        assert!(has_wow_executable(&dir));
        assert!(!inspect(&dir).has_config);
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn free_space_works() {
        let free = free_space(Path::new("/tmp")).unwrap();
        assert!(free.is_some());
        assert!(free.unwrap() > 0);
    }
}
