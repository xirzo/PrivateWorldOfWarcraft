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
///
/// Google Drive share links are handled specially (see `drive_file_id` and
/// friends): the public page URL is resolved through the `uc?export=download`
/// endpoint, and files over ~100 MB require a second request carrying the
/// virus-scan confirmation token from the intermediate HTML page.
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

    let is_html = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/html"))
        .unwrap_or(false);

    let resp = if is_drive_url(url) && is_html {
        // Drive served the virus-scan confirmation page instead of the file.
        let html = resp.text()?;
        let token = drive_confirm_token(&html).ok_or_else(|| {
            Error::Msg(
                "Google Drive confirmation page did not include a download token".to_string(),
            )
        })?;
        let id = drive_file_id(url).ok_or_else(|| {
            Error::Msg(format!(
                "could not extract file id from Google Drive URL: {url}"
            ))
        })?;
        let mut dl_url = format!(
            "https://drive.usercontent.google.com/download?id={id}&export=download&confirm={token}"
        );
        if let Some(uuid) = drive_uuid(&html) {
            dl_url.push_str(&format!("&uuid={uuid}"));
        }
        let resp = client()?.get(&dl_url).send()?;
        let status = resp.status();
        if !status.is_success() {
            return Err(Error::Msg(format!(
                "Google Drive download returned {status}"
            )));
        }
        resp
    } else {
        resp
    };

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

/// Whether a URL points at Google Drive (needs the special download flow).
fn is_drive_url(url: &str) -> bool {
    url.contains("drive.google.com") || url.contains("drive.usercontent.google.com")
}

/// Extract the file ID from common Google Drive share URLs:
/// `/file/d/<ID>/view`, `?id=<ID>`, `uc?export=download&id=<ID>`.
pub fn drive_file_id(url: &str) -> Option<String> {
    if let Some(query) = url.split('?').nth(1)
        && let Some(id) = query.split('&').find_map(|p| p.strip_prefix("id="))
        && !id.is_empty()
    {
        return Some(id.to_string());
    }
    if let Some(i) = url.find("/file/d/") {
        let rest = &url[i + "/file/d/".len()..];
        let id = rest.split(['/', '?']).next()?;
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    None
}

/// The virus-scan confirmation token from Drive's intermediate HTML page.
fn drive_confirm_token(html: &str) -> Option<String> {
    extract_hidden_attr(html, "confirm")
}

/// The (optional) uuid field from the same page.
fn drive_uuid(html: &str) -> Option<String> {
    extract_hidden_attr(html, "uuid")
}

/// Read the value of `<input type="hidden" name="<name>" value="<value>">`.
fn extract_hidden_attr(html: &str, name: &str) -> Option<String> {
    let pat = format!("name=\"{name}\" value=\"");
    let start = html.find(&pat)? + pat.len();
    let end = html[start..].find('"')?;
    Some(html[start..start + end].to_string())
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

    #[test]
    fn drive_file_id_variants() {
        assert_eq!(
            drive_file_id("https://drive.google.com/file/d/AbC123xyz/view?usp=sharing").as_deref(),
            Some("AbC123xyz")
        );
        assert_eq!(
            drive_file_id("https://drive.google.com/open?id=AbC123xyz").as_deref(),
            Some("AbC123xyz")
        );
        assert_eq!(
            drive_file_id(
                "https://drive.usercontent.google.com/download?id=AbC123xyz&export=download"
            )
            .as_deref(),
            Some("AbC123xyz")
        );
        assert_eq!(drive_file_id("https://example.com/x.zip"), None);
        assert!(!is_drive_url("https://example.com/x.zip"));
        assert!(is_drive_url("https://drive.google.com/file/d/x/view"));
    }

    #[test]
    fn drive_confirm_page_token_extraction() {
        let html = "<html><body><form action=\"https://drive.usercontent.google.com/download\">\
            <input type=\"hidden\" name=\"id\" value=\"AbC123xyz\">\
            <input type=\"hidden\" name=\"export\" value=\"download\">\
            <input type=\"hidden\" name=\"confirm\" value=\"t3kEN123\">\
            <input type=\"hidden\" name=\"uuid\" value=\"xyz-456\">\
            </form></body></html>";
        assert_eq!(drive_confirm_token(html).as_deref(), Some("t3kEN123"));
        assert_eq!(drive_uuid(html).as_deref(), Some("xyz-456"));
        assert_eq!(drive_confirm_token("<html>no form here</html>"), None);
    }
}
