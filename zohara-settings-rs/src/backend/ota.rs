// OTA update backend.
//
// The settings app queries a small JSON manifest hosted on GitHub Releases
// every time the user clicks "Check for updates". The manifest lives at
//   https://github.com/Zohaib8090/zohara/releases/latest/download/latest.json
// and the OTA bundle itself lives at
//   https://github.com/Zohaib8090/zohara/releases/latest/download/zohara-update-<DATE>.sh
//
// Design constraints:
//   * We only DISPLAY a "Download" and "Run" button. We never auto-run.
//   * Local version is read by invoking `/usr/bin/zohara-settings --version`
//     (the `main` function in src/main.rs short-circuits to print + exit when
//     given --version, so it doesn't need a display).
//   * The downloaded bundle is saved to a well-known tmp path so the
//     "Run update" button can pass it to pkexec without re-downloading.
//   * The user always sees the bundle's own `install_update.sh` output
//     (we just exec it under pkexec), so they see what is happening.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

const MANIFEST_URL: &str =
    "https://github.com/Zohaib8090/zohara/releases/latest/download/latest.json";

/// Parsed view of `latest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    /// The published Zohara OS version (e.g. "1.0.0").
    pub version: String,
    /// ISO 8601 date the bundle was built.
    pub date: String,
    /// Direct download URL of the self-extracting .sh bundle.
    pub download_url: String,
    /// Bundle size in bytes (used for the progress UI).
    pub size_bytes: u64,
    /// Optional SHA-256 (hex) for client-side integrity. Optional because
    /// the bundle is itself a `tar` archive that gets verified by the
    /// user's `tar` invocation; we don't double-verify by default.
    #[serde(default)]
    pub sha256: Option<String>,
    /// Free-text changelog shown next to the download button.
    #[serde(default)]
    pub changelog: String,
    /// The minimum local `zohara-settings` version required to install
    /// this update. If the user's local version is older, the update
    /// page shows a "too old, reinstall" warning instead of the
    /// download button.
    #[serde(default)]
    pub min_zohara_settings_version: Option<String>,
}

/// The result of comparing the local version to the manifest.
#[derive(Debug, Clone)]
pub enum UpdateStatus {
    /// Either no manifest was reachable, or the local version is at or
    /// above the published version. The UI shows "You're up to date".
    UpToDate,
    /// Local version < published version. UI shows the download button.
    UpdateAvailable(UpdateManifest),
    /// Local version < min_zohara_settings_version. UI shows a
    /// "Reinstall required" warning and disables the download button.
    TooOld {
        manifest: UpdateManifest,
        local: String,
    },
    /// Could not reach the manifest endpoint (offline, DNS, etc.).
    /// UI shows "Update check failed: <reason>".
    Unreachable(String),
    /// Manifest was reachable but malformed JSON. UI shows an error.
    Malformed(String),
    /// Local `zohara-settings --version` could not be parsed. The binary
    /// may be too old. UI shows an error pointing at the install.
    LocalUnknown(String),
}

impl UpdateStatus {
    pub fn is_update_available(&self) -> bool {
        matches!(self, UpdateStatus::UpdateAvailable(_))
    }
}

