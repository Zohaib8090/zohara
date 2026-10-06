//! The app update engine behind the Store's Updates page.
//!
//! The Store updates **apps**: the ones installed from Flathub (`flatpak remote-ls --updates`), one by one, and it can
//! take an app back to an earlier version. The operating system (the system packages, Zohara's own programs and apps
//! installed from the Arch repositories) is updated in Zohara Settings, a separate program with its own engine.
//! The two share no code and can be used one without the other.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

// ── What's available ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct FlatpakUpdate {
    pub app_id: String,
    pub name: String,
    pub branch: String,
    pub origin: String,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateSet {
    pub flatpak: Vec<FlatpakUpdate>,
    /// Sources that couldn't be checked (offline, tool missing).
    pub errors: Vec<String>,
}

impl UpdateSet {
    pub fn total(&self) -> usize {
        self.flatpak.len()
    }
}

/// `flatpak remote-ls --updates --columns=application,name,version,branch,origin`.
pub fn parse_flatpak_updates(text: &str) -> Vec<FlatpakUpdate> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let app_id = f.first()?.trim();
            if app_id.is_empty() || !app_id.contains('.') {
                return None;
            }
            Some(FlatpakUpdate {
                app_id: app_id.into(),
                name: f.get(1).map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or(app_id).into(),
                branch: f.get(3).map(|s| s.trim().to_string()).unwrap_or_default(),
                origin: f.get(4).map(|s| s.trim().to_string()).unwrap_or_default(),
            })
        })
        .collect()
}

fn check_flatpak() -> Result<Vec<FlatpakUpdate>, String> {
    let Ok(o) = Command::new("flatpak").args(["remote-ls", "--updates", "--app", "--columns=application,name,version,branch,origin"]).output() else {
        return Ok(Vec::new()); // no Flatpak: nothing to update
    };
    if !o.status.success() {
        return Err("Couldn't check for app updates. Are you online?".into());
    }
    Ok(parse_flatpak_updates(&String::from_utf8_lossy(&o.stdout)))
}

/// Looks for app updates. Slow (network); call off the UI thread.
pub fn check_all() -> UpdateSet {
    let mut set = UpdateSet::default();
    match check_flatpak() {
        Ok(f) => set.flatpak = f,
        Err(e) => set.errors.push(e),
    }
    set
}

// ── Running things, with live output ───────────────────────────────────────

fn safe_name(s: &str) -> bool {
    !s.is_empty() && s.len() < 200 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"@._+-".contains(&b))
}

/// Runs `cmd`, sending each output line to `tx`. pkexec's "dismissed" codes
/// become a friendly message.
pub fn run_logged(mut cmd: Command, tx: &Sender<String>) -> Result<(), String> {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("{e}"))?;
    let stderr = child.stderr.take();
    let tx2 = tx.clone();
    let err_thread = std::thread::spawn(move || {
        if let Some(e) = stderr {
            for l in BufReader::new(e).lines().map_while(Result::ok) {
                let _ = tx2.send(l);
            }
        }
    });
    if let Some(out) = child.stdout.take() {
        for l in BufReader::new(out).lines().map_while(Result::ok) {
            let _ = tx.send(l);
        }
    }
    let _ = err_thread.join();
    let status = child.wait().map_err(|e| e.to_string())?;
    match status.code() {
        Some(0) => Ok(()),
        Some(126) | Some(127) => Err("The password prompt was cancelled, so nothing was changed.".into()),
        _ => Err("It didn't finish. The details above say why.".into()),
    }
}

/// Updates the chosen apps.
pub fn apply(flatpaks: &[String], tx: &Sender<String>) -> Result<(), String> {
    if flatpaks.iter().any(|n| !safe_name(n)) {
        return Err("An app name looked wrong, so nothing was changed.".into());
    }
    let _ = tx.send(format!("Updating {} app(s)…", flatpaks.len()));
    let mut c = Command::new("flatpak");
    c.args(["update", "-y", "--noninteractive"]).args(flatpaks);
    run_logged(c, tx)
}

// ── Flatpak: earlier versions ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct FlatpakInstalled {
    pub app_id: String,
    pub name: String,
    pub origin: String,
}

pub fn flatpak_installed() -> Vec<FlatpakInstalled> {
    let Ok(o) = Command::new("flatpak").args(["list", "--app", "--columns=application,name,origin"]).output() else { return Vec::new() };
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let id = f.first()?.trim();
            (!id.is_empty()).then(|| FlatpakInstalled { app_id: id.into(), name: f.get(1).map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or(id).into(), origin: f.get(2).map(|s| s.trim().to_string()).unwrap_or_default() })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlatpakCommit {
    pub hash: String,
    pub subject: String,
    pub date: String,
}

/// `flatpak remote-info --log`: repeated Commit / Subject / Date blocks, newest first.
pub fn parse_flatpak_log(text: &str) -> Vec<FlatpakCommit> {
    let mut out: Vec<FlatpakCommit> = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if let Some(h) = l.strip_prefix("Commit:") {
            out.push(FlatpakCommit { hash: h.trim().into(), subject: String::new(), date: String::new() });
        } else if let (Some(s), Some(c)) = (l.strip_prefix("Subject:"), out.last_mut()) {
            c.subject = s.trim().into();
        } else if let (Some(d), Some(c)) = (l.strip_prefix("Date:"), out.last_mut()) {
            c.date = d.trim().into();
        }
    }
    out
}

