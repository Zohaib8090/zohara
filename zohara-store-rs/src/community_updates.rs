//! Update checks for the two kinds of app nothing else keeps current: packages built from the AUR and AppImages.
//!
//! These only **look and tell**. Nothing here installs by itself: the Updates page lists what is newer, and the person presses
//! **Update** on the ones they want (an AUR package asks the same question as installing one; an AppImage is downloaded and
//! replaced).
//!
//! * **AUR**: the built packages are the "foreign" ones (`pacman -Qm`); the AUR is asked for their current version and
//!   `vercmp` (pacman's own comparison) says whether it is newer.
//! * **AppImage**: many carry update information (`.upd_info`, for GitHub releases: `gh-releases-zsync|owner|repo|latest|App-*.zsync`).
//!   The Store asks GitHub for that release and compares it with the file it installed from.

use crate::appimage::{self, Dirs, Meta};
use crate::aur;
use std::cmp::Ordering;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct AurUpdate {
    pub name: String,
    pub installed: String,
    pub available: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageUpdate {
    pub slug: String,
    pub name: String,
    pub asset: String,
    pub url: String,
    /// When the new file was published (seconds since 1970).
    pub stamp: u64,
    pub size: u64,
}

// ── AUR ────────────────────────────────────────────────────────────────────

/// `pacman -Qm` lines (`name version`) -> (name, version). Zohara's own packages never come from the AUR.
pub fn parse_qm(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let (n, v) = l.trim().split_once(' ')?;
            (aur::valid_name(n) && !n.starts_with("zohara-") && !v.trim().is_empty()).then(|| (n.to_string(), v.trim().to_string()))
        })
        .collect()
}

/// pacman's own version comparison, so epochs and `-pkgrel` are judged exactly as pacman would.
pub fn vercmp(a: &str, b: &str) -> Ordering {
    match Command::new("vercmp").args([a, b]).output() {
        Ok(o) if o.status.success() => match String::from_utf8_lossy(&o.stdout).trim() {
            "-1" => Ordering::Less,
            "1" => Ordering::Greater,
            _ => Ordering::Equal,
        },
        _ => a.cmp(b),
    }
}

