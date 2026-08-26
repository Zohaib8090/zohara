use std::collections::HashSet;
use std::process::{Command, Stdio};

use crate::app_info::{AppCategory, AppSource};

/// Build a set of ALL installed pacman packages (one `pacman -Qq` call).
/// Build a set of ALL installed flatpak apps (one `flatpak list` call).
/// This replaces per-app spawns — 2 calls total instead of N.
pub struct InstalledCache {
    pacman: HashSet<String>,
    flatpak: HashSet<String>,
}

impl InstalledCache {
    pub fn load() -> Self {
        let pacman = Self::load_pacman();
        let flatpak = Self::load_flatpak();
        InstalledCache { pacman, flatpak }
    }

    fn load_pacman() -> HashSet<String> {
        let out = Command::new("pacman")
            .arg("-Qq")
            .stdout(std::process::Stdio::piped())
            .stderr(Stdio::null())
            .output();
        match out {
            Ok(o) => String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .collect(),
            Err(_) => HashSet::new(),
        }
    }

    fn load_flatpak() -> HashSet<String> {
        // `flatpak list --app --columns=application` gives one app-id per line
        let out = Command::new("flatpak")
            .args(["list", "--app", "--columns=application"])
            .stdout(std::process::Stdio::piped())
            .stderr(Stdio::null())
            .output();
        match out {
            Ok(o) => String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .collect(),
            Err(_) => HashSet::new(),
        }
    }

    pub fn is_installed(&self, source: &AppSource, package_name: &str) -> bool {
        match source {
            AppSource::Pacman  => self.pacman.contains(package_name),
            AppSource::Flatpak => self.flatpak.contains(package_name),
        }
    }
}

pub fn install_app(source: &AppSource, package_name: &str) -> bool {
    match source {
        AppSource::Pacman => {
            // 1. Try non-interactive sudo (works instantly in Live ISO / NOPASSWD environment)
            let sudo_out = Command::new("sudo")
                .arg("-n")
                .arg("pacman")
                .arg("-S")
                .arg("--noconfirm")
                .arg("--needed")
                .arg(package_name)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .status();

            if let Ok(s) = sudo_out {
                if s.success() {
                    return true;
                }
            }

            // 2. Fall back to pkexec (triggers Polkit GUI authentication prompt in installed OS)
            let pkexec_out = Command::new("pkexec")
                .arg("pacman")
                .arg("-S")
                .arg("--noconfirm")
                .arg("--needed")
                .arg(package_name)
                .stdin(Stdio::null())
                .status();

            pkexec_out.map(|s| s.success()).unwrap_or(false)
        }
        AppSource::Flatpak => {
            let out = Command::new("flatpak")
                .arg("install")
                .arg("-y")
                .arg("flathub")
                .arg(package_name)
                .stdin(Stdio::null())
                .status();
            out.map(|s| s.success()).unwrap_or(false)
        }
    }
}

pub fn remove_app(source: &AppSource, package_name: &str) -> bool {
    match source {
        AppSource::Pacman => {
            let sudo_out = Command::new("sudo")
                .arg("-n")
                .arg("pacman")
                .arg("-Rs")
                .arg("--noconfirm")
                .arg(package_name)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .status();

            if let Ok(s) = sudo_out {
                if s.success() {
                    return true;
                }
            }

            let pkexec_out = Command::new("pkexec")
                .arg("pacman")
                .arg("-Rs")
                .arg("--noconfirm")
                .arg(package_name)
                .stdin(Stdio::null())
                .status();

            pkexec_out.map(|s| s.success()).unwrap_or(false)
        }
        AppSource::Flatpak => {
            let out = Command::new("flatpak")
                .arg("uninstall")
                .arg("-y")
                .arg(package_name)
                .stdin(Stdio::null())
                .status();
            out.map(|s| s.success()).unwrap_or(false)
        }
    }
}

pub fn search_apps(query: &str) -> Vec<crate::app_info::AppInfo> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return vec![];
    }

    let mut results = Vec::new();
    let mut seen_ids = HashSet::new();

    // 1. Search curated apps catalog first
    for app in crate::app_info::get_curated_apps() {
        if app.name.to_lowercase().contains(&q)
            || app.id.to_lowercase().contains(&q)
            || app.package_name.to_lowercase().contains(&q)
            || app.description.to_lowercase().contains(&q)
        {
            seen_ids.insert(app.id.clone());
            results.push(app);
        }
    }

    // 2. Search pacman repository
    let out = Command::new("pacman")
        .arg("-Ss")
        .arg(&q)
        .output();

    if let Ok(output) = out {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let pkg_line = lines[i];
            if !pkg_line.starts_with(' ') && !pkg_line.is_empty() {
                let desc_line = if i + 1 < lines.len() {
                    lines[i + 1].trim()
                } else {
                    ""
                };

                if let Some(space_idx) = pkg_line.find(' ') {
                    let full_name = &pkg_line[..space_idx];
                    if let Some(slash_idx) = full_name.find('/') {
                        let package_name = &full_name[slash_idx + 1..];

                        if !seen_ids.contains(package_name) {
                            seen_ids.insert(package_name.to_string());
                            let mut name = package_name.to_string();
                            if name.ends_with("-bin") {
                                name = name.replace("-bin", "");
                            }
                            if let Some(first) = name.get_mut(0..1) {
                                first.make_ascii_uppercase();
                            }

                            results.push(crate::app_info::AppInfo {
                                id: package_name.to_string(),
                                name,
                                publisher: "Arch Repository".to_string(),
                                description: desc_line.to_string(),
                                icon_name: package_name.to_string(),
                                source: AppSource::Pacman,
                                package_name: package_name.to_string(),
                                category: AppCategory::Utilities,
                                rating: 0.0,
                            });
                        }
                    }
                }
            }
            i += 1;
        }
    }

    results
}
