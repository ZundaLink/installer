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

    // Detect and configure rendering backend
    let renderer = detect_renderer();
    log::info!("Using renderer: {}", renderer);

    // Set environment variables for software rendering if needed
    setup_software_rendering(&renderer);

    // Load icon from embedded bytes
    let icon_data = load_icon();

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(icon_data),
        ..Default::default()
    };

    // Build window title with renderer info
    let renderer_label = if renderer == "software" { "[Software]" } else { "[Hardware]" };
    let window_title = format!("ZundaLink Installer {} {}", FULL_VERSION, renderer_label);

    eframe::run_native(
        &window_title,
        options,
        Box::new(|cc| Box::new(ZundaLinkApp::new(cc))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run app: {}", e))?;

    Ok(())
}

/// Detect which rendering backend to use
/// 
/// Priority:
/// 1. User-specified renderer via environment variable
/// 2. Hardware-accelerated OpenGL
/// 3. Software rendering (LLVMpipe/Mesa)
fn detect_renderer() -> String {
    // Check if user has specified a renderer
    if let Ok(renderer) = std::env::var("ZUNDALINK_RENDERER") {
        return renderer;
    }

    // Windows system detection
    #[cfg(windows)]
    {
        // Check if hardware acceleration is available
        if is_hardware_acceleration_available() {
            return "hardware".to_string();
        }
    }

    // Default to software rendering for maximum compatibility
    "software".to_string()
}

/// Check if Windows system supports hardware acceleration
#[cfg(windows)]
fn is_hardware_acceleration_available() -> bool {
    use winapi::um::libloaderapi::GetModuleHandleW;

    unsafe {
        // Check if OpenGL32.dll can be loaded
        let opengl_dll = GetModuleHandleW(encode_wide("opengl32.dll").as_ptr());
        if opengl_dll.is_null() {
            log::warn!("OpenGL32.dll not available, hardware acceleration unavailable");
            return false;
        }
    }

    // Detect common virtual machine or remote desktop environments
    if is_running_in_vm_or_rdp() {
        log::info!("Running in VM or RDP session, preferring software rendering");
        return false;
    }

    true
}

#[cfg(windows)]
fn encode_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsString::from(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Detect if running in a virtual machine or remote desktop session
#[cfg(windows)]
fn is_running_in_vm_or_rdp() -> bool {
    use winapi::um::winuser::GetSystemMetrics;
    use winapi::um::winuser::SM_REMOTESESSION;

    unsafe {
        // Check if this is a remote desktop session
        let is_rdp = GetSystemMetrics(SM_REMOTESESSION) != 0;
        if is_rdp {
            return true;
        }
    }

    // Check common virtual machine environment variables
    let vm_indicators = [
        "VIRTUAL_ENV",
        "VMWARE_PLAYER",
        "VBOX_MSI_INSTALL_PATH",
        "QEMU",
    ];

    for indicator in &vm_indicators {
        if std::env::var(indicator).is_ok() {
            return true;
        }
    }

    false
}

#[cfg(not(windows))]
fn is_hardware_acceleration_available() -> bool {
    // Non-Windows systems default to trying hardware acceleration
    true
}

/// Configure software rendering environment variables
fn setup_software_rendering(renderer: &str) {
    // Note: In Rust 2024 edition, set_var is unsafe
    // But it's safe to set environment variables at program startup since there are no other threads yet
    unsafe {
        if renderer == "software" {
            // Mesa LLVMpipe - use CPU for OpenGL software rendering
            std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
            std::env::set_var("GALLIUM_DRIVER", "llvmpipe");
            
            log::info!("Software rendering enabled (Mesa LLVMpipe)");
            log::info!("This may reduce performance but ensures compatibility");
        }

        // Set additional OpenGL optimization options
        // Disable vsync to avoid performance issues with software rendering
        std::env::set_var("vblank_mode", "0");
    }
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