/// Which of the installed packages the AUR's `info` answer lists with a newer version.
pub fn aur_updates_from(installed: &[(String, String)], info_json: &str, cmp: &dyn Fn(&str, &str) -> Ordering) -> Result<Vec<AurUpdate>, String> {
    let v: serde_json::Value = serde_json::from_str(info_json).map_err(|_| "The AUR sent an answer the Store could not read.".to_string())?;
    if v.get("type").and_then(|t| t.as_str()) == Some("error") {
        return Err(format!("The AUR said: {}", v.get("error").and_then(|e| e.as_str()).unwrap_or("unknown error")));
    }
    let results = v.get("results").and_then(|r| r.as_array()).ok_or_else(|| "The AUR sent an answer the Store could not read.".to_string())?;
    let mut out = Vec::new();
    for r in results {
        let (Some(name), Some(version)) = (r.get("Name").and_then(|n| n.as_str()), r.get("Version").and_then(|n| n.as_str())) else { continue };
        if let Some((_, have)) = installed.iter().find(|(n, _)| n == name) {
            if cmp(version, have) == Ordering::Greater {
                out.push(AurUpdate { name: name.to_string(), installed: have.clone(), available: version.to_string() });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Asks the AUR about every package built from it. Needs the network.
pub fn check_aur() -> Result<Vec<AurUpdate>, String> {
    let Ok(o) = Command::new("pacman").arg("-Qm").output() else { return Ok(Vec::new()) };
    let installed = parse_qm(&String::from_utf8_lossy(&o.stdout));
    if installed.is_empty() {
        return Ok(Vec::new());
    }
    let mut all = Vec::new();
    for chunk in installed.chunks(100) {
        let args: String = chunk.iter().map(|(n, _)| format!("arg%5B%5D={}", aur::encode_term(n))).collect::<Vec<_>>().join("&");
        let json = curl_json(&format!("https://aur.archlinux.org/rpc/v5/info?{args}"))?;
        all.extend(aur_updates_from(chunk, &json, &vercmp)?);
    }
    Ok(all)
}

fn curl_json(url: &str) -> Result<String, String> {
    let o = Command::new("curl")
        .args(["-sfL", "--max-time", "20", "-A", "ZoharaStore", "-H", "Accept: application/json"])
        .arg(url)
        .output()
        .map_err(|_| "The Store could not check (curl is missing).".to_string())?;
    if !o.status.success() {
        return Err("Couldn't reach the server to check for updates. Are you online?".into());
    }
    Ok(String::from_utf8_lossy(&o.stdout).to_string())
}

// ── AppImage ───────────────────────────────────────────────────────────────

/// Where an AppImage says its updates are published.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateSource {
    GitHub { owner: String, repo: String, tag: String, pattern: String },
}

fn plain(s: &str) -> bool {
    !s.is_empty() && s.len() <= 100 && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))
}

/// `gh-releases-zsync|owner|repo|latest|App-*x86_64.AppImage.zsync`. Other kinds (plain `zsync|url`, bintray) are not checked.
pub fn parse_update_info(info: &str) -> Option<UpdateSource> {
    let p: Vec<&str> = info.trim().split('|').collect();
    if p.len() != 5 || !(p[0] == "gh-releases-zsync") {
        return None;
    }
    (plain(p[1]) && plain(p[2]) && plain(p[3]) && !p[4].is_empty() && p[4].len() <= 200 && !p[4].contains('/')).then(|| UpdateSource::GitHub {
        owner: p[1].into(),
        repo: p[2].into(),
        tag: p[3].into(),
        pattern: p[4].into(),
    })
}

/// `*` and `?` matching, as in the update information's file pattern.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let (p, n): (Vec<char>, Vec<char>) = (pattern.chars().collect(), name.chars().collect());
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// `2026-10-04T14:58:08Z` -> seconds since 1970.
pub fn iso_to_epoch(s: &str) -> Option<u64> {
    let s = s.trim().trim_end_matches('Z');
    let (date, time) = s.split_once('T')?;
    let d: Vec<i64> = date.split('-').filter_map(|x| x.parse().ok()).collect();
    let t: Vec<i64> = time.split(':').filter_map(|x| x.split('.').next()?.parse().ok()).collect();
    if d.len() != 3 || t.len() != 3 {
        return None;
    }
    let (y, m, day) = (d[0], d[1], d[2]);
    // days from civil (Howard Hinnant's algorithm)
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let secs = days * 86400 + t[0] * 3600 + t[1] * 60 + t[2];
    (secs >= 0).then_some(secs as u64)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub stamp: u64,
    pub size: u64,
}

/// The files of a GitHub release (`GET /repos/{owner}/{repo}/releases/latest`).
pub fn parse_release(json: &str) -> Vec<Asset> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    v.get("assets")
        .and_then(|a| a.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|a| {
                    let url = a.get("browser_download_url")?.as_str()?.to_string();
                    // Only GitHub's own download addresses are ever fetched.
                    if !url.starts_with("https://github.com/") {
                        return None;
                    }
                    Some(Asset {
                        name: a.get("name")?.as_str()?.to_string(),
                        url,
                        stamp: a.get("updated_at").and_then(|d| d.as_str()).and_then(iso_to_epoch).unwrap_or(0),
                        size: a.get("size").and_then(|d| d.as_u64()).unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The release file that is newer than the one installed, if any. The pattern names the `.zsync` helper file; the
/// AppImage itself has the same name without it. A file counts as newer when it was published after the installed one was
/// made (a minute of slack), or, for an app installed before dates were recorded, when its name differs.
pub fn newer_asset(assets: &[Asset], pattern: &str, meta: &Meta) -> Option<Asset> {
    let want = pattern.strip_suffix(".zsync").unwrap_or(pattern);
    let cand = assets.iter().filter(|a| glob_match(want, &a.name) && !a.name.ends_with(".zsync")).max_by_key(|a| a.stamp)?;
    let newer = if meta.source_stamp == 0 { cand.name != meta.source_name } else { cand.stamp > meta.source_stamp + 60 };
    newer.then(|| cand.clone())
}

fn latest_release_url(owner: &str, repo: &str, tag: &str) -> String {
    if tag == "latest" {
        format!("https://api.github.com/repos/{owner}/{repo}/releases/latest")
    } else {
        format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}")
    }
}

/// Checks every AppImage the Store installed that says where its updates are. Returns what is newer, and a line for
/// anything that could not be checked.
pub fn check_images(dirs: &Dirs) -> (Vec<ImageUpdate>, Vec<String>) {
    let (mut found, mut errors) = (Vec::new(), Vec::new());
    for app in appimage::list(dirs) {
        let mut meta = appimage::read_meta(&app.slug, dirs);
        if meta.update_info.is_empty() {
            meta.update_info = appimage::read_update_info(&dirs.apps.join(format!("{}.AppImage", app.slug))).unwrap_or_default();
        }
        let Some(UpdateSource::GitHub { owner, repo, tag, pattern }) = parse_update_info(&meta.update_info) else { continue };
        if meta.source_stamp == 0 {
            meta.source_stamp = std::fs::metadata(dirs.apps.join(format!("{}.AppImage", app.slug))).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
        }
        match curl_json(&latest_release_url(&owner, &repo, &tag)) {
            Ok(json) => {
                if let Some(a) = newer_asset(&parse_release(&json), &pattern, &meta) {
                    found.push(ImageUpdate { slug: app.slug.clone(), name: app.name.clone(), asset: a.name, url: a.url, stamp: a.stamp, size: a.size });
                }
            }
            Err(_) => errors.push(format!("Couldn't check {} for updates.", app.name)),
        }
    }
    (found, errors)
}

/// Downloads the newer file and puts it in place of the installed one.
pub fn apply_image(u: &ImageUpdate, dirs: &Dirs, say: &dyn Fn(&str)) -> Result<(), String> {
    if !u.url.starts_with("https://github.com/") {
        return Err("That download address looked wrong, so nothing was changed.".into());
    }
    say(&format!("Downloading {}…", u.asset));
    let tmp_dir = std::env::temp_dir().join(format!("zohara-store-update-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let file = tmp_dir.join(&u.asset.replace('/', "_"));
    let ok = Command::new("curl")
        .args(["-fL", "--max-time", "3600", "--max-filesize", "4294967296", "-A", "ZoharaStore", "-o"])
        .arg(&file)
        .arg(&u.url)
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let result = if ok {
        say("Installing the new version…");
        appimage::install_with(&file, dirs, Some(&u.slug), Some(u.stamp)).map(|_| ())
    } else {
        Err("Couldn't download it. Check your internet connection and try again.".to_string())
    };
    let _ = std::fs::remove_dir_all(&tmp_dir);
    result
}

/// Builds the newer version of an AUR package and installs it (the same build as installing one), with its output sent to `tx`.
pub fn apply_aur(name: &str, tx: &std::sync::mpsc::Sender<String>) -> Result<(), String> {
    if !aur::have_builder() {
        return Err("Building from the AUR needs the 'yay' program, which isn't installed.".into());
    }
    let cmd = aur::install_command(name).ok_or_else(|| "That is not a valid package name.".to_string())?;
    let _ = tx.send(format!("Building {name} from the AUR…"));
    crate::updates::run_logged(cmd, tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numeric(a: &str, b: &str) -> Ordering {
        let n = |s: &str| s.split(['.', '-']).filter_map(|p| p.parse::<u64>().ok()).collect::<Vec<_>>();
        n(a).cmp(&n(b))
    }

    #[test]
    fn built_packages_are_read_and_zoharas_own_are_left_out() {
        let got = parse_qm("yay 12.5.0-1\nventoy 1.1.07-2\nzohara-voice 0.1-1\n\nbad;name 1-1\nnoversion\n");
        assert_eq!(got, vec![("yay".to_string(), "12.5.0-1".to_string()), ("ventoy".to_string(), "1.1.07-2".to_string())]);
    }

    #[test]
    fn only_newer_aur_versions_are_reported() {
        let installed = vec![("yay".to_string(), "12.5.0-1".to_string()), ("ventoy".to_string(), "1.1.07-2".to_string()), ("gone".to_string(), "1-1".to_string())];
        let json = r#"{"type":"multiinfo","results":[{"Name":"yay","Version":"12.5.1-1"},{"Name":"ventoy","Version":"1.1.07-2"},{"Name":"unknown","Version":"9-1"}]}"#;
        let got = aur_updates_from(&installed, json, &numeric).unwrap();
        assert_eq!(got, vec![AurUpdate { name: "yay".into(), installed: "12.5.0-1".into(), available: "12.5.1-1".into() }]);
        assert!(aur_updates_from(&installed, r#"{"type":"error","error":"Too many package results."}"#, &numeric).unwrap_err().contains("Too many"));
        assert!(aur_updates_from(&installed, "nope", &numeric).is_err());
    }

    #[test]
    fn pacmans_own_comparison_judges_epochs_and_releases() {
        // Uses the real `vercmp` that ships with pacman.
        assert_eq!(vercmp("1.0-2", "1.0-1"), Ordering::Greater);
        assert_eq!(vercmp("1.9", "1.10"), Ordering::Less);
        assert_eq!(vercmp("1:1.0", "2.0"), Ordering::Greater);
        assert_eq!(vercmp("1.0-1", "1.0-1"), Ordering::Equal);
    }

    #[test]
    fn update_information_is_read_and_unsafe_or_unknown_kinds_are_refused() {
        assert_eq!(
            parse_update_info("gh-releases-zsync|AppImage|appimagetool|continuous|appimagetool-*x86_64.AppImage.zsync"),
            Some(UpdateSource::GitHub { owner: "AppImage".into(), repo: "appimagetool".into(), tag: "continuous".into(), pattern: "appimagetool-*x86_64.AppImage.zsync".into() })
        );
        for bad in ["", "zsync|https://x/y.zsync", "gh-releases-zsync|a|b|latest", "gh-releases-zsync|a/b|c|latest|x*", "gh-releases-zsync|a|b|latest|../x", "gh-releases-zsync|a|b c|latest|x*"] {
            assert_eq!(parse_update_info(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn file_patterns_match_with_stars_and_question_marks() {
        assert!(glob_match("App-*x86_64.AppImage", "App-1.2.3-x86_64.AppImage"));
        assert!(glob_match("appimagetool-*x86_64.AppImage", "appimagetool-x86_64.AppImage"));
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("App-*x86_64.AppImage", "App-1.2.3-aarch64.AppImage"));
        assert!(!glob_match("a?c", "ac"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn release_times_are_read() {
        assert_eq!(iso_to_epoch("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(iso_to_epoch("2026-10-04T14:58:08Z"), Some(1791125888));
        assert_eq!(iso_to_epoch("2000-03-01T00:00:00Z"), Some(951868800));
        assert_eq!(iso_to_epoch("nonsense"), None);
    }

    fn release() -> String {
        r#"{"tag_name":"continuous","assets":[
          {"name":"App-x86_64.AppImage","browser_download_url":"https://github.com/o/r/releases/download/continuous/App-x86_64.AppImage","updated_at":"2026-10-04T14:58:08Z","size":1000},
          {"name":"App-x86_64.AppImage.zsync","browser_download_url":"https://github.com/o/r/releases/download/continuous/App-x86_64.AppImage.zsync","updated_at":"2026-10-04T14:58:08Z","size":10},
          {"name":"App-aarch64.AppImage","browser_download_url":"https://github.com/o/r/releases/download/continuous/App-aarch64.AppImage","updated_at":"2026-10-04T14:58:08Z","size":1000},
          {"name":"Evil-x86_64.AppImage","browser_download_url":"https://evil.example/x","updated_at":"2026-10-04T14:58:08Z","size":1}]}"#
            .to_string()
    }

    #[test]
    fn a_release_is_newer_only_when_published_after_the_installed_file() {
        let assets = parse_release(&release());
        assert_eq!(assets.len(), 3, "downloads from other hosts are dropped");
        let pat = "App-*x86_64.AppImage.zsync";
        let released = iso_to_epoch("2026-10-04T14:58:08Z").unwrap();
        let installed = |stamp: u64, name: &str| Meta { source_name: name.into(), source_stamp: stamp, update_info: String::new() };
        // Installed after the release was published: nothing newer.
        assert!(newer_asset(&assets, pat, &installed(released + 3600, "App-x86_64.AppImage")).is_none());
        // Installed from a file older than the release: newer, and the AppImage is picked, not the .zsync helper or another CPU.
        let n = newer_asset(&assets, pat, &installed(released - 86400, "App-x86_64.AppImage")).unwrap();
        assert_eq!(n.name, "App-x86_64.AppImage");
        // No recorded time (installed before dates were kept): a different name counts, the same name does not.
        assert!(newer_asset(&assets, pat, &installed(0, "App-x86_64.AppImage")).is_none());
        assert!(newer_asset(&assets, pat, &installed(0, "App-1.0-x86_64.AppImage")).is_some());
        assert!(newer_asset(&[], pat, &installed(0, "x")).is_none());
    }

    /// A tiny ELF with one extra section named `.upd_info`.
    fn elf_with_update_info(info: &[u8]) -> Vec<u8> {
        let strtab = b"\0.shstrtab\0.upd_info\0";
        let mut b = vec![0u8; 64];
        b[..4].copy_from_slice(b"\x7fELF");
        b[4] = 2;
        b[5] = 1;
        let data_off = 64u64;
        b.extend_from_slice(strtab);
        let info_off = b.len() as u64;
        b.extend_from_slice(info);
        let shoff = b.len() as u64;
        let sh = |name: u32, off: u64, size: u64| {
            let mut e = vec![0u8; 64];
            e[0..4].copy_from_slice(&name.to_le_bytes());
            e[0x18..0x20].copy_from_slice(&off.to_le_bytes());
            e[0x20..0x28].copy_from_slice(&size.to_le_bytes());
            e
        };
        b.extend(vec![0u8; 64]); // section 0: null
        b.extend(sh(1, data_off, strtab.len() as u64)); // 1: .shstrtab
        b.extend(sh(11, info_off, info.len() as u64)); // 2: .upd_info
        b[0x28..0x30].copy_from_slice(&shoff.to_le_bytes());
        b[0x3A..0x3C].copy_from_slice(&64u16.to_le_bytes());
        b[0x3C..0x3E].copy_from_slice(&3u16.to_le_bytes());
        b[0x3E..0x40].copy_from_slice(&1u16.to_le_bytes());
        b
    }

    #[test]
    fn the_update_information_is_read_from_the_program_without_running_it() {
        let elf = elf_with_update_info(b"gh-releases-zsync|o|r|latest|App-*.zsync\0\0\0\0");
        let got = appimage::elf_section(&mut std::io::Cursor::new(&elf), ".upd_info", 1024).unwrap();
        assert!(got.starts_with(b"gh-releases-zsync|o|r|latest|App-*.zsync"));
        assert!(appimage::elf_section(&mut std::io::Cursor::new(&elf), ".nothing", 1024).is_none());
        assert!(appimage::elf_section(&mut std::io::Cursor::new(b"not an elf at all, just some text padding.................................."), ".upd_info", 8).is_none());
    }

    #[test]
    fn what_was_installed_is_remembered_for_the_next_check() {
        let m = Meta { source_name: "App-1.0.AppImage".into(), source_stamp: 1791125888, update_info: "gh-releases-zsync|o|r|latest|App-*.zsync".into() };
        assert_eq!(appimage::parse_meta(&appimage::meta_text(&m)), m);
        assert_eq!(appimage::parse_meta(""), Meta::default());
    }

    /// The real AUR and pacman's `vercmp`: `cargo test community_updates::tests::live_aur -- --ignored`.
    #[test]
    #[ignore]
    fn live_aur_finds_a_newer_version_and_leaves_a_current_one_alone() {
        let url = "https://aur.archlinux.org/rpc/v5/info?arg%5B%5D=yay&arg%5B%5D=ventoy";
        let json = curl_json(url).expect("the AUR answers");
        let current: Vec<(String, String)> = serde_json::from_str::<serde_json::Value>(&json).unwrap()["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["Name"].as_str().unwrap().to_string(), r["Version"].as_str().unwrap().to_string()))
            .collect();
        assert_eq!(current.len(), 2);
        // Installed at the current versions: nothing to update.
        assert!(aur_updates_from(&current, &json, &vercmp).unwrap().is_empty());
        // Installed at an old version of yay: yay is reported.
        let old = vec![("yay".to_string(), "1.0-1".to_string()), current[1].clone()];
        let got = aur_updates_from(&old, &json, &vercmp).unwrap();
        println!("{got:?}");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "yay");
    }

    /// A real AppImage with update information, and GitHub: `ZOHARA_TEST_APPIMAGE=... cargo test community_updates::tests::live_appimage -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_appimage_update_is_found_downloaded_and_installed_over_the_old_one() {
        let src = std::path::PathBuf::from(std::env::var("ZOHARA_TEST_APPIMAGE").expect("set ZOHARA_TEST_APPIMAGE"));
        let root = std::env::temp_dir().join(format!("zohara-upd-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dirs = Dirs { apps: root.join("apps"), icons: root.join("icons"), launchers: root.join("launchers") };
        let got = appimage::install(&src, &dirs).expect("install");
        let meta = appimage::read_meta(&got.slug, &dirs);
        println!("update info in the file: {:?}", meta.update_info);
        assert!(parse_update_info(&meta.update_info).is_some(), "the AppImage should say where its updates are");

        // Installed from a file made just now: GitHub has nothing newer.
        let (none, errs) = check_images(&dirs);
        println!("fresh install -> updates {none:?}, errors {errs:?}");
        assert!(errs.is_empty(), "{errs:?}");
        assert!(none.is_empty());

        // Pretend it was installed from a file made long ago: the release is newer.
        let old = Meta { source_stamp: 1_700_000_000, ..meta.clone() };
        std::fs::write(appimage::meta_path(&got.slug, &dirs), appimage::meta_text(&old)).unwrap();
        let (found, errs) = check_images(&dirs);
        println!("old install -> {found:?}, errors {errs:?}");
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(found.len(), 1);
        let u = found[0].clone();
        assert!(u.url.starts_with("https://github.com/") && !u.asset.ends_with(".zsync"));

        // Press Update: download and replace in place, keeping the same app.
        let say = |l: &str| println!("  {l}");
        apply_image(&u, &dirs, &say).expect("update");
        let after = appimage::read_meta(&got.slug, &dirs);
        assert_eq!(after.source_stamp, u.stamp);
        assert!(appimage::is_installed(&got.slug, &dirs));
        assert_eq!(appimage::list(&dirs).len(), 1, "still one app, not two");
        let (again, _) = check_images(&dirs);
        assert!(again.is_empty(), "after updating there is nothing newer");
        let _ = std::fs::remove_dir_all(&root);
    }
}
