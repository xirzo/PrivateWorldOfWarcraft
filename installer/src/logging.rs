use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const LOG_FILENAME: &str = "wow_installer.log";

/// Minimal logger: appends timestamps to a log file next to the binary and
/// mirrors everything to stderr. Thread-safe.
pub struct Logger {
    file: Mutex<Option<fs::File>>,
}

static GLOBAL: OnceLock<Arc<Logger>> = OnceLock::new();
static VERBOSE: AtomicBool = AtomicBool::new(true);

impl Logger {
    pub fn global() -> &'static Arc<Logger> {
        GLOBAL.get_or_init(|| Arc::new(Logger::open()))
    }

    fn open() -> Logger {
        let path = log_path();
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .inspect_err(|_| eprintln!("warning: cannot open log file {path:?}"))
            .ok();
        Logger {
            file: Mutex::new(file),
        }
    }

    pub fn info(&self, msg: impl AsRef<str>) {
        let line = format!("[{}] {}", timestamp(), msg.as_ref());
        if VERBOSE.load(Ordering::Relaxed) {
            eprintln!("{line}");
        }
        if let Ok(mut guard) = self.file.lock()
            && let Some(f) = guard.as_mut()
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

pub fn log(msg: impl AsRef<str>) {
    Logger::global().info(msg);
}

fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}s")
}

/// Log file location: `<directory containing the binary>/wow_installer.log`.
pub fn log_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(LOG_FILENAME)))
        .unwrap_or_else(|| PathBuf::from(LOG_FILENAME))
}
