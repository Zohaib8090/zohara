//! The Arch User Repository (AUR): packages that community members publish as build scripts, not binaries.
//!
//! The Store finds them through the AUR's own search (`aur.archlinux.org/rpc`) and shows them in a separate group, because
//! installing one means building it on this computer from the author's script, which Zohara has not reviewed. Packages the
//! Arch and Chaotic-AUR repositories already carry are left out here: they are prebuilt and install through pacman.

use crate::app_info::{AppCategory, AppInfo, AppSource};
use std::collections::HashSet;
use std::process::Command;

/// One search hit from the AUR.
#[derive(Debug, Clone, PartialEq)]
pub struct AurPackage {
    pub name: String,
    pub description: String,
    pub version: String,
    pub votes: u32,
    pub popularity: f64,
    pub maintainer: Option<String>,
    pub out_of_date: bool,
}

/// AUR package names are letters, digits and `@ . _ + -`. Anything else never reaches a URL or a command.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && !name.starts_with(['-', '.'])
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '+' | '-'))
}

/// Percent-encodes a search term for a URL path.
pub fn encode_term(term: &str) -> String {
    let mut out = String::new();
    for b in term.trim().bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Reads the AUR's `search` answer (RPC v5).
pub fn parse_search(json: &str) -> Result<Vec<AurPackage>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| "The AUR sent an answer the Store could not read.".to_string())?;
    if v.get("type").and_then(|t| t.as_str()) == Some("error") {
        let why = v.get("error").and_then(|e| e.as_str()).unwrap_or("unknown error");
        return Err(format!("The AUR said: {why}"));
    }
    let list = v.get("results").and_then(|r| r.as_array()).ok_or_else(|| "The AUR sent an answer the Store could not read.".to_string())?;
    Ok(list
        .iter()
        .filter_map(|p| {
            let name = p.get("Name")?.as_str()?.to_string();
            if !valid_name(&name) {
                return None;
            }
            Some(AurPackage {
                name,
                description: p.get("Description").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                version: p.get("Version").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                votes: p.get("NumVotes").and_then(|d| d.as_u64()).unwrap_or(0) as u32,
                popularity: p.get("Popularity").and_then(|d| d.as_f64()).unwrap_or(0.0),
                maintainer: p.get("Maintainer").and_then(|d| d.as_str()).map(str::to_string),
                out_of_date: !p.get("OutOfDate").map(|d| d.is_null()).unwrap_or(true),
            })
        })
        .collect())
}

/// What to show: packages the repositories already carry are dropped, so are libraries and packages nobody looks after or
/// that are flagged out of date, and the most used come first. At most `limit`.
pub fn pick(found: Vec<AurPackage>, known: &HashSet<String>, limit: usize) -> Vec<AurPackage> {
    let mut v: Vec<AurPackage> = found
        .into_iter()
        .filter(|p| !known.contains(&p.name) && p.maintainer.is_some() && !p.out_of_date && !crate::backend::looks_like_library(&p.name))
        .collect();
    v.sort_by(|a, b| b.popularity.partial_cmp(&a.popularity).unwrap_or(std::cmp::Ordering::Equal).then(b.votes.cmp(&a.votes)).then(a.name.cmp(&b.name)));
    v.truncate(limit);
    v
}

pub fn to_app(p: &AurPackage) -> AppInfo {
    let pretty = {
        let base = p.name.strip_suffix("-bin").unwrap_or(&p.name).replace(['-', '_'], " ");
        let mut c = base.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    };
    AppInfo {
        id: p.name.clone(),
        name: pretty,
        publisher: "AUR (community)".into(),
        description: p.description.clone(),
        icon_name: p.name.clone(),
        source: AppSource::Aur,
        package_name: p.name.clone(),
        category: AppCategory::Utilities,
        rating: 0.0,
    }
}

fn curl(url: &str) -> Result<String, String> {
    let o = Command::new("curl")
        .args(["-sfL", "--max-time", "10", "-A", "ZoharaStore", url])
        .output()
        .map_err(|_| "The Store could not reach the AUR (curl is missing).".to_string())?;
    if !o.status.success() {
        return Err("The Store could not reach the AUR. Check your internet connection.".into());
    }
    Ok(String::from_utf8_lossy(&o.stdout).to_string())
}

/// Searches names and descriptions on the AUR.
pub fn search(term: &str) -> Result<Vec<AurPackage>, String> {
    parse_search(&curl(&format!("https://aur.archlinux.org/rpc/v5/search/{}?by=name-desc", encode_term(term)))?)
}

/// The build script of a package, for the "View script" button.
pub fn fetch_pkgbuild(name: &str) -> Result<String, String> {
    if !valid_name(name) {
        return Err("That is not a valid package name.".into());
    }
    curl(&format!("https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={name}"))
}

/// Whether `yay`, which builds AUR packages, is installed.
pub fn have_builder() -> bool {
    std::env::var("PATH").unwrap_or_default().split(':').any(|d| std::path::Path::new(d).join("yay").is_file())
}

