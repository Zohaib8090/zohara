//! The signed update manifest: which Arch snapshot date Zohara users follow.
//!
//! Zohara users never track live Arch. `manifest.json` (in the
//! `Zohaib8090/zohara-pipeline` repo) names an `approved_date`; the official
//! package mirrors are read from the Arch Linux Archive as they were on that
//! day. Approving an update means moving the date forward.
//!
//! The manifest is signed with minisign. The public key ships in the OS
//! (`/usr/share/zohara/manifest.pub`); the private key never leaves the
//! maintainer. Anything that fails verification is discarded, so a hacked
//! repository, a captured connection or a replayed old manifest cannot move
//! users onto packages nobody approved.
//!
//! Everything here is pure (no GTK, no root), so it is unit-tested on its own.

use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;
use std::cmp::Ordering;
use std::path::Path;
use std::process::Command;

pub const MANIFEST_URL: &str = "https://raw.githubusercontent.com/Zohaib8090/zohara-pipeline/main/manifest.json";
pub const SIGNATURE_URL: &str = "https://raw.githubusercontent.com/Zohaib8090/zohara-pipeline/main/manifest.json.minisig";
pub const PUBLIC_KEY_PATH: &str = "/usr/share/zohara/manifest.pub";
pub const MIRRORLIST: &str = "/etc/pacman.d/mirrorlist";
pub const DEFAULT_MIRROR: &str = "https://archive.archlinux.org/repos";
/// Marks the pinned date inside the mirrorlist, so it can be read back.
const DATE_MARK: &str = "# zohara-approved-date:";

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Manifest {
    /// `YYYY/MM/DD`: the Arch Linux Archive day users are pinned to.
    pub approved_date: String,
    /// Packages that must stay at the version already installed.
    #[serde(default)]
    pub held_packages: Vec<String>,
    /// Oldest Store that may act on this manifest.
    pub min_updater_version: String,
    /// Where the dated snapshots live. Defaults to the Arch Linux Archive.
    #[serde(default)]
    pub mirror: Option<String>,
}

// ── Verifying ──────────────────────────────────────────────────────────────

/// Checks `signature` over `manifest` with `public_key` (both minisign text
/// formats), then parses and validates the manifest.
pub fn verify(manifest: &[u8], signature: &str, public_key: &str) -> Result<Manifest, String> {
    let pk = PublicKey::decode(public_key).map_err(|e| format!("The built-in update key is unreadable ({e})"))?;
    let sig = Signature::decode(signature).map_err(|_| "The update list's signature is unreadable".to_string())?;
    pk.verify(manifest, &sig, false).map_err(|_| "The update list didn't pass its signature check, so it was ignored".to_string())?;
    let m: Manifest = serde_json::from_slice(manifest).map_err(|e| format!("The update list is malformed ({e})"))?;
    validate(&m)?;
    Ok(m)
}

fn validate(m: &Manifest) -> Result<(), String> {
    if !valid_date(&m.approved_date) {
        return Err(format!("The update list has an invalid date ({:?})", m.approved_date));
    }
    if parse_version(&m.min_updater_version).is_none() {
        return Err("The update list has an invalid version requirement".into());
    }
    if m.held_packages.iter().any(|p| !valid_pkg_name(p)) {
        return Err("The update list names a package that doesn't look valid".into());
    }
    if let Some(mirror) = &m.mirror {
        if !valid_mirror(mirror) {
            return Err("The update list names a mirror that doesn't look valid".into());
        }
    }
    Ok(())
}

/// A real calendar date in `YYYY/MM/DD` form. Strict, because it ends up in a URL.
pub fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'/' || b[7] != b'/' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| s.get(r).filter(|t| t.bytes().all(|c| c.is_ascii_digit())).and_then(|t| t.parse::<u32>().ok());
    let (Some(y), Some(mo), Some(d)) = (num(0..4), num(5..7), num(8..10)) else { return false };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = match mo {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if leap { 29 } else { 28 },
        _ => return false,
    };
    y >= 2013 && (1..=days).contains(&d)
}

fn valid_pkg_name(s: &str) -> bool {
    !s.is_empty() && s.len() < 100 && !s.starts_with(['-', '.']) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"@._+-".contains(&b))
}

/// An https URL with nothing that could break out of a mirrorlist line.
fn valid_mirror(s: &str) -> bool {
    s.starts_with("https://") && s.len() < 200 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._~:/".contains(&b))
}

