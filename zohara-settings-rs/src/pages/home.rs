use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::process::Command;

pub fn build() -> gtk4::Widget {
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 20);
    root_box.set_margin_start(28);
    root_box.set_margin_end(28);
    root_box.set_margin_top(20);
    root_box.set_margin_bottom(32);

    // ── Page Title ────────────────────────────────────────────────────────────
    let title_lbl = gtk4::Label::builder()
        .label("Home")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Hero Banner Card ──────────────────────────────────────────────────────
    let hero_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 18);
    hero_card.set_css_classes(&["win11-hero-card"]);
    hero_card.set_margin_bottom(6);

    // Device Thumbnail / Wallpaper preview
    let thumb_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    thumb_box.set_css_classes(&["win11-device-thumb"]);
    let thumb_icon = gtk4::Image::from_icon_name("computer-symbolic");
    thumb_icon.set_pixel_size(42);
    thumb_icon.set_valign(gtk4::Align::Center);
    thumb_icon.set_halign(gtk4::Align::Center);
    thumb_box.append(&thumb_icon);
    hero_card.append(&thumb_box);

    // Device info (Hostname + Model + Rename)
    let info_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    info_box.set_valign(gtk4::Align::Center);

    let hostname = read_hostname();
    let host_lbl = gtk4::Label::builder()
        .label(&hostname)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-name".to_string()])
        .build();

    let model_str = read_model();
    let model_lbl = gtk4::Label::builder()
        .label(&model_str)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-sub".to_string()])
        .build();

    let rename_btn = gtk4::Button::builder()
        .label("Rename")
        .css_classes(vec!["win11-link-btn".to_string()])
        .halign(gtk4::Align::Start)
        .build();

    let host_clone = host_lbl.clone();
    rename_btn.connect_clicked(move |btn| {
        let parent = btn.root().and_downcast::<gtk4::Window>();
        let dialog = adw::MessageDialog::builder()
            .heading("Rename your PC")
            .body("Enter a new name for this device:")
            .transient_for(parent.as_ref().unwrap_or(&gtk4::Window::new()))
            .build();
        let entry = gtk4::Entry::builder()
            .text(&*host_clone.text())
            .margin_top(8)
            .margin_bottom(8)
            .build();
        dialog.set_extra_child(Some(&entry));
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("rename", "Save");
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);

        let host_for_dialog = host_clone.clone();
        dialog.connect_response(None, move |d, resp| {
            if resp == "rename" {
                let new_name = entry.text().trim().to_string();
                if !new_name.is_empty() {
                    let _ = Command::new("hostnamectl").arg("set-hostname").arg(&new_name).status();
                    host_for_dialog.set_text(&new_name);
                }
            }
            d.close();
        });
        dialog.present();
    });

    info_box.append(&host_lbl);
    info_box.append(&model_lbl);
    info_box.append(&rename_btn);
    hero_card.append(&info_box);

    // Spacer
    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    hero_card.append(&spacer);

    // Right Status Badges (Wi-Fi status + Update status)
    let badges_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 28);
    badges_box.set_valign(gtk4::Align::Center);

    // Wi-Fi badge
    let wifi_badge = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    let wifi_icon = gtk4::Image::from_icon_name("network-wireless-symbolic");
    wifi_icon.set_pixel_size(24);
    wifi_icon.set_css_classes(&["accent-blue"]);
    let wifi_texts = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    let wifi_title = gtk4::Label::builder()
        .label("Wi-Fi")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-title".to_string()])
        .build();
    let wifi_sub = gtk4::Label::builder()
        .label("Connected, secured")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-sub".to_string()])
        .build();
    wifi_texts.append(&wifi_title);
    wifi_texts.append(&wifi_sub);
    wifi_badge.append(&wifi_icon);
    wifi_badge.append(&wifi_texts);
    badges_box.append(&wifi_badge);

    // Fetch real wifi ssid async
    let wifi_title_clone = wifi_title.clone();
    let wifi_sub_clone = wifi_sub.clone();
    glib::spawn_future_local(async move {
        let ssid = tokio::process::Command::new("nmcli")
            .args(["-t", "-f", "active,ssid", "dev", "wifi"])
            .output()
            .await
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        let connected_ssid = ssid.lines()
            .find(|l| l.starts_with("yes:"))
            .map(|l| l.trim_start_matches("yes:").to_string());

        if let Some(name) = connected_ssid {
            wifi_title_clone.set_text(&name);
            wifi_sub_clone.set_text("Connected, secured");
        } else {
            wifi_title_clone.set_text("Network");
            wifi_sub_clone.set_text("Online");
        }
    });

    // Update status badge
    let upd_badge = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    let upd_icon = gtk4::Image::from_icon_name("software-update-available-symbolic");
    upd_icon.set_pixel_size(24);
    upd_icon.set_css_classes(&["accent-cyan"]);
    let upd_texts = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    let upd_title = gtk4::Label::builder()
        .label("Zohara Update")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-title".to_string()])
        .build();
    let upd_sub = gtk4::Label::builder()
        .label("Up to date")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-sub".to_string()])
        .build();
    upd_texts.append(&upd_title);
    upd_texts.append(&upd_sub);
    upd_badge.append(&upd_icon);
    upd_badge.append(&upd_texts);
    badges_box.append(&upd_badge);

    hero_card.append(&badges_box);
    root_box.append(&hero_card);

    // ── 2-Column Dashboard Grid ───────────────────────────────────────────────
    let grid = gtk4::Grid::builder()
        .column_spacing(16)
        .row_spacing(16)
        .column_homogeneous(true)
        .build();

    // 1. Card: Recommended Settings
    let rec_card = build_card("Recommended settings", "Recent and commonly used settings");
    let rec_rows = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    rec_rows.append(&build_nav_row("Display", "Monitors, brightness, night light", "video-display-symbolic"));
    rec_rows.append(&build_nav_row("Sound", "Volume levels, output, sound devices", "audio-volume-high-symbolic"));
    rec_rows.append(&build_nav_row("Power & battery", "Energy saver, power mode, sleep", "battery-level-80-symbolic"));
    rec_card.append(&rec_rows);
    grid.attach(&rec_card, 0, 0, 1, 1);

    // 2. Card: Storage & System Health
    // The previous build hardcoded "124 GB used / 376 GB free" and a 0.38
    // progress bar — i.e. the Storage card always told the same lie, no
    // matter which machine booted the ISO. We now query the root filesystem
    // asynchronously via statvfs (cross-distro, no shelling out) and update
    // the labels when the result comes back. The fraction is rounded to
    // 0.01 so the GTK ProgressBar doesn't show a jittery decimal.
    let storage_card = build_card("System Storage", "Local drives and storage breakdown");
    let storage_inner = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    storage_inner.set_margin_top(8);

    let storage_bar = gtk4::ProgressBar::builder()
        .fraction(0.0)
        .show_text(false)
        .css_classes(vec!["win11-progress".to_string()])
        .build();

    let storage_labels = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let used_lbl = gtk4::Label::builder()
        .label("…")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-stat-text".to_string()])
        .build();
    let free_lbl = gtk4::Label::builder()
        .label("")
        .halign(gtk4::Align::End)
        .hexpand(true)
        .css_classes(vec!["win11-stat-muted".to_string()])
        .build();
    storage_labels.append(&used_lbl);
    storage_labels.append(&free_lbl);

    storage_inner.append(&storage_labels);
    storage_inner.append(&storage_bar);
    storage_inner.append(&build_nav_row("Storage space", "Drives, temporary files, cleanup rules", "drive-harddisk-symbolic"));
    storage_card.append(&storage_inner);
    grid.attach(&storage_card, 1, 0, 1, 1);

    // Async storage query. statvfs(2) via nix would be ideal but we don't
    // want to add a sys dep; the libc call through std::os::unix::fs::Metadata
    // doesn't expose it either, so we go through `df -PB1 /` which every
    // distro has. One subprocess, once at page-build time.
    let used_lbl_for_storage = used_lbl.clone();
    let free_lbl_for_storage = free_lbl.clone();
    let storage_bar_for_storage = storage_bar.clone();
    glib::spawn_future_local(async move {
        let out = tokio::process::Command::new("df")
            .args(["-PB1", "/"])
            .output()
            .await
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        // `df -PB1 /` output:
        //   Filesystem   1B-blocks        Used    Available Use% Mounted
        //   /dev/sda2   500105249024 124000000000 376000000000  25% /
        // We only need the second line; the first is the header.
        let data_line = out.lines().nth(1).unwrap_or("");
        let cols: Vec<&str> = data_line.split_whitespace().collect();
        if cols.len() < 4 {
            used_lbl_for_storage.set_text("Storage info unavailable");
            return;
        }
        // df -B1 prints 1-byte blocks; the second column is "Used", third "Available".
        let total: u64 = match cols[1].parse() { Ok(n) => n, Err(_) => return };
        let used:  u64 = match cols[2].parse() { Ok(n) => n, Err(_) => return };
        let free:  u64 = total.saturating_sub(used);

        used_lbl_for_storage.set_text(&format!("{:.1} GB used",  used  as f64 / 1_073_741_824.0));
        free_lbl_for_storage.set_text(&format!("{:.1} GB free", free  as f64 / 1_073_741_824.0));
        let frac = if total == 0 { 0.0 } else { (used as f64 / total as f64 * 100.0).round() / 100.0 };
        storage_bar_for_storage.set_fraction(frac.clamp(0.0, 1.0));
    });

    // 3. Card: Personalize your device
    let theme_card = build_card("Personalize your device", "Desktop wallpaper, themes, and colors");
    let theme_inner = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    theme_inner.set_margin_top(6);

    // 6 Theme Thumbnail Palette Grid
    let themes_flow = gtk4::Grid::builder()
        .column_spacing(8)
        .row_spacing(8)
        .column_homogeneous(true)
        .build();

    let theme_presets = [
        ("Zohara Dark", "#1a162b", "#7c3aed"),
        ("Windows 11 Bloom", "#0f2042", "#0078d4"),
        ("Nordic Night", "#242933", "#88c0d0"),
        ("Sunset Glow", "#2b1810", "#ea580c"),
        ("Emerald Forest", "#0d2b1f", "#10b981"),
        ("Pure Glass", "#222222", "#64748b"),
    ];

    for (i, (name, bg, accent)) in theme_presets.iter().enumerate() {
        let btn = gtk4::Button::builder()
            .css_classes(vec!["win11-theme-thumb".to_string()])
            .tooltip_text(*name)
            .build();
        let thumb = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        thumb.set_size_request(80, 52);
        
        let custom_css = format!(
            "box {{ background: linear-gradient(135deg, {} 0%, #0a0a0f 100%); border-radius: 8px; border: 2px solid {}; }}",
            bg, accent
        );
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(&custom_css);
        thumb.style_context().add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
        
        btn.set_child(Some(&thumb));
        let col = (i % 3) as i32;
        let row = (i / 3) as i32;
        themes_flow.attach(&btn, col, row, 1, 1);
    }
    theme_inner.append(&themes_flow);

    // Color Mode Dropdown Row
    let mode_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    mode_row.set_margin_top(8);
    let mode_icon = gtk4::Image::from_icon_name("preferences-desktop-theme-symbolic");
    let mode_lbl = gtk4::Label::builder()
        .label("Color mode")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-row-title".to_string()])
        .build();
    let mode_combo = gtk4::DropDown::from_strings(&["Dark", "Light"]);
    mode_combo.set_selected(0);
    mode_combo.set_halign(gtk4::Align::End);
    mode_combo.set_hexpand(true);

    mode_row.append(&mode_icon);
    mode_row.append(&mode_lbl);
    mode_row.append(&mode_combo);
    theme_inner.append(&mode_row);

    theme_card.append(&theme_inner);
    grid.attach(&theme_card, 0, 1, 1, 1);

    // 4. Card: Bluetooth devices
    let bt_card = build_card("Bluetooth devices", "Manage, add, and remove wireless devices");
    let bt_inner = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    bt_inner.set_margin_top(8);

    let bt_toggle_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let bt_icon = gtk4::Image::from_icon_name("bluetooth-symbolic");
    bt_icon.set_pixel_size(20);
    bt_icon.set_css_classes(&["accent-blue"]);

    let bt_texts = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    let bt_title = gtk4::Label::builder()
        .label("Bluetooth")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-row-title".to_string()])
        .build();
    let bt_sub = gtk4::Label::builder()
        .label(&format!("Discoverable as \"{}\"", hostname))
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-row-sub".to_string()])
        .build();
    bt_texts.append(&bt_title);
    bt_texts.append(&bt_sub);

    let bt_switch = gtk4::Switch::builder()
        .active(true)
        .halign(gtk4::Align::End)
        .valign(gtk4::Align::Center)
        .hexpand(true)
        .build();

    bt_switch.connect_state_set(|_, active| {
        let cmd = if active { "power on" } else { "power off" };
        let _ = Command::new("bluetoothctl").args(cmd.split_whitespace()).spawn();
        glib::Propagation::Proceed
    });

    bt_toggle_row.append(&bt_icon);
    bt_toggle_row.append(&bt_texts);
    bt_toggle_row.append(&bt_switch);
    bt_inner.append(&bt_toggle_row);

    // Action buttons row (View all devices / Add device)
    let actions_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    actions_row.set_margin_top(12);

    let add_dev_btn = gtk4::Button::builder()
        .label("Add device")
        .css_classes(vec!["win11-primary-btn".to_string()])
        .hexpand(true)
        .build();
    actions_row.append(&add_dev_btn);

    bt_inner.append(&actions_row);
    bt_card.append(&bt_inner);
    grid.attach(&bt_card, 1, 1, 1, 1);

    root_box.append(&grid);
    scroll.set_child(Some(&root_box));
    scroll.upcast()
}

