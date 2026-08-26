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
        .label("Time & language")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Hero Live Clock Banner ────────────────────────────────────────────────
    let hero_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 20);
    hero_card.set_css_classes(&["win11-hero-card"]);
    hero_card.set_margin_bottom(4);

    let clock_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    clock_box.set_valign(gtk4::Align::Center);

    let time_lbl = gtk4::Label::builder()
        .label("12:37 AM")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-hero-clock".to_string()])
        .build();

    let date_lbl = gtk4::Label::builder()
        .label("Friday, August 21, 2026")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-sub".to_string()])
        .build();

    clock_box.append(&time_lbl);
    clock_box.append(&date_lbl);
    hero_card.append(&clock_box);

    // Dynamic Clock update
    let time_clone = time_lbl.clone();
    let date_clone = date_lbl.clone();
    glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
        if let Ok(now) = glib::DateTime::now_local() {
            time_clone.set_text(&now.format("%l:%M %p").unwrap_or_default());
            date_clone.set_text(&now.format("%A, %B %e, %Y").unwrap_or_default());
        }
        glib::ControlFlow::Continue
    });

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    hero_card.append(&spacer);

    // Right status badges (Time zone + Region)
    let badges_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 28);
    badges_box.set_valign(gtk4::Align::Center);

    // Timezone badge
    let tz_badge = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    let tz_icon = gtk4::Image::from_icon_name("preferences-system-time-symbolic");
    tz_icon.set_pixel_size(22);
    let tz_texts = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    let tz_title = gtk4::Label::builder()
        .label("Time zone")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-title".to_string()])
        .build();
    let tz_sub = gtk4::Label::builder()
        .label(read_timezone().as_str())
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-sub".to_string()])
        .build();
    tz_texts.append(&tz_title);
    tz_texts.append(&tz_sub);
    tz_badge.append(&tz_icon);
    tz_badge.append(&tz_texts);
    badges_box.append(&tz_badge);

    // Region badge
    let reg_badge = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    let reg_icon = gtk4::Image::from_icon_name("preferences-desktop-locale-symbolic");
    reg_icon.set_pixel_size(22);
    let reg_texts = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    let reg_title = gtk4::Label::builder()
        .label("Region")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-title".to_string()])
        .build();
    let reg_sub = gtk4::Label::builder()
        .label("English (US)")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-sub".to_string()])
        .build();
    reg_texts.append(&reg_title);
    reg_texts.append(&reg_sub);
    reg_badge.append(&reg_icon);
    reg_badge.append(&reg_texts);
    badges_box.append(&reg_badge);

    hero_card.append(&badges_box);
    root_box.append(&hero_card);

    // ── Grouped Rows ──────────────────────────────────────────────────────────
    let rows_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    rows_box.set_css_classes(&["win11-card-group"]);

    // 1. Date & Time (Expander with in-app NTP & Timezone dropdown)
    let time_exp = adw::ExpanderRow::new();
    time_exp.set_title("Date & time");
    time_exp.set_subtitle("Time zones, automatic clock settings, calendar display");
    time_exp.add_prefix(&gtk4::Image::from_icon_name("preferences-system-time-symbolic"));
    time_exp.set_css_classes(&["win11-expander-row"]);

    let ntp_row = adw::SwitchRow::new();
    ntp_row.set_title("Set time automatically");
    ntp_row.set_subtitle("Synchronize clock with network time servers (NTP)");
    ntp_row.set_active(true);
    ntp_row.connect_active_notify(|sw| {
        let active = if sw.is_active() { "true" } else { "false" };
        let _ = Command::new("timedatectl").args(["set-ntp", active]).spawn();
    });
    time_exp.add_row(&ntp_row);

    let tz_row = adw::ComboRow::new();
    tz_row.set_title("Time zone");
    let tz_model = gtk4::StringList::new(&[
        "UTC (Coordinated Universal Time)",
        "Asia/Karachi (UTC+05:00)",
        "America/New_York (UTC-05:00)",
        "Europe/London (UTC+00:00)",
        "Asia/Dubai (UTC+04:00)",
        "Asia/Tokyo (UTC+09:00)",
    ]);
    tz_row.set_model(Some(&tz_model));
    tz_row.set_selected(1);
    tz_row.connect_selected_notify(|row| {
        let tz = match row.selected() {
            1 => "Asia/Karachi",
            2 => "America/New_York",
            3 => "Europe/London",
            4 => "Asia/Dubai",
            5 => "Asia/Tokyo",
            _ => "UTC",
        };
        let _ = Command::new("timedatectl").args(["set-timezone", tz]).spawn();
    });
    time_exp.add_row(&tz_row);

    rows_box.append(&time_exp);

    // 2. Language & Region
    let lang_row = build_action_row("Language & region", "Zohara OS display language, preferred languages, regional formats", "preferences-desktop-locale-symbolic");
    rows_box.append(&lang_row);

    // 3. Typing
    let type_row = build_action_row("Typing", "Touch keyboard, text suggestions, preferences", "input-keyboard-symbolic");
    rows_box.append(&type_row);

    // 4. Speech
    let speech_row = build_action_row("Speech", "Speech language, speech recognition microphone setup, voices", "audio-input-microphone-symbolic");
    rows_box.append(&speech_row);

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

fn read_timezone() -> String {
    std::fs::read_link("/etc/localtime")
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .and_then(|s| s.split("zoneinfo/").nth(1).map(|s| s.to_string()))
        .unwrap_or_else(|| "UTC".to_string())
}
