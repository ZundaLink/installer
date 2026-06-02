use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    
    if target_os == "windows" {
        // Set the Windows application icon
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/zundalink.ico");
        res.compile().expect("Failed to compile Windows resources");
    }
}
