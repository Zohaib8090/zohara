use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

pub fn build() -> gtk4::Widget {
    let prefs_page = adw::PreferencesPage::new();

    let apps_group = adw::PreferencesGroup::new();
    apps_group.set_title("Installed applications");

    let loading_row = adw::ActionRow::new();
    loading_row.set_title("Scanning installed apps…");
    let spinner = gtk4::Spinner::new();
    spinner.start();
    loading_row.add_suffix(&spinner);
    apps_group.add(&loading_row);

    // Search bar
    let search = gtk4::SearchEntry::new();
    search.set_placeholder_text(Some("Search apps…"));
    search.set_margin_start(12);
    search.set_margin_end(12);
    search.set_margin_top(8);
    search.set_margin_bottom(4);

    let apps_group_clone = apps_group.clone();
    let loading_row_clone = loading_row.clone();
    glib::spawn_future_local(async move {
        // Get native pacman packages with size
        let result = tokio::process::Command::new("bash")
            .args(["-c", "pacman -Qq | head -200"])
            .output()
            .await;

        apps_group_clone.remove(&loading_row_clone);

        if let Ok(out) = result {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for pkg in stdout.lines().take(100) {
                let info_result = tokio::process::Command::new("bash")
                    .args(["-c", &format!("pacman -Qi {} 2>/dev/null | grep -E '^(Name|Installed Size)' | awk -F': ' '{{print $2}}'", pkg)])
                    .output().await;

                let row = adw::ActionRow::new();
                row.set_title(pkg);
                if let Ok(info) = info_result {
                    let info_str = String::from_utf8_lossy(&info.stdout);
                    let lines: Vec<&str> = info_str.lines().collect();
                    let size = lines.get(1).unwrap_or(&"").trim();
                    row.set_subtitle(&format!("Native • {}", size));
                }
                apps_group_clone.add(&row);
            }
        }
    });

    prefs_page.add(&apps_group);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&prefs_page));
    toolbar_view.upcast()
}
