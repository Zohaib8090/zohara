//! Shared code for the Zohara welcome app and migration tool.

pub mod migrate;
pub mod packages;

use std::os::unix::fs::MetadataExt;

/// True when the current process runs as root (no pkexec needed).
pub fn is_root() -> bool {
    std::fs::metadata("/proc/self").map(|m| m.uid() == 0).unwrap_or(false)
}

/// True on the live USB, where installing is possible and migrating is not.
pub fn is_live() -> bool {
    std::path::Path::new("/run/archiso").exists()
}
