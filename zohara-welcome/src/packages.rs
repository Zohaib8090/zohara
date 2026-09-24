//! Debian/Ubuntu package names -> Arch package names.

/// `None` means there is deliberately no Arch equivalent.
pub const DEB_TO_ARCH: &[(&str, Option<&str>)] = &[
    ("python3", Some("python")), ("python3-pip", Some("python-pip")),
    ("python3-dev", Some("python")), ("python3-venv", Some("python")),
    ("libqt5core5a", Some("qt5-base")), ("libqt5gui5", Some("qt5-base")),
    ("libgtk-3-0", Some("gtk3")), ("libgtk-4-1", Some("gtk4")),
    ("libnss3", Some("nss")), ("ffmpeg", Some("ffmpeg")), ("vlc", Some("vlc")),
    ("gimp", Some("gimp")), ("inkscape", Some("inkscape")),
    ("libreoffice-common", Some("libreoffice-still")),
    ("libreoffice-core", Some("libreoffice-still")),
    ("libreoffice-writer", Some("libreoffice-still")),
    ("libreoffice-calc", Some("libreoffice-still")),
    ("libreoffice-impress", Some("libreoffice-still")),
    ("git", Some("git")), ("curl", Some("curl")), ("wget", Some("wget")),
    ("vim", Some("vim")), ("nano", Some("nano")), ("htop", Some("htop")), ("btop", Some("btop")),
    ("neofetch", Some("fastfetch")), ("chromium-browser", Some("chromium")),
    ("google-chrome-stable", Some("google-chrome")), ("firefox", Some("firefox")),
    ("thunderbird", Some("thunderbird")), ("code", Some("code")),
    ("discord", Some("discord")), ("telegram-desktop", Some("telegram-desktop")),
    ("obs-studio", Some("obs-studio")), ("audacity", Some("audacity")),
    ("krita", Some("krita")), ("kdenlive", Some("kdenlive")),
    ("docker", Some("docker")), ("docker-ce", Some("docker")),
    ("nodejs", Some("nodejs")), ("npm", Some("npm")),
    ("default-jdk", Some("jdk-openjdk")), ("openjdk-11-jdk", Some("jdk11-openjdk")),
    ("openjdk-17-jdk", Some("jdk17-openjdk")),
    ("mysql-server", Some("mariadb")), ("postgresql", Some("postgresql")),
    ("nginx", Some("nginx")), ("openssh-server", Some("openssh")),
    ("ufw", Some("ufw")), ("wine", Some("wine")), ("steam", Some("steam")),
    ("lutris", Some("lutris")), ("flatpak", Some("flatpak")),
    ("snap", None),
];

/// System packages that make no sense to reinstall on Arch.
const EXCLUDED_PREFIXES: &[&str] = &[
    "lib", "linux-image", "linux-headers", "grub", "plymouth", "xserver", "xorg", "fonts-",
    "tzdata", "locales", "keyboard", "console-", "initramfs", "apt", "dpkg", "debconf", "base-",
];

fn lookup(pkg: &str) -> Option<Option<&'static str>> {
    DEB_TO_ARCH.iter().find(|(d, _)| *d == pkg).map(|(_, a)| *a)
}

/// Names of installed packages from a dpkg `status` file.
pub fn parse_dpkg_status(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for block in text.split("\n\n") {
        let mut name = None;
        let mut installed = false;
        for line in block.lines() {
            if let Some(v) = line.strip_prefix("Package:") {
                name = Some(v.trim().to_string());
            } else if let Some(v) = line.strip_prefix("Status:") {
                // "install ok installed", but not "deinstall ok config-files".
                installed = v.trim().ends_with(" installed");
            }
        }
        if let (Some(n), true) = (name, installed) {
            out.push(n);
        }
    }
    out
}

/// Returns (Arch packages to install, explanations for the ones with no equivalent).
/// The table is consulted before the exclusion list, so mapped `lib*` packages
/// such as `libreoffice-core` are not silently dropped.
pub fn translate(deb: &[String]) -> (Vec<String>, Vec<String>) {
    let mut arch = std::collections::BTreeSet::new();
    let mut unmapped = Vec::new();
    for pkg in deb {
        match lookup(pkg) {
            Some(Some(a)) => {
                arch.insert(a.to_string());
            }
            Some(None) => unmapped.push(format!("{pkg}  →  no Arch equivalent (Ubuntu-specific)")),
            None if EXCLUDED_PREFIXES.iter().any(|p| pkg.starts_with(p)) => {}
            None => unmapped.push(format!("{pkg}  →  not in the translation table")),
        }
    }
    (arch.into_iter().collect(), unmapped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status() {
        let s = "Package: vlc\nStatus: install ok installed\n\nPackage: old\nStatus: deinstall ok config-files\n\nPackage: git\nStatus: install ok installed\n";
        assert_eq!(parse_dpkg_status(s), ["vlc", "git"]);
    }

    #[test]
    fn maps_and_excludes() {
        let deb: Vec<String> = ["libreoffice-core", "libreoffice-writer", "snap", "libc6", "someapp", "vlc"]
            .iter().map(|s| s.to_string()).collect();
        let (arch, unmapped) = translate(&deb);
        assert_eq!(arch, ["libreoffice-still", "vlc"]);
        assert_eq!(unmapped.len(), 2);
        assert!(unmapped[0].starts_with("snap") && unmapped[1].starts_with("someapp"));
    }
}
