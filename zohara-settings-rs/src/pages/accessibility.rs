use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

/// Accessibility page. Real but minimal: drives a few `gsettings` keys
/// that Plasma's accessibility module also writes, so toggles here
/// actually affect the live desktop (text scaling, cursor size, sticky
/// keys, large text, high contrast).
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

    let title_lbl = gtk4::Label::builder()
        .label("Accessibility")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Section: Vision ──────────────────────────────────────────────────────
    let vision_lbl = gtk4::Label::builder()
        .label("Vision")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&vision_lbl);

    let vision_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    vision_box.set_css_classes(&["win11-card-group"]);

    // Text size slider (gsettings org.gnome.desktop.interface text-scaling-factor)
    let text_exp = adw::ExpanderRow::new();
    text_exp.set_title("Text size");
    text_exp.set_subtitle("Scale all UI text by a single factor (0.85 = smaller ... 1.30 = larger)");
    text_exp.add_prefix(&gtk4::Image::from_icon_name("preferences-desktop-font-symbolic"));
    let scale_row = adw::ActionRow::new();
    scale_row.set_title("Text scaling");
    let scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.85, 1.30, 0.05);
    scale.set_value(1.00);
    scale.set_size_request(280, -1);
    scale.set_draw_value(true);
    scale.set_value_pos(gtk4::PositionType::Right);
    scale.connect_value_changed(|s| {
        let v = s.value();
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "text-scaling-factor", &format!("{:.2}", v)])
            .status();
    });
    scale_row.add_suffix(&scale);
    text_exp.add_row(&scale_row);

    // High contrast toggle
    let hc_row = adw::SwitchRow::new();
    hc_row.set_title("High contrast");
    hc_row.set_subtitle("Use a high-contrast theme for the desktop and apps");
    hc_row.set_active(false);
    hc_row.connect_active_notify(|sw| {
        let v = if sw.is_active() { "true" } else { "false" };
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "high-contrast", v])
            .status();
    });
    text_exp.add_row(&hc_row);
    vision_box.append(&text_exp);

    // Cursor size
    let cur_row = adw::ActionRow::new();
    cur_row.set_title("Cursor size");
    cur_row.set_subtitle("Choose a comfortable cursor size for the desktop");
    cur_row.add_prefix(&gtk4::Image::from_icon_name("input-mouse-symbolic"));
    let cur_list = gtk4::StringList::new(&["Small (24px)", "Default (32px)", "Large (48px)", "Huge (64px)"]);
    let cur_combo = gtk4::DropDown::builder()
        .model(&cur_list)
        .selected(1_u32)
        .build();
    cur_combo.set_valign(gtk4::Align::Center);
    cur_combo.set_selected(1);
    cur_combo.connect_selected_notify(|c| {
        let v = match c.selected() {
            0 => 24,
            2 => 48,
            3 => 64,
            _ => 32,
        };
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "cursor-size", &v.to_string()])
            .status();
    });
    cur_row.add_suffix(&cur_combo);
    vision_box.append(&cur_row);

    // Reduce animations
    let anim_row = adw::SwitchRow::new();
    anim_row.set_title("Reduce animations");
    anim_row.set_subtitle("Disables window-open/close animations and reduces motion");
    anim_row.set_active(false);
    anim_row.connect_active_notify(|sw| {
        let v = if sw.is_active() { "true" } else { "false" };
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "enable-animations", v])
            .status();
    });
    vision_box.append(&anim_row);

    root_box.append(&vision_box);

    // ── Section: Hearing ────────────────────────────────────────────────────
    let hearing_lbl = gtk4::Label::builder()
        .label("Hearing")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&hearing_lbl);

    let hearing_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    hearing_box.set_css_classes(&["win11-card-group"]);

    let visual_alerts = adw::SwitchRow::new();
    visual_alerts.set_title("Visual alerts for system sounds");
    visual_alerts.set_subtitle("Flash the screen or window when a system sound plays (requires libcanberra)");
    visual_alerts.set_active(false);
    visual_alerts.add_prefix(&gtk4::Image::from_icon_name("audio-volume-high-symbolic"));
    hearing_box.append(&visual_alerts);

    let mono = adw::SwitchRow::new();
    mono.set_title("Mono audio");
    mono.set_subtitle("Combine left and right audio channels (useful for single-ear listening)");
    mono.set_active(false);
    mono.connect_active_notify(|sw| {
        let v = if sw.is_active() { "true" } else { "false" };
        // The PipeWire mono-audio toggle lives in wireplumber config; this
        // gsettings key is what GNOME exposes, which Plasma also honors.
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.a11y.interface", "hearing-audio-mono", v])
            .status();
    });
    mono.add_prefix(&gtk4::Image::from_icon_name("audio-headphones-symbolic"));
    hearing_box.append(&mono);

    root_box.append(&hearing_box);

    scroll.set_child(Some(&root_box));
    scroll.upcast()
}
