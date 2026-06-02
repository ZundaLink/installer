use eframe::egui;
use tokio::sync::mpsc;

use crate::config::{InstallerConfig, VersionInfo};
use crate::download::{download_all_files, DownloadProgress, DownloadStatus};
use crate::format::{copy_files_to_drive_with_progress, CopyProgress, DiskFormatter};
use crate::usb::{UsbDetector, UsbDevice};

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Loading,
    VersionSelection,
    UsbSelection,
    Downloading,
    FormatConfirmation,
    Formatting,
    Copying,
    Completed,
    Error(String),
}

pub struct ZundaLinkApp {
    state: AppState,
    config: Option<InstallerConfig>,
    selected_version: Option<VersionInfo>,
    usb_devices: Vec<UsbDevice>,
    selected_usb: Option<UsbDevice>,
    manual_drive_letter: String,
    download_progress: Vec<DownloadProgress>,
    error_message: Option<String>,

    // Async communication
    progress_rx: Option<mpsc::Receiver<DownloadProgress>>,
    config_rx: Option<mpsc::Receiver<Result<InstallerConfig, String>>>,
    format_copy_rx: Option<mpsc::Receiver<Result<String, String>>>,
    copy_progress_rx: Option<mpsc::Receiver<CopyProgress>>,
    usb_rx: Option<mpsc::Receiver<Result<Vec<UsbDevice>, String>>>,
    runtime: tokio::runtime::Handle,

    // UI state
    show_format_warning: bool,
    format_confirmed: bool,
    download_started: bool,
    format_copy_status: String,

    // Copy progress
    copy_progress: Vec<CopyProgress>,
    current_copy_file: String,

    // Temp directory settings
    temp_dir: String,
    show_disk_space_warning: bool,
    required_space: u64,
    available_space: u64,

    // Skip verification
    skip_verify_tx: Option<mpsc::Sender<bool>>,
    show_skip_verify_dialog: bool,
    verifying_filename: Option<String>,
}

