//! AppImages: one-file apps that many projects publish on their GitHub releases page.
//!
//! "Installing" one means copying it into the person's own folder (no administrator rights), reading its name and icon
//! from inside it, and adding a launcher to the menu. The file is **not run** to do this: its name and icon are read
//! with `unsquashfs` (from `squashfs-tools`), and without that the name comes from the file name and the icon is generic.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Where the pieces of installed AppImages live.
#[derive(Clone, Debug)]
pub struct Dirs {
    /// The AppImage files themselves.
    pub apps: PathBuf,
    /// Their icons.
    pub icons: PathBuf,
    /// Their menu launchers.
    pub launchers: PathBuf,
}

impl Dirs {
    pub fn for_user() -> Dirs {
        let data = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share"));
        Dirs { apps: data.join("zohara-store/appimages"), icons: data.join("zohara-store/appimage-icons"), launchers: data.join("applications") }
    }
}

/// An installed AppImage as the Library lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub slug: String,
    pub name: String,
    pub comment: String,
    pub icon: String,
}

/// Whether these first bytes are an AppImage of the current kind (type 2): an ELF program with `AI\x02` at byte 8.
pub fn is_type2(head: &[u8]) -> bool {
    head.len() >= 11 && head.starts_with(b"\x7fELF") && &head[8..11] == b"AI\x02"
}

/// Where the compressed file system inside a type 2 AppImage starts: just after the program's section headers
/// (64-bit little-endian ELF: `e_shoff + e_shentsize * e_shnum`).
pub fn squashfs_offset(head: &[u8]) -> Option<u64> {
    if head.len() < 0x40 || !head.starts_with(b"\x7fELF") || head[4] != 2 || head[5] != 1 {
        return None;
    }
    let shoff = u64::from_le_bytes(head[0x28..0x30].try_into().ok()?);
    let shentsize = u16::from_le_bytes(head[0x3A..0x3C].try_into().ok()?) as u64;
    let shnum = u16::from_le_bytes(head[0x3C..0x3E].try_into().ok()?) as u64;
    shoff.checked_add(shentsize.checked_mul(shnum)?)
}

/// A folder-name-safe id from a file name: lower case letters and digits, joined by dashes, without the extension.
pub fn slug(file_name: &str) -> String {
    let stem = file_name.rsplit_once('.').map(|(s, e)| if e.eq_ignore_ascii_case("appimage") { s } else { file_name }).unwrap_or(file_name);
    let s: String = stem.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let joined = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if joined.is_empty() {
        "appimage".to_string()
    } else {
        joined
    }
}

/// A readable name from a file name such as `Cool_App-1.4.2-x86_64.AppImage` -> "Cool App".
pub fn guess_name(file_name: &str) -> String {
    let stem = file_name.rsplit_once('.').map(|(s, e)| if e.eq_ignore_ascii_case("appimage") { s } else { file_name }).unwrap_or(file_name);
    let mut words = Vec::new();
    for w in stem.split(['-', '_', ' ']).filter(|w| !w.is_empty()) {
        let lw = w.to_lowercase();
        let version = lw.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) || (lw.starts_with('v') && lw[1..].starts_with(|c: char| c.is_ascii_digit()));
        if version || matches!(lw.as_str(), "x86" | "x86_64" | "amd64" | "linux" | "appimage" | "x64" | "64bit") {
            break;
        }
        words.push(w.to_string());
    }
    if words.is_empty() {
        return "AppImage".to_string();
    }
    words.join(" ")
}

/// (name, comment, icon, categories) from a launcher file's `[Desktop Entry]`.
pub fn read_entry(text: &str) -> (String, String, String, String) {
    let (mut name, mut comment, mut icon, mut cats) = (String::new(), String::new(), String::new(), String::new());
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == "[Desktop Entry]";
            continue;
        }
        if !inside {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            match k {
                "Name" => name = v.trim().to_string(),
                "Comment" => comment = v.trim().to_string(),
                "Icon" => icon = v.trim().to_string(),
                "Categories" => cats = v.trim().to_string(),
                _ => {}
            }
        }
    }
    (name, comment, icon, cats)
}

