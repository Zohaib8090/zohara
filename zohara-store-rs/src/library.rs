//! The Library tab: the apps installed on this computer.
//!
//! Three sources, each app once: what the Store installed (a small record, `library.json` in the user's state
//! folder), the installed Flatpak apps, and the Arch packages that were installed on purpose and have a menu launcher.
//! Libraries, tools without a launcher and Zohara's own programs are not listed.

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

// ── Arch apps: installed on purpose, with a launcher ─────────────────────────

/// `pacman -Ql` lines (`package /path`) -> (package, desktop file) for the launchers in /usr/share/applications.
pub fn parse_pacman_ql(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let (pkg, path) = l.split_once(' ')?;
            let path = path.trim();
            (path.starts_with("/usr/share/applications/") && path.ends_with(".desktop") && !path["/usr/share/applications/".len()..].contains('/'))
                .then(|| (pkg.to_string(), path.to_string()))
        })
        .collect()
}

/// (name, icon, comment) of a launcher a person would see in the menu; None for hidden entries and settings panels.
pub fn parse_desktop(text: &str) -> Option<(String, String, String)> {
    let (mut name, mut icon, mut comment) = (None, String::new(), String::new());
    let mut in_entry = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        match k {
            "Name" => name = Some(v.trim().to_string()),
            "Icon" => icon = v.trim().to_string(),
            "Comment" => comment = v.trim().to_string(),
            "NoDisplay" | "Hidden" if v.trim() == "true" => return None,
            "Type" if v.trim() != "Application" => return None,
            "Categories" if v.split(';').any(|c| c == "Settings") => return None,
            _ => {}
        }
    }
    name.filter(|n| !n.is_empty()).map(|n| (n, icon, comment))
}

fn arch_app(pkg: &str, name: String, icon: String, comment: String) -> AppInfo {
    AppInfo {
        id: pkg.to_string(),
        name,
        publisher: "Arch Linux".into(),
        description: comment,
        icon_name: if icon.is_empty() { pkg.to_string() } else { icon },
        source: AppSource::Pacman,
        package_name: pkg.to_string(),
        category: AppCategory::Utilities,
        rating: 0.0,
    }
}

/// Parts of the desktop itself. They have launchers (an emoji picker, a login screen) but removing them would
/// break the session, so the Library never offers them.
fn is_protected(pkg: &str) -> bool {
    pkg.starts_with("zohara-")
        || pkg.starts_with("linux")
        || matches!(
            pkg,
            "plasma-desktop" | "plasma-workspace" | "plasma-meta" | "kwin" | "systemsettings" | "sddm" | "networkmanager" | "pacman" | "systemd"
        )
}

/// A package with several launchers (LibreOffice has one per program): the one that stands for the whole package.
/// A launcher named like the package wins, then a "start center" or "main" one, then the shortest name.
pub fn pick_launcher<'a>(pkg: &str, paths: &'a [String]) -> Option<&'a String> {
    let stem = |p: &str| p.rsplit('/').next().unwrap_or(p).trim_end_matches(".desktop").to_string();
    paths.iter().min_by_key(|p| {
        let s = stem(p);
        let rank = if s == pkg || s.ends_with(&format!(".{pkg}")) {
            0
        } else if s.contains("startcenter") || s.ends_with("main") {
            1
        } else {
            2
        };
        (rank, s.len(), s)
    })
}

/// Packages installed on purpose that have a menu launcher, as apps. Runs two pacman commands and reads the
/// launchers, so call it off the UI thread.
pub fn arch_apps() -> Vec<AppInfo> {
    use std::process::{Command, Stdio};
    let explicit = match Command::new("pacman").arg("-Qeq").stderr(Stdio::null()).output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty() && !is_protected(l)).collect::<Vec<_>>(),
        Err(_) => return Vec::new(),
    };
    if explicit.is_empty() {
        return Vec::new();
    }
    let Ok(o) = Command::new("pacman").arg("-Ql").args(&explicit).stderr(Stdio::null()).output() else { return Vec::new() };
    let mut by_package: Vec<(String, Vec<String>)> = Vec::new();
    for (pkg, path) in parse_pacman_ql(&String::from_utf8_lossy(&o.stdout)) {
        match by_package.iter_mut().find(|(p, _)| *p == pkg) {
            Some((_, paths)) => paths.push(path),
            None => by_package.push((pkg, vec![path])),
        }
    }
    let mut apps = Vec::new();
    for (pkg, paths) in by_package {
        // One entry per package: its main launcher, or failing that the first one that is a real app.
        let mut order: Vec<&String> = pick_launcher(&pkg, &paths).into_iter().collect();
        order.extend(paths.iter());
        let found = order
            .into_iter()
            .find_map(|p| std::fs::read_to_string(p).ok().as_deref().and_then(parse_desktop));
        if let Some((name, icon, comment)) = found {
            apps.push(arch_app(&pkg, name, icon, comment));
        }
    }
    apps
}

