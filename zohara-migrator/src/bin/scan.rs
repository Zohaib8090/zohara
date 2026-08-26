use clap::Parser;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "zohara-scan")]
#[command(about = "Scans existing Linux installations to generate a package list for migration", long_about = None)]
struct Cli {
    // Must live outside /tmp: zohara-migrator.service consumes this file at boot,
    // and systemd clears /tmp on boot, so a /tmp default could never be read.
    #[arg(short, long, default_value = "/var/lib/zohara/scan_results.json")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("Scanning system for installed packages...");
    
    // In a real implementation, this would detect the distribution (Debian/Ubuntu, Fedora) 
    // and run `dpkg-query` or `rpm -qa` to extract packages.
    // For this demonstration, we'll output a mock list of detected packages.
    
    let detected_packages = json!({
        "distribution": "ubuntu",
        "packages": [
            "google-chrome-stable",
            "vlc",
            "python3",
            "htop"
        ]
    });
    
    let json_str = serde_json::to_string_pretty(&detected_packages)?;
    // fs::write does not create intermediate directories, and the default output
    // now lives under /var/lib/zohara which may not exist yet.
    if let Some(parent) = cli.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&cli.output, json_str)?;
    
    println!("Scan complete. Results saved to: {}", cli.output.display());
    Ok(())
}
