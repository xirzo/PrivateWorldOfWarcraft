use crate::core::server::{DEFAULT_HOST, Server};
use crate::ui::{App, Screen, ServerMode};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.lang;

    app.footer_buttons(
        ui,
        true,
        lang.s("Back", "Назад"),
        valid(app),
        lang.s("Next ▶", "Далее ▶"),
        |app| app.screen = Screen::Locales,
    );

    egui::CentralPanel::default().show(ui, |ui| {
        ui.add_space(16.0);
        ui.heading(app.header(lang.s("Choose your server", "Выберите сервер")));
        ui.add_space(8.0);

        ui.label(
            lang.s(
                "The game connects to the server address written in its realmlist file.",
                "Игра подключается к адресу сервера, записанному в файле realmlist.",
            ),
        );
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.radio_value(&mut app.server_mode, ServerMode::Local, DEFAULT_HOST);
            ui.label(
                lang.s(
                    "Local machine — the server you run with install-wow-wotlk.sh",
                    "Локальная машина — сервер, который вы запускаете через install-wow-wotlk.sh",
                ),
            );
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.radio_value(&mut app.server_mode, ServerMode::Custom, "");
            ui.label(lang.s("Real server address:", "Адрес реального сервера:"));
            ui.add(
                egui::TextEdit::singleline(&mut app.custom_server)
                    .hint_text("play.example.com[:port]")
                    .desired_width(220.0),
            );
        });

        if app.server_mode == ServerMode::Custom {
            match Server::parse(app.custom_server.trim()) {
                Ok(s) => {
                    ui.colored_label(
                        egui::Color32::from_rgb(80, 180, 80),
                        format!(
                            "{} {}",
                            lang.s(
                                "The installer will write: set realmlist ",
                                "Установщик запишет: set realmlist ",
                            ),
                            s.realmlist_value()
                        ),
                    );
                }
                Err(e) => {
                    ui.colored_label(egui::Color32::from_rgb(200, 80, 80), e.to_string());
                }
            }
        }

        ui.add_space(16.0);
        if app.has_existing && !app.dir.trim().is_empty() {
            let dir = std::path::PathBuf::from(app.dir.trim());
            if let Ok(Some(cur)) =
                crate::core::realmlist::read_realmlist(&dir.join("realmlist.wtf"))
            {
                ui.colored_label(
                    egui::Color32::from_rgb(180, 180, 120),
                    format!("{} {cur}", lang.s("Current realmlist:", "Текущий realmlist:")),
                );
            }
            ui.add_space(8.0);
        }
        egui::Frame::NONE
            .fill(egui::Color32::from_rgb(60, 60, 40))
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.label(
                    lang.s(
                        "Default is 127.0.0.1 (this computer). To play on a real server you MUST enter its address here.",
                        "По умолчанию стоит 127.0.0.1 (этот компьютер). Чтобы играть на реальном сервере, ОБЯЗАТЕЛЬНО укажите его адрес.",
                    ),
                );
            });
    });
}

fn valid(app: &App) -> bool {
    match app.server_mode {
        ServerMode::Local => true,
        ServerMode::Custom => {
            !app.custom_server.trim().is_empty() && Server::parse(app.custom_server.trim()).is_ok()
        }
    }
}
