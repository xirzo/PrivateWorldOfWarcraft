use crate::core::locale;
use crate::ui::App;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.lang;

    app.footer_buttons(
        ui,
        true,
        lang.s("Back", "Назад"),
        !app.selected_locales().is_empty(),
        if app.has_existing {
            lang.s("Reconfigure ▶", "Перенастроить ▶")
        } else {
            lang.s("Install ▶", "Установить ▶")
        },
        |app| app.start_install(),
    );

    egui::CentralPanel::default().show(ui, |ui| {
        ui.add_space(16.0);
        ui.heading(app.header(lang.s("Language packs", "Языковые пакеты")));
        ui.add_space(8.0);
        ui.label(
            lang.s(
                "The client always includes English. Optionally download and enable extra languages:",
                "Клиент всегда включает английский. Дополнительно можно скачать и включить другие языки:",
            ),
        );
        ui.add_space(12.0);

        let registry = locale::registry().unwrap_or_default();

        egui::Grid::new("locales").num_columns(2).show(ui, |ui| {
            for (id, checked) in app.locale_flags.iter_mut() {
                let spec = registry.get(id);
                let label = spec.map(|s| s.name.as_str()).unwrap_or(id.as_str());
                if id == "enUS" {
                    ui.add_enabled(false, egui::Checkbox::new(checked, label));
                } else {
                    ui.checkbox(checked, label);
                }
                match spec {
                    Some(s) if s.url.is_some() => {
                        ui.weak(lang.s("downloadable", "можно скачать"));
                    }
                    _ => {
                        ui.weak(lang.s("included", "в комплекте"));
                    }
                }
                ui.end_row();
            }
        });

        ui.add_space(16.0);
        if let Some(err) = &app.view.error {
            ui.colored_label(egui::Color32::from_rgb(200, 80, 80), err);
        }
    });
}
