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
        .label("Privacy & security")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Section 1: Security ───────────────────────────────────────────────────
    let sec_lbl = gtk4::Label::builder()
        .label("Security")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&sec_lbl);

    let sec_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    sec_box.set_css_classes(&["win11-card-group"]);

    // Windows/Zohara Security (Expander with UFW Firewall Status)
    let ufw_exp = adw::ExpanderRow::new();
    ufw_exp.set_title("Zohara Security");
    ufw_exp.set_subtitle("Antivirus, browser, firewall, and network protection for your device");
    ufw_exp.add_prefix(&gtk4::Image::from_icon_name("security-high-symbolic"));
    ufw_exp.set_css_classes(&["win11-expander-row"]);

    let ufw_sw = adw::SwitchRow::new();
    ufw_sw.set_title("UFW System Firewall");
    ufw_sw.set_subtitle("Block unauthorized incoming network connections");
    ufw_sw.set_active(true);
    ufw_sw.connect_active_notify(|sw| {
        let cmd = if sw.is_active() { "enable" } else { "disable" };
        let _ = Command::new("pkexec").args(["ufw", cmd]).spawn();
    });
    ufw_exp.add_row(&ufw_sw);
    sec_box.append(&ufw_exp);

    sec_box.append(&build_action_row("Find my device", "Track your device if you think you've lost it", "find-location-symbolic"));
    root_box.append(&sec_box);

    // ── Section 2: Windows permissions ────────────────────────────────────────
    let perm_lbl = gtk4::Label::builder()
        .label("System permissions")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&perm_lbl);

    let perm_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    perm_box.set_css_classes(&["win11-card-group"]);

    perm_box.append(&build_action_row("Recommendations & offers", "Advertising ID, personalized suggestions, productivity tools", "dialog-information-symbolic"));
    perm_box.append(&build_action_row("Speech", "Speech recognition for dictation and voice interactions", "audio-input-microphone-symbolic"));
    perm_box.append(&build_action_row("Inking & typing personalization", "Custom dictionary, word predictions", "format-text-underline-symbolic"));
    perm_box.append(&build_action_row("Diagnostics & feedback", "Diagnostic data, crash reporting, telemetry control", "utilities-system-monitor-symbolic"));
    perm_box.append(&build_action_row("Search", "Search history, search apps, file indexing", "system-search-symbolic"));
    root_box.append(&perm_box);

    // ── Section 3: App permissions ────────────────────────────────────────────
    let app_perm_lbl = gtk4::Label::builder()
        .label("App permissions")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&app_perm_lbl);

    let app_perm_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    app_perm_box.set_css_classes(&["win11-card-group"]);

    // Location Switch
    let loc_sw = adw::SwitchRow::new();
    loc_sw.set_title("Location");
    loc_sw.set_subtitle("Allow apps to access your physical location");
    loc_sw.add_prefix(&gtk4::Image::from_icon_name("find-location-symbolic"));
    loc_sw.set_active(true);
    loc_sw.set_css_classes(&["win11-expander-row"]);
    app_perm_box.append(&loc_sw);

    // Camera Switch
    let cam_sw = adw::SwitchRow::new();
    cam_sw.set_title("Camera");
    cam_sw.set_subtitle("Allow apps to access your camera hardware");
    cam_sw.add_prefix(&gtk4::Image::from_icon_name("camera-web-symbolic"));
    cam_sw.set_active(true);
    cam_sw.set_css_classes(&["win11-expander-row"]);
    app_perm_box.append(&cam_sw);

    // Microphone Switch
    let mic_sw = adw::SwitchRow::new();
    mic_sw.set_title("Microphone");
    mic_sw.set_subtitle("Allow apps to access your microphone");
    mic_sw.add_prefix(&gtk4::Image::from_icon_name("audio-input-microphone-symbolic"));
    mic_sw.set_active(true);
    mic_sw.set_css_classes(&["win11-expander-row"]);
    app_perm_box.append(&mic_sw);

    root_box.append(&app_perm_box);

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
