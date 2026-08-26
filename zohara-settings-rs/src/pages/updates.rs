use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;


pub fn build() -> gtk4::Widget {
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    root_box.set_margin_start(28);
    root_box.set_margin_end(28);
    root_box.set_margin_top(20);
    root_box.set_margin_bottom(32);

    // ── Page Title ────────────────────────────────────────────────────────────
    let title_lbl = gtk4::Label::builder()
        .label("Zohara Update")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Hero Update Status Banner ─────────────────────────────────────────────
    let hero_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 20);
    hero_card.set_css_classes(&["win11-hero-card"]);
    hero_card.set_margin_bottom(4);

    let sync_icon = gtk4::Image::from_icon_name("software-update-available-symbolic");
    sync_icon.set_pixel_size(48);
    sync_icon.set_css_classes(&["accent-cyan"]);
    hero_card.append(&sync_icon);

    let info_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    info_box.set_valign(gtk4::Align::Center);

    let status_title = gtk4::Label::builder()
        .label("You're up to date")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-name".to_string()])
        .build();

    let last_check_lbl = gtk4::Label::builder()
        .label("Last checked: Today")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-sub".to_string()])
        .build();

    info_box.append(&status_title);
    info_box.append(&last_check_lbl);
    hero_card.append(&info_box);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    hero_card.append(&spacer);

    // Right "Check for updates" Button
    let check_btn = gtk4::Button::builder()
        .label("Check for updates")
        .css_classes(vec!["win11-update-btn".to_string()])
        .valign(gtk4::Align::Center)
        .build();

    let status_title_clone = status_title.clone();
    let last_check_clone = last_check_lbl.clone();
    let check_btn_clone = check_btn.clone();
    check_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        status_title_clone.set_text("Checking for updates…");
        last_check_clone.set_text("Synchronizing repositories…");
        let title_c = status_title_clone.clone();
        let last_c = last_check_clone.clone();
        let check_c = check_btn_clone.clone();
        glib::spawn_future_local(async move {
            // Step 1: sync the package databases. This is the only way to
            // know if the local package versions are stale relative to the
            // mirrors. We deliberately do NOT use -Syu here -- "Check for
            // updates" should not also install updates.
            let sync = tokio::process::Command::new("pacman")
                .args(["-Sy"])
                .output()
                .await;

            if sync.is_err() || !sync.as_ref().unwrap().status.success() {
                // Render the error: prefer the subprocess's stderr (if it
                // ran and exited non-zero); fall back to the io::Error from
                // spawning itself. The original code moved `sync` into
                // `.err()` and then tried to use `sync` again inside the
                // closure, which is a use-after-move E0382.
                let err_line = match sync {
                    Ok(out) => String::from_utf8_lossy(&out.stderr)
                        .lines()
                        .next()
                        .unwrap_or("pacman exited non-zero")
                        .to_string(),
                    Err(e) => e.to_string(),
                };
                title_c.set_text("Check failed");
                last_c.set_text(&format!("Error: {}", err_line));
                check_c.set_sensitive(true);
                return;
            }

            // Step 2: query the local DB for out-of-date packages. `pacman -Qu`
            // prints one line per upgradeable package: "name current -> new".
            let qu = tokio::process::Command::new("pacman")
                .args(["-Qu"])
                .output()
                .await
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_default();

            let upgrades: Vec<&str> = qu.lines().filter(|l| !l.is_empty()).collect();
            let count = upgrades.len();

            // Use a fixed timestamp format rather than chrono (which isn't
            // a dep). glib::DateTime::now_local() gives us the local time
            // already; we just format it.
            let now = glib::DateTime::now_local()
                .and_then(|t| t.format("%Y-%m-%d %H:%M"))
                .unwrap_or_else(|_| "now".into());

            if count == 0 {
                title_c.set_text("You're up to date");
                last_c.set_text(&format!("Last checked: {}", now));
            } else if count == 1 {
                title_c.set_text("1 update available");
                last_c.set_text(&format!(
                    "Last checked: {} • {}",
                    now,
                    upgrades[0].split_whitespace().next().unwrap_or("(unknown package)")
                ));
            } else {
                title_c.set_text(&format!("{} updates available", count));
                last_c.set_text(&format!("Last checked: {}", now));
            }
            check_c.set_sensitive(true);
        });
    });
    hero_card.append(&check_btn);
    root_box.append(&hero_card);

    // ── Section 1: More options ───────────────────────────────────────────────
    let more_lbl = gtk4::Label::builder()
        .label("More options")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&more_lbl);

    let more_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    more_box.set_css_classes(&["win11-card-group"]);

    // Latest updates toggle
    let fast_sw = adw::SwitchRow::new();
    fast_sw.set_title("Get the latest updates as soon as they're available");
    fast_sw.set_subtitle("Be among the first to get the latest non-security updates, fixes, and improvements");
    fast_sw.add_prefix(&gtk4::Image::from_icon_name("starred-symbolic"));
    fast_sw.set_active(false);
    fast_sw.set_css_classes(&["win11-expander-row"]);
    more_box.append(&fast_sw);

    // Pause updates row
    let pause_row = adw::ActionRow::new();
    pause_row.set_title("Pause updates");
    pause_row.set_subtitle("Select the duration to pause automatic updates");
    pause_row.add_prefix(&gtk4::Image::from_icon_name("media-playback-pause-symbolic"));
    let pause_combo = gtk4::DropDown::from_strings(&["Pause for 1 week", "Pause for 2 weeks", "Pause for 3 weeks", "Pause for 4 weeks"]);
    pause_combo.set_valign(gtk4::Align::Center);
    pause_row.add_suffix(&pause_combo);
    pause_row.set_css_classes(&["win11-expander-row"]);
    more_box.append(&pause_row);

    // Update History
    let hist_row = build_action_row("Update history", "View installed packages and system upgrade logs", "document-open-recent-symbolic");
    more_box.append(&hist_row);

    // Advanced Options
    let adv_row = build_action_row("Advanced options", "Delivery optimization, optional updates, active hours, mirror selector", "preferences-system-symbolic");
    more_box.append(&adv_row);

    // Zohara Insider Program — REMOVED. There is no insider program; the
    // previous UI implied one existed and clicking the row did nothing.
    // Better to ship four honest rows than five where one is a stub.

    root_box.append(&more_box);

    // ── Section 2: Related support ────────────────────────────────────────────
    let support_lbl = gtk4::Label::builder()
        .label("Related support")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&support_lbl);

    let support_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    support_box.set_css_classes(&["win11-card-group"]);

    let help_exp = adw::ExpanderRow::new();
    help_exp.set_title("Help with Zohara Update");
    help_exp.add_prefix(&gtk4::Image::from_icon_name("help-browser-symbolic"));
    help_exp.set_css_classes(&["win11-expander-row"]);

    for item in &[
        "Troubleshooting package download errors",
        "Rolling back or downgrading a package",
        "Configuring custom Arch mirrors in pacman.conf",
        "Cleaning up package cache safely",
    ] {
        let r = adw::ActionRow::new();
        r.set_title(item);
        r.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        r.set_activatable(true);
        help_exp.add_row(&r);
    }
    support_box.append(&help_exp);
    root_box.append(&support_box);

    scroll.set_child(Some(&root_box));
    scroll.upcast()
}

fn build_action_row(title: &str, subtitle: &str, icon_name: &str) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(title);
    row.set_subtitle(subtitle);
    row.add_prefix(&gtk4::Image::from_icon_name(icon_name));
    row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
    row.set_css_classes(&["win11-expander-row"]);
    row.set_activatable(true);
    row
}
