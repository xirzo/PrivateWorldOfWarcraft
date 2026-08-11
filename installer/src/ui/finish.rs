use crate::core::locale;
use crate::core::server::Server;
use crate::ui::{App, ServerMode};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.lang;
    let install_dir = app.dir.trim().to_string();
    let server = match app.server_mode {
        ServerMode::Local => Server::default(),
        ServerMode::Custom => Server::parse(app.custom_server.trim()).unwrap_or_default(),
    };
    let current_locale = if install_dir.is_empty() {
        None
    } else {
        locale::get_locale(&locale::config_path(std::path::Path::new(&install_dir)))
            .ok()
            .flatten()
    };

    egui::CentralPanel::default().show(ui, |ui| {
        ui.add_space(16.0);
        ui.heading(app.header(lang.s("Done!", "Готово!")));
        ui.add_space(12.0);

        if let Some(err) = &app.view.error {
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(90, 40, 40))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label(format!("{} {err}", lang.s("Error:", "Ошибка:")));
                });
            ui.add_space(12.0);
        }

        egui::Grid::new("summary")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label(lang.s("Client:", "Клиент:"));
                ui.label(install_dir.as_str());
                ui.end_row();
                ui.label(lang.s("Server:", "Сервер:"));
                ui.label(server.realmlist_value());
                ui.end_row();
                ui.label(lang.s("Languages:", "Языки:"));
                ui.label(app.selected_locales().join(", "));
                ui.end_row();
                if let Some(l) = &current_locale {
                    ui.label(lang.s("In-game locale:", "Язык в игре:"));
                    ui.label(l);
                    ui.end_row();
                }
            });

        if !server.is_local() {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(lang.s(
                    "Note: the chosen server is not this machine. Make sure it is online and reachable.",
                    "Внимание: выбранный сервер — не эта машина. Убедитесь, что он запущен и доступен.",
                ))
                .weak(),
            );
        }

        ui.add_space(20.0);
        ui.label(
            egui::RichText::new(lang.s(
                "Launch the game from Steam",
                "Запускайте игру через Steam",
            ))
            .size(16.0)
            .strong(),
        );
        ui.add_space(6.0);
        ui.label(
            lang.s(
                "1. Open Steam -> Library -> \"Add a game\" -> \"Add a Non-Steam game\".\n2. Browse to and select WoW.exe in the install folder.\n3. Click Play on the newly added WoW entry.",
                "1. Откройте Steam -> Библиотека -> «Добавить игру» -> «Добавить стороннюю игру».\n2. Укажите файл WoW.exe в папке установки.\n3. Нажмите «Играть» на добавленной записи WoW.",
            ),
        );
        ui.add_space(4.0);
        ui.label(
            lang.s(
                "Do NOT run WoW.exe directly — Steam (and Proton on Linux) must be running.",
                "Не запускайте WoW.exe напрямую — должен работать Steam (и Proton на Linux).",
            ),
        );

        if let Some(locale_id) = &current_locale
            && locale_id != "enUS"
        {
            ui.add_space(16.0);
                ui.label(
                    egui::RichText::new(lang.s(
                        "Force the game language",
                        "Принудительная локализация",
                    ))
                    .size(16.0)
                    .strong(),
                );
                ui.add_space(6.0);
                let tag = locale_tag(locale_id);
                let en = format!(
                    "The game language is set to {locale_id}. If it still shows English, force it via Steam launch options: right-click WoW in your Library -> Properties -> Launch Options, paste this, then click Play:"
                );
                let ru = format!(
                    "Язык игры установлен: {locale_id}. Если игра всё ещё на английском, укажите язык в параметрах запуска Steam: правый клик по WoW в Библиотеке -> Свойства -> Параметры запуска, вставьте эту команду и нажмите «Играть»:"
                );
                ui.label(lang.s(&en, &ru));
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("LC_ALL={tag}.UTF-8 LANG={tag}.UTF-8 %command%"))
                        .monospace(),
                );
        }

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.label(lang.s("Steam:", "Steam:"));
            match crate::steam::detect_steam_root() {
                Some(root) => {
                    ui.colored_label(
                        egui::Color32::from_rgb(80, 180, 80),
                        format!(
                            "{} {}",
                            lang.s("found at", "найден в"),
                            root.display()
                        ),
                    );
                }
                None => {
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 150, 80),
                        lang.s("not detected — install Steam first", "не обнаружен — установите Steam"),
                    );
                }
            }
        });

        ui.add_space(20.0);
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(lang.s("Open folder", "Открыть папку"))
                        .min_size(egui::vec2(150.0, 36.0)),
                )
                .clicked()
            {
                open_path(&install_dir);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(lang.s("Close", "Закрыть"))
                            .min_size(egui::vec2(120.0, 36.0)),
                    )
                    .clicked()
                {
                    std::process::exit(0);
                }
            });
        });
    });
}

/// "ruRU" -> "ru_RU", "enUS" -> "en_US" (POSIX-style locale tag).
fn locale_tag(locale: &str) -> String {
    let chars: Vec<char> = locale.chars().collect();
    if chars.len() >= 4 {
        let lang_part: String = chars[..2].iter().collect();
        let region: String = chars[chars.len() - 2..].iter().collect();
        format!(
            "{}_{}",
            lang_part.to_ascii_lowercase(),
            region.to_ascii_uppercase()
        )
    } else {
        locale.to_ascii_lowercase()
    }
}

fn open_path(path: &str) {
    #[cfg(target_os = "linux")]
    let status = std::process::Command::new("xdg-open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("open").arg(path).spawn();
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    let status = Err(std::io::Error::other("unsupported platform"));

    if let Err(e) = status {
        crate::logging::log(format!("failed to open {path}: {e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::locale_tag;

    #[test]
    fn locale_tags() {
        assert_eq!(locale_tag("ruRU"), "ru_RU");
        assert_eq!(locale_tag("enUS"), "en_US");
        assert_eq!(locale_tag("deDE"), "de_DE");
        assert_eq!(locale_tag("xx"), "xx");
    }
}
