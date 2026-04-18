#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod mod_data;
mod sorting;
mod steam;
mod ui;

use app::RustRim;

fn main() -> eframe::Result<()> {
    // std::panic::set_hook(Box::new(|info| {
    //     eprintln!("Паника: {:?}", info);
    //     if let Some(location) = info.location() {
    //         eprintln!("  в {}:{}:{}", location.file(), location.line(), location.column());
    //     }
    //     if let Some(payload) = info.payload().downcast_ref::<&str>() {
    //         eprintln!("  сообщение: {}", payload);
    //     } else if let Some(payload) = info.payload().downcast_ref::<String>() {
    //         eprintln!("  сообщение: {}", payload);
    //     }
    // }));

    // // Инициализация логирования через env_logger (Cargo.toml: env_logger = "0.11")
    // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
    //     .format_timestamp(None)
    //     .init();

    let icon = eframe::icon_data::from_png_bytes(
        include_bytes!("../assets/icon.png")
    ).ok();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("RustRim")
        .with_min_inner_size([900.0, 600.0])
        .with_inner_size([1400.0, 900.0]);

    if let Some(icon_data) = icon {
        viewport = viewport.with_icon(std::sync::Arc::new(icon_data));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Rust Rim",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            let mut fonts = egui::FontDefinitions::default();
            let font_bytes = include_bytes!("../assets/FiraCode.ttf").to_vec();
            fonts.font_data.insert(
                "FiraCode".to_owned(),
                egui::FontData::from_owned(font_bytes).into(),
            );

            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "FiraCode".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "FiraCode".to_owned());

            cc.egui_ctx.set_fonts(fonts);
            cc.egui_ctx.set_pixels_per_point(1.2);

            Ok(Box::new(RustRim::default()))
        }),
    )
}