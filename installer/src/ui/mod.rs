//! Wizard screens and shared UI state.

pub mod dir;
pub mod finish;
pub mod locales;
pub mod running;
pub mod server;
pub mod welcome;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use crate::core::config::AppConfig;
use crate::core::extract::ExtractProgress;
use crate::core::server::Server;
use crate::engine::events::DownloadEvent;
use crate::error::Result;
use crate::flow::FlowEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Ru,
}

impl Lang {
    pub fn detect() -> Lang {
        for var in ["LANG", "LC_ALL", "LC_MESSAGES"] {
            if let Ok(v) = std::env::var(var)
                && v.to_ascii_lowercase().contains("ru")
            {
                return Lang::Ru;
            }
        }
        Lang::En
    }

    pub fn s<'a>(&self, en: &'a str, ru: &'a str) -> &'a str {
        match self {
            Lang::En => en,
            Lang::Ru => ru,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    Directory,
    Server,
    Locales,
    Running,
    Finish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMode {
    Local,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceMode {
    Magnet,
    Zip,
}

/// Events a background install thread sends to the UI thread.
#[derive(Debug)]
pub enum WorkerEvent {
    Log(String),
    Download(DownloadEvent),
    Extract(ExtractProgress),
    Flow(FlowEvent),
    Finished(Result<()>),
}

/// Snapshot of the settings needed to run the install, copied to the worker.
#[derive(Debug, Clone)]
pub struct InstallInput {
    pub install_dir: PathBuf,
    pub server: Server,
    pub locales: Vec<String>,
    pub source_mode: SourceMode,
    pub zip_path: Option<PathBuf>,
    pub cancel: Arc<AtomicBool>,
}

/// What is currently being shown on the running screen.
#[derive(Debug, Default)]
pub struct ProgressView {
    pub phase: String,
    pub detail: String,
    pub fraction: Option<f64>,
    pub speed_bps: u64,
    pub peers: Option<u32>,
    pub completed: bool,
    pub error: Option<String>,
}

/// The running install worker (thread + event channel).
pub struct Worker {
    pub rx: Receiver<WorkerEvent>,
    pub handle: JoinHandle<()>,
}

/// The main wizard application state.
pub struct App {
    pub lang: Lang,
    pub screen: Screen,
    pub cfg: AppConfig,
    pub dir: String,
    pub dir_error: Option<String>,
    pub has_existing: bool,
    pub server_mode: ServerMode,
    pub custom_server: String,
    pub source_mode: SourceMode,
    pub zip_path: String,
    pub locale_flags: Vec<(String, bool)>,
    pub worker: Option<Worker>,
    pub cancel: Arc<AtomicBool>,
    pub view: ProgressView,
    pub logs: Vec<String>,
    pub add_to_steam: bool,
    pub steam_status: Option<String>,
    pub just_finished: bool,
}

impl App {
    /// Re-scan the install directory (existing client?, disk space).
    pub fn refresh_dir_state(&mut self) {
        let dir = std::path::PathBuf::from(&self.dir);
        self.has_existing = crate::core::check::inspect(&dir).has_client;
        match crate::core::check::free_space(&dir) {
            Ok(Some(free)) => {
                let need = crate::core::check::CLIENT_SIZE_GUESS_BYTES;
                self.dir_error = if free < need {
                    Some(format!(
                        "only {} free, at least {} needed",
                        crate::app::human_bytes(free),
                        crate::app::human_bytes(need)
                    ))
                } else {
                    None
                };
            }
            _ => self.dir_error = None,
        }
    }

    /// Populate the locale checkboxes from the embedded registry.
    pub fn refresh_locale_flags(&mut self) {
        let mut flags: Vec<(String, bool)> = crate::core::locale::downloadable_locales()
            .unwrap_or_default()
            .into_iter()
            .map(|(id, _)| (id, false))
            .collect();
        // Base locale enUS is always applied; check it and keep it first.
        flags.insert(0, ("enUS".to_string(), true));
        if self.locale_flags.is_empty() {
            // Default to ruRU enabled when available (mirrors the old installer).
            for (id, checked) in flags.iter_mut() {
                if id == "ruRU" {
                    *checked = true;
                }
            }
            self.locale_flags = flags;
        }
    }

    pub fn selected_locales(&self) -> Vec<String> {
        self.locale_flags
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(id, _)| id.clone())
            .collect()
    }

    fn current_server(&self) -> std::result::Result<Server, String> {
        match self.server_mode {
            ServerMode::Local => Ok(Server::default()),
            ServerMode::Custom => Server::parse(&self.custom_server).map_err(|e| e.to_string()),
        }
    }

    fn install_input(&self) -> std::result::Result<InstallInput, String> {
        if self.dir.trim().is_empty() {
            return Err(self
                .lang
                .s(
                    "Please choose an installation directory.",
                    "Выберите папку для установки.",
                )
                .to_string());
        }
        let server = self.current_server()?;
        let locales = self.selected_locales();
        let zip_path = if self.source_mode == SourceMode::Zip && !self.zip_path.trim().is_empty() {
            Some(std::path::PathBuf::from(self.zip_path.trim()))
        } else {
            None
        };
        Ok(InstallInput {
            install_dir: std::path::PathBuf::from(self.dir.trim()),
            server,
            locales,
            source_mode: self.source_mode,
            zip_path,
            cancel: self.cancel.clone(),
        })
    }

    fn start_install(&mut self) {
        let input = match self.install_input() {
            Ok(i) => i,
            Err(e) => {
                self.view.error = Some(e);
                return;
            }
        };
        if self.has_existing {
            crate::logging::log("existing install — repair mode");
        }
        self.view = ProgressView::default();
        self.logs.clear();
        let (tx, rx) = std::sync::mpsc::channel::<WorkerEvent>();
        let handle = std::thread::spawn(move || worker_main(input, tx));
        self.worker = Some(Worker { rx, handle });
        self.screen = Screen::Running;
    }

    fn cancel_install(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Create a desktop shortcut for the installed game and record the result.
    pub fn create_desktop_shortcut(&mut self) {
        let lang = self.lang;
        if self.dir.trim().is_empty() {
            self.steam_status = Some(
                lang.s("Install directory is empty.", "Папка установки пуста.")
                    .to_string(),
            );
            return;
        }
        let install_dir = std::path::PathBuf::from(self.dir.trim());
        let exe = install_dir.join(crate::core::client::WOW_EXE);
        if !exe.exists() {
            self.steam_status = Some(
                lang.s(
                    "Game executable not found yet.",
                    "Игровой файл ещё не найден.",
                )
                .to_string(),
            );
            return;
        }
        match crate::steam::create_desktop_shortcut(&install_dir, &exe) {
            Ok(path) => {
                let msg = format!(
                    "{} {}",
                    lang.s("Shortcut created:", "Ярлык создан:"),
                    path.display()
                );
                crate::logging::log(&msg);
                self.steam_status = Some(msg);
            }
            Err(e) => {
                let msg = format!(
                    "{} {e}",
                    lang.s("Failed to create shortcut:", "Не удалось создать ярлык:")
                );
                crate::logging::log(&msg);
                self.steam_status = Some(msg);
            }
        }
    }

    pub fn poll_worker(&mut self) {
        let mut finished: Option<Result<()>> = None;
        if let Some(worker) = self.worker.take() {
            while let Ok(event) = worker.rx.try_recv() {
                match event {
                    WorkerEvent::Log(s) => self.push_log(s),
                    WorkerEvent::Download(e) => self.apply_download_event(e),
                    WorkerEvent::Extract(p) => self.apply_extract(p),
                    WorkerEvent::Flow(f) => self.apply_flow_event(f),
                    WorkerEvent::Finished(res) => finished = Some(res),
                }
            }
            if finished.is_some() {
                // Join the thread so its resources are released.
                let _ = worker.handle.join();
            } else {
                self.worker = Some(worker);
            }
        }

        if let Some(res) = finished {
            match res {
                Ok(()) => {
                    self.view.completed = true;
                    self.screen = Screen::Finish;
                    self.just_finished = true;
                    // Persist settings for next run.
                    self.cfg.install_dir = Some(self.dir.trim().into());
                    if let Ok(s) = self.current_server() {
                        self.cfg.server = s;
                    }
                    self.cfg.locales = self.selected_locales();
                    if let Err(e) = self.cfg.save() {
                        crate::logging::log(format!("failed to save config: {e}"));
                    }
                }
                Err(crate::error::Error::Cancelled) => {
                    self.view.error = Some(
                        self.lang
                            .s("Installation cancelled.", "Установка отменена.")
                            .to_string(),
                    );
                    self.screen = Screen::Locales;
                    self.cancel
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                }
                Err(e) => {
                    self.view.error = Some(e.to_string());
                    self.view.completed = true;
                    self.screen = Screen::Finish;
                }
            }
        }
    }

    fn push_log(&mut self, s: String) {
        crate::logging::log(&s);
        self.logs.push(s);
        if self.logs.len() > 500 {
            self.logs.drain(..self.logs.len() - 500);
        }
    }

    fn apply_download_event(&mut self, e: DownloadEvent) {
        match &e {
            DownloadEvent::Connecting => {
                self.view.phase = self
                    .lang
                    .s("Downloading client...", "Скачивание клиента...")
                    .to_string();
                self.view.fraction = None;
            }
            DownloadEvent::Metadata { name, .. } => {
                self.view.detail = name.to_string();
                self.view.fraction = Some(0.0);
            }
            DownloadEvent::Progress {
                downloaded,
                total_bytes,
                speed_bps,
                peers,
            } => {
                self.view.fraction = e.fraction();
                self.view.speed_bps = *speed_bps;
                self.view.peers = *peers;
                self.view.detail = format!(
                    "{} / {}",
                    crate::app::human_bytes(*downloaded),
                    total_bytes.map(crate::app::human_bytes).unwrap_or_default()
                );
            }
            DownloadEvent::Done => {}
        }
    }

    fn apply_extract(&mut self, p: ExtractProgress) {
        self.view.phase = self
            .lang
            .s("Extracting client...", "Распаковка клиента...")
            .to_string();
        self.view.fraction = Some(p.fraction());
        self.view.detail = format!(
            "{} / {}  ·  {}",
            p.files_done, p.files_total, p.current_file
        );
    }

    fn apply_flow_event(&mut self, f: FlowEvent) {
        match f {
            FlowEvent::Step(s) => self.push_log(s),
            FlowEvent::DownloadLocale { locale, event } => {
                self.view.phase = format!(
                    "{} {locale}",
                    self.lang
                        .s("Downloading language pack:", "Скачивание языкового пакета:")
                );
                self.apply_download_event(event);
            }
            FlowEvent::ExtractingLocale { locale } => {
                self.view.phase = format!(
                    "{} {locale}",
                    self.lang
                        .s("Applying language pack:", "Применение языкового пакета:")
                );
                self.view.fraction = None;
            }
            FlowEvent::LocaleApplied { locale } => {
                self.push_log(format!("locale applied: {locale}"));
            }
            FlowEvent::RealmlistApplied { locale } => {
                self.push_log(format!("realmlist written: {locale}"));
            }
        }
    }

    fn header(&self, title: &str) -> egui::RichText {
        egui::RichText::new(title).size(20.0).strong()
    }

    fn footer_buttons(
        &mut self,
        ui: &mut egui::Ui,
        can_back: bool,
        back_label: &str,
        can_next: bool,
        next_label: &str,
        next_action: impl FnOnce(&mut Self),
    ) {
        egui::Panel::bottom("footer").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if can_back && ui.button(back_label).clicked() {
                    self.screen = prev_screen(self.screen);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(can_next, egui::Button::new(next_label))
                        .clicked()
                    {
                        next_action(self);
                    }
                });
            });
            ui.add_space(4.0);
        });
    }
}

