//! eframe entry point and app wrapper.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::core::config::AppConfig;
use crate::ui::{App, Lang};

pub fn run() -> crate::error::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 540.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("WoW Installer"),
        ..Default::default()
    };
    eframe::run_native(
        "wow_installer",
        options,
        Box::new(|_cc| Ok(Box::new(WowInstallerApp::new()))),
    )
    .map_err(|e| crate::error::Error::Msg(format!("failed to start GUI: {e}")))
}

pub struct WowInstallerApp {
    pub app: App,
}

impl WowInstallerApp {
    pub fn new() -> Self {
        let cfg = AppConfig::load();
        let lang = Lang::detect();
        let dir = cfg.resolved_install_dir().to_string_lossy().to_string();

        let mut app = App {
            lang,
            screen: crate::ui::Screen::Welcome,
            cfg,
            dir,
            dir_error: None,
            has_existing: false,
            server_mode: crate::ui::ServerMode::Local,
            custom_server: String::new(),
            source_mode: crate::ui::SourceMode::Magnet,
            zip_path: String::new(),
            locale_flags: Vec::new(),
            worker: None,
            cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            view: crate::ui::ProgressView::default(),
            logs: Vec::new(),
        };
        app.refresh_dir_state();
        app.refresh_locale_flags();
        WowInstallerApp { app }
    }
}

impl eframe::App for WowInstallerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.poll_worker();

        match self.app.screen {
            crate::ui::Screen::Welcome => crate::ui::welcome::show(&mut self.app, ui),
            crate::ui::Screen::Directory => crate::ui::dir::show(&mut self.app, ui),
            crate::ui::Screen::Server => crate::ui::server::show(&mut self.app, ui),
            crate::ui::Screen::Locales => crate::ui::locales::show(&mut self.app, ui),
            crate::ui::Screen::Running => crate::ui::running::show(&mut self.app, ui),
            crate::ui::Screen::Finish => crate::ui::finish::show(&mut self.app, ui),
        }

        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(120));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.app.worker.is_some() {
            self.app.cancel.store(true, Ordering::Relaxed);
        }
    }
}

pub fn human_bytes(bytes: u64) -> String {
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
