mod core;
mod engine;
mod error;
mod flow;
mod logging;

#[cfg(feature = "gui")]
mod app;
#[cfg(feature = "gui")]
mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use clap::Parser;

use crate::core::check;
use crate::core::config::AppConfig;
use crate::core::extract::ExtractProgress;
use crate::core::server::Server;
use crate::engine::events::DownloadEvent;
use crate::engine::torrent;
use crate::error::{Error, Result};
use crate::flow::FlowEvent;

#[derive(Debug, Parser)]
#[command(
    name = "wow_installer",
    version,
    about = "Installer for the WoW 3.3.5a client (download, extract, localize, configure server)",
    long_about = None
)]
struct Cli {
    /// Launch the GUI wizard (default).
    #[arg(long)]
    gui: bool,

    /// Headless install suitable for scripts and testing.
    #[arg(long)]
    cli: bool,

    /// Install directory.
    #[arg(long)]
    dir: Option<PathBuf>,

    /// Server address: `host` or `host:port` (default: 127.0.0.1).
    #[arg(long)]
    server: Option<String>,

    /// Locale(s) to apply, e.g. `--locale ruRU` (repeatable; default: enUS).
    #[arg(long, value_name = "LOCALE")]
    locale: Vec<String>,

    /// Client source: path to a client zip, or a magnet: URI.
    #[arg(long)]
    client: Option<String>,

    /// Do not prompt (currently all CLI steps are non-interactive).
    #[arg(long)]
    yes: bool,
}

