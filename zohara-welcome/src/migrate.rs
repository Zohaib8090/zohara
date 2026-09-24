//! Move-verify-delete migration from an old Linux install.
//!
//! Files are moved one at a time: copied beside the destination, SHA-256
//! checked against the source, renamed into place, and only then deleted from
//! the old disk. Only one file's worth of extra space is ever needed, so it
//! works on nearly full disks.
//!
//! Safety rules the engine keeps:
//! - nothing already on this system is overwritten (a clash is saved as
//!   "name (old system).ext"; an identical file is just not copied twice);
//! - symlinks are moved as symlinks, never followed;
//! - a file that fails to verify stays on the old disk, and the old system
//!   files are only deleted when every file succeeded and the user asked for it;
//! - the old partition must not be mounted anywhere else, so this system's
//!   own disk can't be picked by mistake.

use crate::packages;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::{chown, symlink, FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const MOUNT_POINT: &str = "/mnt/zohara_old";
pub const LOG_FILE: &str = "/var/log/zohara-migrate.log";
const NEW_HOME: &str = "/home";
/// Top-level directories of the old system, removed after a fully successful move.
const OLD_SYSTEM_DIRS: &[&str] = &["bin", "boot", "etc", "lib", "lib32", "lib64", "opt", "root", "sbin", "srv", "usr", "var"];

/// What the privileged helper tells the window, one JSON object per line.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "t")]
pub enum Event {
    #[serde(rename = "p")]
    Progress { pct: u8, msg: String },
    #[serde(rename = "f")]
    File { name: String, status: String },
    #[serde(rename = "d")]
    Done(Report),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Report {
    pub installed: Vec<String>,
    pub unmapped: Vec<String>,
    pub failed: Vec<String>,
    /// Things worth knowing that aren't failures (kept files, skipped steps).
    pub notes: Vec<String>,
    pub files_moved: u64,
    pub system_deleted: bool,
    pub cancelled: bool,
    pub fatal: Option<String>,
}

// ── Files ──────────────────────────────────────────────────────────────────

pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Moved,
    /// The same file was already on this system; the old copy was removed.
    AlreadyThere,
    /// Saved under a different name because the original name was taken.
    MovedRenamed(PathBuf),
}

fn exists(p: &Path) -> bool {
    fs::symlink_metadata(p).is_ok()
}

/// "report.pdf" -> "report (old system).pdf", then "report (old system 2).pdf", ...
fn alternative(dst: &Path) -> PathBuf {
    let stem = dst.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = dst.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let dir = dst.parent().unwrap_or(Path::new("."));
    let mut n = 1;
    loop {
        let tag = if n == 1 { "old system".to_string() } else { format!("old system {n}") };
        let p = dir.join(format!("{stem} ({tag}){ext}"));
        if !exists(&p) {
            return p;
        }
        n += 1;
    }
}

fn set_owner(path: &Path, uid: u32, gid: u32) {
    // Running as root, the copy would otherwise belong to root.
    let _ = std::os::unix::fs::lchown(path, Some(uid), Some(gid));
}

/// Moves one file or symlink. On any error the source is left untouched.
pub fn move_file(src: &Path, dst: &Path) -> Result<Outcome, String> {
    let meta = fs::symlink_metadata(src).map_err(|e| e.to_string())?;
    let (uid, gid) = (meta.uid(), meta.gid());
    let ft = meta.file_type();

    if ft.is_symlink() {
        let target = fs::read_link(src).map_err(|e| e.to_string())?;
        let mut dst = dst.to_path_buf();
        let mut renamed = false;
        if exists(&dst) {
            if fs::read_link(&dst).map(|t| t == target).unwrap_or(false) {
                fs::remove_file(src).map_err(|e| e.to_string())?;
                return Ok(Outcome::AlreadyThere);
            }
            dst = alternative(&dst);
            renamed = true;
        }
        symlink(&target, &dst).map_err(|e| e.to_string())?;
        set_owner(&dst, uid, gid);
        fs::remove_file(src).map_err(|e| e.to_string())?;
        return Ok(if renamed { Outcome::MovedRenamed(dst) } else { Outcome::Moved });
    }
    if !ft.is_file() {
        let kind = if ft.is_socket() { "socket" } else if ft.is_fifo() { "pipe" } else { "device file" };
        return Err(format!("{kind} skipped"));
    }

    let src_hash = sha256_file(src).map_err(|e| format!("reading: {e}"))?;
    let mut dst = dst.to_path_buf();
    let mut renamed = false;
    if exists(&dst) {
        if dst.is_file() && sha256_file(&dst).map(|h| h == src_hash).unwrap_or(false) {
            fs::remove_file(src).map_err(|e| e.to_string())?;
            return Ok(Outcome::AlreadyThere);
        }
        dst = alternative(&dst);
        renamed = true;
    }

    let part = {
        let mut n = dst.as_os_str().to_owned();
        n.push(".zohara-part");
        PathBuf::from(n)
    };
    let copied = (|| -> Result<(), String> {
        fs::copy(src, &part).map_err(|e| format!("copying: {e}"))?;
        let got = sha256_file(&part).map_err(|e| format!("verifying: {e}"))?;
        if got != src_hash {
            return Err("checksum mismatch".into());
        }
        if let Ok(m) = meta.modified() {
            if let Ok(f) = fs::OpenOptions::new().write(true).open(&part) {
                let _ = f.set_modified(m);
            }
        }
        let _ = chown(&part, Some(uid), Some(gid));
        fs::rename(&part, &dst).map_err(|e| format!("renaming: {e}"))
    })();
    if let Err(e) = copied {
        let _ = fs::remove_file(&part);
        return Err(e);
    }
    // Only now, with a verified copy in place, is the original removed.
    fs::remove_file(src).map_err(|e| format!("copied, but couldn't remove the original: {e}"))?;
    Ok(if renamed { Outcome::MovedRenamed(dst) } else { Outcome::Moved })
}

// ── Whole-tree walk ────────────────────────────────────────────────────────

struct Tally {
    files: u64,
    bytes: u64,
}

fn count(dir: &Path, t: &mut Tally) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(m) = fs::symlink_metadata(e.path()) else { continue };
        if m.is_dir() {
            count(&e.path(), t);
        } else {
            t.files += 1;
            t.bytes += m.len();
        }
    }
}