/// Read the local `zohara-settings` version by invoking the binary.
///
/// The binary is always at `/usr/bin/zohara-settings` on an installed
/// Zohara OS. We don't hardcode an absolute path in the binary call --
/// we just call `zohara-settings` so PATH resolution finds it.
pub fn read_local_version() -> Result<String, String> {
    let out = Command::new("zohara-settings")
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to spawn zohara-settings --version: {}", e))?;

    if !out.status.success() {
        return Err(format!(
            "zohara-settings --version exited with {:?}",
            out.status.code()
        ));
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // Output is "zohara-settings <version>"; keep only the version.
    let version = s.split_whitespace().last().unwrap_or("").to_string();
    if version.is_empty() {
        return Err(format!("empty version string: {:?}", s));
    }
    Ok(version)
}

/// Compare two semver-ish strings. Returns true if `local` < `remote`.
///
/// We don't pull in a semver crate for one comparison. Versions are
/// dotted-decimal (`1.2.3`); we compare component-by-component. Anything
/// non-numeric in a component is treated as 0. The function is permissive:
/// "1.0" and "1.0.0" compare equal, "1.10" > "1.9", and so on.
pub fn version_lt(local: &str, remote: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .map(|p| p.parse::<u32>().unwrap_or(0))
            .collect()
    };
    let l = parse(local);
    let r = parse(remote);
    let n = l.len().max(r.len());
    for i in 0..n {
        let a = l.get(i).copied().unwrap_or(0);
        let b = r.get(i).copied().unwrap_or(0);
        if a < b {
            return true;
        }
        if a > b {
            return false;
        }
    }
    false
}

/// Download the manifest JSON over HTTPS.
///
/// We don't add a `reqwest` dependency for one GET. `curl` is present on
/// every Zohara OS install, and `curl -fsSL` with a 10s timeout is enough
/// for a tiny JSON file. Errors include the curl exit code for debugging.
pub fn fetch_manifest() -> Result<UpdateManifest, String> {
    let out = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time", "10",
            "-H", "Accept: application/json",
            MANIFEST_URL,
        ])
        .output()
        .map_err(|e| format!("Failed to spawn curl: {}", e))?;

    if !out.status.success() {
        let code = out.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("curl exited {}: {}", code, stderr.trim()));
    }

    serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("Failed to parse manifest JSON: {}", e))
}

/// Compare the local version to the manifest and return a UI-friendly
/// status. This is the single function the updates page calls.
pub async fn check_for_updates() -> UpdateStatus {
    let local = match read_local_version() {
        Ok(v) => v,
        Err(e) => return UpdateStatus::LocalUnknown(e),
    };

    // Fetch runs in a blocking task because we use std::process::Command,
    // not tokio::process::Command (avoids a tokio::process::Command dep).
    let manifest_result = tokio::task::spawn_blocking(fetch_manifest).await;

    let manifest = match manifest_result {
        Ok(Ok(m)) => m,
        Ok(Err(e)) => return UpdateStatus::Unreachable(e),
        Err(e) => return UpdateStatus::Unreachable(format!("manifest task panicked: {}", e)),
    };

    // If a min-zohara-settings-version is set and the local is older than
    // that, the user must reinstall from ISO; we don't offer a download.
    if let Some(min_v) = manifest.min_zohara_settings_version.as_deref() {
        if version_lt(&local, min_v) {
            return UpdateStatus::TooOld {
                manifest,
                local,
            };
        }
    }

    if version_lt(&local, &manifest.version) {
        UpdateStatus::UpdateAvailable(manifest)
    } else {
        UpdateStatus::UpToDate
    }
}

/// Download the OTA bundle to a stable tmp path and return the path.
///
/// Caller (the UI) decides what to do with it: show a "Run now" button
/// that execs the script under pkexec, or just open a file manager
/// showing the file. We never run it automatically.
pub async fn download_bundle(manifest: &UpdateManifest) -> Result<PathBuf, String> {
    let dest = PathBuf::from(format!(
        "/tmp/zohara-update-{}.sh",
        manifest.date.replace('.', "-")
    ));
    let url = manifest.download_url.clone();
    let dest_clone = dest.clone();
    tokio::task::spawn_blocking(move || {
        let status = Command::new("curl")
            .args([
                "-fL",
                "--progress-bar",
                "-o", dest_clone.to_str().unwrap_or("/tmp/zohara-update.sh"),
                &url,
            ])
            .status()
            .map_err(|e| format!("Failed to spawn curl: {}", e))?;
        if !status.success() {
            return Err(format!("curl exited with {:?}", status.code()));
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("download task panicked: {}", e))??;

    Ok(dest)
}