fn prev_screen(s: Screen) -> Screen {
    match s {
        Screen::Directory => Screen::Welcome,
        Screen::Server => Screen::Directory,
        Screen::Locales => Screen::Server,
        _ => Screen::Welcome,
    }
}

/// Background install thread: download → extract → configure.
fn worker_main(input: InstallInput, tx: std::sync::mpsc::Sender<WorkerEvent>) {
    let emit = |e| {
        let _ = tx.send(e);
    };
    let cancel = &input.cancel;
    let install_dir = &input.install_dir;
    let temp_dir = install_dir.join(".wow_installer_temp");

    let result = (|| -> Result<()> {
        let need_client = !crate::core::check::has_wow_executable(install_dir);

        if need_client {
            let zip_path = match input.source_mode {
                SourceMode::Magnet => {
                    emit(WorkerEvent::Log(
                        "Downloading client via BitTorrent...".to_string(),
                    ));
                    let magnet = crate::core::client::CLIENT_MAGNET;
                    crate::engine::torrent::download_torrent(
                        crate::engine::torrent::TorrentOptions {
                            magnet,
                            save_dir: &temp_dir,
                            cancel,
                            on_progress: &|e| emit(WorkerEvent::Download(e)),
                        },
                    )?
                }
                SourceMode::Zip => {
                    let path = input.zip_path.clone().ok_or_else(|| {
                        crate::error::Error::Msg("no client archive selected".to_string())
                    })?;
                    if !path.exists() {
                        return Err(crate::error::Error::NotFound(path));
                    }
                    path
                }
            };

            emit(WorkerEvent::Log("Extracting client...".to_string()));
            crate::flow::install_client(
                crate::flow::InstallClient {
                    zip_path: &zip_path,
                    install_dir,
                    cancel,
                },
                &mut |p: &ExtractProgress| emit(WorkerEvent::Extract(p.clone())),
            )?;
        } else {
            emit(WorkerEvent::Log(
                "Existing client detected — applying configuration only.".to_string(),
            ));
        }

        emit(WorkerEvent::Log(
            "Configuring server and locales...".to_string(),
        ));
        crate::flow::apply_config(
            crate::flow::ApplyConfig {
                install_dir,
                server: &input.server,
                locales: &input.locales,
                cancel,
                temp_dir: &temp_dir,
            },
            &|e| emit(WorkerEvent::Flow(e.clone())),
        )?;

        Ok(())
    })();

    crate::flow::cleanup_temp(&temp_dir);
    emit(WorkerEvent::Finished(result));
}
