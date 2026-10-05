use std::collections::HashSet;
use std::process::{Command, Stdio};

use crate::app_info::{AppCategory, AppSource};

/// What is installed: one `pacman -Qq` and one `flatpak list` call in total, not one per app. It is shared by every
/// button in the window; when an install or removal finishes the cache is updated and every button re-reads it,
/// so the same app never shows "Get" in one place and "Remove" in another.
pub struct InstalledCache {
    pacman: std::cell::RefCell<HashSet<String>>,
    flatpak: std::cell::RefCell<HashSet<String>>,
    listeners: std::cell::RefCell<Vec<Box<dyn Fn() -> bool>>>,
}

impl InstalledCache {
    pub fn load() -> Self {
        InstalledCache {
            pacman: std::cell::RefCell::new(Self::load_pacman()),
            flatpak: std::cell::RefCell::new(Self::load_flatpak()),
            listeners: Default::default(),
        }
    }

    fn lines_of(cmd: &mut Command) -> HashSet<String> {
        match cmd.stdout(Stdio::piped()).stderr(Stdio::null()).output() {
            Ok(o) => String::from_utf8_lossy(&o.stdout).lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            Err(_) => HashSet::new(),
        }
    }

    fn load_pacman() -> HashSet<String> {
        Self::lines_of(Command::new("pacman").arg("-Qq"))
    }

    fn load_flatpak() -> HashSet<String> {
        // one app id per line
        Self::lines_of(Command::new("flatpak").args(["list", "--app", "--columns=application"]))
    }

    pub fn is_installed(&self, source: &AppSource, package_name: &str) -> bool {
        match source {
            AppSource::Pacman => self.pacman.borrow().contains(package_name),
            AppSource::Flatpak => self.flatpak.borrow().contains(package_name),
        }
    }

    /// Records that `package_name` is now installed (or gone) and tells every listening button.
    pub fn set_installed(&self, source: &AppSource, package_name: &str, installed: bool) {
        let mut set = match source {
            AppSource::Pacman => self.pacman.borrow_mut(),
            AppSource::Flatpak => self.flatpak.borrow_mut(),
        };
        if installed {
            set.insert(package_name.to_string());
        } else {
            set.remove(package_name);
        }
        drop(set);
        // A listener returns false once its widget is gone; those are dropped.
        self.listeners.borrow_mut().retain(|f| f());
    }

    pub fn on_change(&self, f: impl Fn() -> bool + 'static) {
        self.listeners.borrow_mut().push(Box::new(f));
    }
}

/// Only one package operation at a time, across the whole Store (installs,
/// removals and the Updates page). pacman holds a database lock while it
/// works; a second pacman started meanwhile fails immediately with "unable
/// to lock database", which used to make the second of two quick installs
/// fail with no message. Waiting here queues it instead.
pub static PACKAGE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// What an install or removal is doing right now, for the progress bar. `fraction` is 0..1 when it can be told, and
/// `None` when all that is known is "still working" (the bar then pulses).
#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub fraction: Option<f64>,
    pub text: String,
}

/// What the install thread tells the window.
pub enum Event {
    Progress(Progress),
    Done(Result<(), String>),
}

fn mib(bytes: u64) -> String {
    let m = bytes as f64 / 1_048_576.0;
    if m >= 1024.0 { format!("{:.1} GiB", m / 1024.0) } else if m >= 10.0 { format!("{m:.0} MiB") } else { format!("{m:.1} MiB") }
}