pub fn human(n: u64) -> String {
    let mut v = n as f64;
    for unit in ["B", "KB", "MB", "GB"] {
        if v < 1024.0 {
            return format!("{v:.1} {unit}");
        }
        v /= 1024.0;
    }
    format!("{v:.1} TB")
}

struct Ctx<'a> {
    emit: &'a dyn Fn(Event),
    cancel: &'a AtomicBool,
    total_files: u64,
    total_bytes: u64,
    done_files: u64,
    done_bytes: u64,
    report: &'a mut Report,
    log: Vec<String>,
}

impl Ctx<'_> {
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

fn short(rel: &str) -> String {
    let n = rel.chars().count();
    if n < 60 {
        rel.to_string()
    } else {
        format!("…{}", rel.chars().skip(n - 57).collect::<String>())
    }
}

fn walk(src_dir: &Path, dst_dir: &Path, rel: &Path, ctx: &mut Ctx) {
    if let Ok(m) = fs::symlink_metadata(src_dir) {
        if !exists(dst_dir) {
            if fs::create_dir_all(dst_dir).is_ok() {
                let _ = fs::set_permissions(dst_dir, fs::Permissions::from_mode(m.mode() & 0o7777));
                set_owner(dst_dir, m.uid(), m.gid());
            }
        }
    }
    let Ok(rd) = fs::read_dir(src_dir) else {
        ctx.report.failed.push(format!("{}  [couldn't read folder]", rel.display()));
        return;
    };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        if ctx.cancelled() {
            return;
        }
        let path = e.path();
        let rel = rel.join(e.file_name());
        let dst = dst_dir.join(e.file_name());
        let Ok(m) = fs::symlink_metadata(&path) else { continue };
        if m.is_dir() {
            walk(&path, &dst, &rel, ctx);
            continue;
        }
        let name = short(&rel.to_string_lossy());
        match move_file(&path, &dst) {
            Ok(Outcome::Moved) => {
                ctx.report.files_moved += 1;
                (ctx.emit)(Event::File { name, status: "✓ moved".into() });
            }
            Ok(Outcome::AlreadyThere) => {
                ctx.report.files_moved += 1;
                (ctx.emit)(Event::File { name, status: "✓ already there".into() });
            }
            Ok(Outcome::MovedRenamed(to)) => {
                ctx.report.files_moved += 1;
                ctx.report.notes.push(format!(
                    "{} already existed here, so the old one was saved as {}",
                    rel.display(),
                    to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
                ));
                (ctx.emit)(Event::File { name, status: "✓ moved (renamed)".into() });
            }
            Err(err) => {
                ctx.report.failed.push(format!("{}  [{err}]", rel.display()));
                ctx.log.push(format!("FAILED {}: {err}", path.display()));
                (ctx.emit)(Event::File { name, status: format!("✗ {err}") });
            }
        }
        ctx.done_files += 1;
        ctx.done_bytes += m.len();
        let pct = 15 + (ctx.done_bytes as f64 / ctx.total_bytes.max(1) as f64 * 55.0) as u8;
        (ctx.emit)(Event::Progress {
            pct: pct.min(70),
            msg: format!(
                "File {}/{} — {}/{}",
                ctx.done_files,
                ctx.total_files,
                human(ctx.done_bytes),
                human(ctx.total_bytes)
            ),
        });
    }
}