impl ZundaLinkApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure fonts for Chinese support
        Self::configure_fonts(&cc.egui_ctx);
        
        let (_progress_tx, progress_rx) = mpsc::channel(100);
        let runtime = tokio::runtime::Handle::current();
        
        let mut app = Self {
            state: AppState::Loading,
            config: None,
            selected_version: None,
            usb_devices: Vec::new(),
            selected_usb: None,
            manual_drive_letter: String::new(),
            download_progress: Vec::new(),
            error_message: None,
            progress_rx: Some(progress_rx),
            config_rx: None,
            format_copy_rx: None,
            copy_progress_rx: None,
            usb_rx: None,
            runtime,
            show_format_warning: false,
            format_confirmed: false,
            download_started: false,
            format_copy_status: String::new(),
            copy_progress: Vec::new(),
            current_copy_file: String::new(),
            temp_dir: "./temp".to_string(),
            show_disk_space_warning: false,
            required_space: 0,
            available_space: 0,
            skip_verify_tx: None,
            show_skip_verify_dialog: false,
            verifying_filename: None,
        };
        
        // Start loading config
        app.load_config();
        
        app
    }
    
    fn configure_fonts(ctx: &egui::Context) {
        use egui::FontFamily;
        
        // Get system font for Chinese characters
        let mut fonts = egui::FontDefinitions::default();
        
        // Try to load system fonts that support Chinese
        #[cfg(target_os = "windows")]
        let system_fonts = vec![
            "C:\\Windows\\Fonts\\msyh.ttc",      // Microsoft YaHei
            "C:\\Windows\\Fonts\\simsun.ttc",    // SimSun
            "C:\\Windows\\Fonts\\simhei.ttf",   // SimHei
            "C:\\Windows\\Fonts\\msyhbd.ttc",   // Microsoft YaHei Bold
        ];
        
        #[cfg(target_os = "macos")]
        let system_fonts = vec![
            "/System/Library/Fonts/PingFang.ttc",           // PingFang
            "/System/Library/Fonts/STHeiti Light.ttc",      // STHeiti
            "/Library/Fonts/Arial Unicode.ttf",             // Arial Unicode
            "/System/Library/Fonts/Hiragino Sans GB.ttc",   // Hiragino Sans GB
        ];
        
        #[cfg(target_os = "linux")]
        let system_fonts = vec![
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",     // WenQuanYi Zen Hei
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",   // WenQuanYi Micro Hei
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", // Noto Sans CJK
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",  // DejaVu Sans
        ];
        
        // Try to load the first available font
        for font_path in system_fonts {
            if std::path::Path::new(font_path).exists() {
                if let Ok(font_data) = std::fs::read(font_path) {
                    // Add font data with a unique name
                    fonts.font_data.insert(
                        "chinese_font".to_owned(),
                        egui::FontData::from_owned(font_data),
                    );
                    
                    // Add to proportional font family
                    fonts
                        .families
                        .entry(FontFamily::Proportional)
                        .or_default()
                        .push("chinese_font".to_owned());
                    
                    // Add to monospace font family
                    fonts
                        .families
                        .entry(FontFamily::Monospace)
                        .or_default()
                        .push("chinese_font".to_owned());
                    
                    log::info!("Loaded Chinese font: {}", font_path);
                    break;
                }
            }
        }
        
        // Set the fonts
        ctx.set_fonts(fonts);
    }
    
    fn load_config(&mut self) {
        let runtime = self.runtime.clone();
        
        // Create a channel to receive the config
        let (tx, rx) = mpsc::channel::<Result<InstallerConfig, String>>(1);
        
        runtime.spawn(async move {
            match InstallerConfig::fetch().await {
                Ok(config) => {
                    log::info!("Config loaded successfully");
                    let _ = tx.send(Ok(config)).await;
                }
                Err(e) => {
                    log::error!("Failed to load config: {}", e);
                    let _ = tx.send(Err(e.to_string())).await;
                }
            }
        });
        
        // Store the receiver to check in update loop
        self.config_rx = Some(rx);
    }
    
    fn detect_usb_devices(&mut self) {
        // Create channel for async USB detection
        let (tx, rx) = mpsc::channel::<Result<Vec<UsbDevice>, String>>(1);
        self.usb_rx = Some(rx);
        
        let runtime = self.runtime.clone();
        
        // Spawn USB detection in background
        runtime.spawn(async move {
            let detector = UsbDetector::new();
            match detector.detect_usb_drives() {
                Ok(devices) => {
                    log::info!("Detected {} USB devices", devices.len());
                    let _ = tx.send(Ok(devices)).await;
                }
                Err(e) => {
                    log::error!("Failed to detect USB devices: {}", e);
                    let _ = tx.send(Err(e.to_string())).await;
                }
            }
        });
    }
}