/// What the tab lists: the record, the Arch apps and the installed Flatpak apps, each once (the record wins), still
/// installed according to `is_installed`, by name.
pub fn entries(
    recorded: Vec<AppInfo>,
    arch: Vec<AppInfo>,
    flatpaks: &[FlatpakInstalled],
    is_installed: impl Fn(&AppSource, &str) -> bool,
) -> Vec<AppInfo> {
    let mut all: Vec<AppInfo> = Vec::new();
    let candidates = recorded.into_iter().chain(arch).chain(flatpaks.iter().map(from_flatpak));
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
    fn lists_recorded_arch_and_flatpak_apps_once_each_sorted_by_name() {
        let recorded = vec![app("Zeta", AppSource::Pacman, "zeta"), app("Firefox", AppSource::Pacman, "firefox"), app("Gimp", AppSource::Pacman, "gimp")];
        let flatpaks = vec![fp("org.example.Cool", "Cool App"), fp("org.example.Cool", "Cool App")];
        let installed = |s: &AppSource, p: &str| matches!((s, p), (AppSource::Pacman, "zeta" | "firefox") | (AppSource::Flatpak, "org.example.Cool"));
        let arch = vec![app("Mozilla Firefox", AppSource::Pacman, "firefox"), app("Kate", AppSource::Pacman, "zeta")];
        let names: Vec<String> = entries(recorded, arch, &flatpaks, installed).into_iter().map(|a| a.name).collect();
        // Listed once each (the recorded Firefox and Zeta win); Gimp is recorded but no longer installed.
        assert_eq!(names, vec!["Cool App", "Firefox", "Zeta"]);
    }

    #[test]
    fn an_app_removed_elsewhere_drops_out_even_if_recorded() {
        let recorded = vec![app("Gone", AppSource::Pacman, "gone")];
        assert!(entries(recorded, Vec::new(), &[], |_, _| false).is_empty());
    }

    #[test]
    fn only_launchers_directly_in_the_applications_folder_are_taken_from_pacman_ql() {
        let text = "firefox /usr/\nfirefox /usr/share/applications/firefox.desktop\nfirefox /usr/share/applications/\nkate /usr/share/applications/org.kde.kate.desktop\nfoo /usr/share/applications/sub/x.desktop\nfoo /usr/bin/foo\n";
        assert_eq!(
            parse_pacman_ql(text),
            vec![("firefox".to_string(), "/usr/share/applications/firefox.desktop".to_string()), ("kate".to_string(), "/usr/share/applications/org.kde.kate.desktop".to_string())]
        );
    }

    #[test]
    fn a_menu_launcher_gives_name_icon_and_comment() {
        let d = "[Desktop Entry]\nType=Application\nName=Kate\nName[de]=Kate DE\nComment=Edit text\nIcon=kate\nCategories=Utility;\n\n[Desktop Action new]\nName=New window\n";
        assert_eq!(parse_desktop(d), Some(("Kate".into(), "kate".into(), "Edit text".into())));
    }

    #[test]
    fn hidden_entries_and_settings_panels_are_not_apps() {
        assert_eq!(parse_desktop("[Desktop Entry]\nType=Application\nName=X\nNoDisplay=true\n"), None);
        assert_eq!(parse_desktop("[Desktop Entry]\nType=Application\nName=Display\nCategories=Qt;KDE;Settings;\n"), None);
        assert_eq!(parse_desktop("[Desktop Entry]\nType=Link\nName=A link\n"), None);
        assert_eq!(parse_desktop("[Desktop Entry]\nType=Application\n"), None);
    }

    #[test]
    fn the_launcher_that_stands_for_the_whole_package_is_chosen() {
        let p = |v: &[&str]| v.iter().map(|s| format!("/usr/share/applications/{s}")).collect::<Vec<_>>();
        let office = p(&["libreoffice-base.desktop", "libreoffice-writer.desktop", "libreoffice-startcenter.desktop"]);
        assert!(pick_launcher("libreoffice-fresh", &office).unwrap().ends_with("libreoffice-startcenter.desktop"));
        let named = p(&["org.kde.kate.desktop", "kate.desktop", "x.desktop"]);
        assert!(pick_launcher("kate", &named).unwrap().ends_with("/kate.desktop"));
        assert!(pick_launcher("kate", &[]).is_none());
    }

    #[test]
    fn the_desktop_itself_is_never_offered_for_removal() {
        for p in ["plasma-desktop", "zohara-settings", "linux-zen", "sddm", "kwin"] {
            assert!(is_protected(p), "{p}");
        }
        assert!(!is_protected("kate"));
    }

    #[test]
    fn the_record_round_trips_as_json() {
        let a = app("Firefox", AppSource::Pacman, "firefox");
        let text = serde_json::to_string(&vec![a.clone()]).unwrap();
        let back: Vec<AppInfo> = serde_json::from_str(&text).unwrap();
        assert!(same(&back[0], &AppSource::Pacman, "firefox"));
    }

    /// Reads this computer's real packages. `cargo test arch_apps_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn arch_apps_live() {
        let apps = arch_apps();
        for a in &apps {
            println!("{:<28} {:<24} icon={}", a.name, a.package_name, a.icon_name);
        }
        println!("{} apps", apps.len());
        assert!(!apps.iter().any(|a| a.package_name.starts_with("zohara-")));
    }
}