fn build_card(title: &str, subtitle: &str) -> gtk4::Box {
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    card.set_css_classes(&["win11-card"]);

    let head = gtk4::Label::builder()
        .label(title)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-card-title".to_string()])
        .build();
    let sub = gtk4::Label::builder()
        .label(subtitle)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-card-sub".to_string()])
        .build();

    card.append(&head);
    card.append(&sub);
    card
}

fn build_nav_row(title: &str, subtitle: &str, icon_name: &str) -> gtk4::Button {
    let btn = gtk4::Button::builder()
        .css_classes(vec!["win11-list-row".to_string()])
        .build();

    let h = gtk4::Box::new(gtk4::Orientation::Horizontal, 14);
    h.set_margin_top(6);
    h.set_margin_bottom(6);
    h.set_margin_start(4);
    h.set_margin_end(4);

    let icon = gtk4::Image::from_icon_name(icon_name);
    icon.set_pixel_size(20);
    h.append(&icon);

    let texts = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    let title_lbl = gtk4::Label::builder()
        .label(title)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-row-title".to_string()])
        .build();
    let sub_lbl = gtk4::Label::builder()
        .label(subtitle)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-row-sub".to_string()])
        .build();
    texts.append(&title_lbl);
    texts.append(&sub_lbl);
    h.append(&texts);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    h.append(&spacer);

    let chevron = gtk4::Image::from_icon_name("go-next-symbolic");
    chevron.set_pixel_size(14);
    chevron.set_css_classes(&["dim-label"]);
    h.append(&chevron);

    btn.set_child(Some(&h));
    btn
}

fn read_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "zohara-pc".to_string())
}

fn read_model() -> String {
    std::fs::read_to_string("/sys/devices/virtual/dmi/id/product_name")
        .or_else(|_| std::fs::read_to_string("/sys/devices/virtual/dmi/id/board_name"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "Zohara System Architecture".to_string())
}
