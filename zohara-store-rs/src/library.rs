//! The Library tab: the apps installed with the Store.
//!
//! The Store keeps a small record of what it installs (`library.json` in the user's state folder), so an app from
//! any source shows up here, not only the ones in the curated list. Flatpak apps and curated apps that are installed
//! are included too, because they were installed through the Store before the record existed.

use std::path::PathBuf;

use crate::app_info::{AppCategory, AppInfo, AppSource};
use crate::updates::FlatpakInstalled;

fn record_path() -> PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/state"));
    base.join("zohara-store").join("library.json")
}

pub fn load_record() -> Vec<AppInfo> {
    std::fs::read_to_string(record_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save_record(apps: &[AppInfo]) {
    let path = record_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(apps) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(tmp, path);
        }
    }
}

fn same(a: &AppInfo, source: &AppSource, package: &str) -> bool {
    a.source == *source && a.package_name == package
}

/// The Store installed `app`: remember it.
pub fn record_install(app: &AppInfo) {
    let mut apps = load_record();
    apps.retain(|a| !same(a, &app.source, &app.package_name));
    apps.push(app.clone());
    save_record(&apps);
}

/// The Store removed an app: forget it.
pub fn record_removal(source: &AppSource, package: &str) {
    let mut apps = load_record();
    let before = apps.len();
    apps.retain(|a| !same(a, source, package));
    if apps.len() != before {
        save_record(&apps);
    }
}

fn from_flatpak(f: &FlatpakInstalled) -> AppInfo {
    AppInfo {
        id: f.app_id.clone(),
        name: f.name.clone(),
        publisher: if f.origin.is_empty() { "Flatpak".into() } else { f.origin.clone() },
        description: String::new(),
        icon_name: f.app_id.clone(),
        source: AppSource::Flatpak,
        package_name: f.app_id.clone(),
        category: AppCategory::Utilities,
        rating: 0.0,
    }
}

/// What the tab lists: the record, the installed curated apps and the installed Flatpak apps, each once, still
/// installed according to `is_installed`, by name.
pub fn entries(
    recorded: Vec<AppInfo>,
    curated: &[AppInfo],
    flatpaks: &[FlatpakInstalled],
    is_installed: impl Fn(&AppSource, &str) -> bool,
) -> Vec<AppInfo> {
    let mut all: Vec<AppInfo> = Vec::new();
    let candidates = recorded.into_iter().chain(curated.iter().cloned()).chain(flatpaks.iter().map(from_flatpak));
    for app in candidates {
        if is_installed(&app.source, &app.package_name) && !all.iter().any(|a| same(a, &app.source, &app.package_name)) {
            all.push(app);
        }
    }
    all.sort_by_key(|a| a.name.to_lowercase());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, source: AppSource, pkg: &str) -> AppInfo {
        AppInfo {
            id: pkg.into(),
            name: name.into(),
            publisher: "x".into(),
            description: String::new(),
            icon_name: pkg.into(),
            source,
            package_name: pkg.into(),
            category: AppCategory::Utilities,
            rating: 0.0,
        }
    }
    fn fp(id: &str, name: &str) -> FlatpakInstalled {
        FlatpakInstalled { app_id: id.into(), name: name.into(), origin: "flathub".into() }
    }

    #[test]
    fn lists_recorded_curated_and_flatpak_apps_once_each_sorted_by_name() {
        let recorded = vec![app("Zeta", AppSource::Pacman, "zeta"), app("Firefox", AppSource::Pacman, "firefox")];
        let curated = vec![app("Mozilla Firefox", AppSource::Pacman, "firefox"), app("VLC", AppSource::Pacman, "vlc"), app("GIMP", AppSource::Pacman, "gimp")];
        let flatpaks = vec![fp("org.example.Cool", "Cool App")];
        let installed = |s: &AppSource, p: &str| matches!((s, p), (AppSource::Pacman, "zeta" | "firefox" | "vlc") | (AppSource::Flatpak, "org.example.Cool"));
        let names: Vec<String> = entries(recorded, &curated, &flatpaks, installed).into_iter().map(|a| a.name).collect();
        // Firefox appears once (the recorded entry wins); GIMP is not installed so it is left out.
        assert_eq!(names, vec!["Cool App", "Firefox", "VLC", "Zeta"]);
    }

    #[test]
    fn an_app_removed_elsewhere_drops_out_even_if_recorded() {
        let recorded = vec![app("Gone", AppSource::Pacman, "gone")];
        assert!(entries(recorded, &[], &[], |_, _| false).is_empty());
    }

    #[test]
    fn the_record_round_trips_as_json() {
        let a = app("Firefox", AppSource::Pacman, "firefox");
        let text = serde_json::to_string(&vec![a.clone()]).unwrap();
        let back: Vec<AppInfo> = serde_json::from_str(&text).unwrap();
        assert!(same(&back[0], &AppSource::Pacman, "firefox"));
    }
}