/// Removes emptied folders left behind on the old disk. `remove_dir` only
/// succeeds on empty folders, so anything that failed to move stays put.
fn prune_empty(dir: &Path) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        if fs::symlink_metadata(e.path()).map(|m| m.is_dir()).unwrap_or(false) {
            prune_empty(&e.path());
            let _ = fs::remove_dir(e.path());
        }
    }
}

// ── The whole job ──────────────────────────────────────────────────────────

fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    let o = Command::new(cmd).args(args).output().map_err(|e| format!("{cmd}: {e}"))?;
    if o.status.success() {
        return Ok(String::from_utf8_lossy(&o.stdout).into_owned());
    }
    let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
    Err(if err.is_empty() { format!("{cmd} failed") } else { err })
}

/// The device is a real, unmounted partition of a supported file system.
pub fn check_device(dev: &str) -> Result<(), String> {
    if !dev.starts_with("/dev/") || dev.contains("..") {
        return Err("That is not a disk partition.".into());
    }
    let meta = fs::metadata(dev).map_err(|_| "That partition no longer exists.".to_string())?;
    if !meta.file_type().is_block_device() {
        return Err("That is not a disk partition.".into());
    }
    let mounts = fs::read_to_string("/proc/self/mountinfo").unwrap_or_default();
    let real = fs::canonicalize(dev).unwrap_or_else(|_| PathBuf::from(dev));
    if mounts.lines().any(|l| l.split(" - ").nth(1).and_then(|r| r.split_whitespace().nth(1)).map(|s| Path::new(s) == real || s == dev).unwrap_or(false)) {
        return Err("That partition is in use (mounted). Choose the one that holds the old system.".into());
    }
    let fs_type = run("lsblk", &["-no", "FSTYPE", dev]).unwrap_or_default();
    if !matches!(fs_type.trim(), "ext4" | "btrfs" | "xfs") {
        return Err(format!("Unsupported file system “{}”. Supported: ext4, btrfs, xfs.", fs_type.trim()));
    }
    Ok(())
}

fn looks_like_old_system(mp: &Path) -> bool {
    mp.join("etc").is_dir() && (mp.join("usr").is_dir() || mp.join("bin").exists())
}

fn same_disk_as_root(mp: &Path) -> bool {
    match (fs::metadata(mp), fs::metadata("/")) {
        (Ok(a), Ok(b)) => a.dev() == b.dev(),
        _ => true,
    }
}

fn append_log(lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(LOG_FILE) {
        use std::io::Write;
        let _ = writeln!(f, "--- {} ---", run("date", &["-Is"]).unwrap_or_default().trim());
        for l in lines {
            let _ = writeln!(f, "{l}");
        }
    }
}

