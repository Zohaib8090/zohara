use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use tokio::process::Command;

#[derive(Deserialize, Debug)]
struct ScanResult {
    distribution: String,
    packages: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct TranslationDb {
    ubuntu: HashMap<String, String>,
    fedora: HashMap<String, String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Zohara Migrator Daemon...");
    
    // Hardcoded paths for demonstration purposes
    let scan_file = PathBuf::from("/tmp/zohara_scan_results.json");
    let db_file = PathBuf::from("/etc/zohara/translation_db.json");
    
    // Wait for the scan file to exist (in a real daemon, we'd use inotify or systemd dependencies)
    if !scan_file.exists() {
        println!("Scan results not found. Exiting daemon.");
        return Ok(());
    }
    
    // Load translation DB
    let db_content = fs::read_to_string(&db_file).context("Failed to read translation_db.json")?;
    let db: TranslationDb = serde_json::from_str(&db_content).context("Failed to parse translation_db.json")?;
    
    // Load Scan Results
    let scan_content = fs::read_to_string(&scan_file).context("Failed to read scan results")?;
    let scan: ScanResult = serde_json::from_str(&scan_content).context("Failed to parse scan results")?;
    
    println!("Loaded scan results for distribution: {}", scan.distribution);
    
    let mut arch_packages = Vec::new();
    
    // Translate packages
    let distro_map = if scan.distribution == "ubuntu" {
        &db.ubuntu
    } else {
        &db.fedora
    };
    
    for pkg in scan.packages {
        if let Some(arch_pkg) = distro_map.get(&pkg) {
            println!("Translated {} -> {}", pkg, arch_pkg);
            arch_packages.push(arch_pkg.clone());
        } else {
            println!("No translation found for {}", pkg);
            // Optionally, fallback to Zohara DEB Engine if no AUR/pacman equivalent exists
        }
    }
    
    if !arch_packages.is_empty() {
        println!("Installing translated packages: {:?}", arch_packages);
        // Execute pacman installation silently
        let status = Command::new("sudo")
            .arg("pacman")
            .arg("-S")
            .arg("--noconfirm")
            .args(&arch_packages)
            .status()
            .await?;
            
        if status.success() {
            println!("Successfully migrated packages.");
        } else {
            println!("Failed to install some packages.");
        }
    } else {
        println!("No packages to migrate.");
    }
    
    // Remove scan file to prevent re-running
    let _ = fs::remove_file(scan_file);
    
    Ok(())
}
