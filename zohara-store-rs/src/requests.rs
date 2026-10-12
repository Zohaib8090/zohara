//! What other programs ask the Store to do: open it on an app (`appstream://org.example.App`, the "Install" buttons on
//! Flathub and app-store web pages), install a downloaded `.flatpakref` or AppImage (the files GitHub release pages and
//! Flathub give out), or show a page.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// `--page updates`
    Page(String),
    /// An app id from an `appstream:` link.
    AppStream(String),
    /// A file to install.
    File(PathBuf),
}

/// `appstream://org.gnome.Gedit`, `appstream:org.gnome.Gedit.desktop`, `appstream://org.gnome.Gedit?x=1` -> `org.gnome.Gedit`.
pub fn appstream_id(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("appstream:")?;
    let rest = rest.trim_start_matches('/');
    let rest = rest.split(['?', '#']).next().unwrap_or(rest);
    let rest = rest.strip_suffix(".desktop").unwrap_or(rest);
    let ok = !rest.is_empty() && rest.len() <= 200 && rest.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'));
    ok.then(|| rest.to_string())
}

/// `file:///home/a/My%20App.AppImage` -> `/home/a/My App.AppImage`.
pub fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(PathBuf::from(String::from_utf8(out).ok()?))
}

/// The command-line arguments (after the program name) as requests. Unknown options and addresses are ignored.
pub fn parse_args(args: &[String]) -> Vec<Request> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--page" {
            if let Some(p) = args.get(i + 1) {
                out.push(Request::Page(p.clone()));
                i += 1;
            }
        } else if a.starts_with("appstream:") {
            if let Some(id) = appstream_id(a) {
                out.push(Request::AppStream(id));
            }
        } else if a.starts_with("file://") {
            if let Some(p) = file_uri_to_path(a) {
                out.push(Request::File(p));
            }
        } else if !a.starts_with('-') && !a.contains("://") {
            out.push(Request::File(PathBuf::from(a)));
        }
        i += 1;
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    FlatpakRef,
    AppImage,
    Deb,
    Unknown,
}

pub fn kind_of(path: &Path) -> Kind {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    if name.ends_with(".flatpakref") {
        Kind::FlatpakRef
    } else if name.ends_with(".appimage") {
        Kind::AppImage
    } else if name.ends_with(".deb") {
        Kind::Deb
    } else {
        Kind::Unknown
    }
}

/// What a `.flatpakref` file says about the app.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatpakRef {
    pub name: String,
    pub title: String,
    pub url: String,
}

pub fn parse_flatpakref(text: &str) -> Option<FlatpakRef> {
    let (mut name, mut title, mut url) = (String::new(), String::new(), String::new());
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line.eq_ignore_ascii_case("[Flatpak Ref]");
            continue;
        }
        if !inside {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "Name" => name = v.trim().to_string(),
                "Title" => title = v.trim().to_string(),
                "Url" => url = v.trim().to_string(),
                _ => {}
            }
        }
    }
    let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    valid.then_some(FlatpakRef { name, title, url })
}

/// Whether a repository address is Flathub's own (the only one the Store installs from on its own).
pub fn is_flathub(url: &str) -> bool {
    let host = url.strip_prefix("https://").and_then(|r| r.split('/').next());
    matches!(host, Some("dl.flathub.org") | Some("flathub.org"))
}

/// A search word from a Debian package file name: `cool-app_1.2.3_amd64.deb` -> `cool-app`.
pub fn deb_search_term(file_name: &str) -> String {
    file_name.trim_end_matches(".deb").split('_').next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn appstream_links_give_the_app_id() {
        assert_eq!(appstream_id("appstream://org.gnome.Gedit").as_deref(), Some("org.gnome.Gedit"));
        assert_eq!(appstream_id("appstream:org.gnome.Gedit.desktop").as_deref(), Some("org.gnome.Gedit"));
        assert_eq!(appstream_id("appstream://firefox?x=1").as_deref(), Some("firefox"));
        for bad in ["appstream:", "appstream://", "appstream://a b", "appstream://a;rm", "https://x", "appstream://../x/y"] {
            assert_eq!(appstream_id(bad), None, "{bad}");
        }
    }

    #[test]
    fn file_addresses_are_turned_into_paths() {
        assert_eq!(file_uri_to_path("file:///home/a/My%20App.AppImage"), Some(PathBuf::from("/home/a/My App.AppImage")));
        assert_eq!(file_uri_to_path("file://localhost/tmp/x"), Some(PathBuf::from("/tmp/x")));
        assert_eq!(file_uri_to_path("file://remote-host/tmp/x"), None);
        assert_eq!(file_uri_to_path("https://x"), None);
        assert_eq!(file_uri_to_path("file:///a%zz"), None);
    }

    #[test]
    fn arguments_become_requests_and_strangers_are_ignored() {
        let got = parse_args(&s(&["--page", "updates", "appstream://org.x.App", "file:///tmp/a.flatpakref", "/tmp/b.AppImage", "--weird", "https://evil.example/x.sh", "ftp://x"]));
        assert_eq!(
            got,
            vec![
                Request::Page("updates".into()),
                Request::AppStream("org.x.App".into()),
                Request::File("/tmp/a.flatpakref".into()),
                Request::File("/tmp/b.AppImage".into()),
            ]
        );
        assert!(parse_args(&[]).is_empty());
    }

    #[test]
    fn files_are_told_apart_by_their_ending() {
        assert_eq!(kind_of(Path::new("/x/A.FlatpakRef")), Kind::FlatpakRef);
        assert_eq!(kind_of(Path::new("/x/a.appimage")), Kind::AppImage);
        assert_eq!(kind_of(Path::new("/x/a_1_amd64.deb")), Kind::Deb);
        assert_eq!(kind_of(Path::new("/x/a.sh")), Kind::Unknown);
    }

    #[test]
    fn a_flatpakref_gives_the_app_and_where_it_comes_from() {
        let t = "[Flatpak Ref]\nTitle=Cool App\nName=org.cool.App\nBranch=stable\nUrl=https://dl.flathub.org/repo/\nIsRuntime=false\nRuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo\n";
        let r = parse_flatpakref(t).unwrap();
        assert_eq!((r.name.as_str(), r.title.as_str()), ("org.cool.App", "Cool App"));
        assert!(is_flathub(&r.url));
        assert!(!is_flathub("https://evil.example/repo/"));
        assert!(!is_flathub("https://dl.flathub.org.evil.example/repo/"));
        assert!(parse_flatpakref("[Other]\nName=x\n").is_none());
        assert!(parse_flatpakref("[Flatpak Ref]\nName=a b; rm\n").is_none());
    }

    #[test]
    fn a_debian_file_name_gives_a_search_word() {
        assert_eq!(deb_search_term("cool-app_1.2.3_amd64.deb"), "cool-app");
        assert_eq!(deb_search_term("plain.deb"), "plain");
    }
}
