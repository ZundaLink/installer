use eframe::NativeOptions;

mod app;
mod config;
mod download;
mod usb;
mod format;

use app::ZundaLinkApp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    // Load icon from embedded bytes
    let icon_data = load_icon();
    
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(icon_data),
        ..Default::default()
    };
    
    eframe::run_native(
        "ZundaLink Installer",
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
