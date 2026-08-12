use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

pub fn build() -> gtk4::Widget {
    let prefs_page = adw::PreferencesPage::new();

    // ── Users ─────────────────────────────────────────────────────────────────
    let users_group = adw::PreferencesGroup::new();
    users_group.set_title("Local accounts");

    let add_btn = gtk4::Button::builder()
        .label("Add account")
        .css_classes(vec!["flat".to_string()])
        .icon_name("list-add-symbolic")
        .build();
    users_group.set_header_suffix(Some(&add_btn));

    let users_group_clone = users_group.clone();
    glib::spawn_future_local(async move {
        let result = tokio::process::Command::new("bash")
            .args(["-c", "getent passwd | awk -F: '$3 >= 1000 && $3 < 65534 {print $1 \":\" $3 \":\" $5}'"])
            .output()
            .await;

        if let Ok(out) = result {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(3, ':').collect();
                if parts.len() < 2 { continue; }
                let username = parts[0];
                let uid: u32 = parts[1].parse().unwrap_or(0);
                let fullname = parts.get(2).unwrap_or(&"").trim();

                // Check admin
                let is_admin = tokio::process::Command::new("bash")
                    .args(["-c", &format!("id -nG {} | grep -qw wheel && echo yes", username)])
                    .output().await
                    .map(|o| o.status.success())
                    .unwrap_or(false);

                let row = adw::ActionRow::new();
                let display_name = if fullname.is_empty() || fullname == username {
                    username.to_string()
                } else {
                    format!("{} ({})", fullname, username)
                };
                row.set_title(&display_name);
                row.set_subtitle(&format!(
                    "{} • uid {}",
                    if is_admin { "Administrator" } else { "Standard user" },
                    uid
                ));

                let icon = gtk4::Image::from_icon_name(
                    if is_admin { "dialog-password-symbolic" } else { "system-users-symbolic" }
                );
                row.add_prefix(&icon);

                users_group_clone.add(&row);
            }
        }
    });

    prefs_page.add(&users_group);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&prefs_page));
    toolbar_view.upcast()
}