/// "0.18 MiB" / "12.5 KiB" / "1.2 GiB" -> bytes.
fn parse_size(text: &str) -> Option<u64> {
    let mut it = text.split_whitespace();
    let value: f64 = it.next()?.replace(',', ".").parse().ok()?;
    let mult = match it.next()? {
        "B" => 1.0,
        "KiB" => 1024.0,
        "MiB" => 1024.0 * 1024.0,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((value * mult) as u64)
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum Stage {
    #[default]
    Resolving,
    Downloading,
    Verifying,
    Working, // installing or removing
    Finishing,
}

/// Follows pacman's output (it is not on a terminal, so it prints plain lines): how many packages, how big the
/// download is, which package is downloading, then the checking and installing steps. Downloads are measured in
/// bytes by watching the package cache (`downloaded`), because pacman prints no percentages to a pipe.
#[derive(Default, Debug)]
pub struct PacmanProgress {
    stage: Stage,
    removing: bool,
    total: usize,
    total_bytes: u64,
    started: usize,  // packages whose download has begun
    finished: usize, // packages installed or removed so far
    pub names: Vec<String>,
}

impl PacmanProgress {
    pub fn new(removing: bool) -> Self {
        Self { removing, ..Self::default() }
    }

    /// Overall 0..1 for the current stage; the stages share the bar: 0-5 resolving, 5-60 download, 60-70 checks,
    /// 70-97 installing, then done.
    fn overall(&self, within: f64) -> f64 {
        let (lo, hi) = match self.stage {
            Stage::Resolving => (0.0, 0.05),
            Stage::Downloading => (0.05, 0.60),
            Stage::Verifying => (0.60, 0.70),
            Stage::Working => (0.70, 0.97),
            Stage::Finishing => (0.97, 1.0),
        };
        lo + (hi - lo) * within.clamp(0.0, 1.0)
    }

    fn working_text(&self) -> String {
        let verb = if self.removing { "Removing" } else { "Installing" };
        if self.total > 1 {
            format!("{verb} {} of {}", (self.finished + 1).min(self.total), self.total)
        } else {
            format!("{verb}…")
        }
    }

    /// Feed one output line; returns what to show if it changes anything.
    pub fn feed(&mut self, line: &str) -> Option<Progress> {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("Total Download Size:") {
            self.total_bytes = parse_size(rest).unwrap_or(0);
            return None;
        }
        if let Some(rest) = l.strip_prefix("Package (").or_else(|| l.strip_prefix("Packages (")) {
            self.total = rest.split(')').next().and_then(|n| n.trim().parse().ok()).unwrap_or(0);
            return None;
        }
        if l.starts_with(":: Retrieving packages") {
            self.stage = Stage::Downloading;
            return Some(Progress { fraction: Some(self.overall(0.0)), text: "Starting the download…".into() });
        }
        if let Some(name) = l.strip_suffix(" downloading...") {
            let name = name.trim().to_string();
            self.stage = Stage::Downloading;
            self.started += 1;
            self.names.push(name);
            let within = if self.total > 0 { (self.started - 1) as f64 / self.total as f64 } else { 0.0 };
            let text = if self.total > 1 { format!("Downloading {} of {}", self.started.min(self.total), self.total) } else { "Downloading…".to_string() };
            return Some(Progress { fraction: Some(self.overall(within)), text });
        }
        if l.starts_with("checking ") || l.starts_with("loading package files") || l.starts_with("resolving dependencies") {
            if l.starts_with("checking keyring") || l.starts_with("checking package integrity") || l.starts_with("loading package files") {
                self.stage = Stage::Verifying;
                return Some(Progress { fraction: Some(self.overall(0.3)), text: "Checking the download…".into() });
            }
            if l.starts_with("checking for file conflicts") || l.starts_with("checking available disk space") {
                self.stage = Stage::Verifying;
                return Some(Progress { fraction: Some(self.overall(0.9)), text: "Checking for conflicts…".into() });
            }
            return None;
        }
        if l.starts_with(":: Processing package changes") {
            self.stage = Stage::Working;
            return Some(Progress { fraction: Some(self.overall(0.0)), text: self.working_text() });
        }
        for verb in ["installing ", "upgrading ", "reinstalling ", "removing "] {
            if l.starts_with(verb) && l.ends_with("...") {
                self.stage = Stage::Working;
                let within = if self.total > 0 { self.finished as f64 / self.total as f64 } else { 0.5 };
                let p = Progress { fraction: Some(self.overall(within)), text: self.working_text() };
                self.finished += 1;
                return Some(p);
            }
        }
        if l.starts_with(":: Running post-transaction hooks") {
            self.stage = Stage::Finishing;
            return Some(Progress { fraction: Some(self.overall(0.0)), text: "Finishing up…".into() });
        }
        None
    }

    /// While downloading: progress from how many bytes of the packages are in the cache.
    pub fn download_progress(&self, bytes_in_cache: u64) -> Option<Progress> {
        if self.stage != Stage::Downloading {
            return None;
        }
        if self.total_bytes > 0 {
            let within = bytes_in_cache as f64 / self.total_bytes as f64;
            let shown = bytes_in_cache.min(self.total_bytes);
            Some(Progress { fraction: Some(self.overall(within)), text: format!("Downloading {} of {}", mib(shown), mib(self.total_bytes)) })
        } else {
            None
        }
    }
}

/// Bytes of the named packages now in pacman's cache: finished files, and the `.part` files of downloads in flight.
/// Pacman downloads (several at a time) into a `download-XXXXXX` folder inside the cache and moves each file up when
/// it is complete, so both places are counted.
fn cache_bytes(names: &[String]) -> u64 {
    fn sum_dir(dir: &std::path::Path, names: &[String]) -> u64 {
        let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
        rd.flatten()
            .filter(|e| {
                let f = e.file_name().to_string_lossy().to_string();
                !f.ends_with(".sig") && names.iter().any(|n| f.starts_with(n.as_str()))
            })
            .filter_map(|e| e.metadata().ok())
            .filter(|m| m.is_file())
            .map(|m| m.len())
            .sum()
    }
    let root = std::path::Path::new("/var/cache/pacman/pkg");
    let mut total = sum_dir(root, names);
    if let Ok(rd) = std::fs::read_dir(root) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with("download-") && e.path().is_dir() {
                total += sum_dir(&e.path(), names);
            }
        }
    }
    total
}

