use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

pub fn build() -> gtk4::Widget {
    let prefs_page = adw::PreferencesPage::new();

    // ── Status ────────────────────────────────────────────────────────────────
    let status_group = adw::PreferencesGroup::new();

    let status_row = adw::ActionRow::new();
    status_row.set_title("System is up to date");
    status_row.set_subtitle("Checking for updates…");

    let check_btn = gtk4::Button::builder()
        .label("Check for updates")
        .css_classes(vec!["suggested-action".to_string()])
        .valign(gtk4::Align::Center)
        .build();

    let install_btn = gtk4::Button::builder()
        .label("Install all")
        .valign(gtk4::Align::Center)
        .build();
    install_btn.set_sensitive(false);

    let status_row_clone = status_row.clone();
    let install_btn_clone = install_btn.clone();

    let updates_group = adw::PreferencesGroup::new();
    updates_group.set_title("Available updates");
    updates_group.set_visible(false);

    let updates_group_clone = updates_group.clone();

    let do_check = {
        let status_row = status_row_clone.clone();
        let install_btn = install_btn_clone.clone();
        let updates_group = updates_group_clone.clone();
        move || {
            let status_row = status_row.clone();
            let install_btn = install_btn.clone();
            let updates_group = updates_group.clone();
            glib::spawn_future_local(async move {
                status_row.set_subtitle("Checking for updates…");

                let result = tokio::process::Command::new("checkupdates")
                    .output()
                    .await;

                if let Ok(out) = result {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let lines: Vec<&str> = stdout.lines().collect();
                    let count = lines.len();

                    if count == 0 {
                        status_row.set_title("System is up to date");
                        status_row.set_subtitle("Last checked just now");
                        install_btn.set_sensitive(false);
                        updates_group.set_visible(false);
                    } else {
                        status_row.set_title(&format!("{} updates available", count));
                        status_row.set_subtitle("Updates ready to install");
                        install_btn.set_sensitive(true);
                        updates_group.set_visible(true);

                        // Populate update rows
                        for line in &lines {
                            // Format: "package oldver -> newver"
                            let parts: Vec<&str> = line.splitn(3, ' ').collect();
                            let row = adw::ActionRow::new();
                            row.set_title(parts.first().unwrap_or(&""));
                            if parts.len() >= 3 {
                                row.set_subtitle(parts[2]); // "oldver -> newver"
                            }
                            updates_group.add(&row);
                        }
                    }
                }
            });
        }
    };

    // Run check on load
    do_check();

    check_btn.connect_clicked(move |_| do_check());

    let install_row = adw::ActionRow::new();
    install_row.set_title("Installing…");
    install_row.set_visible(false);

    install_btn.connect_clicked({
        let install_row = install_row.clone();
        move |btn| {
            btn.set_sensitive(false);
            install_row.set_visible(true);
            glib::spawn_future_local(async move {
                let _ = tokio::process::Command::new("pkexec")
                    .args(["pacman", "-Syu", "--noconfirm"])
                    .output()
                    .await;
            });
        }
    });

    status_row.add_suffix(&check_btn);
    status_row.add_suffix(&install_btn);
    status_group.add(&status_row);

    // ── History ───────────────────────────────────────────────────────────────
    let history_group = adw::PreferencesGroup::new();
    history_group.set_title("Recent update history");

    let history_row = adw::ActionRow::new();
    history_row.set_title("Load history");
    history_row.set_activatable(true);
    history_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));

    let history_group_clone = history_group.clone();
    history_row.connect_activated(move |row| {
        row.set_sensitive(false);
        let history_group = history_group_clone.clone();
        glib::spawn_future_local(async move {
            let result = tokio::process::Command::new("bash")
                .args(["-c", "grep 'upgraded\\|installed\\|removed' /var/log/pacman.log | tail -30 | tac"])
                .output()
                .await;

            if let Ok(out) = result {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let entry = adw::ActionRow::new();
                    // Format: "[2026-08-08T12:00:00+0000] [ALPM] upgraded pkg (1.0 -> 1.1)"
                    let clean = line.trim_start_matches('[');
                    let parts: Vec<&str> = clean.splitn(3, ']').collect();
                    let date = parts.first().map(|s| &s[..16.min(s.len())]).unwrap_or("");
                    let action = parts.last().unwrap_or(&line).trim();
                    entry.set_title(action);
                    entry.set_subtitle(date);
                    history_group.add(&entry);
                }
            }
        });
    });

    history_group.add(&history_row);

    prefs_page.add(&status_group);
    prefs_page.add(&updates_group);
    prefs_page.add(&history_group);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&prefs_page));
    toolbar_view.upcast()
}
