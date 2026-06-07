use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Base64 encoded volume label
const VOLUME_LABEL_B64: &str = "U0VHQV9JTlM=";

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

pub struct DiskFormatter;

impl DiskFormatter {
    /// Get the volume label by decoding the base64 constant
    fn get_volume_label() -> String {
        BASE64.decode(VOLUME_LABEL_B64)
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
            .expect("Failed to decode volume label")
    }

    pub fn new() -> Self {
        Self
    }
    
    pub fn format_usb_drive(&self, device_path: &str, drive_letter: Option<&str>) -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            self.format_windows(device_path, drive_letter)
        }
        
        #[cfg(target_os = "linux")]
        {
            self.format_linux(device_path)
        }
        
        #[cfg(target_os = "macos")]
        {
            self.format_macos(device_path)
        }
    }
    
    #[cfg(target_os = "windows")]
    fn format_windows(&self, device_path: &str, drive_letter: Option<&str>) -> Result<String> {
        let target_drive = drive_letter.unwrap_or("Q:");
        let target_drive = if target_drive.ends_with(':') {
            target_drive.to_string()
        } else {
            format!("{}:", target_drive)
        };
        let target_letter = target_drive.trim_end_matches(':').to_ascii_uppercase();
        
        // Extract drive letter from device path to find the disk
        let current_drive_letter = if device_path.starts_with("\\\\.\\") {
            device_path.chars().nth(4).unwrap_or('C')
        } else {
            device_path.chars().next().unwrap_or('C')
        };
        
        // Step 1: Get disk number using PowerShell (most reliable method)
        let disk_number = self.get_disk_number_powershell(current_drive_letter)?;
        
        log::info!("Formatting disk {} to drive {}", disk_number, target_drive);
        
        // Step 2: Create diskpart script
        let volume_label = Self::get_volume_label();
        let diskpart_script = format!(
            "select disk {}\n\
             clean\n\
             create partition primary\n\
             format fs=exfat quick label={}\n\
             assign letter={}\n\
             exit",
            disk_number,
            volume_label,
            target_letter
        );
        
        let script_path = "diskpart_script.txt";
        fs::write(script_path, &diskpart_script)?;
        
        log::info!("Running diskpart with script: {}", diskpart_script);
        
        // Step 3: Execute diskpart with UTF-8 encoding to prevent garbled output
        let output = create_hidden_command("cmd")
            .args(&["/c", "chcp", "65001", ">nul", "&&", "diskpart", "/s", script_path])
            .output()?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        log::info!("DiskPart stdout: {}", stdout);
        log::info!("DiskPart stderr: {}", stderr);
        
        fs::remove_file(script_path)?;
        
        if !output.status.success() {
            return Err(anyhow!("DiskPart failed: {}\nStderr: {}", stdout, stderr));
        }
        
        // Step 4: Verify the drive was created
        std::thread::sleep(std::time::Duration::from_secs(3));
        
        // Check if the drive exists
        let drive_path = format!("{}\\", target_drive);
        if !std::path::Path::new(&drive_path).exists() {
            return Err(anyhow!("格式化后无法访问目标驱动器 {}", target_drive));
        }
        
        log::info!("Successfully formatted disk {} as {}", disk_number, target_drive);
        
        Ok(target_drive)
    }
    
    #[cfg(target_os = "windows")]
    fn get_disk_number_powershell(&self, drive_letter: char) -> Result<u32> {
        // Use PowerShell to get the disk number reliably
        let ps_script = format!(
            "$vol = Get-Volume -DriveLetter '{}'; \
             $part = Get-Partition | Where-Object {{ $_.AccessPaths -like '*{}:*' }}; \
             if ($part) {{ $part.DiskNumber }} else {{ -1 }}",
            drive_letter.to_ascii_uppercase(),
            drive_letter.to_ascii_uppercase()
        );
        
        log::info!("PowerShell script: {}", ps_script);
        
        let output = create_hidden_command("powershell")
            .args(&["-Command", &ps_script])
            .output()?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        log::info!("PowerShell output: {}", output_str);
        
        if output.status.success() {
            let trimmed = output_str.trim();
            if let Ok(disk_num) = trimmed.parse::<i32>() {
                if disk_num >= 0 {
                    return Ok(disk_num as u32);
                }
            }
        }
        
        // Try alternative PowerShell method
        let ps_script = format!(
            "(Get-Partition | Where-Object {{ $_.DriveLetter -eq '{}' }}).DiskNumber",
            drive_letter.to_ascii_uppercase()
        );
        
        let output = create_hidden_command("powershell")
            .args(&["-Command", &ps_script])
            .output()?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        log::info!("PowerShell alternative output: {}", output_str);
        
        if output.status.success() {
            let trimmed = output_str.trim();
            if let Ok(disk_num) = trimmed.parse::<u32>() {
                return Ok(disk_num);
            }
        }
        
        Err(anyhow!("无法通过PowerShell获取磁盘号，驱动器字母: {}", drive_letter))
    }
    
    #[cfg(target_os = "windows")]
    fn get_disk_number(&self, device_path: &str) -> Result<u32> {
        // Extract drive letter from device path (e.g., "\\\\.\\E:" -> 'E')
        let drive_letter = if device_path.starts_with("\\\\.\\") {
            device_path.chars().nth(4).unwrap_or('C')
        } else {
            device_path.chars().next().unwrap_or('C')
        };
        
        // Use wmic to find the disk number for this drive letter
        let output = create_hidden_command("wmic")
            .args(&[
                "path",
                "win32_logicaldisk",
                "where",
                &format!("DeviceID='{}:'", drive_letter),
                "get",
                "DeviceID,VolumeName",
                "/format:csv",
            ])
            .output()?;
        
        // Use diskpart to list disks and find the one with matching size
        let output = create_hidden_command("diskpart")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        
        // For now, use a more reliable method: get disk number from drive letter
        // by querying the partition -> disk relationship
        let output = create_hidden_command("wmic")
            .args(&[
                "path",
                "win32_logicaldisktopartition",
                "get",
                "Antecedent,Dependent",
                "/format:csv",
            ])
            .output()?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        
        // Parse to find the disk number
        // Format: \\COMPUTERNAME\root\cimv2:Win32_DiskPartition.DeviceID="Disk #0, Partition #1"
        for line in output_str.lines() {
            if line.contains(&format!("{}:", drive_letter)) {
                // Extract disk number from the line
                if let Some(start) = line.find("Disk #") {
                    let rest = &line[start + 6..];
                    if let Some(end) = rest.find(",") {
                        if let Ok(disk_num) = rest[..end].parse::<u32>() {
                            return Ok(disk_num);
                        }
                    }
                }
            }
        }
        
        // Alternative: use fsutil to get disk information
        let output = create_hidden_command("fsutil")
            .args(&["fsinfo", "drives"])
            .output()?;
        
        // If we still can't determine, try using the drive letter directly with diskpart
        // This is safer than guessing disk 0
        Err(anyhow!("无法确定磁盘号，请手动选择磁盘"))
    }
    
    #[cfg(target_os = "windows")]
    fn get_disk_number_from_drive_letter(&self, drive_letter: char) -> Result<u32> {
        // Use powershell to get the disk number
        let ps_script = format!(
            "(Get-Partition -DriveLetter {}).DiskNumber",
            drive_letter.to_ascii_uppercase()
        );
        
        let output = create_hidden_command("powershell")
            .args(&["-Command", &ps_script])
            .output()?;
        
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(disk_num) = output_str.trim().parse::<u32>() {
                return Ok(disk_num);
            }
        }
        
        Err(anyhow!("无法获取磁盘号，驱动器字母: {}", drive_letter))
    }
    
    #[cfg(target_os = "linux")]
    fn format_linux(&self, device_path: &str) -> Result<String> {
        // Unmount the device if mounted
        let _ = create_hidden_command("umount")
            .arg(device_path)
            .output();
        
        // Create new partition table
        let output = create_hidden_command("parted")
            .args(&["-s", device_path, "mklabel", "msdos"])
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow!("Failed to create partition table"));
        }
        
        // Create primary partition
        let output = create_hidden_command("parted")
            .args(&["-s", device_path, "mkpart", "primary", "fat32", "0%", "100%"])
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow!("Failed to create partition"));
        }
        
        // Format as exFAT
        let partition_path = format!("{}1", device_path);
        let volume_label = Self::get_volume_label();
        let output = create_hidden_command("mkfs.exfat")
            .args(&["-n", &volume_label, &partition_path])
            .output()?;
        
        if !output.status.success() {
            // Try mkexfatfs if mkfs.exfat is not available
            let output = create_hidden_command("mkexfatfs")
                .args(&["-n", &volume_label, &partition_path])
                .output()?;
            
            if !output.status.success() {
                return Err(anyhow!("Failed to format as exFAT"));
            }
        }
        
        // Mount the drive
        let mount_point = "/mnt/zundalink";
        fs::create_dir_all(mount_point)?;
        
        let output = create_hidden_command("mount")
            .args(&[&partition_path, mount_point])
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow!("Failed to mount drive"));
        }
        
        Ok(mount_point.to_string())
    }
    
    #[cfg(target_os = "macos")]
    fn format_macos(&self, device_path: &str) -> Result<String> {
        // Unmount the disk
        let output = create_hidden_command("diskutil")
            .args(&["unmountDisk", device_path])
            .output()?;
        
        // Erase and format as ExFAT
        let volume_label = Self::get_volume_label();
        let output = create_hidden_command("diskutil")
            .args(&[
                "eraseDisk",
                "ExFAT",
                &volume_label,
                device_path,
            ])
            .output()?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("diskutil failed: {}", error));
        }
        
        // Get mount point
        let output = create_hidden_command("diskutil")
            .args(&["info", "-plist", device_path])
            .output()?;
        
        let info_str = String::from_utf8_lossy(&output.stdout);
        let mount_point = Self::extract_mount_point(&info_str)
            .unwrap_or_else(|| format!("{}", device_path));
        
        Ok(mount_point)
    }
    
    #[cfg(target_os = "macos")]
    fn extract_mount_point(info_str: &str) -> Option<String> {
        // Look for MountPoint in plist output
        for line in info_str.lines() {
            if line.contains("MountPoint") {
                if let Some(start) = line.find("<string>") {
                    if let Some(end) = line.find("</string>") {
                        return Some(line[start + 8..end].to_string());
                    }
                }
            }
        }
        None
    }
}

