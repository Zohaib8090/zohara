//! Which update channel this computer follows: stable, beta or alpha.
//!
//! A channel is one of Zohara's three package repositories (separate releases in `Zohaib8090/zohara-packages`); the
//! computer follows exactly one. The switching itself is done by the `zohara-channel` tool that ships in the ISO (the
//! same one Zohara Settings uses): it writes `/etc/zohara/channel`, turns on the matching `[zohara-*]` section of
//! `pacman.conf` and refreshes the databases. This module only reads the current choice and asks that tool to change it.

use std::process::{Command, Stdio};

pub const CHANNEL_FILE: &str = "/etc/zohara/channel";

/// (id, name shown, one-line explanation)
pub const CHANNELS: [(&str, &str, &str); 3] = [
    ("stable", "Stable", "Tested updates. Recommended."),
    ("beta", "Beta", "Newer updates that are still being tried out. Some rough edges."),
    ("alpha", "Alpha", "The newest builds, straight from development. Expect things to break."),
];

/// "stable" / "beta" / "alpha" from the text of the channel file; anything else counts as stable.
pub fn parse_channel(text: &str) -> &'static str {
    let t = text.trim();
    CHANNELS.iter().map(|c| c.0).find(|id| *id == t).unwrap_or("stable")
}

pub fn index_of(id: &str) -> u32 {
    CHANNELS.iter().position(|c| c.0 == id).unwrap_or(0) as u32
}

/// The channel this computer is on now (stable when the file is missing, as on a fresh install).
pub fn current() -> &'static str {
    parse_channel(&std::fs::read_to_string(CHANNEL_FILE).unwrap_or_default())
}

/// Switches to `id` as administrator: non-interactive sudo first (the live ISO), then pkexec (asks for the password on
/// an installed system). Success means the channel file really says `id` afterwards.
pub fn set(id: &str) -> Result<(), String> {
    if !CHANNELS.iter().any(|c| c.0 == id) {
        return Err("That isn't one of Zohara's update channels.".into());
    }
    let sudo_ok = Command::new("sudo")
        .args(["-n", "zohara-channel", "set", id])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !sudo_ok {
        let out = Command::new("pkexec")
            .args(["zohara-channel", "set", id])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("Couldn't start the channel switcher ({e})."))?;
        if !out.status.success() {
            return Err(match out.status.code() {
                Some(126) | Some(127) => "The password prompt was cancelled, so nothing was changed.".into(),
                _ => "The channel switcher didn't finish. Nothing was changed.".into(),
            });
        }
    }
    if current() == id {
        Ok(())
    } else {
        Err("The switch didn't take effect. Your channel is unchanged.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_file_text_is_read_safely() {
        assert_eq!(parse_channel("beta\n"), "beta");
        assert_eq!(parse_channel("  alpha "), "alpha");
        assert_eq!(parse_channel("stable"), "stable");
        assert_eq!(parse_channel(""), "stable");
        assert_eq!(parse_channel("nightly"), "stable");
        assert_eq!(parse_channel("beta; rm -rf /"), "stable");
    }

    #[test]
    fn picker_order_matches_the_channel_list() {
        assert_eq!(index_of("stable"), 0);
        assert_eq!(index_of("beta"), 1);
        assert_eq!(index_of("alpha"), 2);
        assert_eq!(index_of("whatever"), 0);
    }

    #[test]
    fn unknown_channels_are_refused_before_any_command_runs() {
        assert!(set("nightly").is_err());
        assert!(set("beta; id").is_err());
    }
}
