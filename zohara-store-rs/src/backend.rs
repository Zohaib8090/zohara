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
