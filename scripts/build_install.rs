//! Build orchestration script for assembling the INSTALL/ output tree.

use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("Starting Multiverse build orchestration...");

    // 1. Run cargo build
    let status = Command::new("cargo")
        .arg("build")
        .arg("--workspace")
        .status()
        .expect("Failed to execute cargo build");

    if !status.success() {
        eprintln!("Cargo build failed!");
        std::process::exit(1);
    }

    // 2. Create INSTALL/ directory tree
    let install_dir = Path::new("INSTALL");
    let dirs = [
        install_dir.join("efi/boot"),
        install_dir.join("boot"),
        install_dir.join("config"),
        install_dir.join("runtime"),
        install_dir.join("tools"),
    ];

    for d in &dirs {
        fs::create_dir_all(d).expect("Failed to create directory");
        println!("Created: {}", d.display());
    }

    // 3. Copy default config if present
    let config_src = install_dir.join("config/config.json");
    if config_src.exists() {
        println!("Config template ready at: {}", config_src.display());
    }

    println!("INSTALL/ tree successfully assembled!");
}
