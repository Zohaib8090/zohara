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
        .label("Gaming")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Grouped Rows ──────────────────────────────────────────────────────────
    let rows_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    rows_box.set_css_classes(&["win11-card-group"]);

    // 1. Game Bar
    let bar_row = build_action_row("Game Bar", "Controller and keyboard shortcuts, HUD widgets", "input-gaming-symbolic");
    rows_box.append(&bar_row);

    // 2. Captures
    let cap_row = build_action_row("Captures", "Save location, screenshots, recording preferences", "camera-video-symbolic");
    rows_box.append(&cap_row);

    // 3. Game Mode (with In-App GameMode Daemon Toggle)
    let gm_exp = adw::ExpanderRow::new();
    gm_exp.set_title("Game Mode");
    gm_exp.set_subtitle("Optimize your PC for play (Feral GameMode & CPU governor)");
    gm_exp.add_prefix(&gtk4::Image::from_icon_name("applications-games-symbolic"));
    gm_exp.set_css_classes(&["win11-expander-row"]);

    let gm_sw = adw::SwitchRow::new();
    gm_sw.set_title("Game Mode");
    gm_sw.set_subtitle("Turn on Game Mode to prevent background tasks from slowing down gameplay");
    gm_sw.set_active(true);
    gm_exp.add_row(&gm_sw);

    let hud_sw = adw::SwitchRow::new();
    hud_sw.set_title("MangoHud Overlay");
    hud_sw.set_subtitle("Display FPS, GPU/CPU temperatures, and frame times during games");
    hud_sw.set_active(false);
    gm_exp.add_row(&hud_sw);

    rows_box.append(&gm_exp);

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
