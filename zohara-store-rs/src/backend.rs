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

/// Only one package operation at a time, across the whole Store (installs,
/// removals and the Updates page). pacman holds a database lock while it
/// works; a second pacman started meanwhile fails immediately with "unable
/// to lock database", which used to make the second of two quick installs
/// fail with no message. Waiting here queues it instead.
pub static PACKAGE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Runs a package command and turns its failure into a sentence for the user.
fn run_pkg(mut cmd: Command) -> Result<(), String> {
    let out = cmd.stdin(Stdio::null()).output().map_err(|e| format!("Couldn't start the installer ({e})"))?;
    if out.status.success() {
        return Ok(());
    }
    if matches!(out.status.code(), Some(126) | Some(127)) {
        return Err("The password prompt was cancelled, so nothing was changed.".into());
    }
    Err(explain_failure(&String::from_utf8_lossy(&out.stderr)))
}

/// A readable reason from pacman/flatpak's error output.
pub fn explain_failure(stderr: &str) -> String {
    let e = stderr.to_lowercase();
    if e.contains("unable to lock database") {
        "Another program is installing or updating software right now. Try again when it has finished.".into()
    } else if e.contains("target not found") || e.contains("no remote refs found") || e.contains("nothing matches") {
        "This app isn't available from Zohara's software sources right now.".into()
    } else if e.contains("failed retrieving file") || e.contains("could not resolve host") || e.contains("couldn't resolve") {
        "Couldn't download it. Check your internet connection and try again.".into()
    } else if e.contains("not enough free disk space") || e.contains("no space left") {
        "There isn't enough free disk space.".into()
    } else if e.contains("conflicting files") || e.contains("exists in filesystem") || e.contains("conflicts with") {
        "It conflicts with software that's already installed.".into()
    } else if e.contains("invalid or corrupted package") || e.contains("signature") {
        "The download didn't pass its integrity check, so it wasn't installed.".into()
    } else {
        let last = stderr.lines().rev().map(str::trim).find(|l| !l.is_empty()).unwrap_or("unknown error");
        format!("It didn't finish ({last}).")
    }
}

/// Runs pacman as administrator: non-interactive sudo first (the live ISO),
/// then pkexec (asks for the password on an installed system).
fn pacman_admin(args: &[&str]) -> Result<(), String> {
    let mut sudo = Command::new("sudo");
    sudo.arg("-n").arg("pacman").args(args);
    if let Ok(o) = sudo.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status() {
        if o.success() {
            return Ok(());
        }
    }
    let mut pk = Command::new("pkexec");
    pk.arg("pacman").args(args);
    run_pkg(pk)
}

pub fn install_app(source: &AppSource, package_name: &str) -> Result<(), String> {
    let _guard = PACKAGE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    match source {
        AppSource::Pacman => pacman_admin(&["-S", "--noconfirm", "--needed", package_name]),
        AppSource::Flatpak => {
            let mut c = Command::new("flatpak");
            c.args(["install", "-y", "--noninteractive", "flathub", package_name]);
            run_pkg(c)
        }
    }
}

pub fn remove_app(source: &AppSource, package_name: &str) -> Result<(), String> {
    let _guard = PACKAGE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    match source {
        AppSource::Pacman => pacman_admin(&["-Rs", "--noconfirm", package_name]),
        AppSource::Flatpak => {
            let mut c = Command::new("flatpak");
            c.args(["uninstall", "-y", "--noninteractive", package_name]);
            run_pkg(c)
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

#[cfg(test)]
mod tests {
    use super::explain_failure;

    #[test]
    fn common_failures_read_as_sentences() {
        assert!(explain_failure("error: failed to init transaction (unable to lock database)").contains("Another program"));
        assert!(explain_failure("error: target not found: vscodium-bin").contains("isn't available"));
        assert!(explain_failure("error: failed retrieving file 'x.pkg.tar.zst' from mirror").contains("internet"));
        assert!(explain_failure("error: failed to commit transaction (conflicting files)\nfoo: /usr/bin/foo exists in filesystem").contains("conflicts"));
    }

    #[test]
    fn unknown_failure_keeps_the_last_line() {
        assert_eq!(explain_failure("warning: x\nerror: something odd\n\n"), "It didn't finish (error: something odd).");
        assert_eq!(explain_failure(""), "It didn't finish (unknown error).");
    }
}
