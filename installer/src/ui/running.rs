use crate::app::human_bytes;
use crate::ui::App;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.lang;
    let phase = app.view.phase.clone();
    let detail = app.view.detail.clone();
    let fraction = app.view.fraction;
    let speed_bps = app.view.speed_bps;
    let peers = app.view.peers;

    egui::CentralPanel::default().show(ui, |ui| {
        ui.add_space(32.0);
        ui.vertical_centered(|ui| {
            ui.heading(lang.s("Installing…", "Установка…"));
            ui.add_space(16.0);
        });

        ui.add_space(16.0);
        ui.label(egui::RichText::new(&phase).size(16.0).strong());
        ui.add_space(8.0);

        if !detail.is_empty() {
            ui.label(&detail);
        }

        ui.add_space(8.0);
        match fraction {
            Some(f) => {
                ui.add(egui::ProgressBar::new(f.clamp(0.0, 1.0) as f32).show_percentage());
                let mut status = String::new();
                if speed_bps > 0 {
                    status.push_str(&format!("{}/s", human_bytes(speed_bps)));
                }
                if let Some(p) = peers {
                    if !status.is_empty() {
                        status.push_str("  •  ");
                    }
                    status.push_str(lang.s("peers: ", "пиров: "));
                    status.push_str(&p.to_string());
                }
                if !status.is_empty() {
                    ui.weak(status);
                }
            }
            None => {
                ui.spinner();
            }
        }

        ui.add_space(24.0);
        egui::ScrollArea::vertical()
            .max_height(120.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for line in app.logs.iter().rev().take(30) {
                    ui.small(egui::RichText::new(line).monospace().weak());
                }
            });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(lang.s("Cancel", "Отмена")).min_size(egui::vec2(120.0, 32.0)),
                )
                .clicked()
            {
                app.cancel_install();
            }
        });
    });
}
