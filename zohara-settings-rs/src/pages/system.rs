use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::process::Command;

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
        .label("System")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Hero Device Card ──────────────────────────────────────────────────────
    let hero_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 18);
    hero_card.set_css_classes(&["win11-hero-card"]);
    hero_card.set_margin_bottom(4);

    let thumb_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    thumb_box.set_css_classes(&["win11-device-thumb"]);
    let thumb_icon = gtk4::Image::from_icon_name("computer-symbolic");
    thumb_icon.set_pixel_size(42);
    thumb_icon.set_valign(gtk4::Align::Center);
    thumb_icon.set_halign(gtk4::Align::Center);
    thumb_box.append(&thumb_icon);
    hero_card.append(&thumb_box);

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

    info_box.append(&host_lbl);
    info_box.append(&model_lbl);
    info_box.append(&rename_btn);
    hero_card.append(&info_box);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    hero_card.append(&spacer);

    // Status Badges on the right
    let badges_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
    badges_box.set_valign(gtk4::Align::Center);

    // Storage badge
    let stor_badge = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let stor_icon = gtk4::Image::from_icon_name("drive-harddisk-symbolic");
    stor_icon.set_pixel_size(20);
    stor_icon.set_css_classes(&["accent-blue"]);
    let stor_lbl = gtk4::Label::builder()
        .label("Storage\nHealthy")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-sub".to_string()])
        .build();
    stor_badge.append(&stor_icon);
    stor_badge.append(&stor_lbl);
    badges_box.append(&stor_badge);

    // Update badge
    let upd_badge = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let upd_icon = gtk4::Image::from_icon_name("software-update-available-symbolic");
    upd_icon.set_pixel_size(20);
    upd_icon.set_css_classes(&["accent-cyan"]);
    let upd_lbl = gtk4::Label::builder()
        .label("Zohara Update\nUp to date")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-sub".to_string()])
        .build();
    upd_badge.append(&upd_icon);
    upd_badge.append(&upd_lbl);
    badges_box.append(&upd_badge);

    hero_card.append(&badges_box);
    root_box.append(&hero_card);

    // ── Windows 11 Grouped Rows (In-App standalone controls) ──────────────────
    let rows_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    rows_box.set_css_classes(&["win11-card-group"]);

    // 1. Display (with in-app expander)
    let display_exp = adw::ExpanderRow::new();
    display_exp.set_title("Display");
    display_exp.set_subtitle("Monitors, brightness, night light, display profile");
    display_exp.add_prefix(&gtk4::Image::from_icon_name("video-display-symbolic"));
    display_exp.set_css_classes(&["win11-expander-row"]);

    // Brightness Slider
    let bright_row = adw::ActionRow::new();
    bright_row.set_title("Brightness");
    let bright_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 10.0, 100.0, 1.0);
    bright_scale.set_value(80.0);
    bright_scale.set_size_request(200, -1);
    bright_scale.connect_value_changed(|scale| {
        let val = scale.value() as u32;
        let _ = Command::new("brightnessctl").arg("set").arg(format!("{}%", val)).spawn();
    });
    bright_row.add_suffix(&bright_scale);
    display_exp.add_row(&bright_row);

    // Night Light Switch
    let night_row = adw::SwitchRow::new();
    night_row.set_title("Night light");
    night_row.set_subtitle("Use warmer colors to help you sleep");
    night_row.connect_active_notify(|sw| {
        let active = sw.is_active();
        if active {
            let _ = Command::new("gammastep").args(["-O", "4500"]).spawn();
        } else {
            let _ = Command::new("gammastep").arg("-x").spawn();
        }
    });
    display_exp.add_row(&night_row);

    // Display Scale Dropdown
    let scale_row = adw::ComboRow::new();
    scale_row.set_title("Scale");
    scale_row.set_subtitle("Change the size of text, apps, and other items");
    let scale_model = gtk4::StringList::new(&["100% (Recommended)", "125%", "150%", "175%", "200%"]);
    scale_row.set_model(Some(&scale_model));
    scale_row.set_selected(0);
    display_exp.add_row(&scale_row);

    rows_box.append(&display_exp);

    // 2. Sound (with in-app expander)
    let sound_exp = adw::ExpanderRow::new();
    sound_exp.set_title("Sound");
    sound_exp.set_subtitle("Volume levels, output, input, sound devices");
    sound_exp.add_prefix(&gtk4::Image::from_icon_name("audio-volume-high-symbolic"));
    sound_exp.set_css_classes(&["win11-expander-row"]);

    // Master Volume Slider
    let vol_row = adw::ActionRow::new();
    vol_row.set_title("Master volume");
    let vol_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 100.0, 1.0);
    vol_scale.set_value(65.0);
    vol_scale.set_size_request(200, -1);
    vol_scale.connect_value_changed(|scale| {
        let val = (scale.value() / 100.0) as f32;
        let _ = Command::new("wpctl").args(["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{:.2}", val)]).spawn();
    });
    vol_row.add_suffix(&vol_scale);
    sound_exp.add_row(&vol_row);

    // Mute Switch
    let mute_row = adw::SwitchRow::new();
    mute_row.set_title("Mute all audio");
    mute_row.connect_active_notify(|sw| {
        let active = sw.is_active();
        let _ = Command::new("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SINK@", if active { "1" } else { "0" }]).spawn();
    });
    sound_exp.add_row(&mute_row);

    // Sound Output device selector
    let out_row = adw::ComboRow::new();
    out_row.set_title("Output device");
    let out_model = gtk4::StringList::new(&["Default Audio Sink (Speakers/Headphones)", "HDMI / DisplayPort Audio", "Bluetooth Audio"]);
    out_row.set_model(Some(&out_model));
    out_row.set_selected(0);
    sound_exp.add_row(&out_row);

    rows_box.append(&sound_exp);

    // 3. Notifications
    let notif_row = adw::SwitchRow::new();
    notif_row.set_title("Notifications");
    notif_row.set_subtitle("Alerts from apps and system, do not disturb");
    notif_row.add_prefix(&gtk4::Image::from_icon_name("preferences-system-notifications-symbolic"));
    notif_row.set_active(true);
    notif_row.set_css_classes(&["win11-expander-row"]);
    rows_box.append(&notif_row);

    // 4. Focus
    let focus_row = build_action_row("Focus", "Reduce distractions and mute alerts", "weather-clear-night-symbolic");
    rows_box.append(&focus_row);

    // 5. Power & battery (with in-app expander)
    let power_exp = adw::ExpanderRow::new();
    power_exp.set_title("Power & battery");
    power_exp.set_subtitle("Screen and sleep, battery usage, power mode, energy saver");
    power_exp.add_prefix(&gtk4::Image::from_icon_name("battery-level-80-symbolic"));
    power_exp.set_css_classes(&["win11-expander-row"]);

    let pwr_mode_row = adw::ComboRow::new();
    pwr_mode_row.set_title("Power mode");
    pwr_mode_row.set_subtitle("Choose a mode to optimize performance or battery life");
    let pwr_model = gtk4::StringList::new(&["Best power efficiency", "Balanced (Recommended)", "Best performance"]);
    pwr_mode_row.set_model(Some(&pwr_model));
    pwr_mode_row.set_selected(1);
    pwr_mode_row.connect_selected_notify(|row| {
        let profile = match row.selected() {
            0 => "power-saver",
            2 => "performance",
            _ => "balanced",
        };
        let _ = Command::new("powerprofilesctl").args(["set", profile]).spawn();
    });
    power_exp.add_row(&pwr_mode_row);

    let sleep_row = adw::ComboRow::new();
    sleep_row.set_title("Screen turns off after");
    let sleep_model = gtk4::StringList::new(&["5 minutes", "10 minutes", "15 minutes", "30 minutes", "Never"]);
    sleep_row.set_model(Some(&sleep_model));
    sleep_row.set_selected(2);
    power_exp.add_row(&sleep_row);

    rows_box.append(&power_exp);

    // 6. Storage (with in-app expander)
    let stor_exp = adw::ExpanderRow::new();
    stor_exp.set_title("Storage");
    stor_exp.set_subtitle("Storage space, drives, configuration rules");
    stor_exp.add_prefix(&gtk4::Image::from_icon_name("drive-harddisk-symbolic"));
    stor_exp.set_css_classes(&["win11-expander-row"]);

    let clean_row = adw::ActionRow::new();
    clean_row.set_title("Temporary files");
    clean_row.set_subtitle("Free up disk space by removing caches and junk files");
    let clean_btn = gtk4::Button::builder()
        .label("Clean up")
        .css_classes(vec!["win11-secondary-btn".to_string()])
        .build();
    let clean_row_clone = clean_row.clone();
    clean_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        let rm_ok = Command::new("sh")
            .args(["-c", "rm -rf /tmp/* 2>/dev/null"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        let _ = Command::new("paccache").args(["-r", "-k1"]).spawn();
        btn.set_sensitive(true);
        if rm_ok {
            clean_row_clone.set_subtitle("Temporary files cleaned successfully!");
        } else {
            clean_row_clone.set_subtitle("Failed to clean temporary files.");
        }
    });
    clean_row.add_suffix(&clean_btn);
    stor_exp.add_row(&clean_row);

    rows_box.append(&stor_exp);

    // 7. Nearby sharing
    let nearby_row = build_action_row("Nearby sharing", "Discoverability, received files location", "network-workgroup-symbolic");
    rows_box.append(&nearby_row);

    // 8. Multitasking
    let multi_row = build_action_row("Multitasking", "Snap windows, desktops, task switching", "preferences-desktop-display-symbolic");
    rows_box.append(&multi_row);

    // 9. About (in-app specs display)
    let about_exp = adw::ExpanderRow::new();
    about_exp.set_title("About");
    about_exp.set_subtitle("Device specifications, rename PC, Windows specifications");
    about_exp.add_prefix(&gtk4::Image::from_icon_name("dialog-information-symbolic"));
    about_exp.set_css_classes(&["win11-expander-row"]);

    let os_ver = read_os_version();
    let kernel = read_kernel();
    let cpu = read_cpu();
    let ram = read_ram();

    for (t, v) in &[
        ("Edition", "Zohara OS 2026.08 (Nexus)"),
        ("OS Build", "Rolling Release (Zen Kernel)"),
        ("Kernel", kernel.as_str()),
        ("Processor", cpu.as_str()),
        ("Installed RAM", ram.as_str()),
    ] {
        let r = adw::ActionRow::new();
        r.set_title(t);
        r.set_subtitle(v);
        about_exp.add_row(&r);
    }

    let copy_spec_row = adw::ActionRow::new();
    copy_spec_row.set_title("Copy device specifications");
    copy_spec_row.set_activatable(true);
    copy_spec_row.add_suffix(&gtk4::Image::from_icon_name("edit-copy-symbolic"));
    let specs_copy = format!("OS: {}\nKernel: {}\nCPU: {}\nRAM: {}", os_ver, kernel, cpu, ram);
    copy_spec_row.connect_activated(move |row| {
        let display = row.display();
        display.clipboard().set_text(&specs_copy);
    });
    about_exp.add_row(&copy_spec_row);

    rows_box.append(&about_exp);

    root_box.append(&rows_box);
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

fn read_os_version() -> String {
    std::fs::read_to_string("/etc/os-release")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("PRETTY_NAME="))
        .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        .unwrap_or_else(|| "Zohara OS".to_string())
}

fn read_kernel() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "linux-zen".to_string())
}

fn read_cpu() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.splitn(2, ':').nth(1))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "x86_64 Processor".to_string())
}

fn read_ram() -> String {
    std::fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("MemTotal:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|kb| kb.parse::<f64>().ok())
        .map(|kb| format!("{:.1} GB", kb / 1_048_576.0))
        .unwrap_or_else(|| "16.0 GB".to_string())
}