impl eframe::App for ZundaLinkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for config loading completion
        if let Some(ref mut rx) = self.config_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(config) => {
                        self.config = Some(config);
                        self.state = AppState::VersionSelection;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("加载配置失败: {}", e));
                        self.state = AppState::Error(e);
                    }
                }
                self.config_rx = None;
            }
        }
        
        // Check for progress updates
        let mut progress_updated = false;
        if let Some(ref mut rx) = self.progress_rx {
            while let Ok(progress) = rx.try_recv() {
                // Update progress for existing file or add new
                if let Some(existing) = self.download_progress.iter_mut()
                    .find(|p| p.filename == progress.filename) {
                    *existing = progress;
                } else {
                    self.download_progress.push(progress);
                }
                progress_updated = true;
            }
        }

        // Request repaint if progress was updated or if downloading to ensure smooth UI updates
        if progress_updated || matches!(self.state, AppState::Downloading) {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        
        // Check for USB detection completion
        if let Some(ref mut rx) = self.usb_rx {
            match rx.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(devices) => {
                            self.usb_devices = devices;
                        }
                        Err(e) => {
                            self.error_message = Some(format!("检测U盘失败: {}", e));
                        }
                    }
                    self.usb_rx = None;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.usb_rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // Still detecting, request repaint to check again
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
            }
        }
        
        // Check for format/copy operation completion
        if let Some(ref mut rx) = self.format_copy_rx {
            match rx.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(message) => {
                            if self.state == AppState::Formatting {
                                // Format completed, start copying
                                self.format_copy_status = message;
                                self.state = AppState::Copying;
                                self.start_copying();
                            } else if self.state == AppState::Copying {
                                // Copy completed
                                self.format_copy_status = message;
                                self.state = AppState::Completed;
                                self.format_copy_rx = None;
                            }
                        }
                        Err(e) => {
                            self.error_message = Some(format!("操作失败: {}", e));
                            self.state = AppState::Error(e);
                            self.format_copy_rx = None;
                        }
                    }
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Channel closed, operation might have completed
                    if self.state == AppState::Formatting || self.state == AppState::Copying {
                        // Don't change state here, let the success message handle it
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // No message yet, continue waiting
                }
            }
        }
        
        // Check for copy progress updates
        let mut copy_progress_updated = false;
        if let Some(ref mut rx) = self.copy_progress_rx {
            while let Ok(progress) = rx.try_recv() {
                self.current_copy_file = progress.filename.clone();
                
                // Update progress for existing file or add new
                if let Some(existing) = self.copy_progress.iter_mut()
                    .find(|p| p.filename == progress.filename) {
                    *existing = progress;
                } else {
                    self.copy_progress.push(progress);
                }
                copy_progress_updated = true;
            }
        }
        
        // Request repaint if copy progress was updated
        if copy_progress_updated {
            ctx.request_repaint();
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            match &self.state {
                AppState::Loading => {
                    ui.heading("ZundaLink Installer");
                    ui.add_space(20.0);
                    ui.label("正在加载配置...");
                    ui.spinner();
                }
                
                AppState::VersionSelection => {
                    ui.horizontal(|ui| {
                        ui.heading("选择安装版本");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🔄 刷新").clicked() {
                                self.state = AppState::Loading;
                                self.config = None;
                                self.selected_version = None;
                                self.load_config();
                            }
                        });
                    });
                    ui.add_space(20.0);

                    if let Some(ref config) = self.config {
                        ui.label("可用版本:");

                        for version in &config.version_list {
                            let is_selected = self.selected_version.as_ref()
                                .map(|v| v.name == version.name)
                                .unwrap_or(false);

                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    let response = ui.radio(is_selected, &version.name);
                                    if response.clicked() {
                                        if is_selected {
                                            // Deselect if already selected
                                            self.selected_version = None;
                                        } else {
                                            self.selected_version = Some(version.clone());
                                        }
                                    }
                                    ui.label(&version.summary);
                                    if version.name == config.latest_version {
                                        ui.colored_label(egui::Color32::GREEN, "(最新)");
                                    }
                                });
                            });
                        }

                        ui.add_space(20.0);

                        // Temp directory setting
                        ui.group(|ui| {
                            ui.label("临时文件目录:");
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut self.temp_dir);
                                if ui.button("选择目录").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        self.temp_dir = path.to_string_lossy().to_string();
                                    }
                                }
                            });
                            ui.label("下载的文件将保存在此目录");
                        });

                        ui.add_space(20.0);

                        if self.selected_version.is_some() {
                            if ui.button("下一步 →").clicked() {
                                // Check disk space before proceeding
                                if let Some(ref version) = self.selected_version {
                                    self.required_space = version.install_list.iter().map(|f| f.size).sum();
                                    if let Some(available) = crate::usb::get_disk_free_space(&self.temp_dir) {
                                        self.available_space = available;
                                        if self.required_space > available {
                                            self.show_disk_space_warning = true;
                                        } else {
                                            self.detect_usb_devices();
                                            self.state = AppState::UsbSelection;
                                        }
                                    } else {
                                        // Cannot determine disk space, proceed anyway
                                        self.detect_usb_devices();
                                        self.state = AppState::UsbSelection;
                                    }
                                }
                            }
                        }
                    }

                    // Disk space warning dialog
                    if self.show_disk_space_warning {
                        egui::Window::new("[!] 磁盘空间不足")
                            .collapsible(false)
                            .resizable(false)
                            .show(ctx, |ui| {
                                ui.colored_label(
                                    egui::Color32::RED,
                                    "临时目录所在磁盘空间不足！"
                                );
                                ui.label(format!("所需空间: {:.2} GB", self.required_space as f64 / (1024.0 * 1024.0 * 1024.0)));
                                ui.label(format!("可用空间: {:.2} GB", self.available_space as f64 / (1024.0 * 1024.0 * 1024.0)));
                                ui.add_space(10.0);
                                ui.label("请选择其他有足够空间的目录。");

                                ui.add_space(10.0);

                                ui.horizontal(|ui| {
                                    if ui.button("选择其他目录").clicked() {
                                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                            self.temp_dir = path.to_string_lossy().to_string();
                                            // Recheck space
                                            if let Some(available) = crate::usb::get_disk_free_space(&self.temp_dir) {
                                                self.available_space = available;
                                                if self.required_space <= available {
                                                    self.show_disk_space_warning = false;
                                                    self.detect_usb_devices();
                                                    self.state = AppState::UsbSelection;
                                                }
                                            }
                                        }
                                    }
                                    if ui.button("取消").clicked() {
                                        self.show_disk_space_warning = false;
                                    }
                                });
                            });
                    }
                }
                
                AppState::UsbSelection => {
                    ui.heading("选择目标U盘");
                    ui.add_space(20.0);
                    
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "[!] 警告：此操作将格式化选中的U盘并清除所有数据！"
                    );
                    
                    ui.add_space(20.0);
                    
                    if self.usb_devices.is_empty() {
                        ui.label("未检测到U盘设备。请手动输入盘符:");
                        ui.text_edit_singleline(&mut self.manual_drive_letter);
                        ui.label("例如: Q 或 Q:");
                    } else {
                        ui.label("检测到的U盘设备:");
                        
                        for device in &self.usb_devices {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    let is_selected = self.selected_usb.as_ref()
                                        .map(|d| d.device_path == device.device_path)
                                        .unwrap_or(false);

                                    let response = ui.radio(is_selected, &device.model);
                                    if response.clicked() {
                                        if is_selected {
                                            // Deselect if already selected
                                            self.selected_usb = None;
                                        } else {
                                            self.selected_usb = Some(device.clone());
                                        }
                                    }
                                    ui.label(format!("{:.2} GB", device.size_gb()));
                                    if let Some(ref letter) = device.drive_letter {
                                        ui.label(format!("({})", letter));
                                    }
                                });
                            });
                        }
                    }
                    
                    ui.add_space(20.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("← 返回").clicked() {
                            self.state = AppState::VersionSelection;
                        }
                        
                        let can_proceed = self.selected_usb.is_some() 
                            || !self.manual_drive_letter.is_empty();
                        
                        if can_proceed && ui.button("下一步 →").clicked() {
                            self.show_format_warning = true;
                        }
                    });
                    
                    // Format warning dialog
                    if self.show_format_warning {
                        egui::Window::new("[!] 重要警告")
                            .collapsible(false)
                            .resizable(false)
                            .show(ctx, |ui| {
                                ui.colored_label(
                                    egui::Color32::RED,
                                    "此操作将格式化U盘并删除所有数据！"
                                );
                                ui.label("请确认您已备份重要数据。");
                                
                                ui.checkbox(&mut self.format_confirmed, "我已了解风险并确认继续");
                                
                                ui.add_space(10.0);
                                
                                ui.horizontal(|ui| {
                                    if ui.button("取消").clicked() {
                                        self.show_format_warning = false;
                                        self.format_confirmed = false;
                                    }
                                    
                                    if self.format_confirmed && ui.button("确认继续").clicked() {
                                        self.show_format_warning = false;
                                        self.state = AppState::Downloading;
                                        self.start_download();
                                    }
                                });
                            });
                    }
                }
                
                AppState::Downloading => {
                    ui.heading("下载安装文件");
                    ui.add_space(20.0);
                    
                    // Show skip verification confirmation dialog
                    if self.show_skip_verify_dialog {
                        egui::Window::new("确认跳过校验")
                            .collapsible(false)
                            .resizable(false)
                            .show(ctx, |ui| {
                                ui.label("确定要跳过文件校验吗？");
                                ui.label("跳过校验可能会导致安装损坏的文件。");
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    if ui.button("取消").clicked() {
                                        self.show_skip_verify_dialog = false;
                                        self.verifying_filename = None;
                                    }
                                    if ui.button("确定跳过").clicked() {
                                        // Send skip signal
                                        if let Some(ref tx) = self.skip_verify_tx {
                                            let _ = tx.try_send(true);
                                        }
                                        self.show_skip_verify_dialog = false;
                                        self.verifying_filename = None;
                                    }
                                });
                            });
                    }
                    
                    if self.download_progress.is_empty() {
                        ui.label("准备下载...");
                        ui.spinner();
                    } else {
                        for progress in &self.download_progress {
                            ui.group(|ui| {
                                ui.label(&progress.filename);
                                
                                // Show different progress bar based on status
                                match &progress.status {
                                    DownloadStatus::Verifying => {
                                        // Show verification progress with skip button
                                        ui.horizontal(|ui| {
                                            let verify_percent = progress.verify_progress * 100.0;
                                            let verify_text = format!(
                                                "校验中: {:.1}%",
                                                verify_percent
                                            );
                                            ui.add(egui::ProgressBar::new(progress.verify_progress)
                                                .text(verify_text)
                                                .desired_width(ui.available_width() - 120.0));
                                            
                                            // Show skip button
                                            if ui.button("跳过当前文件校验").clicked() {
                                                self.verifying_filename = Some(progress.filename.clone());
                                                self.show_skip_verify_dialog = true;
                                            }
                                        });
                                    }
                                    DownloadStatus::Skipped => {
                                        ui.colored_label(egui::Color32::YELLOW, "已跳过校验");
                                    }
                                    _ => {
                                        // Show download progress
                                        let percent = if progress.total > 0 {
                                            (progress.downloaded as f32 / progress.total as f32) * 100.0
                                        } else {
                                            0.0
                                        };
                                        
                                        let status_text = match &progress.status {
                                            DownloadStatus::Pending => "等待中",
                                            DownloadStatus::Downloading => "下载中",
                                            DownloadStatus::Verifying => "校验中",
                                            DownloadStatus::Completed => "完成",
                                            DownloadStatus::Failed(_) => "失败",
                                            DownloadStatus::Skipped => "已跳过校验",
                                        };
                                        
                                        let progress_text = format!(
                                            "{:.1}% ({:.2} MB / {:.2} MB) - {}",
                                            percent,
                                            progress.downloaded as f32 / (1024.0 * 1024.0),
                                            progress.total as f32 / (1024.0 * 1024.0),
                                            status_text
                                        );
                                        
                                        ui.add(egui::ProgressBar::new(percent / 100.0)
                                            .text(progress_text));
                                    }
                                }
                            });
                        }
                        
                        // Check if all downloads are complete (including skipped)
                        let all_complete = self.download_progress.iter()
                            .all(|p| matches!(p.status, DownloadStatus::Completed | DownloadStatus::Skipped));
                        
                        let any_failed = self.download_progress.iter()
                            .any(|p| matches!(p.status, DownloadStatus::Failed(_)));
                        
                        if all_complete {
                            ui.add_space(20.0);
                            ui.colored_label(egui::Color32::GREEN, "[OK] 所有文件下载完成！");
                            ui.add_space(10.0);
                            if ui.button("下一步：格式化U盘 →").clicked() {
                                self.state = AppState::FormatConfirmation;
                            }
                        } else if any_failed {
                            ui.add_space(20.0);
                            ui.colored_label(egui::Color32::RED, "[X] 部分文件下载失败");
                            
                            // Show detailed error for each failed file
                            ui.add_space(10.0);
                            ui.label("失败详情:");
                            for progress in &self.download_progress {
                                if let DownloadStatus::Failed(ref error) = progress.status {
                                    ui.horizontal(|ui| {
                                        ui.colored_label(egui::Color32::RED, "●");
                                        ui.label(format!("{}: {}", progress.filename, error));
                                    });
                                }
                            }
                            
                            // Show help message for 403 errors
                            let has_403 = self.download_progress.iter()
                                .any(|p| matches!(p.status, DownloadStatus::Failed(ref e) if e.contains("403")));
                            
                            if has_403 {
                                ui.add_space(15.0);
                                ui.group(|ui| {
                                    ui.colored_label(egui::Color32::YELLOW, "[i] 提示:");
                                    ui.label("遇到 403 错误通常表示:");
                                    ui.label("• 下载链接已过期");
                                    ui.label("• 需要更新安装器版本");
                                    ui.label("• 服务器限制了访问");
                                    ui.add_space(5.0);
                                    ui.label("建议: 请检查是否有新版本安装器，或稍后重试。");
                                });
                            }
                            
                            ui.add_space(20.0);
                            ui.horizontal(|ui| {
                                if ui.button("← 返回版本选择").clicked() {
                                    // Reset all state including version and USB selection
                                    self.reset_all_state();
                                    self.state = AppState::VersionSelection;
                                }
                                if ui.button("[重试] 重新下载").clicked() {
                                    self.reset_download_state();
                                    self.start_download();
                                }
                            });
                        }
                    }
                }
                
                AppState::FormatConfirmation => {
                    ui.heading("最终确认");
                    ui.add_space(20.0);
                    
                    ui.colored_label(
                        egui::Color32::RED,
                        "[!] 最后一次警告！"
                    );
                    
                    ui.label("即将执行以下操作:");
                    ui.label("1. 清除U盘所有分区和数据");
                    ui.label("2. 创建新的exFAT分区");
                    ui.label("3. 设置卷标");
                    ui.label("4. 分配盘符为 Q:");
                    ui.label("5. 复制安装文件到U盘");
                    
                    ui.add_space(20.0);
                    
                    if let Some(ref usb) = self.selected_usb {
                        ui.label(format!("目标设备: {}", usb.model));
                    }
                    
                    ui.add_space(20.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("← 返回").clicked() {
                            self.state = AppState::UsbSelection;
                        }
                        
                        if ui.button("开始格式化并安装").clicked() {
                            self.state = AppState::Formatting;
                            self.format_and_install();
                        }
                    });
                }
                
                AppState::Formatting => {
                    ui.heading("正在格式化U盘...");
                    ui.add_space(20.0);
                    ui.spinner();
                    ui.label("请稍候，正在清除分区并创建新分区...");
                    ui.add_space(10.0);
                    ui.label("此过程可能需要几分钟，请勿拔出U盘。");
                    
                    if !self.format_copy_status.is_empty() {
                        ui.add_space(10.0);
                        ui.label(format!("状态: {}", self.format_copy_status));
                    }
                }
                
                AppState::Copying => {
                    ui.heading("正在复制文件...");
                    ui.add_space(20.0);
                    ui.spinner();
                    ui.label("正在将安装文件复制到U盘...");
                    ui.add_space(10.0);
                    
                    // Display copy progress for each file
                    for progress in &self.copy_progress {
                        ui.group(|ui| {
                            ui.label(&progress.filename);
                            
                            let percent = if progress.total > 0 {
                                (progress.copied as f32 / progress.total as f32) * 100.0
                            } else {
                                0.0
                            };
                            
                            let progress_text = format!(
                                "{:.1}% ({:.2} MB / {:.2} MB)",
                                percent,
                                progress.copied as f32 / (1024.0 * 1024.0),
                                progress.total as f32 / (1024.0 * 1024.0)
                            );
                            
                            ui.add(egui::ProgressBar::new(percent / 100.0)
                                .text(progress_text));
                        });
                    }
                    
                    // Show current file being copied
                    if !self.current_copy_file.is_empty() {
                        ui.add_space(10.0);
                        ui.label(format!("当前文件: {}", self.current_copy_file));
                    }
                }
                
                AppState::Completed => {
                    ui.heading("[OK] 安装完成！");
                    ui.add_space(20.0);
                    
                    ui.colored_label(
                        egui::Color32::GREEN,
                        "ZundaLink 安装U盘制作成功！"
                    );
                    
                    ui.label("您的U盘现在可以用于安装 ZundaLink 系统。");
                    
                    ui.add_space(20.0);
                    
                    if ui.button("← 返回主页面").clicked() {
                        self.reset_all_state();
                        self.state = AppState::VersionSelection;
                    }
                }
                
                AppState::Error(ref msg) => {
                    ui.heading("[X] 错误");
                    ui.add_space(20.0);
                    
                    ui.colored_label(egui::Color32::RED, msg);
                    
                    ui.add_space(20.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("← 返回主页面").clicked() {
                            self.reset_all_state();
                            self.state = AppState::VersionSelection;
                        }
                    });
                }
            }
            
            // Show error message if any
            if let Some(ref error) = self.error_message {
                ui.add_space(20.0);
                ui.colored_label(egui::Color32::RED, format!("错误: {}", error));
            }

            // Show build info at the bottom
            ui.add_space(20.0);
            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                ui.label(format!("{}", crate::build_info::FULL_VERSION));
            });
        });
    }
}

