//! The update engine behind the Store's Updates page.
//!
//! Every update on Zohara goes through here, and every action is a click:
//!
//! - **Zohara apps** (`zohara-*` packages such as Settings and the Store) and
//!   **system packages** come from pacman. Available updates are found with
//!   `checkupdates`, which reads a private copy of the package databases, so
//!   looking for updates never touches the system.
//! - **Apps** installed from Flathub come from `flatpak remote-ls --updates`.
//!
//! Zohara apps and Flatpak apps can be updated one by one. System packages are
//! updated together (`pacman -Syu`): on Arch, updating a few of them and not
//! the rest can leave libraries out of step and break programs.
//!
//! Going back: pacman keeps every package it installed in
//! `/var/cache/pacman/pkg`, so an older version is one `pacman -U` away, and
//! `/var/log/pacman.log` says exactly what the last update changed, which is
//! what "Undo the last update" reverses.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

const CACHE_DIR: &str = "/var/cache/pacman/pkg";
const PACMAN_LOG: &str = "/var/log/pacman.log";

// ── What's available ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct PkgUpdate {
    pub name: String,
    pub old: String,
    pub new: String,
}

impl PkgUpdate {
    pub fn is_zohara(&self) -> bool {
        self.name.starts_with("zohara-")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlatpakUpdate {
    pub app_id: String,
    pub name: String,
    pub branch: String,
    pub origin: String,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateSet {
    /// Zohara's own packages: can be updated one at a time.
    pub zohara: Vec<PkgUpdate>,
    /// Everything else from pacman: updated together.
    pub system: Vec<PkgUpdate>,
    pub flatpak: Vec<FlatpakUpdate>,
    /// Sources that couldn't be checked (offline, tool missing).
    pub errors: Vec<String>,
}

impl UpdateSet {
    pub fn total(&self) -> usize {
        self.zohara.len() + self.system.len() + self.flatpak.len()
    }

    /// Whether installing all of `system` needs a restart to take effect.
    pub fn system_needs_restart(&self) -> bool {
        needs_restart(&self.system)
    }
}

pub fn needs_restart(pkgs: &[PkgUpdate]) -> bool {
    pkgs.iter().any(|p| {
        let n = p.name.as_str();
        n == "linux"
            || n == "systemd"
            || n.starts_with("nvidia")
            || ["-zen", "-lts", "-hardened", "-rt"].iter().any(|s| n.starts_with("linux") && n.ends_with(s) && !n.contains("headers") && !n.contains("docs"))
    })
}

/// `checkupdates` output: `name old -> new`, one per line.
pub fn parse_checkupdates(text: &str) -> Vec<PkgUpdate> {
    text.lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let (name, old, arrow, new) = (f.next()?, f.next()?, f.next()?, f.next()?);
            (arrow == "->").then(|| PkgUpdate { name: name.into(), old: old.into(), new: new.into() })
        })
        .collect()
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

fn check_pacman() -> Result<Vec<PkgUpdate>, String> {
    let o = Command::new("checkupdates").output().map_err(|_| "Checking for system updates needs pacman-contrib, which isn't installed".to_string())?;
    match o.status.code() {
        Some(0) => Ok(parse_checkupdates(&String::from_utf8_lossy(&o.stdout))),
        Some(2) => Ok(Vec::new()),
        _ => {
            let e = String::from_utf8_lossy(&o.stderr);
            let last = e.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("unknown error");
            Err(format!("Couldn't check for system updates ({last}). Are you online?"))
        }
    }
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

/// Looks for updates everywhere. Slow (network); call off the UI thread.
pub fn check_all() -> UpdateSet {
    let flatpak = std::thread::spawn(check_flatpak);
    let mut set = UpdateSet::default();
    match check_pacman() {
        Ok(all) => {
            let (zohara, system): (Vec<_>, Vec<_>) = all.into_iter().partition(PkgUpdate::is_zohara);
            set.zohara = zohara;
            set.system = system;
        }
        Err(e) => set.errors.push(e),
    }
    match flatpak.join() {
        Ok(Ok(f)) => set.flatpak = f,
        Ok(Err(e)) => set.errors.push(e),
        Err(_) => set.errors.push("Checking app updates failed unexpectedly".into()),
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

fn pkexec(args: &[&str]) -> Command {
    let mut c = Command::new("pkexec");
    c.args(args);
    c
}

/// Installs what was chosen. `system` updates all of pacman's packages
/// (Zohara's included); otherwise only the named Zohara packages are updated.
pub fn apply(system: bool, zohara: &[String], flatpaks: &[String], tx: &Sender<String>) -> Result<(), String> {
    if zohara.iter().chain(flatpaks).any(|n| !safe_name(n)) {
        return Err("A package name looked wrong, so nothing was changed.".into());
    }
    let mut failures: Vec<String> = Vec::new();

    if system {
        let _ = tx.send("Updating the system…".into());
        if let Err(e) = run_logged(pkexec(&["pacman", "-Syu", "--noconfirm"]), tx) {
            failures.push(e);
        }
    } else if !zohara.is_empty() {
        let _ = tx.send(format!("Updating {}…", zohara.join(", ")));
        // `-Sy` here refreshes only what these packages need to be found.
        let mut c = pkexec(&["sh", "-c", "pacman -Sy --noconfirm && exec pacman -S --noconfirm --needed \"$@\"", "sh"]);
        c.args(zohara);
        if let Err(e) = run_logged(c, tx) {
            failures.push(e);
        }
    }

    if !flatpaks.is_empty() {
        let _ = tx.send(format!("Updating {} app(s)…", flatpaks.len()));
        let mut c = Command::new("flatpak");
        c.args(["update", "-y", "--noninteractive"]).args(flatpaks);
        if let Err(e) = run_logged(c, tx) {
            failures.push(e);
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n"))
    }
}

/// Whether a newer kernel (or other core piece) is installed than the one running.
pub fn restart_pending() -> bool {
    let release = Command::new("uname").arg("-r").output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    !release.is_empty() && !Path::new("/usr/lib/modules").join(&release).exists()
}

// ── Going back ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Cached {
    pub name: String,
    /// `pkgver-pkgrel`, like pacman prints it.
    pub version: String,
    pub path: PathBuf,
}

/// `foo-bar-1.2.3-1-x86_64.pkg.tar.zst` -> (`foo-bar`, `1.2.3-1`).
pub fn parse_cache_filename(file: &str) -> Option<(String, String)> {
    let stem = [".pkg.tar.zst", ".pkg.tar.xz", ".pkg.tar.gz", ".pkg.tar"].iter().find_map(|s| file.strip_suffix(s))?;
    let mut parts = stem.rsplitn(3, '-');
    let _arch = parts.next()?;
    let rel = parts.next()?;
    let rest = parts.next()?;
    let (name, ver) = rest.rsplit_once('-')?;
    Some((name.into(), format!("{ver}-{rel}")))
}

pub fn vercmp(a: &str, b: &str) -> Ordering {
    let o = Command::new("vercmp").args([a, b]).output();
    match o.ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).as_deref() {
        Some("-1") => Ordering::Less,
        Some("1") => Ordering::Greater,
        _ => Ordering::Equal,
    }
}

pub fn installed_version(pkg: &str) -> Option<String> {
    let o = Command::new("pacman").args(["-Q", pkg]).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).split_whitespace().nth(1).map(str::to_string)).flatten()
}

/// Older copies of `pkg` still in pacman's cache, newest first.
pub fn older_versions(pkg: &str) -> Vec<Cached> {
    let Some(current) = installed_version(pkg) else { return Vec::new() };
    let mut found: Vec<Cached> = std::fs::read_dir(CACHE_DIR)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().into_owned();
            let (name, version) = parse_cache_filename(&file)?;
            (name == pkg && vercmp(&version, &current) == Ordering::Less).then(|| Cached { name, version, path: e.path() })
        })
        .collect();
    found.sort_by(|a, b| vercmp(&b.version, &a.version));
    found.dedup_by(|a, b| a.version == b.version);
    found
}

