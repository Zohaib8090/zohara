use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub description: String,
    pub icon_name: String,
    pub source: AppSource,
    pub package_name: String,
    pub category: AppCategory,
    pub rating: f32,  // 0.0 - 5.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppSource {
    Pacman,
    Flatpak,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppCategory {
    Browser,
    Development,
    Multimedia,
    Communication,
    Graphics,
    Productivity,
    Games,
    Utilities,
    Education,
}

impl AppCategory {
    pub fn label(&self) -> &'static str {
        match self {
            AppCategory::Browser => "Browser",
            AppCategory::Development => "Development",
            AppCategory::Multimedia => "Multimedia",
            AppCategory::Communication => "Communication",
            AppCategory::Graphics => "Graphics & Design",
            AppCategory::Productivity => "Productivity",
            AppCategory::Games => "Games",
            AppCategory::Utilities => "Utilities",
            AppCategory::Education => "Education",
        }
    }

    pub fn is_game(&self) -> bool {
        matches!(self, AppCategory::Games)
    }
}

pub fn get_curated_apps() -> Vec<AppInfo> {
    vec![
        // ── Browsers ──────────────────────────────────────────────────────────
        AppInfo {
            id: "firefox".into(),
            name: "Mozilla Firefox".into(),
            publisher: "Mozilla Foundation".into(),
            description: "Fast, Private & Safe Web Browser".into(),
            icon_name: "firefox".into(),
            source: AppSource::Pacman,
            package_name: "firefox".into(),
            category: AppCategory::Browser,
            rating: 4.7,
        },
        AppInfo {
            id: "chromium".into(),
            name: "Chromium".into(),
            publisher: "The Chromium Project".into(),
            description: "Open-source web browser powering Google Chrome".into(),
            icon_name: "chromium".into(),
            source: AppSource::Pacman,
            package_name: "chromium".into(),
            category: AppCategory::Browser,
            rating: 4.5,
        },
        // ── Development ───────────────────────────────────────────────────────
        AppInfo {
            id: "vscodium".into(),
            name: "VSCodium".into(),
            publisher: "VSCodium Community".into(),
            description: "Free/Libre Open Source Binaries of VS Code".into(),
            icon_name: "vscodium".into(),
            source: AppSource::Pacman,
            package_name: "vscodium-bin".into(),
            category: AppCategory::Development,
            rating: 4.8,
        },
        AppInfo {
            id: "git".into(),
            name: "Git".into(),
            publisher: "Software Freedom Conservancy".into(),
            description: "Fast distributed version control system".into(),
            icon_name: "git".into(),
            source: AppSource::Pacman,
            package_name: "git".into(),
            category: AppCategory::Development,
            rating: 4.9,
        },
        // ── Multimedia ────────────────────────────────────────────────────────
        AppInfo {
            id: "vlc".into(),
            name: "VLC Media Player".into(),
            publisher: "VideoLAN".into(),
            description: "Read, play, broadcast your multimedia streams".into(),
            icon_name: "vlc".into(),
            source: AppSource::Pacman,
            package_name: "vlc".into(),
            category: AppCategory::Multimedia,
            rating: 4.8,
        },
        AppInfo {
            id: "spotify".into(),
            name: "Spotify".into(),
            publisher: "Spotify AB".into(),
            description: "Discover, manage and share over 100 million tracks".into(),
            icon_name: "spotify-client".into(),
            source: AppSource::Flatpak,
            package_name: "com.spotify.Client".into(),
            category: AppCategory::Multimedia,
            rating: 4.4,
        },
        AppInfo {
            id: "mpv".into(),
            name: "mpv".into(),
            publisher: "mpv Contributors".into(),
            description: "A free, open source, and cross-platform media player".into(),
            icon_name: "mpv".into(),
            source: AppSource::Pacman,
            package_name: "mpv".into(),
            category: AppCategory::Multimedia,
            rating: 4.7,
        },
        // ── Communication ─────────────────────────────────────────────────────
        AppInfo {
            id: "discord".into(),
            name: "Discord".into(),
            publisher: "Discord Inc.".into(),
            description: "Chat for Communities and Friends".into(),
            icon_name: "discord".into(),
            source: AppSource::Pacman,
            package_name: "discord".into(),
            category: AppCategory::Communication,
            rating: 4.3,
        },
        AppInfo {
            id: "telegram-desktop".into(),
            name: "Telegram".into(),
            publisher: "Telegram FZ-LLC".into(),
            description: "A new era of messaging — fast, secure, and open".into(),
            icon_name: "telegram".into(),
            source: AppSource::Pacman,
            package_name: "telegram-desktop".into(),
            category: AppCategory::Communication,
            rating: 4.6,
        },
        // ── Graphics ──────────────────────────────────────────────────────────
        AppInfo {
            id: "gimp".into(),
            name: "GIMP".into(),
            publisher: "The GIMP Team".into(),
            description: "GNU Image Manipulation Program for professionals".into(),
            icon_name: "gimp".into(),
            source: AppSource::Pacman,
            package_name: "gimp".into(),
            category: AppCategory::Graphics,
            rating: 4.5,
        },
        AppInfo {
            id: "inkscape".into(),
            name: "Inkscape".into(),
            publisher: "Inkscape Team".into(),
            description: "Professional vector graphics editor".into(),
            icon_name: "inkscape".into(),
            source: AppSource::Pacman,
            package_name: "inkscape".into(),
            category: AppCategory::Graphics,
            rating: 4.6,
        },
        AppInfo {
            id: "krita".into(),
            name: "Krita".into(),
            publisher: "KDE".into(),
            description: "Digital painting and illustration app for artists".into(),
            icon_name: "krita".into(),
            source: AppSource::Pacman,
            package_name: "krita".into(),
            category: AppCategory::Graphics,
            rating: 4.7,
        },
        // ── Productivity ──────────────────────────────────────────────────────
        AppInfo {
            id: "libreoffice".into(),
            name: "LibreOffice".into(),
            publisher: "The Document Foundation".into(),
            description: "Powerful office suite — write, calc, impress, draw".into(),
            icon_name: "libreoffice-main".into(),
            source: AppSource::Pacman,
            package_name: "libreoffice-fresh".into(),
            category: AppCategory::Productivity,
            rating: 4.4,
        },
        AppInfo {
            id: "obsidian".into(),
            name: "Obsidian".into(),
            publisher: "Dynalist Inc.".into(),
            description: "A second brain, for you, forever".into(),
            icon_name: "obsidian".into(),
            source: AppSource::Flatpak,
            package_name: "md.obsidian.Obsidian".into(),
            category: AppCategory::Productivity,
            rating: 4.8,
        },
        // ── Games ─────────────────────────────────────────────────────────────
        AppInfo {
            id: "steam".into(),
            name: "Steam".into(),
            publisher: "Valve Corporation".into(),
            description: "The ultimate destination for playing & discussing games".into(),
            icon_name: "steam".into(),
            source: AppSource::Pacman,
            package_name: "steam".into(),
            category: AppCategory::Games,
            rating: 4.6,
        },
        AppInfo {
            id: "lutris".into(),
            name: "Lutris".into(),
            publisher: "Mathieu Comandon".into(),
            description: "Open gaming platform for Linux".into(),
            icon_name: "lutris".into(),
            source: AppSource::Pacman,
            package_name: "lutris".into(),
            category: AppCategory::Games,
            rating: 4.4,
        },
        AppInfo {
            id: "heroic".into(),
            name: "Heroic Games Launcher".into(),
            publisher: "Heroic Games Launcher".into(),
            description: "Open Source GOG, Epic Games & Amazon Prime Games".into(),
            icon_name: "heroic".into(),
            source: AppSource::Flatpak,
            package_name: "com.heroicgameslauncher.hgl".into(),
            category: AppCategory::Games,
            rating: 4.5,
        },
        AppInfo {
            id: "supertuxkart".into(),
            name: "SuperTuxKart".into(),
            publisher: "SuperTuxKart Team".into(),
            description: "A 3D open-source arcade kart racing game".into(),
            icon_name: "supertuxkart".into(),
            source: AppSource::Pacman,
            package_name: "supertuxkart".into(),
            category: AppCategory::Games,
            rating: 4.2,
        },
        AppInfo {
            id: "0ad".into(),
            name: "0 A.D.".into(),
            publisher: "Wildfire Games".into(),
            description: "A historical real-time strategy game".into(),
            icon_name: "0ad".into(),
            source: AppSource::Pacman,
            package_name: "0ad".into(),
            category: AppCategory::Games,
            rating: 4.3,
        },
        // ── Utilities ─────────────────────────────────────────────────────────
        AppInfo {
            id: "htop".into(),
            name: "htop".into(),
            publisher: "htop Contributors".into(),
            description: "Interactive process viewer and system monitor".into(),
            icon_name: "htop".into(),
            source: AppSource::Pacman,
            package_name: "htop".into(),
            category: AppCategory::Utilities,
            rating: 4.7,
        },
        AppInfo {
            id: "timeshift".into(),
            name: "Timeshift".into(),
            publisher: "Tony George".into(),
            description: "System restore utility — like Time Machine for Linux".into(),
            icon_name: "timeshift".into(),
            source: AppSource::Pacman,
            package_name: "timeshift".into(),
            category: AppCategory::Utilities,
            rating: 4.6,
        },
    ]
}
