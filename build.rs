use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    if target_os == "windows" {
        // Set the Windows application icon
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/zundalink.ico");
        res.compile().expect("Failed to compile Windows resources");
    }

    // Generate build info
    generate_build_info();
}

fn generate_build_info() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("build_info.g.rs");

    // Read version from Makefile
    let version = read_version_from_makefile();

    // Get git commit hash (short)
    let git_hash = get_git_hash();

    // Get build time
    let build_time = get_build_time();

    // Generate version string: v{VERSION}+{YYYYMMDD}.{GIT_HASH}
    let date_suffix = build_time.split(' ').next().unwrap_or("").replace("-", "");
    let full_version = format!("v{}+{}.{}", version, date_suffix, git_hash);

    let build_info = format!(
        r#"pub const VERSION: &str = "{}";
pub const BUILD_HASH: &str = "{}";
pub const BUILD_TIME: &str = "{}";
pub const FULL_VERSION: &str = "{}";
"#,
        version, git_hash, build_time, full_version
    );

    fs::write(&dest_path, build_info).expect("Failed to write build info");
}

fn read_version_from_makefile() -> String {
    let makefile_content = fs::read_to_string("Makefile")
        .unwrap_or_else(|_| "VERSION := 0.0.1".to_string());

    makefile_content
        .lines()
        .find(|line| line.starts_with("VERSION :="))
        .and_then(|line| line.split(":=").nth(1))
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|| "0.0.1".to_string())
}

fn get_git_hash() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn get_build_time() -> String {
    // Use environment variable if set (for reproducible builds in CI)
    if let Ok(timestamp) = env::var("SOURCE_DATE_EPOCH") {
        if let Ok(epoch) = timestamp.parse::<i64>() {
            return format_timestamp(epoch);
        }
    }

    // Otherwise use current time
    Command::new("date")
        .args(["+%Y-%m-%d %H:%M:%S"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(unix)]
fn format_timestamp(epoch: i64) -> String {
    use std::process::Command;

    Command::new("date")
        .args(["-d", &format!("@{}", epoch), "+%Y-%m-%d %H:%M:%S"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(windows)]
fn format_timestamp(_epoch: i64) -> String {
    // On Windows, fallback to current date if we can't format the timestamp
    Command::new("powershell")
        .args(["-Command", "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(not(any(unix, windows)))]
fn format_timestamp(_epoch: i64) -> String {
    "unknown".to_string()
}
