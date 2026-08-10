use crate::core::client::CLIENT_NAME;
use crate::ui::{App, Screen};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.lang;
    let has_existing = app.has_existing;
    let dir_error = app.dir_error.clone();

    app.footer_buttons(
        ui,
        true,
        lang.s("Back", "Назад"),
        !app.dir.trim().is_empty(),
        lang.s("Next ▶", "Далее ▶"),
        |app| {
            app.refresh_dir_state();
            app.screen = Screen::Server;
        },
    );

    egui::CentralPanel::default().show(ui, |ui| {
        ui.add_space(16.0);
        ui.heading(app.header(lang.s(
            "Choose installation directory",
            "Выберите папку для установки",
        )));
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            let edit = egui::TextEdit::singleline(&mut app.dir)
                .hint_text(lang.s("/path/to/WoW", "/путь/к/WoW"))
                .desired_width(460.0);
            ui.add(edit);
            if ui.button(lang.s("Browse…", "Обзор…")).clicked()
                && let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    app.dir = dir.to_string_lossy().to_string();
                    app.refresh_dir_state();
                }
        });
        if let Some(err) = &dir_error {
            ui.colored_label(egui::Color32::from_rgb(200, 80, 80), err);
        }
        ui.add_space(8.0);
        ui.label(format!(
            "{}: {}",
            lang.s("Disk space needed", "Необходимо места на диске"),
            crate::app::human_bytes(crate::core::check::CLIENT_SIZE_GUESS_BYTES)
        ));

        ui.add_space(16.0);
        if has_existing {
            let state = crate::core::check::inspect(std::path::Path::new(app.dir.trim()));
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(40, 60, 40))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label(
                        lang.s(
                            "Existing WoW installation detected — the wizard will skip the download and only reconfigure the server and localization.",
                            "Обнаружена существующая установка WoW — мастер пропустит скачивание и только перенастроит сервер и локализацию.",
                        ),
                    );
                    if !state.locales.is_empty() {
                        ui.label(format!(
                            "{} {}",
                            lang.s("Installed languages:", "Установленные языки:"),
                            state.locales.join(", ")
                        ));
                    }
                    if !state.has_config {
                        ui.label(lang.s(
                            "No WTF/Config.wtf found yet — it will be created.",
                            "WTF/Config.wtf ещё нет — он будет создан.",
                        ));
                    }
                });
        } else {
            ui.label(format!(
                "{} {CLIENT_NAME}",
                lang.s("Will download:", "Будет скачан:")
            ));
        }
    });
}
