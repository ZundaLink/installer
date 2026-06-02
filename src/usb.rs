use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Create a Command with hidden console window on Windows
#[cfg(target_os = "windows")]
fn create_hidden_command(program: &str) -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut cmd = Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(target_os = "windows"))]
fn create_hidden_command(program: &str) -> Command {
    Command::new(program)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDevice {
    pub device_path: String,
    pub drive_letter: Option<String>,
    pub size: u64,
    pub model: String,
    pub is_removable: bool,
}

impl UsbDevice {
    pub fn size_gb(&self) -> f64 {
        self.size as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

pub struct UsbDetector;

impl UsbDetector {
    pub fn new() -> Self {
        Self
    }
    
    pub fn detect_usb_drives(&self) -> Result<Vec<UsbDevice>> {
        #[cfg(target_os = "windows")]
        {
            self.detect_windows()
        }
        
        #[cfg(target_os = "linux")]
        {
            self.detect_linux()
        }
        
        #[cfg(target_os = "macos")]
        {
            self.detect_macos()
        }
    }
    
    #[cfg(target_os = "windows")]
    fn detect_windows(&self) -> Result<Vec<UsbDevice>> {
        use std::ptr::null_mut;
        use winapi::um::fileapi::{CreateFileW, GetDriveTypeW, GetLogicalDrives, GetDiskFreeSpaceExW};
        use winapi::um::handleapi::INVALID_HANDLE_VALUE;
        use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, ULARGE_INTEGER};
        
        let mut devices = Vec::new();
        let drives = unsafe { GetLogicalDrives() };
        
        for i in 0..26 {
            if (drives >> i) & 1 == 1 {
                let drive_letter = format!("{}:", (b'A' + i as u8) as char);
                let drive_path: Vec<u16> = format!("{}\\", drive_letter).encode_utf16().chain(Some(0)).collect();
                
                let drive_type = unsafe { GetDriveTypeW(drive_path.as_ptr()) };
                
                // DRIVE_REMOVABLE = 2
                if drive_type == 2 {
                    let device_path = format!("\\\\.\\{}:", (b'A' + i as u8) as char);
                    let device_path_wide: Vec<u16> = device_path.encode_utf16().chain(Some(0)).collect();
                    
                    unsafe {
                        let handle = CreateFileW(
                            device_path_wide.as_ptr(),
                            GENERIC_READ,
                            FILE_SHARE_READ | FILE_SHARE_WRITE,
                            null_mut(),
                            3, // OPEN_EXISTING
                            0,
                            null_mut(),
                        );
                        
                        if handle != INVALID_HANDLE_VALUE {
                            // Get disk size using GetDiskFreeSpaceExW
                            let mut free_bytes_available: ULARGE_INTEGER = std::mem::zeroed();
                            let mut total_bytes: ULARGE_INTEGER = std::mem::zeroed();
                            let mut total_free_bytes: ULARGE_INTEGER = std::mem::zeroed();
                            
                            let size_result = GetDiskFreeSpaceExW(
                                drive_path.as_ptr(),
                                &mut free_bytes_available,
                                &mut total_bytes,
                                &mut total_free_bytes,
                            );
                            
                            let size = if size_result != 0 {
                                *total_bytes.QuadPart() as u64
                            } else {
                                0
                            };
                            
                            // Get device name using PowerShell
                            let model = Self::get_windows_device_name(&drive_letter);
                            
                            devices.push(UsbDevice {
                                device_path: device_path.clone(),
                                drive_letter: Some(drive_letter.clone()),
                                size,
                                model,
                                is_removable: true,
                            });
                            
                            winapi::um::handleapi::CloseHandle(handle);
                        }
                    }
                }
            }
        }
        
        Ok(devices)
    }
    
    #[cfg(target_os = "windows")]
    fn get_windows_device_name(drive_letter: &str) -> String {
        // First try to get the physical device model from Win32_DiskDrive
        let ps_script = format!(
            "Get-CimInstance -ClassName Win32_DiskDrive | Where-Object {{ $_.MediaType -like '*Removable*' }} | ForEach-Object {{ $_.Model }}"
        );
        
        if let Ok(output) = create_hidden_command("powershell")
            .args(&["-Command", &ps_script])
            .output() {
            if output.status.success() {
                let model = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !model.is_empty() {
                    return format!("{} ({})", model, drive_letter);
                }
            }
        }
        
        // Fallback: Get device model from Win32_LogicalDisk -> Win32_DiskPartition -> Win32_DiskDrive
        let ps_script2 = format!(
            "$disk = Get-CimInstance -ClassName Win32_LogicalDisk | Where-Object {{ $_.DeviceID -eq '{}' }}; \
             $partition = Get-CimInstance -Query \"ASSOCIATORS OF {{Win32_LogicalDisk.DeviceID='$($disk.DeviceID)'}} WHERE AssocClass=Win32_LogicalDiskToPartition\"; \
             $diskdrive = Get-CimInstance -Query \"ASSOCIATORS OF {{Win32_DiskPartition.DeviceID='$($partition.DeviceID)'}} WHERE AssocClass=Win32_DiskDriveToDiskPartition\"; \
             $diskdrive.Model",
            drive_letter.trim_end_matches(':')
        );
        
        if let Ok(output) = create_hidden_command("powershell")
            .args(&["-Command", &ps_script2])
            .output() {
            if output.status.success() {
                let model = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !model.is_empty() {
                    return format!("{} ({})", model, drive_letter);
                }
            }
        }
        
        // Last fallback: try wmic to get model
        let wmic_cmd = format!(
            "wmic diskdrive where \"MediaType='Removable Media'\" get Model /value"
        );
        
        if let Ok(output) = create_hidden_command("cmd")
            .args(&["/C", &wmic_cmd])
            .output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.starts_with("Model=") {
                        let model = line[6..].trim().to_string();
                        if !model.is_empty() {
                            return format!("{} ({})", model, drive_letter);
                        }
                    }
                }
            }
        }
        
        // Default fallback
        format!("可移动磁盘 {}", drive_letter)
    }
    
    #[cfg(target_os = "linux")]
    fn detect_linux(&self) -> Result<Vec<UsbDevice>> {
        let output = create_hidden_command("lsblk")
            .args(&["-J", "-o", "NAME,SIZE,TYPE,MODEL,TRAN,RM,MOUNTPOINT"])
            .output()?;
        
        let mut devices = Vec::new();
        
        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(blockdevices) = json.get("blockdevices").and_then(|v| v.as_array()) {
                    for device in blockdevices {
                        if let Some( tran) = device.get("tran").and_then(|v| v.as_str()) {
                            if tran == "usb" {
                                if let Some(name) = device.get("name").and_then(|v| v.as_str()) {
                                    let device_path = format!("/dev/{}", name);
                                    let size_str = device.get("size").and_then(|v| v.as_str()).unwrap_or("0");
                                    let size = Self::parse_size(size_str);
                                    let model = device.get("model").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                                    
                                    devices.push(UsbDevice {
                                        device_path,
                                        drive_letter: None,
                                        size,
                                        model,
                                        is_removable: true,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(devices)
    }
    
    #[cfg(target_os = "macos")]
    fn detect_macos(&self) -> Result<Vec<UsbDevice>> {
        let output = create_hidden_command("diskutil")
            .args(&["list", "-external", "physical"])
            .output()?;
        
        let mut devices = Vec::new();
        
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse diskutil output
            for line in output_str.lines() {
                if line.starts_with("/dev/disk") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 1 {
                        let device_path = parts[0].to_string();
                        
                        // Get disk info
                        let info_output = create_hidden_command("diskutil")
                            .args(&["info", "-plist", &device_path])
                            .output()?;
                        
                        if info_output.status.success() {
                            // Parse plist or simple output for size
                            let info_str = String::from_utf8_lossy(&info_output.stdout);
                            let size = Self::parse_macos_disk_size(&info_str);
                            
                            devices.push(UsbDevice {
                                device_path: device_path.clone(),
                                drive_letter: None,
                                size,
                                model: format!("External Disk {}", device_path),
                                is_removable: true,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(devices)
    }
    
    #[cfg(target_os = "linux")]
    fn parse_size(size_str: &str) -> u64 {
        let size_str = size_str.trim();
        if size_str.is_empty() {
            return 0;
        }
        
        let mut chars = size_str.chars().peekable();
        let mut num_str = String::new();
        
        while let Some(&c) = chars.peek() {
            if c.is_digit(10) || c == '.' {
                num_str.push(c);
                chars.next();
            } else {
                break;
            }
        }
        
        let unit: String = chars.collect();
        let num: f64 = num_str.parse().unwrap_or(0.0);
        
        match unit.to_uppercase().as_str() {
            "B" => num as u64,
            "K" | "KB" => (num * 1024.0) as u64,
            "M" | "MB" => (num * 1024.0 * 1024.0) as u64,
            "G" | "GB" => (num * 1024.0 * 1024.0 * 1024.0) as u64,
            "T" | "TB" => (num * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
            _ => num as u64,
        }
    }
    
    #[cfg(target_os = "macos")]
    fn parse_macos_disk_size(info_str: &str) -> u64 {
        // Look for TotalSize in plist output
        for line in info_str.lines() {
            if line.contains("TotalSize") {
                if let Some(start) = line.find("<integer>") {
                    if let Some(end) = line.find("</integer>") {
                        let size_str = &line[start + 9..end];
                        return size_str.parse().unwrap_or(0);
                    }
                }
            }
        }
        0
    }
}

pub fn select_usb_device(devices: &[UsbDevice], manual_drive: Option<&str>) -> Result<UsbDevice> {
    if let Some(drive) = manual_drive {
        // Manual drive specified
        let device_path = if drive.len() == 1 || (drive.len() == 2 && drive.ends_with(':')) {
            let letter = drive.chars().next().unwrap().to_ascii_uppercase();
            format!("\\\\.\\{}:", letter)
        } else {
            drive.to_string()
        };

        return Ok(UsbDevice {
            device_path,
            drive_letter: Some(drive.to_string()),
            size: 0,
            model: "Manual Selection".to_string(),
            is_removable: true,
        });
    }

    if devices.is_empty() {
        return Err(anyhow!("No USB devices detected"));
    }

    // Return first removable device
    devices.iter()
        .find(|d| d.is_removable)
        .cloned()
        .ok_or_else(|| anyhow!("No removable USB devices found"))
}

/// Get the free disk space for the drive containing the given path
pub fn get_disk_free_space(path: &str) -> Option<u64> {
    #[cfg(target_os = "windows")]
    {
        use std::path::Path;
        use std::os::windows::ffi::OsStrExt;
        use winapi::um::fileapi::GetDiskFreeSpaceExW;
        use winapi::um::winnt::ULARGE_INTEGER;

        let path = Path::new(path);
        // Get the root of the path (e.g., "C:\" or the path itself if it's already a root)
        let root = if path.is_absolute() {
            if let Some(prefix) = path.components().next() {
                let root_path: std::path::PathBuf = [prefix.as_os_str()].iter().collect();
                root_path
            } else {
                path.to_path_buf()
            }
        } else {
            // For relative paths like "./temp", get current directory's drive
            std::env::current_dir().ok()?.to_path_buf()
        };

        let root_wide: Vec<u16> = root.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();

        unsafe {
            let mut free_bytes_available: ULARGE_INTEGER = std::mem::zeroed();
            let mut total_bytes: ULARGE_INTEGER = std::mem::zeroed();
            let mut total_free_bytes: ULARGE_INTEGER = std::mem::zeroed();

            let result = GetDiskFreeSpaceExW(
                root_wide.as_ptr(),
                &mut free_bytes_available,
                &mut total_bytes,
                &mut total_free_bytes,
            );

            if result != 0 {
                Some(*free_bytes_available.QuadPart() as u64)
            } else {
                None
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;
        use libc::statvfs;

        let c_path = CString::new(path).ok()?;
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if statvfs(c_path.as_ptr(), &mut stat) == 0 {
                Some(stat.f_bavail * stat.f_frsize as u64)
            } else {
                None
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use libc::statvfs;

        let c_path = CString::new(path).ok()?;
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if statvfs(c_path.as_ptr(), &mut stat) == 0 {
                Some(stat.f_bavail * stat.f_frsize as u64)
            } else {
                None
            }
        }
    }
}
