use crate::ui::{App, Lang, Screen};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.lang;
    egui::CentralPanel::default().show(ui, |ui| {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.heading(lang.s("WoW 3.3.5a Installer", "Установщик WoW 3.3.5a"));
            ui.add_space(8.0);
            ui.label(
                lang.s(
                    "This wizard will download the client, apply localization,\nand configure your server — then launch it through Steam.",
                    "Мастер скачает клиент, установит локализацию,\nнастроит сервер — и запустит игру через Steam.",
                ),
            );
            ui.add_space(24.0);
            egui::ComboBox::from_label(lang.s("Language / Язык: ", "Язык: "))
                .selected_text(lang_label(lang))
                .width(110.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.lang, Lang::En, "English");
                    ui.selectable_value(&mut app.lang, Lang::Ru, "Русский");
                });
            ui.add_space(40.0);
            if ui
                .add(
                    egui::Button::new(lang.s("Start ▶", "Начать ▶"))
                        .min_size(egui::vec2(180.0, 40.0)),
                )
                .clicked()
            {
                app.screen = Screen::Directory;
            }
        });
    });
}

fn lang_label(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "English",
        Lang::Ru => "Русский",
    }
}