// ── Versions ───────────────────────────────────────────────────────────────

fn parse_version(v: &str) -> Option<Vec<u64>> {
    let parts: Option<Vec<u64>> = v.split('.').map(|p| p.parse().ok()).collect();
    parts.filter(|p| !p.is_empty())
}

/// Whether `current` (this Store) is at least `required`.
pub fn version_at_least(current: &str, required: &str) -> bool {
    let (Some(mut a), Some(mut b)) = (parse_version(current), parse_version(required)) else { return false };
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a.cmp(&b) != Ordering::Less
}

// ── Turning the manifest into pacman configuration ─────────────────────────

/// The mirrorlist for one pinned day. `$repo` and `$arch` stay literal: pacman fills them in.
pub fn mirrorlist(m: &Manifest) -> String {
    let base = m.mirror.as_deref().unwrap_or(DEFAULT_MIRROR).trim_end_matches('/');
    format!(
        "# Written by Zohara Store. Official packages as they were on {date}.\n# Zohara only offers updates that were tested, so this date moves forward when they are approved.\n{mark} {date}\nServer = {base}/{date}/$repo/os/$arch\n",
        date = m.approved_date,
        mark = DATE_MARK,
    )
}

/// The pinned date recorded in a mirrorlist, if it was written by Zohara.
pub fn pinned_date(mirrorlist_text: &str) -> Option<String> {
    mirrorlist_text
        .lines()
        .find_map(|l| l.strip_prefix(DATE_MARK))
        .map(|d| d.trim().to_string())
        .filter(|d| valid_date(d))
}

