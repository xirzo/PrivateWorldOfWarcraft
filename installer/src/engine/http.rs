use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use sha2::{Digest, Sha256};

use crate::engine::events::{DownloadEvent, ProgressCallback};
use crate::error::{Error, Result};

/// Install a ring-based TLS provider once. Called before any HTTP client is
/// built. Pure-Rust crypto keeps the binary static (no openssl/aws-lc).
pub fn ensure_tls_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn client() -> Result<reqwest::blocking::Client> {
    ensure_tls_provider();
    reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(Error::from)
}

/// Blocking download of `url` to `dest`, with progress and optional SHA-256
/// verification. The file is written to `dest.part` first and renamed on
/// success so a partial file is never mistaken for a complete one.
pub fn download(
    url: &str,
    dest: &Path,
    expected_sha256: Option<&str>,
    cancel: &AtomicBool,
    on_progress: &ProgressCallback<'_>,
) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }

    on_progress(DownloadEvent::Connecting);
    let resp = client()?.get(url).send()?;
    let status = resp.status();
    if !status.is_success() {
        return Err(Error::Msg(format!("server returned {status} for {url}")));
    }

    let total = resp.content_length();
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| url.to_string());
    on_progress(DownloadEvent::Metadata {
        name: name.clone(),
        total_bytes: total,
    });

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let part = dest.with_extension("part");
    let mut out = fs::File::create(&part)?;
    let mut reader = resp;

    let mut downloaded = 0u64;
    let mut buf = [0u8; 128 * 1024];
    let start = Instant::now();
    let mut last_report = Instant::now();

    loop {
        if cancel.load(Ordering::Relaxed) {
            drop(out);
            let _ = fs::remove_file(&part);
            return Err(Error::Cancelled);
        }
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        downloaded += n as u64;

        if last_report.elapsed().as_millis() >= 250 {
            let speed = downloaded as f64 / start.elapsed().as_secs_f64().max(0.001);
            on_progress(DownloadEvent::Progress {
                downloaded,
                total_bytes: total,
                speed_bps: speed as u64,
                peers: None,
            });
            last_report = Instant::now();
        }
    }
    drop(out);

    if let Some(expected) = expected_sha256 {
        verify_sha256(&part, expected)?;
    }

    fs::rename(&part, dest)?;
    on_progress(DownloadEvent::Done);
    Ok(())
}

/// Compute the SHA-256 of a file as a lowercase hex string.
pub fn sha256_hex(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 128 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let actual = sha256_hex(path)?;
    let expected = expected.trim().to_ascii_lowercase();
    if actual != expected {
        let _ = fs::remove_file(path);
        return Err(Error::ChecksumMismatch { expected, actual });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn hex_encoding() {
        assert_eq!(hex(&[0xde, 0xad]), "dead");
        assert_eq!(hex(&[]), "");
    }

    #[test]
    fn sha256_known_vector() {
        let dir = std::env::temp_dir().join(format!(
            "wow_installer_http_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.bin");
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(b"abc").unwrap();
        // sha256("abc")
        assert_eq!(
            sha256_hex(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
