/// Events emitted by download engines (HTTP + torrent).
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    /// Connecting / resolving / acquiring metadata.
    Connecting,
    /// Metadata is known: file name and total size (None when unknown).
    Metadata {
        name: String,
        total_bytes: Option<u64>,
    },
    /// Download progress update.
    Progress {
        downloaded: u64,
        total_bytes: Option<u64>,
        speed_bps: u64,
        /// Peer count (torrent only).
        peers: Option<u32>,
    },
    /// Download finished successfully.
    Done,
}

impl DownloadEvent {
    /// 0.0–1.0 completion, or `None` when total size is unknown.
    pub fn fraction(&self) -> Option<f64> {
        match self {
            DownloadEvent::Progress {
                downloaded,
                total_bytes: Some(total),
                ..
            } if *total > 0 => Some((*downloaded as f64) / (*total as f64)),
            DownloadEvent::Done => Some(1.0),
            _ => None,
        }
    }
}

/// Callback signature used by the download engines.
pub type ProgressCallback<'a> = dyn Fn(DownloadEvent) + Send + Sync + 'a;