/// Installs the given cached package files (an older version).
pub fn downgrade(files: &[PathBuf], tx: &Sender<String>) -> Result<(), String> {
    if files.is_empty() {
        return Err("Nothing to go back to.".into());
    }
    let _ = tx.send("Going back to the earlier version…".into());
    let mut c = pkexec(&["pacman", "-U", "--noconfirm"]);
    c.args(files);
    run_logged(c, tx)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Upgrade {
    pub name: String,
    pub old: String,
    pub new: String,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Transaction {
    pub when: String,
    pub upgraded: Vec<Upgrade>,
    pub downgraded: usize,
}

/// Groups `/var/log/pacman.log` into transactions.
pub fn parse_pacman_log(text: &str) -> Vec<Transaction> {
    let mut out = Vec::new();
    let mut cur: Option<Transaction> = None;
    for line in text.lines() {
        let Some((stamp, rest)) = line.strip_prefix('[').and_then(|l| l.split_once("] ")) else { continue };
        if let Some(what) = rest.strip_prefix("[ALPM] ") {
            if what == "transaction started" {
                cur = Some(Transaction { when: stamp.replace('T', " ").chars().take(16).collect(), ..Default::default() });
            } else if what == "transaction completed" {
                if let Some(t) = cur.take() {
                    out.push(t);
                }
            } else if let Some(t) = cur.as_mut() {
                if let Some(u) = what.strip_prefix("upgraded ") {
                    // name (old -> new)
                    if let Some((name, ver)) = u.split_once(" (") {
                        if let Some((old, new)) = ver.trim_end_matches(')').split_once(" -> ") {
                            t.upgraded.push(Upgrade { name: name.into(), old: old.into(), new: new.into() });
                        }
                    }
                } else if what.starts_with("downgraded ") {
                    t.downgraded += 1;
                }
            }
        }
    }
    out
}

/// The newest update that hasn't been undone, if any.
pub fn last_update() -> Option<Transaction> {
    let text = std::fs::read_to_string(PACMAN_LOG).ok()?;
    let all = parse_pacman_log(&text);
    let idx = all.iter().rposition(|t| !t.upgraded.is_empty())?;
    // Something was downgraded after it: it was already undone.
    if all[idx + 1..].iter().any(|t| t.downgraded > 0) {
        return None;
    }
    Some(all[idx].clone())
}

/// Cached files for the versions an update replaced, and names that are missing.
pub fn files_to_undo(t: &Transaction) -> (Vec<PathBuf>, Vec<String>) {
    let mut files = Vec::new();
    let mut missing = Vec::new();
    for u in &t.upgraded {
        let hit = std::fs::read_dir(CACHE_DIR).into_iter().flatten().flatten().find(|e| {
            parse_cache_filename(&e.file_name().to_string_lossy()).map(|(n, v)| n == u.name && v == u.old).unwrap_or(false)
        });
        match hit {
            Some(e) => files.push(e.path()),
            None => missing.push(format!("{} {}", u.name, u.old)),
        }
    }
    (files, missing)
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
    let mut parts: Vec<String> = set.zohara.iter().chain(&set.system).map(|p| format!("{}={}", p.name, p.new)).collect();
    parts.extend(set.flatpak.iter().map(|f| format!("{}@{}", f.app_id, f.branch)));
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
    let body = if !set.zohara.is_empty() {
        format!("{n} update{} ready, including Zohara itself.", if n == 1 { "" } else { "s" })
    } else {
        format!("{n} update{} ready to install.", if n == 1 { "" } else { "s" })
    };
    let out = Command::new("notify-send")
        .args(["-a", "Zohara Store", "-i", "system-software-update", "--action=open=Open Updates", "--wait", "-t", "30000", "Updates available", &body])
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
    fn checkupdates_lines() {
        let u = parse_checkupdates("zohara-settings 0.1.0-1 -> 0.1.0.20260925-1\nlinux-zen 6.10.1-1 -> 6.10.2-1\ngarbage\n");
        assert_eq!(u.len(), 2);
        assert!(u[0].is_zohara() && !u[1].is_zohara());
        assert!(needs_restart(&u[1..]) && !needs_restart(&u[..1]));
        assert!(!needs_restart(&[PkgUpdate { name: "linux-zen-headers".into(), old: "1".into(), new: "2".into() }]));
    }

    #[test]
    fn flatpak_lines() {
        let u = parse_flatpak_updates("org.mozilla.firefox\tFirefox\t130\tstable\tflathub\nnot an app\nio.github.x.Y\t\t\tstable\tflathub\n");
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].name, "Firefox");
        assert_eq!(u[1].name, "io.github.x.Y");
    }

    #[test]
    fn cache_names() {
        assert_eq!(parse_cache_filename("zohara-settings-0.1.0.5-1-x86_64.pkg.tar.zst"), Some(("zohara-settings".into(), "0.1.0.5-1".into())));
        assert_eq!(parse_cache_filename("lib32-mesa-1:25.1.4-2-x86_64.pkg.tar.zst"), Some(("lib32-mesa".into(), "1:25.1.4-2".into())));
        assert_eq!(parse_cache_filename("foo-1-1-any.pkg.tar.xz"), Some(("foo".into(), "1-1".into())));
        assert_eq!(parse_cache_filename("foo-1-1-any.pkg.tar.zst.sig"), None);
        assert_eq!(parse_cache_filename("download-abc"), None);
    }

    #[test]
    fn pacman_log_transactions() {
        let log = "\
[2026-09-25T10:00:00+0000] [PACMAN] Running 'pacman -Syu'
[2026-09-25T10:00:05+0000] [ALPM] transaction started
[2026-09-25T10:00:06+0000] [ALPM] upgraded foo (1.0-1 -> 1.1-1)
[2026-09-25T10:00:07+0000] [ALPM] upgraded bar-baz (2:3-1 -> 2:4-1)
[2026-09-25T10:00:08+0000] [ALPM] transaction completed
[2026-09-25T11:00:00+0000] [ALPM] transaction started
[2026-09-25T11:00:01+0000] [ALPM] installed qux (1-1)
[2026-09-25T11:00:02+0000] [ALPM] transaction completed
";
        let t = parse_pacman_log(log);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].upgraded.len(), 2);
        assert_eq!(t[0].upgraded[1], Upgrade { name: "bar-baz".into(), old: "2:3-1".into(), new: "2:4-1".into() });
        assert_eq!(t[0].when, "2026-09-25 10:00");
        assert!(t[1].upgraded.is_empty());
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
        let mut s = UpdateSet::default();
        s.system.push(PkgUpdate { name: "b".into(), old: "1".into(), new: "2".into() });
        s.zohara.push(PkgUpdate { name: "zohara-a".into(), old: "1".into(), new: "2".into() });
        assert_eq!(signature(&s), "b=2,zohara-a=2");
    }
}
