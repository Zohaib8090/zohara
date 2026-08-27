// System info backend.
//
// All values are read by parsing /proc or /sys (no shelling out to
// `uname` / `lscpu` / `free` -- the proc filesystem is faster and
// locale-independent). Reads are blocking; wrap with `spawn_blocking` at
// the call site if you need them off the GTK main loop.

use std::fs;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub hostname: String,
    pub kernel: String,
    pub os_pretty: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub ram_total_bytes: u64,
    pub ram_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_mount: String,
}

pub fn read() -> SystemInfo {
    let hostname =
        fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()).unwrap_or_default();
    let kernel = fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let os_pretty = fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "Zohara OS".to_string());
    let cpu_model = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.splitn(2, ':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "Unknown CPU".to_string());
    let cpu_cores = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .map(|s| s.lines().filter(|l| l.starts_with("processor")).count() as u32)
        .filter(|&n| n > 0)
        .unwrap_or(1);

    // /proc/meminfo lines look like "MemTotal:       16384000 kB"
    let (ram_total, ram_avail) = fs::read_to_string("/proc/meminfo")
        .ok()
        .map(|s| {
            let mut total: u64 = 0;
            let mut avail: u64 = 0;
            for line in s.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    if let Some(kb) = rest.split_whitespace().next() {
                        total = kb.parse::<u64>().unwrap_or(0) * 1024;
                    }
                } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
                    if let Some(kb) = rest.split_whitespace().next() {
                        avail = kb.parse::<u64>().unwrap_or(0) * 1024;
                    }
                }
            }
            (total, avail)
        })
        .unwrap_or((0, 0));

    // Disk usage of the root mount via `statvfs(2)` is more correct than
    // shelling out, but we don't have the `nix` crate. Use `df -PB1 /`
    // which is available on every Zohara OS install.
    let (disk_total, disk_used) = read_df_root();

    SystemInfo {
        hostname,
        kernel,
        os_pretty,
        cpu_model,
        cpu_cores,
        ram_total_bytes: ram_total,
        ram_used_bytes: ram_total.saturating_sub(ram_avail),
        disk_total_bytes: disk_total,
        disk_used_bytes: disk_used,
        disk_mount: "/".to_string(),
    }
}

fn read_df_root() -> (u64, u64) {
    let out = std::process::Command::new("df")
        .args(["-PB1", "/"])
        .output()
        .ok();
    if let Some(out) = out {
        let s = String::from_utf8_lossy(&out.stdout);
        // header:  Filesystem   1B-blocks        Used   Available Use% Mounted
        // row:     /dev/sda2   500105249024  12000000000  488000000000   3% /
        if let Some(line) = s.lines().nth(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 3 {
                let total = cols[1].parse::<u64>().unwrap_or(0);
                let used = cols[2].parse::<u64>().unwrap_or(0);
                return (total, used);
            }
        }
    }
    (0, 0)
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}
