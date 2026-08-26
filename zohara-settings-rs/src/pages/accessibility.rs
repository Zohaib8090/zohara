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
        .label("Accessibility")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Section 1: Vision ─────────────────────────────────────────────────────
    let vision_lbl = gtk4::Label::builder()
        .label("Vision")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&vision_lbl);

    let vision_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    vision_box.set_css_classes(&["win11-card-group"]);

    // 1. Text size (with In-App Font Scale Slider)
    let text_exp = adw::ExpanderRow::new();
    text_exp.set_title("Text size");
    text_exp.set_subtitle("Text size that appears throughout Zohara OS and your apps");
    text_exp.add_prefix(&gtk4::Image::from_icon_name("preferences-desktop-font-symbolic"));
    text_exp.set_css_classes(&["win11-expander-row"]);

    let scale_row = adw::ActionRow::new();
    scale_row.set_title("Text scaling");
    let font_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.8, 1.6, 0.05);
    font_scale.set_value(1.0);
    font_scale.set_size_request(200, -1);
    scale_row.add_suffix(&font_scale);
    text_exp.add_row(&scale_row);
    vision_box.append(&text_exp);

    // 2. Visual effects
    let fx_exp = adw::ExpanderRow::new();
    fx_exp.set_title("Visual effects");
    fx_exp.set_subtitle("Scroll bars, transparency, animations, notification timeout");
    fx_exp.add_prefix(&gtk4::Image::from_icon_name("preferences-desktop-theme-symbolic"));
    fx_exp.set_css_classes(&["win11-expander-row"]);

    let anim_sw = adw::SwitchRow::new();
    anim_sw.set_title("Animation effects");
    anim_sw.set_active(true);
    fx_exp.add_row(&anim_sw);

    let trans_sw = adw::SwitchRow::new();
    trans_sw.set_title("Transparency effects");
    trans_sw.set_active(true);
    fx_exp.add_row(&trans_sw);
    vision_box.append(&fx_exp);

    // 3. Mouse pointer and touch
    vision_box.append(&build_action_row("Mouse pointer and touch", "Mouse pointer color, size", "input-mouse-symbolic"));

    // 4. Text cursor
    vision_box.append(&build_action_row("Text cursor", "Appearance and thickness, text cursor indicator", "format-text-underline-symbolic"));

    // 5. Magnifier
    vision_box.append(&build_action_row("Magnifier", "Magnifier reading, zoom increment", "zoom-in-symbolic"));

    // 6. Color filters
    vision_box.append(&build_action_row("Color filters", "Colorblindness filters, grayscale, inverted", "preferences-color-symbolic"));

    // 7. Screen tint
    vision_box.append(&build_action_row("Screen tint", "Add a subtle color overlay to make your screen more comfortable", "weather-clear-night-symbolic"));

    // 8. Contrast themes
    vision_box.append(&build_action_row("Contrast themes", "Color themes for low vision, light sensitivity", "display-projector-symbolic"));

    // 9. Narrator
    vision_box.append(&build_action_row("Narrator", "Voice, verbosity, keyboard, screen reading", "audio-speakers-symbolic"));

    root_box.append(&vision_box);

    // ── Section 2: Hearing ────────────────────────────────────────────────────
    let hearing_lbl = gtk4::Label::builder()
        .label("Hearing")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&hearing_lbl);

    let hearing_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    hearing_box.set_css_classes(&["win11-card-group"]);

    // Mono Audio Switch
    let mono_sw = adw::SwitchRow::new();
    mono_sw.set_title("Mono audio");
    mono_sw.set_subtitle("Combine left and right audio channels into one");
    mono_sw.add_prefix(&gtk4::Image::from_icon_name("audio-volume-high-symbolic"));
    mono_sw.set_css_classes(&["win11-expander-row"]);
    hearing_box.append(&mono_sw);

    hearing_box.append(&build_action_row("Hearing devices", "Manage and pair Bluetooth hearing aids", "audio-headphones-symbolic"));
    root_box.append(&hearing_box);

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
