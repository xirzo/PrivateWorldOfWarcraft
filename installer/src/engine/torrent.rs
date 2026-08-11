use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::engine::events::{DownloadEvent, ProgressCallback};
use crate::error::{Error, Result};
use librqbit::{AddTorrent, AddTorrentOptions, Session, SessionOptions};

/// How long to keep trying to resolve torrent metadata (name/size) before
/// giving up. Metadata needs outbound peer/tracker traffic; on networks that
/// block BitTorrent this never succeeds, and rqbit's peer stream never ends on
/// its own, so without a timeout the wizard would hang on "Downloading…". The
/// caller usually falls back to an HTTP mirror once this fires.
const METADATA_TIMEOUT: Duration = Duration::from_secs(60);
/// How often to report "still connecting" while metadata is resolving.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
/// How long the initial file-integrity check may take after metadata arrives.
const INIT_TIMEOUT: Duration = Duration::from_secs(60);

pub struct TorrentOptions<'a> {
    pub magnet: &'a str,
    pub save_dir: &'a Path,
    pub cancel: &'a AtomicBool,
    pub on_progress: &'a ProgressCallback<'a>,
}

/// Download a magnet link to `save_dir`, blocking until complete.
///
/// Returns the path of the downloaded file (single-file torrents download to
/// `save_dir/<name>`). Progress is reported through `on_progress`; the caller
/// may set `cancel` at any point to abort with `Error::Cancelled`.
pub fn download_torrent(opts: TorrentOptions) -> Result<PathBuf> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::Msg(format!("failed to start tokio runtime: {e}")))?;
    rt.block_on(download_torrent_async(opts))
}

async fn download_torrent_async(opts: TorrentOptions<'_>) -> Result<PathBuf> {
    let (magnet, save_dir, cancel, on_progress) =
        (opts.magnet, opts.save_dir, opts.cancel, opts.on_progress);

    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }

    on_progress(DownloadEvent::Connecting);
    std::fs::create_dir_all(save_dir)?;

    // Listen on a TCP port so peers can connect to us (not just outbound).
    let session = Session::new_with_opts(
        save_dir.to_path_buf(),
        SessionOptions {
            listen_port_range: Some(6881..6890),
            enable_upnp_port_forwarding: true,
            ..Default::default()
        },
    )
    .await
    .map_err(|e| Error::Msg(format!("failed to create torrent session: {e}")))?;

    // Resolving magnet metadata can take a while (DHT bootstrap, tracker
    // round-trips) or never finish when outbound peer traffic is blocked.
    // Report heartbeats while we wait and bail with a clear error on timeout
    // instead of hanging the wizard forever.
    let handle = add_torrent_with_progress(&session, magnet, cancel, on_progress).await?;

    // Wait for the initial file-integrity check with a generous timeout.
    tokio::time::timeout(INIT_TIMEOUT, handle.wait_until_initialized())
        .await
        .map_err(|_| {
            Error::Msg("torrent metadata resolved but the client file check timed out".to_string())
        })?
        .map_err(|e| Error::Msg(format!("failed to fetch torrent metadata: {e}")))?;

    let name = handle.name().unwrap_or_else(|| "client".to_string());
    let total = handle
        .with_metadata(|m| m.file_infos.iter().map(|f| f.len).sum::<u64>())
        .unwrap_or(0);
    on_progress(DownloadEvent::Metadata {
        name: name.clone(),
        total_bytes: Some(total),
    });

    loop {
        if cancel.load(Ordering::Relaxed) {
            session.stop().await;
            return Err(Error::Cancelled);
        }

        let stats = handle.stats();
        if stats.finished {
            break;
        }

        let speed_bps = stats
            .live
            .as_ref()
            .map(|l| (l.download_speed.mbps * 1_048_576.0) as u64)
            .unwrap_or(0);
        let peers = stats.live.as_ref().map(|l| {
            let p = &l.snapshot.peer_stats;
            (p.queued + p.connecting + p.live) as u32
        });

        on_progress(DownloadEvent::Progress {
            downloaded: stats.progress_bytes,
            total_bytes: Some(stats.total_bytes),
            speed_bps,
            peers,
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    handle
        .wait_until_completed()
        .await
        .map_err(|e| Error::Msg(format!("torrent did not finish cleanly: {e}")))?;

    let rel = handle
        .with_metadata(|m| m.file_infos.first().map(|f| f.relative_filename.clone()))
        .map_err(|e| Error::Msg(format!("torrent metadata unavailable: {e}")))?
        .ok_or_else(|| Error::Msg("torrent contains no files".to_string()))?;

    let path = save_dir.join(rel);
    session.stop().await;

    if !path.exists() {
        return Err(Error::Msg(format!(
            "downloaded file not found at {}",
            path.display()
        )));
    }

    on_progress(DownloadEvent::Done);
    Ok(path)
}

/// Add a magnet torrent to the session and wait for its metadata to resolve.
///
/// While metadata is pending, reports periodic heartbeat progress events and
/// checks the cancel flag. On timeout, stops the session and returns a
/// descriptive error instead of blocking forever.
async fn add_torrent_with_progress(
    session: &Arc<Session>,
    magnet: &str,
    cancel: &AtomicBool,
    on_progress: &ProgressCallback<'_>,
) -> Result<Arc<librqbit::ManagedTorrent>> {
    let add = session.add_torrent(
        AddTorrent::from_url(magnet),
        Some(AddTorrentOptions {
            overwrite: true,
            ..Default::default()
        }),
    );
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tokio::pin!(add);

    loop {
        tokio::select! {
            resp = &mut add => {
                match resp {
                    Ok(r) => match r.into_handle() {
                        Some(handle) => return Ok(handle),
                        None => {
                            session.stop().await;
                            return Err(Error::Msg(
                                "torrent session added the torrent in list-only mode".to_string(),
                            ));
                        }
                    },
                    Err(e) => {
                        session.stop().await;
                        return Err(Error::Msg(format!("failed to add torrent: {e}")));
                    }
                }
            }
            _ = tokio::time::sleep(METADATA_TIMEOUT) => {
                session.stop().await;
                return Err(Error::Msg(
                    "could not fetch torrent metadata (no peers/trackers reachable). \
                     BitTorrent traffic may be blocked by the network, Windows Firewall \
                     or antivirus. Trying the HTTP fallback mirror."
                        .to_string(),
                ));
            }
            _ = heartbeat.tick() => {
                if cancel.load(Ordering::Relaxed) {
                    session.stop().await;
                    return Err(Error::Cancelled);
                }
                on_progress(DownloadEvent::Progress {
                    downloaded: 0,
                    total_bytes: None,
                    speed_bps: 0,
                    peers: Some(0),
                });
            }
        }
    }
}