/// Runs the whole migration. Never panics on bad disks; problems land in the report.
pub fn run_job(dev: &str, delete_system: bool, emit: &dyn Fn(Event), cancel: &Arc<AtomicBool>) -> Report {
    let mut report = Report::default();
    let progress = |pct: u8, msg: &str| emit(Event::Progress { pct, msg: msg.to_string() });

    if let Err(e) = check_device(dev) {
        report.fatal = Some(e);
        return report;
    }
    progress(2, "Opening the old system's disk…");
    let mp = Path::new(MOUNT_POINT);
    if let Err(e) = fs::create_dir_all(mp).map_err(|e| e.to_string()).and_then(|_| run("mount", &[dev, MOUNT_POINT]).map(|_| ())) {
        report.fatal = Some(format!("Couldn't open {dev}: {e}"));
        return report;
    }

    let mut log: Vec<String> = vec![format!("migrating {dev}")];
    let result = job_on_mounted(mp, delete_system, emit, cancel, &mut report, &mut log);
    if let Err(e) = result {
        report.fatal = Some(e);
    }

    progress(96, "Closing the old disk…");
    if let Err(e) = run("umount", &[MOUNT_POINT]) {
        report.notes.push(format!("Couldn't unmount {MOUNT_POINT}: {e}"));
    }
    report.cancelled = cancel.load(Ordering::Relaxed);
    log.push(format!(
        "result: moved={} failed={} installed={} unmapped={} system_deleted={} cancelled={} fatal={:?}",
        report.files_moved, report.failed.len(), report.installed.len(), report.unmapped.len(),
        report.system_deleted, report.cancelled, report.fatal
    ));
    append_log(&log);
    report
}