impl ZundaLinkApp {
    fn start_download(&mut self) {
        // Prevent multiple download starts
        if self.download_started {
            return;
        }
        self.download_started = true;
        
        // Clear previous progress
        self.download_progress.clear();
        
        // Create channel for progress updates
        let (progress_tx, progress_rx) = mpsc::channel(100);
        self.progress_rx = Some(progress_rx);
        
        // Create channel for skip verification
        let (skip_tx, skip_rx) = mpsc::channel(1);
        self.skip_verify_tx = Some(skip_tx);
        
        // Initialize progress list and start download
        if let Some(ref version) = self.selected_version {
            if version.install_list.is_empty() {
                log::warn!("No files to download");
                return;
            }
            
            for file in &version.install_list {
                self.download_progress.push(DownloadProgress {
                    filename: file.filename.clone(),
                    downloaded: 0,
                    total: file.size,
                    status: DownloadStatus::Pending,
                    verify_progress: 0.0,
                });
            }
            
            // Clone data for the async task
            let files_to_download = version.install_list.clone();
            let runtime = self.runtime.clone();
            let tx = progress_tx.clone();

            // Get temp directory from app state
            let temp_dir = self.temp_dir.clone();

            // Spawn download task with concurrent file download support
            runtime.spawn(async move {
                // Ensure temp directory exists
                if let Err(e) = std::fs::create_dir_all(&temp_dir) {
                    log::error!("Failed to create temp directory: {}", e);
                    return;
                }

                // Use download_all_files for concurrent downloads (max 5 files at a time)
                match download_all_files(&temp_dir, &files_to_download, tx, skip_rx).await {
                    Ok(paths) => {
                        log::info!("All downloads completed: {} files", paths.len());
                    }
                    Err(e) => {
                        log::error!("Download failed: {}", e);
                    }
                }
            });
        } else {
            log::error!("No version selected for download");
        }
    }
    
