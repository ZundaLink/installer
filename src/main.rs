// Hide console window on Windows for GUI application
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::NativeOptions;

mod app;
mod build_info;
mod config;
mod download;
mod format;
mod i18n;
mod usb;

use app::ZundaLinkApp;
use build_info::FULL_VERSION;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Print build info at startup
    log::info!("Starting ZundaLink Installer {}", FULL_VERSION);
    log::info!("Build Hash: {}", build_info::BUILD_HASH);
    log::info!("Build Time: {}", build_info::BUILD_TIME);

    // Load icon from embedded bytes
    let icon_data = load_icon();

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(icon_data),
        ..Default::default()
    };

    eframe::run_native(
        &format!("ZundaLink Installer {}", FULL_VERSION),
        options,
        Box::new(|cc| Box::new(ZundaLinkApp::new(cc))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run app: {}", e))?;

    Ok(())
}

fn load_icon() -> egui::IconData {
    // Include the icon bytes at compile time
    let icon_bytes = include_bytes!("../assets/zundalink.png");
    
    // Load the image using image crate
    let image = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon");
    
    // Resize to standard icon size if needed
    let image = image.resize(256, 256, image::imageops::FilterType::Lanczos3);
    
    // Convert to RGBA8
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    
    egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }
}
