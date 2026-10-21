mod app;
mod io;
mod model;
mod presets;
mod raster;
mod shortcuts;
mod state;
mod theme;
mod ui;

use app::AtelierApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SUIZA Atelier Suite")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "SUIZA Atelier Suite",
        options,
        Box::new(|cc| Ok(Box::new(AtelierApp::new(cc)))),
    )
}