/// Earlier versions of a Flatpak app (not the newest), from its remote. Needs the network.
pub fn flatpak_history(app: &FlatpakInstalled) -> Result<Vec<FlatpakCommit>, String> {
    let o = Command::new("flatpak").args(["remote-info", "--log", &app.origin, &app.app_id]).output().map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err("Couldn't get the version history. Are you online?".into());
    }
    let mut commits = parse_flatpak_log(&String::from_utf8_lossy(&o.stdout));
    if !commits.is_empty() {
        commits.remove(0); // the newest is what you'd have after updating
    }
    Ok(commits)
}

pub fn flatpak_go_back(app_id: &str, hash: &str, tx: &Sender<String>) -> Result<(), String> {
    if !safe_name(app_id) || hash.len() < 8 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("That version looked wrong, so nothing was changed.".into());
    }
    let mut c = Command::new("flatpak");
    c.args(["update", "-y", "--noninteractive", &format!("--commit={hash}"), app_id]);
    run_logged(c, tx)
}

// ── Notifications for the background check ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoreState {
    /// What we last told the user about, so the same updates aren't announced twice.
    #[serde(default)]
    last_notified: String,
}

fn state_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())).join(".config"));
    base.join("zohara").join("store-state.json")
}

fn load_state() -> StoreState {
    std::fs::read_to_string(state_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_state(s: &StoreState) {
    let p = state_path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(t) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(p, t);
    }
}

/// A fingerprint of what's available, so a notification is sent once per set of updates.
pub fn signature(set: &UpdateSet) -> String {
    let mut parts: Vec<String> = set.flatpak.iter().map(|f| format!("{}@{}", f.app_id, f.branch)).collect();
    parts.sort();
    parts.join(",")
}

/// One notification for whatever is new since last time; clicking it opens the Updates page.
/// Runs from `zohara-store --check-updates` on a timer; touches no GTK state.
pub fn notify_pending(set: &UpdateSet) {
    if set.total() == 0 {
        let mut st = load_state();
        st.last_notified.clear();
        save_state(&st);
        return;
    }
    let sig = signature(set);
    let mut st = load_state();
    if st.last_notified == sig {
        return;
    }
    st.last_notified = sig;
    save_state(&st);

    let n = set.total();
    let body = format!("{n} app update{} ready to install.", if n == 1 { "" } else { "s" });
    let out = Command::new("notify-send")
        .args(["-a", "Zohara Store", "-i", "system-software-update", "--action=open=Open Updates", "--wait", "-t", "30000", "App updates available", &body])
        .output();
    if let Ok(o) = out {
        if String::from_utf8_lossy(&o.stdout).trim() == "open" {
            if let Ok(exe) = std::env::current_exe() {
                let _ = Command::new(exe).args(["--page", "updates"]).spawn();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatpak_lines() {
        let u = parse_flatpak_updates("org.mozilla.firefox\tFirefox\t130\tstable\tflathub\nnot an app\nio.github.x.Y\t\t\tstable\tflathub\n");
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].name, "Firefox");
        assert_eq!(u[1].name, "io.github.x.Y");
    }

    #[test]
    fn flatpak_history() {
        let text = "        ID: org.x.App\n    Commit: aaaa1111\n   Subject: Newest\n      Date: 2026-09-25\n   History:\n    Commit: bbbb2222\n   Subject: Older\n      Date: 2026-09-01\n";
        let c = parse_flatpak_log(text);
        assert_eq!(c.len(), 2);
        assert_eq!(c[1], FlatpakCommit { hash: "bbbb2222".into(), subject: "Older".into(), date: "2026-09-01".into() });
    }

    #[test]
    fn names_are_checked() {
        assert!(safe_name("zohara-settings") && safe_name("org.mozilla.firefox") && safe_name("g++"));
        assert!(!safe_name("") && !safe_name("a b") && !safe_name("a;b") && !safe_name("$(x)"));
    }

    #[test]
    fn signature_is_stable() {
        let set = UpdateSet {
            flatpak: vec![
                FlatpakUpdate { app_id: "b.B".into(), name: "B".into(), branch: "stable".into(), origin: "flathub".into() },
                FlatpakUpdate { app_id: "a.A".into(), name: "A".into(), branch: "stable".into(), origin: "flathub".into() },
            ],
            errors: vec![],
        };
        assert_eq!(signature(&set), "a.A@stable,b.B@stable");
    }
}
