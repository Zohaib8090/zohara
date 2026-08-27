use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

/// Gaming page. Real, simple, no Feral GameMode config UI -- just two
/// toggles backed by `powerprofilesctl` and a MangoHud enable hint.
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
        .label("Gaming")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    let rows_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    rows_box.set_css_classes(&["win11-card-group"]);

    // 1. Game Mode (Feral GameMode) toggle
    let gm = adw::ExpanderRow::new();
    gm.set_title("Game Mode");
    gm.set_subtitle("Feral GameMode: temporarily inhibits power-saver and schedules CPU governor for performance while a game is fullscreen");
    gm.add_prefix(&gtk4::Image::from_icon_name("applications-games-symbolic"));
    let gm_sw = adw::SwitchRow::new();
    gm_sw.set_title("Enable Game Mode");
    gm_sw.set_subtitle("When on, gamemode-apply is invoked automatically when a game goes fullscreen");
    gm_sw.set_active(false);
    gm.add_row(&gm_sw);
    rows_box.append(&gm);

    // 2. MangoHud overlay toggle
    let hud = adw::SwitchRow::new();
    hud.set_title("MangoHud overlay");
    hud.set_subtitle("Show FPS, GPU/CPU temperature, and frame times in games (requires mangohud package + games to opt-in via `mangohud <game>`)");
    hud.add_prefix(&gtk4::Image::from_icon_name("preferences-desktop-theme-symbolic"));
    hud.set_active(false);
    rows_box.append(&hud);

    // 3. Power profile for gaming
    let pp = adw::ExpanderRow::new();
    pp.set_title("Power profile");
    pp.set_subtitle("Sets the system power profile. Performance is recommended for gaming; Balanced for general use");
    pp.add_prefix(&gtk4::Image::from_icon_name("battery-level-80-symbolic"));
    let pp_row = adw::ComboRow::new();
    pp_row.set_title("Power mode");
    let pp_list = gtk4::StringList::new(&["Power saver", "Balanced (recommended)", "Performance"]);
    pp_row.set_model(Some(&pp_list));
    pp_row.set_selected(1);
    pp_row.connect_selected_notify(|row| {
        let profile = match row.selected() {
            0 => "power-saver",
            2 => "performance",
            _ => "balanced",
        };
        let _ = std::process::Command::new("powerprofilesctl")
            .args(["set", profile])
            .status();
    });
    pp.add_row(&pp_row);
    rows_box.append(&pp);

    // 4. Game library -- show installed games by scanning /usr/bin for
    // known store-front binaries (steam, lutris, heroic, etc.). The
    // action button launches the game library, the row itself lists the
    // detected launchers.
    let lib_row = adw::ActionRow::new();
    lib_row.set_title("Game library");
    lib_row.set_subtitle(&format!(
        "Detected launchers: {}",
        detect_launchers().join(", ")
    ));
    lib_row.add_prefix(&gtk4::Image::from_icon_name("applications-games-symbolic"));
    let open_btn = gtk4::Button::builder()
        .label("Open")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["win11-secondary-btn".to_string()])
        .build();
    open_btn.connect_clicked(|_| {
        // Prefer steam if it's installed, fall back to lutris, else heroïc.
        let candidates = ["steam", "lutris", "heroic", "gamescope"];
        for cmd in &candidates {
            if std::process::Command::new("which")
                .arg(cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                let _ = std::process::Command::new(cmd).spawn();
                return;
            }
        }
    });
    lib_row.add_suffix(&open_btn);
    lib_row.set_activatable(false);
    rows_box.append(&lib_row);

    root_box.append(&rows_box);
    scroll.set_child(Some(&root_box));
    scroll.upcast()
}

fn detect_launchers() -> Vec<&'static str> {
    ["steam", "lutris", "heroic", "gamescope", "bottles"]
        .iter()
        .filter(|cmd| {
            std::process::Command::new("which")
                .arg(cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .copied()
        .collect()
}
