use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::engine::events::{DownloadEvent, ProgressCallback};
use crate::error::{Error, Result};
use librqbit::{AddTorrent, AddTorrentOptions, Session};

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

    let session = Session::new(save_dir.to_path_buf())
        .await
        .map_err(|e| Error::Msg(format!("failed to create torrent session: {e}")))?;

    let handle = match session
        .add_torrent(
            AddTorrent::from_url(magnet),
            Some(AddTorrentOptions {
                overwrite: true,
                ..Default::default()
            }),
        )
        .await
    {
        Ok(resp) => resp.into_handle().ok_or_else(|| {
            Error::Msg("torrent session added the torrent in list-only mode".to_string())
        })?,
        Err(e) => {
            session.stop().await;
            return Err(Error::Msg(format!("failed to add torrent: {e}")));
        }
    };

    // Wait for metadata so we can report the file name and total size.
    handle
        .wait_until_initialized()
        .await
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