fn main() {
    let cli = Cli::parse();
    logging::Logger::global();

    let use_cli = cli.cli;
    let want_gui = cli.gui || (!use_cli && !cli.cli);

    if use_cli {
        match run_cli(cli) {
            Ok(()) => {}
            Err(Error::Cancelled) => {
                eprintln!("cancelled");
                std::process::exit(130);
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    } else if want_gui {
        #[cfg(feature = "gui")]
        {
            if let Err(e) = app::run() {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        #[cfg(not(feature = "gui"))]
        {
            let _ = cli;
            eprintln!("this build does not include the GUI (built without the `gui` feature)");
            eprintln!("run with `--cli` for the headless installer");
            std::process::exit(1);
        }
    }
}

fn run_cli(cli: Cli) -> Result<()> {
    let mut cfg = AppConfig::load();
    if let Some(dir) = cli.dir.clone() {
        cfg.install_dir = Some(dir);
    }
    if let Some(server) = cli.server.as_deref() {
        cfg.server = Server::parse(server)?;
    }
    if !cli.locale.is_empty() {
        cfg.locales = cli.locale.clone();
    }
    if cfg.locales.is_empty() {
        cfg.locales = vec!["enUS".to_string()];
    }

    let install_dir = cfg.resolved_install_dir();
    let temp_dir = install_dir.join(".wow_installer_temp");
    let cancel = Arc::new(AtomicBool::new(false));
    let on_flow: Arc<dyn Fn(&FlowEvent) + Send + Sync> = Arc::new(|e| match e {
        FlowEvent::Step(s) => println!("* {s}"),
        FlowEvent::DownloadLocale { locale, event } => match event {
            DownloadEvent::Connecting => println!("  [{locale}] connecting..."),
            DownloadEvent::Metadata { name, total_bytes } => {
                println!(
                    "  [{locale}] downloading {name} ({})",
                    human_bytes(total_bytes.unwrap_or(0))
                );
            }
            DownloadEvent::Progress {
                downloaded,
                total_bytes,
                speed_bps,
                peers,
            } => {
                let downloaded = *downloaded;
                let speed_bps = *speed_bps;
                let pct = total_bytes
                    .map(|t| {
                        if t > 0 {
                            downloaded as f64 / t as f64 * 100.0
                        } else {
                            0.0
                        }
                    })
                    .unwrap_or(0.0);
                let peers = peers.map(|p| format!(", peers: {p}")).unwrap_or_default();
                println!(
                    "  [{locale}] {pct:5.1}% ({}/s){peers}",
                    human_bytes(speed_bps)
                );
            }
            DownloadEvent::Done => println!("  [{locale}] done"),
        },
        FlowEvent::ExtractingLocale { locale } => println!("  [{locale}] extracting patch..."),
        FlowEvent::LocaleApplied { locale } => println!("  [{locale}] locale configured"),
        FlowEvent::RealmlistApplied { locale } => println!("  [{locale}] realmlist written"),
    });

    println!("Install directory: {}", install_dir.display());
    println!("Client: {}", crate::core::client::CLIENT_NAME);
    println!("Server: {}", cfg.server.realmlist_value());
    println!("Locales: {}", cfg.locales.join(", "));

    let already_installed = check::has_wow_executable(&install_dir);

    if !already_installed {
        if !check::enough_space_for_client(&install_dir)? {
            return Err(Error::Msg(
                "not enough free disk space for the client (need ~25 GiB)".to_string(),
            ));
        }
        let client = cli
            .client
            .as_deref()
            .unwrap_or(crate::core::client::CLIENT_MAGNET);
        println!(
            "Client source: {}",
            if client.starts_with("magnet:") {
                "magnet link (BitTorrent)"
            } else {
                client
            }
        );

        let client_zip = if client.starts_with("magnet:") {
            println!("Downloading client via BitTorrent...");
            torrent::download_torrent(torrent::TorrentOptions {
                magnet: client,
                save_dir: &temp_dir,
                cancel: &cancel,
                on_progress: &|e: DownloadEvent| match e {
                    DownloadEvent::Connecting => println!("  connecting..."),
                    DownloadEvent::Metadata { name, total_bytes } => {
                        println!(
                            "  downloading {name} ({})",
                            human_bytes(total_bytes.unwrap_or(0))
                        )
                    }
                    DownloadEvent::Progress {
                        downloaded,
                        total_bytes,
                        speed_bps,
                        peers,
                    } => {
                        let pct = total_bytes
                            .map(|t| {
                                if t > 0 {
                                    downloaded as f64 / t as f64 * 100.0
                                } else {
                                    0.0
                                }
                            })
                            .unwrap_or(0.0);
                        let peers = peers.map(|p| format!(", peers: {p}")).unwrap_or_default();
                        println!("  {pct:5.1}% ({}/s){peers}", human_bytes(speed_bps));
                    }
                    DownloadEvent::Done => println!("  download complete"),
                },
            })?
        } else {
            let p = PathBuf::from(client);
            if !p.exists() {
                return Err(Error::NotFound(p));
            }
            println!("Using local client archive: {}", p.display());
            p
        };

        if client_zip.exists() {
            println!("Extracting client...");
            let mut last = 0.0f64;
            flow::install_client(
                flow::InstallClient {
                    zip_path: &client_zip,
                    install_dir: &install_dir,
                    cancel: &cancel,
                },
                &mut |p: &ExtractProgress| {
                    let f = p.fraction();
                    if (f - last).abs() >= 0.01 || f >= 1.0 {
                        last = f;
                        println!("  extract {:.1}%", f * 100.0);
                    }
                },
            )?;
        }
    } else {
        println!("Existing client detected — applying configuration only.");
    }

    flow::apply_config(
        flow::ApplyConfig {
            install_dir: &install_dir,
            server: &cfg.server,
            locales: &cfg.locales,
            cancel: &cancel,
            temp_dir: &temp_dir,
        },
        on_flow.as_ref(),
    )?;

    flow::cleanup_temp(&temp_dir);
    cfg.save()?;

    println!();
    println!("Done.");
    println!("  Client:   {}", install_dir.display());
    println!("  Server:   {}", cfg.server.realmlist_value());
    println!(
        "  Launch:   add {} to Steam, then click Play",
        install_dir.join(crate::core::client::WOW_EXE).display()
    );
    Ok(())
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