    fn format_and_install(&mut self) {
        // Create channel for format/copy operation
        let (tx, rx) = mpsc::channel::<Result<String, String>>(1);
        self.format_copy_rx = Some(rx);
        
        // Get the selected USB device
        let device = self.selected_usb.clone();
        let manual_drive = self.manual_drive_letter.clone();
        let runtime = self.runtime.clone();
        
        // Spawn format task
        runtime.spawn(async move {
            let device_path = if let Some(ref usb) = device {
                usb.device_path.clone()
            } else if !manual_drive.is_empty() {
                let letter = manual_drive.trim_end_matches(':').to_ascii_uppercase();
                format!("\\\\.\\{}:", letter)
            } else {
                let _ = tx.send(Err("未选择U盘设备".to_string())).await;
                return;
            };
            
            log::info!("Starting format for device: {}", device_path);
            
            // Create formatter and format the drive
            let formatter = DiskFormatter::new();
            
            match formatter.format_usb_drive(&device_path, Some("Q:")) {
                Ok(target_drive) => {
                    log::info!("Format completed: {}", target_drive);
                    let _ = tx.send(Ok(format!("格式化完成: {}", target_drive))).await;
                }
                Err(e) => {
                    log::error!("Format failed: {}", e);
                    let _ = tx.send(Err(format!("格式化失败: {}", e))).await;
                }
            }
        });
    }
    