/// A percentage and a short description from one line of flatpak's output.
pub fn parse_flatpak_line(line: &str) -> Option<Progress> {
    let l = line.trim();
    let pct = l.split('%').next().filter(|_| l.contains('%')).and_then(|before| {
        before.rsplit(|c: char| !c.is_ascii_digit()).next().and_then(|d| d.parse::<u32>().ok()).filter(|p| *p <= 100)
    });
    let text = ["Installing", "Downloading", "Updating", "Uninstalling", "Fetching"].iter().find(|v| l.contains(*v)).map(|v| format!("{v}…"));
    match (pct, text) {
        (None, None) => None,
        (p, t) => Some(Progress { fraction: p.map(|p| p as f64 / 100.0), text: t.unwrap_or_else(|| "Working…".to_string()) }),
    }
}

enum Line {
    Out(String),
    Err(String),
}

/// Starts `cmd`, reads its output as it appears (a line ends at a newline or a carriage return, which is how bars
/// redraw), passes each stdout line to `on_line` and, every 300 ms of quiet, asks `on_tick` for news. Returns the exit
/// status and everything that came out on stderr.
fn run_streaming(
    mut cmd: Command,
    mut on_line: impl FnMut(&str) -> Option<Progress>,
    mut on_tick: impl FnMut() -> Option<Progress>,
    tx: &std::sync::mpsc::Sender<Event>,
) -> Result<(std::process::ExitStatus, String), String> {
    use std::io::Read;
    use std::sync::mpsc::{channel, RecvTimeoutError};
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn't start the installer ({e})"))?;
    let (ltx, lrx) = channel::<Line>();
    let spawn_reader = |mut r: Box<dyn Read + Send>, err: bool| {
        let ltx = ltx.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut cur = Vec::new();
            loop {
                match r.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        for &b in &buf[..n] {
                            if b == b'\n' || b == b'\r' {
                                if !cur.is_empty() {
                                    let s = String::from_utf8_lossy(&cur).to_string();
                                    let _ = ltx.send(if err { Line::Err(s) } else { Line::Out(s) });
                                    cur.clear();
                                }
                            } else {
                                cur.push(b);
                            }
                        }
                    }
                }
            }
            if !cur.is_empty() {
                let s = String::from_utf8_lossy(&cur).to_string();
                let _ = ltx.send(if err { Line::Err(s) } else { Line::Out(s) });
            }
        });
    };
    if let Some(o) = child.stdout.take() {
        spawn_reader(Box::new(o), false);
    }
    if let Some(e) = child.stderr.take() {
        spawn_reader(Box::new(e), true);
    }
    drop(ltx);
    let mut errs = String::new();
    loop {
        match lrx.recv_timeout(std::time::Duration::from_millis(300)) {
            Ok(Line::Out(l)) => {
                if let Some(p) = on_line(&l) {
                    let _ = tx.send(Event::Progress(p));
                }
            }
            Ok(Line::Err(l)) => {
                errs.push_str(&l);
                errs.push('\n');
            }
            Err(RecvTimeoutError::Timeout) => {
                if let Some(p) = on_tick() {
                    let _ = tx.send(Event::Progress(p));
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    let st = child.wait().map_err(|e| format!("Couldn't finish the installer ({e})"))?;
    Ok((st, errs))
}

/// A readable reason from pacman/flatpak's error output.
pub fn explain_failure(stderr: &str) -> String {
    let e = stderr.to_lowercase();
    if e.contains("unable to lock database") {
        "Another program is installing or updating software right now. Try again when it has finished.".into()
    } else if e.contains("target not found") || e.contains("no remote refs found") || e.contains("nothing matches") {
        "This app isn't available from Zohara's software sources right now.".into()
    } else if e.contains("failed retrieving file") || e.contains("could not resolve host") || e.contains("couldn't resolve") || e.contains("network") {
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

/// Runs pacman as administrator and reports progress: non-interactive sudo first (the live ISO), then pkexec (asks for
/// the password on an installed system). A pacman error under sudo is final; only sudo itself refusing falls through to
/// the password prompt.
fn pacman_admin(args: &[&str], removing: bool, tx: &std::sync::mpsc::Sender<Event>) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let is_root = std::fs::metadata("/proc/self").map(|m| m.uid() == 0).unwrap_or(false);
    // Already administrator (a root shell, a container): no helper needed.
    let runners: &[(&str, bool)] = if is_root { &[("", false)] } else { &[("sudo", true), ("pkexec", false)] };
    for (i, runner) in runners.iter().enumerate() {
        // `stdbuf -oL` makes pacman write each line as it goes. Into a pipe it would hold its text back until the very
        // end, and the progress bar would sit still for the whole download.
        let mut cmd = if runner.0.is_empty() {
            Command::new("stdbuf")
        } else {
            let mut c = Command::new(runner.0);
            if runner.1 {
                c.arg("-n");
            }
            c.arg("stdbuf");
            c
        };
        cmd.args(["-oL", "pacman"]).args(args);
        let mut pp = PacmanProgress::new(removing);
        let ran = {
            let pp_line = std::cell::RefCell::new(&mut pp);
            run_streaming(
                cmd,
                |l| pp_line.borrow_mut().feed(l),
                || {
                    let b = pp_line.borrow();
                    let bytes = cache_bytes(&b.names);
                    b.download_progress(bytes)
                },
                tx,
            )
        };
        // sudo not installed: go on to the password prompt instead of giving up.
        let (st, err) = match (ran, i) {
            (Err(_), 0) if !is_root => continue,
            (other, _) => other?,
        };
        if st.success() {
            return Ok(());
        }
        let sudo_refused = i == 0 && err.lines().any(|l| l.starts_with("sudo:"));
        if sudo_refused {
            continue;
        }
        if i == 1 && matches!(st.code(), Some(126) | Some(127)) {
            return Err("The password prompt was cancelled, so nothing was changed.".into());
        }
        return Err(explain_failure(&err));
    }
    Err("Couldn't get administrator permission.".into())
}

/// Installs (`remove == false`) or removes an app, sending progress to `tx`, and finishes with `Event::Done`.
pub fn run_job(source: &AppSource, package_name: &str, remove: bool, tx: &std::sync::mpsc::Sender<Event>) {
    let _guard = PACKAGE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let _ = tx.send(Event::Progress(Progress { fraction: None, text: "Starting…".into() }));
    let res = match (source, remove) {
        (AppSource::Pacman, false) => pacman_admin(&["-S", "--noconfirm", "--needed", package_name], false, tx),
        (AppSource::Pacman, true) => pacman_admin(&["-Rs", "--noconfirm", package_name], true, tx),
        (AppSource::Flatpak, rm) => {
            let mut c = Command::new("flatpak");
            if rm {
                c.args(["uninstall", "-y", "--noninteractive", package_name]);
            } else {
                c.args(["install", "-y", "--noninteractive", "flathub", package_name]);
            }
            match run_streaming(c, parse_flatpak_line, || None, tx) {
                Ok((st, _)) if st.success() => Ok(()),
                Ok((st, err)) if matches!(st.code(), Some(126) | Some(127)) => {
                    let _ = err;
                    Err("The password prompt was cancelled, so nothing was changed.".to_string())
                }
                Ok((_, err)) => Err(explain_failure(&err)),
                Err(e) => Err(e),
            }
        }
    };
    let _ = tx.send(Event::Done(res));
}

/// One line of `pacman -Ss` output pair: (repository, package, description).
pub fn parse_pacman_ss(stdout: &str) -> Vec<(String, String, String)> {
    let lines: Vec<&str> = stdout.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if !l.starts_with(' ') && !l.is_empty() {
            if let Some(full) = l.split_whitespace().next() {
                if let Some((repo, name)) = full.split_once('/') {
                    let desc = lines.get(i + 1).map(|d| d.trim()).unwrap_or("");
                    out.push((repo.to_string(), name.to_string(), desc.to_string()));
                }
            }
        }
        i += 1;
    }
    out
}

fn pretty_name(package: &str) -> String {
    let base = package.strip_suffix("-bin").unwrap_or(package).replace(['-', '_'], " ");
    let mut chars = base.chars();
    match chars.next() {
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Libraries, language bindings, fonts and similar are packages, not apps; they stay out of the results unless the
/// name is typed in full.
fn looks_like_library(name: &str) -> bool {
    const PREFIXES: [&str; 14] = [
        "lib", "lib32-", "python-", "perl-", "ruby-", "haskell-", "ttf-", "otf-", "xorg-", "qt5-", "qt6-", "gst-", "php-", "nodejs-",
    ];
    const SUFFIXES: [&str; 6] = ["-docs", "-doc", "-debug", "-headers", "-git", "-devel"];
    PREFIXES.iter().any(|p| name.starts_with(p)) || SUFFIXES.iter().any(|s| name.ends_with(s))
}

fn app_from_package(repo: &str, name: &str, desc: &str) -> crate::app_info::AppInfo {
    crate::app_info::AppInfo {
        id: name.to_string(),
        name: pretty_name(name),
        publisher: if repo == "chaotic-aur" { "Chaotic-AUR".into() } else { "Arch Linux".into() },
        description: desc.to_string(),
        icon_name: name.to_string(),
        source: AppSource::Pacman,
        package_name: name.to_string(),
        category: AppCategory::Utilities,
        rating: 0.0,
    }
}

/// Everything installable, read once in the background so that typing in the search box never starts a program:
/// every package pacman knows plus (when Flathub is set up) every Flathub app, from the local catalog copy.
pub fn load_catalog() -> Vec<crate::app_info::AppInfo> {
    let mut all = Vec::new();
    if let Ok(o) = Command::new("pacman").args(["-Ss", ""]).stderr(Stdio::null()).output() {
        for (repo, name, desc) in parse_pacman_ss(&String::from_utf8_lossy(&o.stdout)) {
            all.push(app_from_package(&repo, &name, &desc));
        }
    }
    if let Ok(o) = Command::new("flatpak")
        .args(["remote-ls", "--app", "--columns=application,name,description", "flathub"])
        .stderr(Stdio::null())
        .output()
    {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let mut f = line.split('\t');
            let (Some(id), Some(name)) = (f.next(), f.next()) else { continue };
            all.push(crate::app_info::AppInfo {
                id: id.trim().to_string(),
                name: name.trim().to_string(),
                publisher: "Flathub".into(),
                description: f.next().unwrap_or("").trim().to_string(),
                icon_name: id.trim().to_string(),
                source: AppSource::Flatpak,
                package_name: id.trim().to_string(),
                category: AppCategory::Utilities,
                rating: 0.0,
            });
        }
    }
    all
}

/// Searches the curated list first, then the catalog, entirely in memory. Names that start with the query rank above
/// names that contain it, which rank above description matches. At most `limit` results.
pub fn search_catalog(
    curated: &[crate::app_info::AppInfo],
    catalog: &[crate::app_info::AppInfo],
    query: &str,
    limit: usize,
) -> Vec<crate::app_info::AppInfo> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return vec![];
    }
    let mut seen: HashSet<(bool, String)> = HashSet::new();
    let mut out: Vec<crate::app_info::AppInfo> = Vec::new();
    let mut scored: Vec<(u8, usize, &crate::app_info::AppInfo)> = Vec::new();

    for a in curated {
        if a.name.to_lowercase().contains(&q) || a.id.to_lowercase().contains(&q) || a.package_name.to_lowercase().contains(&q) || a.description.to_lowercase().contains(&q) {
            seen.insert((a.source == AppSource::Flatpak, a.package_name.clone()));
            out.push(a.clone());
        }
    }
    for a in catalog {
        let name = a.name.to_lowercase();
        let pkg = a.package_name.to_lowercase();
        let exact = pkg == q || name == q;
        if looks_like_library(&pkg) && !exact {
            continue;
        }
        let rank = if exact { 0 } else if name.starts_with(&q) || pkg.starts_with(&q) { 1 } else if name.contains(&q) || pkg.contains(&q) { 2 } else if a.description.to_lowercase().contains(&q) { 3 } else { continue };
        if seen.contains(&(a.source == AppSource::Flatpak, a.package_name.clone())) {
            continue;
        }
        scored.push((rank, a.name.len(), a));
    }
    scored.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)).then_with(|| x.2.name.cmp(&y.2.name)));
    for (_, _, a) in scored {
        if out.len() >= limit {
            break;
        }
        if seen.insert((a.source == AppSource::Flatpak, a.package_name.clone())) {
            out.push(a.clone());
        }
    }
    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::explain_failure;

    fn app(name: &str, pkg: &str, desc: &str) -> crate::app_info::AppInfo {
        app_from_package("extra", pkg, desc).with_name(name)
    }

    fn feed_all(pp: &mut PacmanProgress, text: &str) -> Vec<Progress> {
        text.lines().filter_map(|l| pp.feed(l)).collect()
    }

    const HTOP_INSTALL: &str = "resolving dependencies...\nlooking for conflicting packages...\n\nPackages (2) a-1  htop-3.5.3-1\n\nTotal Download Size:   4.00 MiB\nTotal Installed Size:  9.00 MiB\n\n:: Proceed with installation? [Y/n] \n:: Retrieving packages...\n a-1-x86_64 downloading...\n htop-3.5.3-1-x86_64 downloading...\nchecking keyring...\nchecking package integrity...\nloading package files...\nchecking for file conflicts...\n:: Processing package changes...\ninstalling a...\ninstalling htop...\n:: Running post-transaction hooks...\n";

    #[test]
    fn install_progress_goes_up_through_the_stages() {
        let mut pp = PacmanProgress::new(false);
        let v = feed_all(&mut pp, HTOP_INSTALL);
        let fr: Vec<f64> = v.iter().map(|p| p.fraction.unwrap()).collect();
        assert!(fr.windows(2).all(|w| w[0] <= w[1] + 1e-9), "never goes backwards: {fr:?}");
        assert!(v.iter().any(|p| p.text == "Downloading 2 of 2"));
        assert!(v.iter().any(|p| p.text == "Checking the download…"));
        assert!(v.iter().any(|p| p.text == "Installing 2 of 2"));
        assert_eq!(v.last().unwrap().text, "Finishing up…");
        assert!(*fr.last().unwrap() >= 0.97);
        assert_eq!(pp.names, vec!["a-1-x86_64", "htop-3.5.3-1-x86_64"]);
    }

    #[test]
    fn download_bar_follows_bytes_in_the_cache() {
        let mut pp = PacmanProgress::new(false);
        feed_all(&mut pp, HTOP_INSTALL.split(":: Retrieving").next().unwrap());
        assert!(pp.download_progress(100).is_none()); // not downloading yet
        pp.feed(":: Retrieving packages...");
        let half = pp.download_progress(2 * 1_048_576).unwrap();
        assert_eq!(half.text, "Downloading 2.0 MiB of 4.0 MiB");
        assert!((half.fraction.unwrap() - (0.05 + 0.55 * 0.5)).abs() < 1e-9);
        let over = pp.download_progress(99 * 1_048_576).unwrap(); // a bigger cache never overshoots
        assert!(over.fraction.unwrap() <= 0.60 + 1e-9);
    }

    #[test]
    fn removal_says_removing() {
        let mut pp = PacmanProgress::new(true);
        let v = feed_all(&mut pp, "checking dependencies...\n\nPackage (1)  Old Version\n\n:: Do you want to remove these packages? [Y/n] \n:: Running pre-transaction hooks...\n:: Processing package changes...\nremoving htop...\n:: Running post-transaction hooks...\n");
        assert!(v.iter().any(|p| p.text == "Removing…"));
    }

    #[test]
    fn sizes_parse() {
        assert_eq!(parse_size("  0.18 MiB"), Some((0.18 * 1_048_576.0) as u64));
        assert_eq!(parse_size("12 KiB"), Some(12 * 1024));
        assert_eq!(parse_size("1.5 GiB"), Some((1.5 * 1_073_741_824.0) as u64));
        assert_eq!(parse_size("lots"), None);
        assert_eq!(mib(2 * 1_048_576), "2.0 MiB");
    }

    #[test]
    fn flatpak_lines_give_a_percentage_when_there_is_one() {
        let p = parse_flatpak_line("Installing… ████████            45%  3.2 MB/s").unwrap();
        assert_eq!(p.fraction, Some(0.45));
        assert_eq!(p.text, "Installing…");
        assert_eq!(parse_flatpak_line("Looking for matches…"), None);
        assert_eq!(parse_flatpak_line("Downloading org.gnome.Platform").unwrap().fraction, None);
    }

    #[test]
    fn parses_pacman_search_output() {
        let out = "extra/firefox 130.0-1 (2.0 MiB) [installed]\n    Fast, Private & Safe Web Browser\nchaotic-aur/brave-origin-bin 1:1.96-1\n    Brave browser\n";
        let v = parse_pacman_ss(out);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], ("extra".into(), "firefox".into(), "Fast, Private & Safe Web Browser".into()));
        assert_eq!(v[1].1, "brave-origin-bin");
    }

    #[test]
    fn search_ranks_names_above_descriptions_and_hides_libraries() {
        let catalog = vec![
            app("Libfoo", "libfoo", "a library for foo"),
            app("Foo tools", "foo-tools", "tools"),
            app("Bar", "bar", "works with foo files"),
            app("Foo", "foo", "the foo app"),
        ];
        let r = search_catalog(&[], &catalog, "foo", 10);
        let names: Vec<&str> = r.iter().map(|a| a.package_name.as_str()).collect();
        assert_eq!(names, vec!["foo", "foo-tools", "bar"]); // exact, then prefix, then description; libfoo hidden
        assert_eq!(search_catalog(&[], &catalog, "libfoo", 10)[0].package_name, "libfoo"); // typed in full: shown
        assert!(search_catalog(&[], &catalog, "   ", 10).is_empty());
        assert_eq!(search_catalog(&[], &catalog, "foo", 2).len(), 2);
    }

    #[test]
    fn curated_apps_come_first_and_are_not_repeated() {
        let curated = vec![app("Firefox", "firefox", "browser")];
        let catalog = vec![app("Firefox", "firefox", "browser"), app("Firefox ESR", "firefox-esr", "browser")];
        let r = search_catalog(&curated, &catalog, "firefox", 10);
        assert_eq!(r.iter().filter(|a| a.package_name == "firefox").count(), 1);
        assert_eq!(r[0].package_name, "firefox");
    }

    #[test]
    fn cache_updates_and_notifies() {
        let c = InstalledCache { pacman: Default::default(), flatpak: Default::default(), listeners: Default::default() };
        let hits = std::rc::Rc::new(std::cell::Cell::new(0));
        let h = hits.clone();
        c.on_change(move || { h.set(h.get() + 1); true });
        assert!(!c.is_installed(&AppSource::Pacman, "x"));
        c.set_installed(&AppSource::Pacman, "x", true);
        assert!(c.is_installed(&AppSource::Pacman, "x"));
        assert!(!c.is_installed(&AppSource::Flatpak, "x"));
        c.set_installed(&AppSource::Pacman, "x", false);
        assert!(!c.is_installed(&AppSource::Pacman, "x"));
        assert_eq!(hits.get(), 2);
    }

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
