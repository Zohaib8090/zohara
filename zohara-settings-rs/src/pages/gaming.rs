use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

pub fn build() -> gtk4::Widget {
    let prefs_page = adw::PreferencesPage::new();

    let compat_group = adw::PreferencesGroup::new();
    compat_group.set_title("Compatibility layers");

    // GameMode
    let gamemode_row = adw::SwitchRow::new();
    gamemode_row.set_title("GameMode");
    gamemode_row.set_subtitle("Optimize CPU/GPU for active games");

    let gamemode_row_clone = gamemode_row.clone();
    glib::spawn_future_local(async move {
        let available = tokio::process::Command::new("which")
            .arg("gamemoded").output().await
            .map(|o| o.status.success()).unwrap_or(false);

        if !available {
            gamemode_row_clone.set_sensitive(false);
            gamemode_row_clone.set_subtitle("Not installed — install 'gamemode' package");
            return;
        }

        let active = tokio::process::Command::new("gamemoded")
            .arg("-s").output().await
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("active"))
            .unwrap_or(false);

        gamemode_row_clone.set_active(active);
        gamemode_row_clone.set_subtitle(if active { "Active" } else { "Installed, inactive" });
    });

    compat_group.add(&gamemode_row);

    // Wine
    let wine_row = adw::ActionRow::new();
    wine_row.set_title("Wine (Windows compatibility)");
    wine_row.set_subtitle("Checking…");

    let wine_row_clone = wine_row.clone();
    glib::spawn_future_local(async move {
        let ver = tokio::process::Command::new("wine")
            .arg("--version").output().await
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Not installed".to_string());
        wine_row_clone.set_subtitle(&ver);
    });
    compat_group.add(&wine_row);

    // Waydroid
    let waydroid_row = adw::ActionRow::new();
    waydroid_row.set_title("Waydroid (Android compatibility)");
    waydroid_row.set_subtitle("Checking…");

    let waydroid_row_clone = waydroid_row.clone();
    glib::spawn_future_local(async move {
        let status = tokio::process::Command::new("waydroid")
            .args(["status"]).output().await
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| {
                if s.contains("running") { "Running".to_string() }
                else if s.contains("stopped") { "Stopped".to_string() }
                else { "Not initialized".to_string() }
            })
            .unwrap_or_else(|| "Not installed".to_string());
        waydroid_row_clone.set_subtitle(&status);
    });
    compat_group.add(&waydroid_row);

    prefs_page.add(&compat_group);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&prefs_page));
    toolbar_view.upcast()
}