/// `pacman.conf` with the mirrorlist include pointed at `mirrorlist_path`, so
/// checking for updates can use the approved date without touching the system.
pub fn conf_with_mirrorlist(conf: &str, mirrorlist_path: &str) -> String {
    conf.lines()
        .map(|l| {
            let t = l.trim();
            if t.starts_with("Include") && t.ends_with(MIRRORLIST) {
                format!("Include = {mirrorlist_path}")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// `-y` (refresh the package lists) is only safe together with `-u`: a
/// refreshed list without a full upgrade is a partial upgrade, which is how
/// Arch systems break. `pinned` says the lists come from a fixed day, where a
/// refresh changes nothing in the official repositories.
pub fn sync_args_safe(args: &[&str], pinned: bool) -> bool {
    let short: Vec<&&str> = args.iter().filter(|a| a.starts_with('-') && !a.starts_with("--")).collect();
    let has = |c: char| short.iter().any(|a| a.contains(c)) || args.iter().any(|a| *a == format!("--{}", if c == 'y' { "refresh" } else { "sysupgrade" }));
    let sync = short.iter().any(|a| a.contains('S')) || args.contains(&"--sync");
    !(sync && has('y') && !has('u') && !pinned)
}

// ── Deciding what to do with a fetched manifest ────────────────────────────

#[derive(Debug, PartialEq)]
pub enum Decision {
    /// Already on the approved date.
    UpToDate,
    /// Move to the approved date.
    Move { from: Option<String>, to: String },
}

/// Refuses to move backwards: a captured old (but genuinely signed) manifest
/// must not be able to roll systems back to an older, possibly vulnerable date.
pub fn decide(current: Option<&str>, m: &Manifest, this_version: &str) -> Result<Decision, String> {
    if !version_at_least(this_version, &m.min_updater_version) {
        return Err("This version of the Store is too old for the latest updates. Update Zohara Store first.".into());
    }
    match current {
        Some(c) if c == m.approved_date => Ok(Decision::UpToDate),
        Some(c) if c > m.approved_date.as_str() => Err("The update list is older than what this computer already has, so it was ignored".into()),
        _ => Ok(Decision::Move { from: current.map(str::to_string), to: m.approved_date.clone() }),
    }
}

// ── Getting the manifest ───────────────────────────────────────────────────

/// Downloads the manifest and its signature and verifies them against the key
/// in the OS. Slow (network): call off the UI thread.
pub fn fetch_verified() -> Result<Signed, String> {
    let key = std::fs::read_to_string(PUBLIC_KEY_PATH).map_err(|_| "This system has no update key, so it can't check update approvals".to_string())?;
    let get = |url: &str| -> Result<Vec<u8>, String> {
        let o = Command::new("curl")
            .args(["--fail", "--silent", "--location", "--max-time", "20", "--proto", "=https", url])
            .output()
            .map_err(|_| "Couldn't reach the update list (curl is missing)".to_string())?;
        if o.status.success() {
            Ok(o.stdout)
        } else {
            Err("Couldn't reach the update list. Are you online?".into())
        }
    };
    let body = get(MANIFEST_URL)?;
    let sig = String::from_utf8_lossy(&get(SIGNATURE_URL)?).into_owned();
    let manifest = verify(&body, &sig, &key)?;
    Ok(Signed { body, sig, manifest })
}

/// A manifest that passed verification, with the exact bytes that were signed
/// (the root helper verifies them again itself).
#[derive(Debug, Clone)]
pub struct Signed {
    pub body: Vec<u8>,
    pub sig: String,
    pub manifest: Manifest,
}

/// The date this computer is pinned to now.
pub fn current_pinned_date() -> Option<String> {
    pinned_date(&std::fs::read_to_string(MIRRORLIST).ok()?)
}

/// Whether this system uses signed approvals at all (its OS image ships the key).
pub fn enabled() -> bool {
    Path::new(PUBLIC_KEY_PATH).exists()
}

// ── Root helper: `zohara-store --pin-date MANIFEST SIG` ────────────────────

/// Runs as root (through pkexec). Re-verifies the manifest itself instead of
/// trusting the caller, refuses to go backwards, and rewrites the mirrorlist.
pub fn root_pin(manifest_path: &str, sig_path: &str, this_version: &str) -> Result<String, String> {
    let key = std::fs::read_to_string(PUBLIC_KEY_PATH).map_err(|e| format!("no update key: {e}"))?;
    let body = std::fs::read(manifest_path).map_err(|e| format!("can't read manifest: {e}"))?;
    let sig = std::fs::read_to_string(sig_path).map_err(|e| format!("can't read signature: {e}"))?;
    let m = verify(&body, &sig, &key)?;
    let current = current_pinned_date();
    match decide(current.as_deref(), &m, this_version)? {
        Decision::UpToDate => Ok(format!("Already on {}", m.approved_date)),
        Decision::Move { .. } => {
            let tmp = format!("{MIRRORLIST}.zohara-new");
            std::fs::write(&tmp, mirrorlist(&m)).map_err(|e| format!("can't write mirrorlist: {e}"))?;
            std::fs::rename(&tmp, MIRRORLIST).map_err(|e| format!("can't replace mirrorlist: {e}"))?;
            Ok(format!("Now following {}", m.approved_date))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A throwaway key pair made only for these tests (`minisign -G -W`).
    // The private half was discarded; it protects nothing.
    const TEST_PUB: &str = include_str!("../tests/test.pub");
    const GOOD_MANIFEST: &[u8] = include_bytes!("../tests/manifest.json");
    const GOOD_SIG: &str = include_str!("../tests/manifest.json.minisig");

    #[test]
    fn genuine_manifest_is_accepted() {
        let m = verify(GOOD_MANIFEST, GOOD_SIG, TEST_PUB).expect("genuine manifest should verify");
        assert_eq!(m.approved_date, "2026/09/20");
        assert_eq!(m.held_packages, vec!["mesa".to_string()]);
    }

    #[test]
    fn tampered_manifest_is_rejected() {
        let mut bad = GOOD_MANIFEST.to_vec();
        let s = String::from_utf8(bad.clone()).unwrap().replace("2026/09/20", "2026/09/27");
        bad = s.into_bytes();
        let e = verify(&bad, GOOD_SIG, TEST_PUB).unwrap_err();
        assert!(e.contains("signature"), "{e}");
    }

    #[test]
    fn wrong_key_is_rejected() {
        // Same shape, different key id: the built-in key must match the signer.
        let other = "untrusted comment: minisign public key: 0000000000000000\nRWQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n";
        assert!(verify(GOOD_MANIFEST, GOOD_SIG, other).is_err());
    }

    #[test]
    fn garbage_signature_is_rejected() {
        assert!(verify(GOOD_MANIFEST, "not a signature", TEST_PUB).is_err());
        assert!(verify(GOOD_MANIFEST, "", TEST_PUB).is_err());
    }

    #[test]
    fn dates_are_strict() {
        assert!(valid_date("2026/09/20"));
        assert!(valid_date("2024/02/29"));
        assert!(!valid_date("2025/02/29"));
        assert!(!valid_date("2026/13/01"));
        assert!(!valid_date("2026-09-20"));
        assert!(!valid_date("2026/9/20"));
        assert!(!valid_date("2026/09/20/../x"));
        assert!(!valid_date("2026/09/2\n"));
        assert!(!valid_date("2012/12/31"));
    }

    #[test]
    fn versions_compare() {
        assert!(version_at_least("0.1.0", "0.1.0"));
        assert!(version_at_least("0.2.0", "0.1.9"));
        assert!(version_at_least("1.0", "0.9.9"));
        assert!(version_at_least("0.1.0.202609261200", "0.1.0"));
        assert!(!version_at_least("0.1.0", "0.2.0"));
        assert!(!version_at_least("garbage", "0.1.0"));
    }

    #[test]
    fn mirrorlist_pins_the_date() {
        let m = Manifest { approved_date: "2026/09/20".into(), held_packages: vec![], min_updater_version: "0.1.0".into(), mirror: None };
        let text = mirrorlist(&m);
        assert!(text.contains("Server = https://archive.archlinux.org/repos/2026/09/20/$repo/os/$arch\n"));
        assert_eq!(pinned_date(&text).as_deref(), Some("2026/09/20"));
        assert_eq!(pinned_date("Server = https://x/$repo"), None);
    }

    #[test]
    fn hostile_manifest_fields_are_refused() {
        let mk = |mirror: Option<&str>, pkg: &str| Manifest {
            approved_date: "2026/09/20".into(),
            held_packages: vec![pkg.into()],
            min_updater_version: "0.1.0".into(),
            mirror: mirror.map(str::to_string),
        };
        assert!(validate(&mk(None, "mesa")).is_ok());
        assert!(validate(&mk(Some("https://mirror.example.org/arch"), "mesa")).is_ok());
        assert!(validate(&mk(Some("http://mirror.example.org"), "mesa")).is_err());
        assert!(validate(&mk(Some("https://x.org/\nServer = https://evil.example"), "mesa")).is_err());
        assert!(validate(&mk(None, "mesa; rm -rf /")).is_err());
        assert!(validate(&mk(None, "--overwrite")).is_err());
    }

    #[test]
    fn check_uses_the_pinned_mirrorlist() {
        let conf = "[core]\nInclude = /etc/pacman.d/mirrorlist\n\n[zohara-stable]\nServer = https://example/stable\n";
        let out = conf_with_mirrorlist(conf, "/tmp/x/mirrorlist");
        assert!(out.contains("[core]\nInclude = /tmp/x/mirrorlist\n"));
        assert!(out.contains("Server = https://example/stable"));
        assert!(!out.contains("/etc/pacman.d/mirrorlist"));
    }

    #[test]
    fn refresh_without_full_upgrade_is_refused() {
        assert!(!sync_args_safe(&["-Sy"], false));
        assert!(!sync_args_safe(&["-Sy", "--noconfirm", "archlinux-keyring"], false));
        assert!(!sync_args_safe(&["--sync", "--refresh"], false));
        assert!(sync_args_safe(&["-Syu", "--noconfirm"], false));
        assert!(sync_args_safe(&["-Su"], false));
        assert!(sync_args_safe(&["-S", "--needed", "foo"], false));
        assert!(sync_args_safe(&["-Sy", "foo"], true)); // dated mirror: refresh changes nothing official
    }

    #[test]
    fn never_moves_backwards() {
        let m = |d: &str| Manifest { approved_date: d.into(), held_packages: vec![], min_updater_version: "0.1.0".into(), mirror: None };
        assert_eq!(decide(Some("2026/09/20"), &m("2026/09/20"), "0.1.0"), Ok(Decision::UpToDate));
        assert_eq!(decide(Some("2026/09/20"), &m("2026/09/22"), "0.1.0"), Ok(Decision::Move { from: Some("2026/09/20".into()), to: "2026/09/22".into() }));
        assert_eq!(decide(None, &m("2026/09/22"), "0.1.0"), Ok(Decision::Move { from: None, to: "2026/09/22".into() }));
        assert!(decide(Some("2026/09/22"), &m("2026/09/20"), "0.1.0").is_err());
    }

    #[test]
    fn old_store_is_told_to_update() {
        let m = Manifest { approved_date: "2026/09/20".into(), held_packages: vec![], min_updater_version: "0.3.0".into(), mirror: None };
        assert!(decide(None, &m, "0.1.0").unwrap_err().contains("too old"));
    }
}