    fn start_copying(&mut self) {
        // Create channels for copy operation and progress
        let (tx, rx) = mpsc::channel::<Result<String, String>>(1);
        let (progress_tx, progress_rx) = mpsc::channel::<CopyProgress>(100);

        self.format_copy_rx = Some(rx);
        self.copy_progress_rx = Some(progress_rx);

        // Get downloaded files using the configured temp directory
        let temp_dir = self.temp_dir.clone();
        let downloaded_files: Vec<String> = self.download_progress.iter()
            .filter(|p| matches!(p.status, DownloadStatus::Completed))
            .map(|p| format!("{}/{}", temp_dir, p.filename))
            .collect();
        
        let runtime = self.runtime.clone();
        
        // Spawn copy task
        runtime.spawn(async move {
            if downloaded_files.is_empty() {
                let _ = tx.send(Err("没有可复制的文件".to_string())).await;
                return;
            }
            
            log::info!("Starting copy of {} files to Q:\\", downloaded_files.len());
            
            // Use copy with progress tracking
            let progress_tx = progress_tx.clone();
            match copy_files_to_drive_with_progress(&downloaded_files, "Q:", move |progress| {
                let _ = progress_tx.try_send(progress);
            }) {
                Ok(()) => {
                    log::info!("Copy completed successfully");
                    let _ = tx.send(Ok(format!("已复制 {} 个文件", downloaded_files.len()))).await;
                }
                Err(e) => {
                    log::error!("Copy failed: {}", e);
                    let _ = tx.send(Err(format!("复制失败: {}", e))).await;
                }
            }
        });
    }
    
    fn reset_download_state(&mut self) {
        self.download_started = false;
        self.download_progress.clear();
        self.progress_rx = None;
        // Also reset format/copy related state
        self.format_copy_rx = None;
        self.copy_progress_rx = None;
        self.format_copy_status.clear();
        self.copy_progress.clear();
        self.current_copy_file.clear();
        self.format_confirmed = false;
        self.show_format_warning = false;
    }
    
    fn reset_usb_selection(&mut self) {
        // Reset USB selection state
        self.selected_usb = None;
        self.manual_drive_letter.clear();
        self.usb_devices.clear();
        self.usb_rx = None;
    }
    
    fn reset_all_state(&mut self) {
        // Reset all state to allow starting over
        self.selected_version = None;
        self.reset_usb_selection();
        self.reset_download_state();
        self.error_message = None;
        // Note: We no longer clear the temp directory to allow resuming downloads
    }
}