/// The command that builds and installs one AUR package. It runs as the person (builds must not run as root); `yay`
/// asks for the administrator password itself, through `pkexec`, only for the steps that need it. No questions are asked
/// in the terminal: the Store already showed the warning and the script.
pub fn install_command(name: &str) -> Option<Command> {
    if !valid_name(name) {
        return None;
    }
    let mut c = Command::new("yay");
    c.args([
        "-S", "--noconfirm", "--needed", "--removemake", "--cleanafter", "--sudo", "pkexec", "--answerclean", "None", "--answerdiff", "None", "--answeredit",
        "None", "--answerupgrade", "None", "--",
    ])
    .arg(name);
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"version":5,"type":"search","resultcount":4,"results":[
      {"Name":"popular-bin","Description":"A popular app","Version":"1.0-1","NumVotes":300,"Popularity":12.5,"Maintainer":"alice","OutOfDate":null},
      {"Name":"niche-tool","Description":"Niche","Version":"0.1-1","NumVotes":2,"Popularity":0.1,"Maintainer":"bob","OutOfDate":null},
      {"Name":"orphan-app","Description":"Nobody looks after it","Version":"1-1","NumVotes":50,"Popularity":5.0,"Maintainer":null,"OutOfDate":null},
      {"Name":"stale-app","Description":"Old","Version":"1-1","NumVotes":90,"Popularity":6.0,"Maintainer":"carol","OutOfDate":1700000000}]}"#;

    #[test]
    fn search_answers_are_read() {
        let r = parse_search(SAMPLE).unwrap();
        assert_eq!(r.len(), 4);
        assert_eq!(r[0].name, "popular-bin");
        assert_eq!(r[0].votes, 300);
        assert!(r[2].maintainer.is_none());
        assert!(r[3].out_of_date);
        assert!(parse_search("not json").is_err());
        assert!(parse_search(r#"{"type":"error","error":"Too many package results."}"#).unwrap_err().contains("Too many"));
    }

    #[test]
    fn what_is_shown_leaves_out_known_orphaned_stale_and_library_packages_most_used_first() {
        let mut known = HashSet::new();
        known.insert("niche-tool".to_string());
        let mut found = parse_search(SAMPLE).unwrap();
        found.push(AurPackage { name: "python-something".into(), description: String::new(), version: String::new(), votes: 999, popularity: 99.0, maintainer: Some("x".into()), out_of_date: false });
        let shown = pick(found, &known, 10);
        assert_eq!(shown.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["popular-bin"]);
    }

    #[test]
    fn more_popular_comes_first_and_the_list_is_capped() {
        let mk = |n: &str, pop: f64| AurPackage { name: n.into(), description: String::new(), version: String::new(), votes: 1, popularity: pop, maintainer: Some("m".into()), out_of_date: false };
        let shown = pick(vec![mk("a", 1.0), mk("b", 9.0), mk("c", 5.0)], &HashSet::new(), 2);
        assert_eq!(shown.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["b", "c"]);
    }

    #[test]
    fn package_names_cannot_smuggle_anything_into_a_command_or_address() {
        assert!(valid_name("google-chrome"));
        assert!(valid_name("libc++-git"));
        assert!(valid_name("a@b.c_d+e"));
        for bad in ["", "-S", ".hidden", "a b", "a;rm", "a/b", "a$(x)", "a\nb", "a?b=c", "名前"] {
            assert!(!valid_name(bad), "{bad:?}");
        }
        assert!(install_command("-S").is_none());
        assert!(install_command("a b").is_none());
    }

    #[test]
    fn the_install_command_builds_as_the_person_and_asks_for_the_password_only_through_pkexec() {
        let c = install_command("some-app").unwrap();
        let args: Vec<String> = c.get_args().map(|a| a.to_string_lossy().to_string()).collect();
        assert_eq!(c.get_program(), "yay");
        assert!(args.windows(2).any(|w| w == ["--sudo", "pkexec"]));
        assert!(args.contains(&"--noconfirm".to_string()));
        // The name comes last, after `--`, so it can never be read as an option.
        assert_eq!(&args[args.len() - 2..], ["--", "some-app"]);
    }

    #[test]
    fn search_terms_are_encoded_for_the_address() {
        assert_eq!(encode_term("visual studio"), "visual%20studio");
        assert_eq!(encode_term("a/b?c"), "a%2Fb%3Fc");
        assert_eq!(encode_term(" firefox "), "firefox");
    }

    #[test]
    fn an_aur_hit_becomes_an_app_marked_as_community_made() {
        let a = to_app(&parse_search(SAMPLE).unwrap()[0]);
        assert_eq!(a.source, AppSource::Aur);
        assert_eq!(a.publisher, "AUR (community)");
        assert_eq!(a.name, "Popular");
        assert_eq!(a.package_name, "popular-bin");
    }

    /// Talks to aur.archlinux.org: `cargo test aur::tests::live_ -- --ignored`.
    #[test]
    #[ignore]
    fn live_search_and_script_download_work() {
        let found = search("yay").expect("search");
        assert!(found.iter().any(|p| p.name == "yay"), "yay should be on the AUR");
        let shown = pick(found, &HashSet::new(), 6);
        assert!(!shown.is_empty() && shown.len() <= 6);
        let script = fetch_pkgbuild("yay").expect("script");
        assert!(script.contains("pkgname"), "a PKGBUILD names its package");
        assert!(fetch_pkgbuild("a;b").is_err());
    }
}
