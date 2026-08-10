use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::extract::{ExtractProgress, extract_zip};
use crate::core::locale;
use crate::core::realmlist;
use crate::core::server::Server;
use crate::engine::events::DownloadEvent;
use crate::engine::http;
use crate::error::{Error, Result};

/// High-level progress events produced while applying configuration.
#[derive(Debug, Clone)]
pub enum FlowEvent {
    Step(String),
    DownloadLocale {
        locale: String,
        event: DownloadEvent,
    },
    ExtractingLocale {
        locale: String,
    },
    LocaleApplied {
        locale: String,
    },
    RealmlistApplied {
        locale: String,
    },
}

/// Progress callback used across the install flow.
pub type FlowCallback<'a> = dyn Fn(&FlowEvent) + Send + Sync + 'a;

pub struct ApplyConfig<'a> {
    pub install_dir: &'a Path,
    pub server: &'a Server,
    pub locales: &'a [String],
    pub cancel: &'a AtomicBool,
    pub temp_dir: &'a Path,
}

/// Apply server + localization configuration to an existing client.
///
/// For each chosen locale:
/// - download the patch (if the locale registry provides one), verify its
///   SHA-256 and merge `Data/<locale>` into the client,
/// - set `SET locale "<locale>"` in `WTF/Config.wtf`,
/// - ensure `Data/<locale>/realmlist.wtf` points at the chosen server.
///
/// Finally every other known realmlist file is pointed at the server too.
pub fn apply_config(opts: ApplyConfig<'_>, on: &FlowCallback<'_>) -> Result<()> {
    let registry = locale::registry()?;
    let addr = opts.server.realmlist_value();

    on(&FlowEvent::Step(format!(
        "configuring {} locale(s) for {}",
        opts.locales.len(),
        addr
    )));

    for id in opts.locales {
        if cancelled(opts.cancel) {
            return Err(Error::Cancelled);
        }
        let spec = registry
            .get(id)
            .cloned()
            .ok_or_else(|| Error::InvalidLocale(id.clone()))?;

        if let Some(url) = &spec.url {
            let patch = opts.temp_dir.join(format!("{id}.zip"));
            on(&FlowEvent::DownloadLocale {
                locale: id.clone(),
                event: DownloadEvent::Connecting,
            });
            http::download(url, &patch, spec.sha256.as_deref(), opts.cancel, &|e| {
                on(&FlowEvent::DownloadLocale {
                    locale: id.clone(),
                    event: e,
                })
            })?;

            on(&FlowEvent::ExtractingLocale { locale: id.clone() });
            let prefix = format!("Data/{id}");
            crate::core::extract::merge_zip_prefix(
                &patch,
                opts.install_dir,
                &prefix,
                opts.cancel,
                &mut |_| {},
            )?;
        }

        locale::set_locale_for_install(opts.install_dir, &spec.set_locale)?;
        realmlist::ensure_realmlist_file(opts.install_dir, id, &addr)?;
        on(&FlowEvent::LocaleApplied { locale: id.clone() });
    }

    for (path, _) in realmlist::set_realmlist_all(opts.install_dir, &addr)? {
        let locale = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        on(&FlowEvent::RealmlistApplied { locale });
    }

    Ok(())
}

pub struct InstallClient<'a> {
    pub zip_path: &'a Path,
    pub install_dir: &'a Path,
    pub cancel: &'a AtomicBool,
}

/// Extract a downloaded client zip into the install directory.
pub fn install_client(opts: InstallClient<'_>, on: &mut dyn FnMut(&ExtractProgress)) -> Result<()> {
    extract_zip(opts.zip_path, opts.install_dir, opts.cancel, on)
}

pub fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Relaxed)
}

/// Remove a temporary directory if it exists.
pub fn cleanup_temp(temp_dir: &Path) {
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