/// Menu categories that are safe to write: letters only, so nothing can add a key or a line.
fn clean_categories(cats: &str) -> String {
    let ok: Vec<&str> = cats.split(';').map(str::trim).filter(|c| !c.is_empty() && c.chars().all(|ch| ch.is_ascii_alphabetic())).collect();
    if ok.is_empty() {
        "Utility;".to_string()
    } else {
        format!("{};", ok.join(";"))
    }
}

fn one_line(s: &str) -> String {
    s.replace(['\n', '\r'], " ").trim().to_string()
}

fn exec_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' | '`' | '$' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '%' => out.push_str("%%"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn launcher_text(slug: &str, name: &str, comment: &str, icon: &str, categories: &str, appimage: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={} %U\nIcon={}\nTerminal=false\nCategories={}\nX-Zohara-AppImage={}\n",
        one_line(name),
        one_line(comment),
        exec_quote(&appimage.to_string_lossy()),
        if icon.is_empty() { "application-x-executable".to_string() } else { one_line(icon) },
        clean_categories(categories),
        slug
    )
}

pub fn have_unsquashfs() -> bool {
    std::env::var("PATH").unwrap_or_default().split(':').any(|d| Path::new(d).join("unsquashfs").is_file())
}

/// Pulls files out of the AppImage's own file system without running it. `patterns` are names inside it.
fn extract(file: &Path, offset: u64, into: &Path, patterns: &[&str]) -> bool {
    Command::new("unsquashfs")
        .args(["-no-progress", "-quiet", "-force", "-offset", &offset.to_string(), "-dest"])
        .arg(into)
        .arg(file)
        .args(patterns)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Where a symbolic link inside the AppImage points, as a path inside it: `from` is the link's own path, `target` what
/// it holds. Anything that would leave the AppImage (absolute, or `..` past the top) is refused.
pub fn resolve_link(from: &str, target: &Path) -> Option<String> {
    use std::path::Component;
    let mut parts: Vec<String> = Path::new(from).parent().map(|p| p.components().filter_map(|c| if let Component::Normal(n) = c { n.to_str().map(str::to_string) } else { None }).collect()).unwrap_or_default();
    for c in target.components() {
        match c {
            Component::Normal(n) => parts.push(n.to_str()?.to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Extracts one file by its path inside the AppImage, following links that stay inside it. Returns where it landed.
fn extract_file(file: &Path, offset: u64, tmp: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = rel.to_string();
    for _ in 0..4 {
        if !extract(file, offset, tmp, &[&cur]) {
            return None;
        }
        let p = tmp.join(&cur);
        let meta = std::fs::symlink_metadata(&p).ok()?;
        if meta.file_type().is_symlink() {
            let target = std::fs::read_link(&p).ok()?;
            cur = resolve_link(&cur, &target)?;
        } else if meta.is_file() {
            return Some(p);
        } else {
            return None;
        }
    }
    None
}

/// Reads the AppImage's launcher and icon into `tmp` if it can; returns (name, comment, icon file, categories).
fn read_inside(file: &Path, head: &[u8], tmp: &Path) -> Option<(String, String, Option<PathBuf>, String)> {
    if !have_unsquashfs() {
        return None;
    }
    let offset = squashfs_offset(head)?;
    std::fs::create_dir_all(tmp).ok()?;
    // The launcher sits at the top of the AppImage, usually as a link into usr/share/applications.
    if !extract(file, offset, tmp, &["*.desktop"]) {
        return None;
    }
    let top = std::fs::read_dir(tmp).ok()?.flatten().map(|e| e.file_name().to_string_lossy().to_string()).find(|n| n.ends_with(".desktop"))?;
    let desktop = extract_file(file, offset, tmp, &top)?;
    let (name, comment, icon_name, cats) = read_entry(&std::fs::read_to_string(desktop).ok()?);
    let mut icon_file = None;
    if !icon_name.is_empty() && !icon_name.contains('/') {
        for cand in [format!("{icon_name}.png"), format!("{icon_name}.svg"), ".DirIcon".to_string()] {
            if let Some(p) = extract_file(file, offset, tmp, &cand) {
                icon_file = Some(p);
                break;
            }
        }
    }
    Some((name, comment, icon_file, cats))
}

/// What the Store remembers about where an installed AppImage came from, to tell later whether a newer one exists.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Meta {
    /// The file name it was installed from.
    pub source_name: String,
    /// When that file was made (seconds since 1970): its modified time, or the release time of a downloaded update.
    pub source_stamp: u64,
    /// The update information the AppImage carries about itself (`gh-releases-zsync|owner|repo|latest|App-*.zsync`).
    pub update_info: String,
}

pub fn meta_text(m: &Meta) -> String {
    format!("source_name={}\nsource_stamp={}\nupdate_info={}\n", one_line(&m.source_name), m.source_stamp, one_line(&m.update_info))
}

pub fn parse_meta(text: &str) -> Meta {
    let mut m = Meta::default();
    for l in text.lines() {
        if let Some((k, v)) = l.split_once('=') {
            match k {
                "source_name" => m.source_name = v.to_string(),
                "source_stamp" => m.source_stamp = v.trim().parse().unwrap_or(0),
                "update_info" => m.update_info = v.to_string(),
                _ => {}
            }
        }
    }
    m
}

pub fn meta_path(slug: &str, dirs: &Dirs) -> PathBuf {
    dirs.apps.join(format!("{slug}.meta"))
}

pub fn read_meta(slug: &str, dirs: &Dirs) -> Meta {
    std::fs::read_to_string(meta_path(slug, dirs)).map(|t| parse_meta(&t)).unwrap_or_default()
}

/// The bytes of one named section of an ELF file (64-bit, little-endian), at most `cap` of them.
pub fn elf_section<R: std::io::Read + std::io::Seek>(r: &mut R, name: &str, cap: u64) -> Option<Vec<u8>> {
    use std::io::SeekFrom;
    let mut head = [0u8; 64];
    r.seek(SeekFrom::Start(0)).ok()?;
    r.read_exact(&mut head).ok()?;
    if &head[..4] != b"\x7fELF" || head[4] != 2 || head[5] != 1 {
        return None;
    }
    let shoff = u64::from_le_bytes(head[0x28..0x30].try_into().ok()?);
    let shentsize = u16::from_le_bytes(head[0x3A..0x3C].try_into().ok()?) as u64;
    let shnum = u16::from_le_bytes(head[0x3C..0x3E].try_into().ok()?) as u64;
    let shstrndx = u16::from_le_bytes(head[0x3E..0x40].try_into().ok()?) as u64;
    if shentsize < 0x28 || shnum == 0 || shnum > 4096 || shstrndx >= shnum {
        return None;
    }
    let mut table = vec![0u8; (shentsize * shnum) as usize];
    r.seek(SeekFrom::Start(shoff)).ok()?;
    r.read_exact(&mut table).ok()?;
    let entry = |i: u64| -> Option<(u32, u64, u64)> {
        let b = table.get((i * shentsize) as usize..((i + 1) * shentsize) as usize)?;
        Some((u32::from_le_bytes(b[0..4].try_into().ok()?), u64::from_le_bytes(b[0x18..0x20].try_into().ok()?), u64::from_le_bytes(b[0x20..0x28].try_into().ok()?)))
    };
    let (_, str_off, str_size) = entry(shstrndx)?;
    if str_size > 1 << 20 {
        return None;
    }
    let mut strtab = vec![0u8; str_size as usize];
    r.seek(SeekFrom::Start(str_off)).ok()?;
    r.read_exact(&mut strtab).ok()?;
    for i in 0..shnum {
        let (name_off, off, size) = entry(i)?;
        let n = strtab.get(name_off as usize..)?;
        let end = n.iter().position(|&c| c == 0).unwrap_or(n.len());
        if &n[..end] == name.as_bytes() {
            let take = size.min(cap) as usize;
            let mut data = vec![0u8; take];
            r.seek(SeekFrom::Start(off)).ok()?;
            r.read_exact(&mut data).ok()?;
            return Some(data);
        }
    }
    None
}

/// The update information an AppImage carries (`.upd_info`), if it has any.
pub fn read_update_info(file: &Path) -> Option<String> {
    let mut f = std::fs::File::open(file).ok()?;
    let bytes = elf_section(&mut f, ".upd_info", 1024)?;
    let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
    let s = String::from_utf8_lossy(&bytes[..end]).trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Installs the AppImage at `path` for this person.
pub fn install(path: &Path, dirs: &Dirs) -> Result<Installed, String> {
    install_with(path, dirs, None, None)
}

/// `install`, optionally keeping an existing id (an update replaces the app it updates) and recording a release time for
/// a file that was downloaded as an update.
pub fn install_with(path: &Path, dirs: &Dirs, keep_slug: Option<&str>, stamp: Option<u64>) -> Result<Installed, String> {
    let file_name = path.file_name().and_then(|n| n.to_str()).ok_or("That file has no name.")?.to_string();
    let mut head = vec![0u8; 4096];
    {
        use std::io::Read;
        let mut f = std::fs::File::open(path).map_err(|e| format!("Couldn't open the file ({e})."))?;
        let n = f.read(&mut head).map_err(|e| e.to_string())?;
        head.truncate(n);
    }
    if !is_type2(&head) {
        return Err("This isn't an AppImage the Store can install. Only the current kind (type 2) is supported.".into());
    }
    let slug = match keep_slug {
        Some(s) if !s.is_empty() && !s.contains('/') && !s.starts_with('.') => s.to_string(),
        _ => slug(&file_name),
    };
    std::fs::create_dir_all(&dirs.apps).map_err(|e| e.to_string())?;
    let dest = dirs.apps.join(format!("{slug}.AppImage"));
    // Copy beside it and rename: replacing an app that is running works (it keeps the old file until it closes).
    let staging = dirs.apps.join(format!(".{slug}.AppImage.new"));
    std::fs::copy(path, &staging).map_err(|e| format!("Couldn't copy the file ({e})."))?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&staging, &dest).map_err(|e| format!("Couldn't put the file in place ({e})."))?;
    let source_stamp = stamp.unwrap_or_else(|| std::fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0));
    let meta = Meta { source_name: file_name.clone(), source_stamp, update_info: read_update_info(&dest).unwrap_or_default() };
    let _ = std::fs::write(meta_path(&slug, dirs), meta_text(&meta));

    let tmp = std::env::temp_dir().join(format!("zohara-appimage-{}-{slug}", std::process::id()));
    let inside = read_inside(&dest, &head, &tmp);
    let (name, comment, icon_src, cats) = match inside {
        Some((n, c, i, cats)) if !n.is_empty() => (n, c, i, cats),
        Some((_, c, i, cats)) => (guess_name(&file_name), c, i, cats),
        None => (guess_name(&file_name), String::new(), None, String::new()),
    };
    let mut icon = String::new();
    if let Some(src) = icon_src {
        let ext = if std::fs::read(&src).map(|b| b.starts_with(b"\x89PNG")).unwrap_or(false) { "png" } else if src.extension().map(|e| e == "svg").unwrap_or(false) { "svg" } else { "png" };
        let _ = std::fs::create_dir_all(&dirs.icons);
        let to = dirs.icons.join(format!("{slug}.{ext}"));
        if std::fs::copy(&src, &to).is_ok() {
            icon = to.to_string_lossy().to_string();
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);

    std::fs::create_dir_all(&dirs.launchers).map_err(|e| e.to_string())?;
    let launcher = dirs.launchers.join(format!("zohara-appimage-{slug}.desktop"));
    std::fs::write(&launcher, launcher_text(&slug, &name, &comment, &icon, &cats, &dest)).map_err(|e| e.to_string())?;
    let _ = Command::new("update-desktop-database").arg(&dirs.launchers).stderr(Stdio::null()).status();
    Ok(Installed { slug, name, comment, icon })
}

/// Removes an installed AppImage: the file, its icon and its launcher.
pub fn uninstall(slug: &str, dirs: &Dirs) -> Result<(), String> {
    if slug.is_empty() || slug.contains('/') || slug.starts_with('.') {
        return Err("That is not an installed AppImage.".into());
    }
    let _ = std::fs::remove_file(dirs.apps.join(format!("{slug}.AppImage")));
    for ext in ["png", "svg"] {
        let _ = std::fs::remove_file(dirs.icons.join(format!("{slug}.{ext}")));
    }
    let _ = std::fs::remove_file(dirs.launchers.join(format!("zohara-appimage-{slug}.desktop")));
    let _ = std::fs::remove_file(meta_path(slug, dirs));
    let _ = Command::new("update-desktop-database").arg(&dirs.launchers).stderr(Stdio::null()).status();
    Ok(())
}

/// The AppImages installed through the Store (their launchers carry `X-Zohara-AppImage`).
pub fn list(dirs: &Dirs) -> Vec<Installed> {
    let Ok(rd) = std::fs::read_dir(&dirs.launchers) else { return Vec::new() };
    let mut out: Vec<Installed> = rd
        .flatten()
        .filter_map(|e| {
            let text = std::fs::read_to_string(e.path()).ok()?;
            let slug = text.lines().find_map(|l| l.strip_prefix("X-Zohara-AppImage="))?.trim().to_string();
            if !dirs.apps.join(format!("{slug}.AppImage")).is_file() {
                return None;
            }
            let (name, comment, icon, _) = read_entry(&text);
            Some(Installed { slug, name, comment, icon })
        })
        .collect();
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

pub fn is_installed(slug: &str, dirs: &Dirs) -> bool {
    dirs.apps.join(format!("{slug}.AppImage")).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_appimage(extra: &[u8]) -> Vec<u8> {
        let mut b = vec![0u8; 0x80];
        b[..4].copy_from_slice(b"\x7fELF");
        b[4] = 2; // 64-bit
        b[5] = 1; // little endian
        b[8..11].copy_from_slice(b"AI\x02");
        b[0x28..0x30].copy_from_slice(&1000u64.to_le_bytes()); // e_shoff
        b[0x3A..0x3C].copy_from_slice(&64u16.to_le_bytes()); // e_shentsize
        b[0x3C..0x3E].copy_from_slice(&10u16.to_le_bytes()); // e_shnum
        b.extend_from_slice(extra);
        b
    }

    #[test]
    fn only_current_appimages_are_recognised() {
        assert!(is_type2(&fake_appimage(b"")));
        assert!(!is_type2(b"\x7fELF\x02\x01\x01\x00AI\x01"), "type 1 is not supported");
        assert!(!is_type2(b"#!/bin/sh\nexit"));
        assert!(!is_type2(b""));
    }

    #[test]
    fn the_file_system_starts_after_the_section_headers() {
        assert_eq!(squashfs_offset(&fake_appimage(b"")), Some(1000 + 64 * 10));
        assert_eq!(squashfs_offset(b"short"), None);
        let mut big = fake_appimage(b"");
        big[0x28..0x30].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(squashfs_offset(&big), None, "overflow is refused, not wrapped");
    }

    #[test]
    fn ids_and_names_come_from_file_names() {
        assert_eq!(slug("Cool_App-1.4.2-x86_64.AppImage"), "cool-app-1-4-2-x86-64");
        assert_eq!(slug("../../evil.AppImage"), "evil");
        assert_eq!(slug(".AppImage"), "appimage");
        assert_eq!(guess_name("Cool_App-1.4.2-x86_64.AppImage"), "Cool App");
        assert_eq!(guess_name("kdenlive-25.08.0-x86_64.appimage"), "kdenlive");
        assert_eq!(guess_name("App-v2.0.AppImage"), "App");
        assert_eq!(guess_name("1.2.3.AppImage"), "AppImage");
    }

    #[test]
    fn links_inside_the_appimage_are_followed_but_never_out_of_it() {
        let r = |from: &str, t: &str| resolve_link(from, Path::new(t));
        assert_eq!(r("app.desktop", "usr/share/applications/app.desktop").as_deref(), Some("usr/share/applications/app.desktop"));
        assert_eq!(r("usr/share/a.desktop", "../b.desktop").as_deref(), Some("usr/b.desktop"));
        assert_eq!(r(".DirIcon", "app.png").as_deref(), Some("app.png"));
        assert_eq!(r("app.desktop", "../../etc/passwd"), None);
        assert_eq!(r("app.desktop", "/etc/passwd"), None);
        assert_eq!(r("usr/a", "../../x"), None);
    }

    #[test]
    fn a_launcher_cannot_be_given_extra_lines_or_keys() {
        let t = launcher_text("x", "Name\nExec=evil", "c\r\nTerminal=true", "ic\non", "Utility;Evil\nKey=1;Game;", Path::new("/home/a b/x.AppImage"));
        assert_eq!(t.lines().filter(|l| l.starts_with("Exec=")).count(), 1);
        assert_eq!(t.lines().filter(|l| l.starts_with("Terminal=")).count(), 1);
        assert!(t.contains("Exec=\"/home/a b/x.AppImage\" %U"));
        assert!(t.contains("Categories=Utility;Game;\n"));
        assert!(t.contains("X-Zohara-AppImage=x"));
    }

    #[test]
    fn install_list_and_uninstall_round_trip_without_running_the_file() {
        let root = std::env::temp_dir().join(format!("zohara-ai-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let dirs = Dirs { apps: root.join("apps"), icons: root.join("icons"), launchers: root.join("launchers") };
        let src = root.join("Cool_App-1.0-x86_64.AppImage");
        std::fs::write(&src, fake_appimage(b"not really a squashfs")).unwrap();

        let got = install(&src, &dirs).unwrap();
        assert_eq!((got.slug.as_str(), got.name.as_str()), ("cool-app-1-0-x86-64", "Cool App"));
        assert!(is_installed(&got.slug, &dirs));
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(dirs.apps.join("cool-app-1-0-x86-64.AppImage")).unwrap().permissions().mode() & 0o111, 0o111, "it must be runnable");
        }
        assert_eq!(list(&dirs).iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), vec!["Cool App"]);

        uninstall(&got.slug, &dirs).unwrap();
        assert!(!is_installed(&got.slug, &dirs));
        assert!(list(&dirs).is_empty());
        assert!(uninstall("../x", &dirs).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn something_that_is_not_an_appimage_is_refused_and_nothing_is_left_behind() {
        let root = std::env::temp_dir().join(format!("zohara-ai-test2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let dirs = Dirs { apps: root.join("apps"), icons: root.join("icons"), launchers: root.join("launchers") };
        let src = root.join("script.AppImage");
        std::fs::write(&src, "#!/bin/sh\nrm -rf ~\n").unwrap();
        assert!(install(&src, &dirs).is_err());
        assert!(!dirs.apps.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A real AppImage and `unsquashfs` on the PATH: `ZOHARA_TEST_APPIMAGE=/path/x.AppImage cargo test appimage::tests::live_ -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_a_real_appimage_gives_its_own_name_and_icon_without_being_run() {
        let src = PathBuf::from(std::env::var("ZOHARA_TEST_APPIMAGE").expect("set ZOHARA_TEST_APPIMAGE"));
        assert!(have_unsquashfs(), "put unsquashfs on the PATH");
        let root = std::env::temp_dir().join(format!("zohara-ai-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dirs = Dirs { apps: root.join("apps"), icons: root.join("icons"), launchers: root.join("launchers") };
        let got = install(&src, &dirs).expect("install");
        println!("installed: {got:?}");
        let launcher = std::fs::read_to_string(dirs.launchers.join(format!("zohara-appimage-{}.desktop", got.slug))).unwrap();
        println!("{launcher}");
        assert!(!got.icon.is_empty() && Path::new(&got.icon).is_file(), "the icon was read from inside the file");
        assert!(launcher.contains("X-Zohara-AppImage="));
        assert!(!got.comment.is_empty(), "the description can only have come from inside the file");
        uninstall(&got.slug, &dirs).unwrap();
        assert!(list(&dirs).is_empty() && !Path::new(&got.icon).exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