fn job_on_mounted(
    mp: &Path,
    delete_system: bool,
    emit: &dyn Fn(Event),
    cancel: &Arc<AtomicBool>,
    report: &mut Report,
    log: &mut Vec<String>,
) -> Result<(), String> {
    let progress = |pct: u8, msg: &str| emit(Event::Progress { pct, msg: msg.to_string() });
    if same_disk_as_root(mp) {
        return Err("That is the disk this system is running from. Nothing was changed.".into());
    }
    if !looks_like_old_system(mp) {
        return Err("This partition doesn't look like a Linux system (no /etc). Nothing was changed.".into());
    }

    progress(5, "Reading the list of installed apps…");
    let status = fs::read_to_string(mp.join("var/lib/dpkg/status")).unwrap_or_default();
    let (arch, unmapped) = packages::translate(&packages::parse_dpkg_status(&status));
    report.unmapped = unmapped;

    progress(15, "Counting files…");
    let old_home = mp.join("home");
    if old_home.is_dir() {
        let mut t = Tally { files: 0, bytes: 0 };
        count(&old_home, &mut t);
        progress(15, &format!("Moving {} files ({})…", t.files, human(t.bytes)));
        let mut ctx = Ctx {
            emit,
            cancel: cancel.as_ref(),
            total_files: t.files,
            total_bytes: t.bytes,
            done_files: 0,
            done_bytes: 0,
            report: &mut *report,
            log: Vec::new(),
        };
        walk(&old_home, Path::new(NEW_HOME), Path::new(""), &mut ctx);
        log.append(&mut ctx.log);
        prune_empty(&old_home);
    }
    if cancel.load(Ordering::Relaxed) {
        report.notes.push("Stopped before finishing. Files already moved are safe in /home; the rest are still on the old disk.".into());
        return Ok(());
    }

    if !arch.is_empty() {
        progress(75, &format!("Installing {} matching apps…", arch.len()));
        let mut args = vec!["-S", "--noconfirm", "--needed"];
        args.extend(arch.iter().map(String::as_str));
        match run("pacman", &args) {
            Ok(_) => report.installed = arch,
            Err(e) => {
                // Retry one by one so a single missing package doesn't lose the rest.
                log.push(format!("pacman batch failed: {e}"));
                for pkg in &arch {
                    match run("pacman", &["-S", "--noconfirm", "--needed", pkg]) {
                        Ok(_) => report.installed.push(pkg.clone()),
                        Err(e) => report.unmapped.push(format!("{pkg}  →  couldn't install: {}", e.lines().last().unwrap_or(""))),
                    }
                }
            }
        }
    }

    if delete_system {
        if !report.failed.is_empty() {
            report.notes.push("The old system files were kept because some files couldn't be moved.".into());
        } else {
            progress(90, "Removing the old system files…");
            for d in OLD_SYSTEM_DIRS {
                let target = mp.join(d);
                if fs::symlink_metadata(&target).map(|m| m.is_dir()).unwrap_or(false) {
                    if let Err(e) = fs::remove_dir_all(&target) {
                        report.notes.push(format!("Couldn't fully remove /{d} from the old system: {e}"));
                    }
                } else if fs::symlink_metadata(&target).is_ok() {
                    // usrmerge symlinks like /bin -> usr/bin
                    let _ = fs::remove_file(&target);
                }
            }
            report.system_deleted = true;
        }
    }
    progress(95, "Almost done…");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn moves_and_verifies() {
        let d = tempdir().unwrap();
        let (src, dst) = (d.path().join("a.txt"), d.path().join("b.txt"));
        fs::write(&src, "hello").unwrap();
        assert_eq!(move_file(&src, &dst), Ok(Outcome::Moved));
        assert!(!src.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello");
        assert!(!d.path().join("b.txt.zohara-part").exists());
    }

    #[test]
    fn never_overwrites() {
        let d = tempdir().unwrap();
        let (src, dst) = (d.path().join("a.pdf"), d.path().join("b.pdf"));
        fs::write(&src, "old").unwrap();
        fs::write(&dst, "new").unwrap();
        let Ok(Outcome::MovedRenamed(to)) = move_file(&src, &dst) else { panic!("expected rename") };
        assert_eq!(fs::read_to_string(&dst).unwrap(), "new");
        assert_eq!(fs::read_to_string(&to).unwrap(), "old");
        assert_eq!(to.file_name().unwrap(), "b (old system).pdf");
        assert!(!src.exists());
    }

    #[test]
    fn identical_file_is_not_copied_twice() {
        let d = tempdir().unwrap();
        let (src, dst) = (d.path().join("a"), d.path().join("b"));
        fs::write(&src, "same").unwrap();
        fs::write(&dst, "same").unwrap();
        assert_eq!(move_file(&src, &dst), Ok(Outcome::AlreadyThere));
        assert!(!src.exists() && dst.exists());
    }

    #[test]
    fn symlinks_stay_symlinks() {
        let d = tempdir().unwrap();
        let (target, src, dst) = (d.path().join("t"), d.path().join("link"), d.path().join("moved"));
        fs::write(&target, "x").unwrap();
        symlink("t", &src).unwrap();
        assert_eq!(move_file(&src, &dst), Ok(Outcome::Moved));
        assert!(fs::symlink_metadata(&dst).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_link(&dst).unwrap(), Path::new("t"));
        assert!(target.exists(), "the link's target must be untouched");
    }

    #[test]
    fn failed_move_keeps_source() {
        let d = tempdir().unwrap();
        let src = d.path().join("a");
        fs::write(&src, "data").unwrap();
        // Destination folder doesn't exist, so the copy fails.
        let r = move_file(&src, &d.path().join("nope/b"));
        assert!(r.is_err());
        assert_eq!(fs::read_to_string(&src).unwrap(), "data");
    }

    #[test]
    fn walk_moves_a_tree() {
        let d = tempdir().unwrap();
        let (old, new) = (d.path().join("old"), d.path().join("new"));
        fs::create_dir_all(old.join("bob/Docs")).unwrap();
        fs::write(old.join("bob/Docs/a.txt"), "1").unwrap();
        fs::write(old.join("bob/b.txt"), "2").unwrap();
        fs::create_dir_all(&new).unwrap();
        let cancel = AtomicBool::new(false);
        let mut report = Report::default();
        let events = std::cell::RefCell::new(0);
        let emit = |_e: Event| *events.borrow_mut() += 1;
        let mut ctx = Ctx { emit: &emit, cancel: &cancel, total_files: 2, total_bytes: 2, done_files: 0, done_bytes: 0, report: &mut report, log: vec![] };
        walk(&old, &new, Path::new(""), &mut ctx);
        assert!(report.failed.is_empty());
        assert_eq!(report.files_moved, 2);
        assert_eq!(fs::read_to_string(new.join("bob/Docs/a.txt")).unwrap(), "1");
        prune_empty(&old);
        assert!(!old.join("bob").exists(), "emptied folders are removed");
    }

    #[test]
    fn rejects_bad_devices() {
        assert!(check_device("/etc/passwd").is_err());
        assert!(check_device("/dev/../etc/passwd").is_err());
        assert!(check_device("sda1").is_err());
    }

    #[test]
    fn events_round_trip() {
        let e = Event::Done(Report { failed: vec!["x".into()], ..Default::default() });
        let s = serde_json::to_string(&e).unwrap();
        let Event::Done(r) = serde_json::from_str(&s).unwrap() else { panic!() };
        assert_eq!(r.failed, ["x"]);
    }
}