use std::io::{Read, Write};

pub struct CopyProgress {
    pub filename: String,
    pub copied: u64,
    pub total: u64,
}

pub fn copy_files_to_drive_with_progress<F>(
    source_files: &[String],
    target_drive: &str,
    mut progress_callback: F,
) -> Result<()>
where
    F: FnMut(CopyProgress),
{
    for source_file in source_files {
        let file_name = Path::new(source_file)
            .file_name()
            .ok_or_else(|| anyhow!("Invalid source file path"))?
            .to_str()
            .ok_or_else(|| anyhow!("Invalid file name encoding"))?;
        
        let target_path = format!("{}\\{}", target_drive.trim_end_matches('\\'), file_name);
        
        // Get file size for progress tracking
        let metadata = fs::metadata(source_file)?;
        let total_size = metadata.len();
        
        // Copy with progress tracking
        copy_file_with_progress(source_file, &target_path, total_size, file_name, &mut progress_callback)?;
    }
    
    Ok(())
}

fn copy_file_with_progress<F>(
    source_path: &str,
    target_path: &str,
    total_size: u64,
    filename: &str,
    progress_callback: &mut F,
) -> Result<()>
where
    F: FnMut(CopyProgress),
{
    // Handle empty file (size == 0)
    if total_size == 0 {
        fs::File::create(target_path)?;
        progress_callback(CopyProgress {
            filename: filename.to_string(),
            copied: 0,
            total: 0,
        });
        return Ok(());
    }

    let mut source = fs::File::open(source_path)?;
    let mut target = fs::File::create(target_path)?;
    
    let mut buffer = vec![0u8; 1024 * 256]; // 256MB buffer
    let mut copied: u64 = 0;
    
    loop {
        let bytes_read = source.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        
        target.write_all(&buffer[..bytes_read])?;
        copied += bytes_read as u64;
        
        // Report progress
        progress_callback(CopyProgress {
            filename: filename.to_string(),
            copied,
            total: total_size,
        });
    }
    
    target.sync_all()?;
    
    Ok(())
}

// Keep the original function for backward compatibility
pub fn copy_files_to_drive(source_files: &[String], target_drive: &str) -> Result<()> {
    copy_files_to_drive_with_progress(source_files, target_drive, |_progress| {
        // No-op callback
    })
}
