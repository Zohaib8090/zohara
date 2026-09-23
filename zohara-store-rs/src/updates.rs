//! Update detection, desktop notifications, and per-app "restore previous
//! version" for zohara-store.
//!
//! Design (see zohara-settings/docs/UI-REDESIGN.md for the fuller writeup):
//! pacman's own package cache (`/var/cache/pacman/pkg/`) plus GitHub
//! Releases (which keep every past asset by default) already ARE a restore
//! point for any package -- there's no custom snapshot system here, just
//! `pacman -U` against either the cached .pkg.tar.zst or a re-downloaded
//! copy of the exact previous release asset.
//!
//! First-party apps (publisher == "Zohara OS Team" in zohara-packages'
//! apps.json) always notify on update. Third-party apps only notify if the
//! user opted in for that specific app.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

const APPS_JSON_URL: &str =
    "https://raw.githubusercontent.com/Zohaib8090/zohara-packages/main/apps.json";
const FIRST_PARTY_PUBLISHER: &str = "Zohara OS Team";

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogVersion {
    pub version: String,
    #[serde(default)]
    pub changelog: String,
    #[serde(default)]
    pub download_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogApp {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub package: String,
    pub current_version: String,
    #[serde(default)]
    pub versions: Vec<CatalogVersion>,
}

#[derive(Debug, Clone, Deserialize)]
struct Catalog {
    apps: Vec<CatalogApp>,
}

/// One update the user hasn't installed yet.
#[derive(Debug, Clone)]
pub struct PendingUpdate {
    pub app: CatalogApp,
    pub installed_version: String,
    pub is_first_party: bool,
}

/// Per-app local state: whether a third-party app should notify on update,
/// and the last version we saw installed (so a later update can be rolled
/// back to it).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppState {
    #[serde(default)]
    notify_on_update: bool,
    #[serde(default)]
    last_known_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoreState {
    #[serde(default)]
    apps: HashMap<String, AppState>,
}

fn state_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config")
        });
    base.join("zohara").join("store-state.json")
}

fn load_state() -> StoreState {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_state(state: &StoreState) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, s);
    }
}

pub fn notify_enabled(app_id: &str) -> bool {
    load_state()
        .apps
        .get(app_id)
        .map(|a| a.notify_on_update)
        .unwrap_or(false)
}

pub fn set_notify_enabled(app_id: &str, enabled: bool) {
    let mut state = load_state();
    state.apps.entry(app_id.to_string()).or_default().notify_on_update = enabled;
    save_state(&state);
}

fn fetch_catalog() -> Vec<CatalogApp> {
    let out = Command::new("curl")
        .args(["-sL", "--max-time", "10", APPS_JSON_URL])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            serde_json::from_slice::<Catalog>(&o.stdout)
                .map(|c| c.apps)
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

fn installed_version(package: &str) -> Option<String> {
    let out = Command::new("pacman").args(["-Q", package]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    // pacman -Q prints "pkgname 1.2.3-1"
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .nth(1)
        .map(|s| s.to_string())
}

/// Real update check: fetches the live catalog and compares each app's
/// `current_version` against what pacman actually has installed. Only
/// apps that are actually installed are considered pending -- an update
/// to something you don't have isn't a pending update.
pub fn check_for_updates() -> Vec<PendingUpdate> {
    let catalog = fetch_catalog();
    let mut pending = Vec::new();
    for app in catalog {
        if let Some(installed) = installed_version(&app.package) {
            if installed != app.current_version {
                pending.push(PendingUpdate {
                    is_first_party: app.publisher == FIRST_PARTY_PUBLISHER,
                    installed_version: installed,
                    app,
                });
            }
        }
    }
    pending
}

fn send_notification(title: &str, body: &str) {
    let _ = Command::new("notify-send").arg(title).arg(body).spawn();
}

/// Fires a desktop notification for every pending update that should
/// notify: first-party ALWAYS does; third-party only if the user opted in
/// via `set_notify_enabled`. Safe to call from a background check (store
/// launch, or `zohara-store --check-updates` on a systemd user timer) --
/// does not touch any GTK state.
pub fn notify_pending_updates(pending: &[PendingUpdate]) {
    for update in pending {
        let should_notify = update.is_first_party || notify_enabled(&update.app.id);
        if should_notify {
            send_notification(
                &format!("{} update available", update.app.name),
                &format!("{} \u{2192} {}", update.installed_version, update.app.current_version),
            );
        }
    }
}

/// Records the version currently installed BEFORE applying an update, so
/// `restore_previous_version` has something to roll back to. Call this
/// right before installing an update for `app_id`.
pub fn record_pre_update_version(app_id: &str, version: &str) {
    let mut state = load_state();
    state.apps.entry(app_id.to_string()).or_default().last_known_version = Some(version.to_string());
    save_state(&state);
}

pub fn previous_version(app_id: &str) -> Option<String> {
    load_state().apps.get(app_id).and_then(|a| a.last_known_version.clone())
}

/// Rolls `package` back to `version`. Tries pacman's own package cache
/// first (fast, offline, exactly the file `pacman -U` expects); if that
/// specific version isn't cached (e.g. the cache was cleaned), falls back
/// to re-downloading that exact release asset from zohara-packages, since
/// GitHub Releases keep every past asset by default unless one is
/// explicitly deleted.
pub fn restore_previous_version(
    package: &str,
    version: &str,
    download_url: Option<&str>,
) -> Result<(), String> {
    let cache_glob = format!("/var/cache/pacman/pkg/{package}-{version}-*.pkg.tar.zst");
    let pkg_path: PathBuf = if let Some(p) = glob_first(&cache_glob) {
        p
    } else if let Some(url) = download_url {
        let tmp = std::env::temp_dir().join(format!("{package}-{version}.pkg.tar.zst"));
        let ok = Command::new("curl")
            .args(["-sL", "-o"])
            .arg(&tmp)
            .arg(url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return Err(format!("Could not download {package} {version} from the release"));
        }
        tmp
    } else {
        return Err(format!(
            "{package} {version} isn't in the pacman cache and no download URL is known"
        ));
    };

    // Same sudo -n / pkexec fallback backend::install_app already uses.
    let sudo_ok = Command::new("sudo")
        .args(["-n", "pacman", "-U", "--noconfirm"])
        .arg(&pkg_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if sudo_ok {
        return Ok(());
    }
    let pkexec_ok = Command::new("pkexec")
        .args(["pacman", "-U", "--noconfirm"])
        .arg(&pkg_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if pkexec_ok {
        Ok(())
    } else {
        Err(format!("pacman -U failed for {package} {version}"))
    }
}

/// No `glob` crate dependency -- `sh -c` expands the pattern itself.
fn glob_first(pattern: &str) -> Option<PathBuf> {
    let out = Command::new("sh")
        .args(["-c", &format!("ls {pattern} 2>/dev/null | head -1")])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}
