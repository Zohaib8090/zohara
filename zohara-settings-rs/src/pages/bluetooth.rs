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
        .label("Bluetooth & devices")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Hero "+ Add Device" Card Tile ─────────────────────────────────────────
    let hero_add_btn = gtk4::Button::builder()
        .css_classes(vec!["win11-hero-add-card".to_string()])
        .halign(gtk4::Align::Start)
        .build();

    let add_inner = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    add_inner.set_size_request(180, 140);
    add_inner.set_valign(gtk4::Align::Center);
    add_inner.set_halign(gtk4::Align::Center);

    let plus_icon = gtk4::Image::from_icon_name("list-add-symbolic");
    plus_icon.set_pixel_size(32);
    let add_lbl = gtk4::Label::builder()
        .label("Add device")
        .css_classes(vec!["win11-card-title".to_string()])
        .build();
    add_inner.append(&plus_icon);
    add_inner.append(&add_lbl);
    hero_add_btn.set_child(Some(&add_inner));
    root_box.append(&hero_add_btn);

    let view_more_lbl = gtk4::Label::builder()
        .label("View more devices")
        .halign(gtk4::Align::Center)
        .css_classes(vec!["win11-link-sub".to_string()])
        .build();
    root_box.append(&view_more_lbl);

    // ── Grouped Rows ──────────────────────────────────────────────────────────
    let rows_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    rows_box.set_css_classes(&["win11-card-group"]);

    // Main Bluetooth Switch Row
    let bt_switch_row = adw::SwitchRow::new();
    bt_switch_row.set_title("Bluetooth");
    let hostname = read_hostname();
    bt_switch_row.set_subtitle(&format!("Discoverable as \"{}\"", hostname));
    bt_switch_row.add_prefix(&gtk4::Image::from_icon_name("bluetooth-symbolic"));
    bt_switch_row.set_active(true);
    bt_switch_row.set_css_classes(&["win11-expander-row"]);

    bt_switch_row.connect_active_notify(|sw| {
        let cmd = if sw.is_active() { "power on" } else { "power off" };
        let _ = Command::new("bluetoothctl").args(cmd.split_whitespace()).spawn();
    });
    rows_box.append(&bt_switch_row);

    // Devices (Expander with Paired / Discovered Devices)
    let dev_exp = adw::ExpanderRow::new();
    dev_exp.set_title("Devices");
    dev_exp.set_subtitle("Mouse, keyboard, pen, audio, displays and docks, other devices");
    dev_exp.add_prefix(&gtk4::Image::from_icon_name("input-gaming-symbolic"));
    dev_exp.set_css_classes(&["win11-expander-row"]);

    let scan_action_row = adw::ActionRow::new();
    scan_action_row.set_title("Scan for new Bluetooth devices");
    let scan_btn = gtk4::Button::builder()
        .label("Scan")
        .css_classes(vec!["win11-secondary-btn".to_string()])
        .build();
    
    let scan_action_clone = scan_action_row.clone();
    scan_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        scan_action_clone.set_subtitle("Scanning nearby devices…");

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // Bound the scan duration since `bluetoothctl scan on` runs until stopped.
            let ok = Command::new("timeout")
                .args(["10", "bluetoothctl", "scan", "on"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            let _ = tx.send(ok);
        });

        let btn_c = btn.clone();
        let row_c = scan_action_clone.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            match rx.try_recv() {
                Ok(_) => {
                    btn_c.set_sensitive(true);
                    row_c.set_subtitle("Scan complete");
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    btn_c.set_sensitive(true);
                    row_c.set_subtitle("Scan failed");
                    glib::ControlFlow::Break
                }
            }
        });
    });
    scan_action_row.add_suffix(&scan_btn);
    dev_exp.add_row(&scan_action_row);

    // List sample/paired devices
    let dev1 = adw::ActionRow::new();
    dev1.set_title("Wireless Controller");
    dev1.set_subtitle("Paired • Audio / Input");
    let conn_btn1 = gtk4::Button::builder().label("Connect").css_classes(vec!["win11-secondary-btn".to_string()]).build();
    dev1.add_suffix(&conn_btn1);
    dev_exp.add_row(&dev1);

    rows_box.append(&dev_exp);

    // Printers & Scanners
    let print_row = build_action_row("Printers & scanners", "Preferences, print queues, scan tools", "printer-symbolic");
    rows_box.append(&print_row);

    // Mobile Devices
    let mob_row = build_action_row("Mobile devices", "Instantly access your mobile devices from your PC", "phone-symbolic");
    rows_box.append(&mob_row);

    // Cameras
    let cam_row = build_action_row("Cameras", "Connected cameras, default image settings", "camera-web-symbolic");
    rows_box.append(&cam_row);

    // Mouse (with in-app expander)
    let mouse_exp = adw::ExpanderRow::new();
    mouse_exp.set_title("Mouse");
    mouse_exp.set_subtitle("Buttons, mouse pointer speed, scrolling");
    mouse_exp.add_prefix(&gtk4::Image::from_icon_name("input-mouse-symbolic"));
    mouse_exp.set_css_classes(&["win11-expander-row"]);

    let speed_row = adw::ActionRow::new();
    speed_row.set_title("Mouse pointer speed");
    let speed_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 1.0, 20.0, 1.0);
    speed_scale.set_value(10.0);
    speed_scale.set_size_request(180, -1);
    speed_row.add_suffix(&speed_scale);
    mouse_exp.add_row(&speed_row);

    let nat_scroll = adw::SwitchRow::new();
    nat_scroll.set_title("Natural scrolling");
    nat_scroll.set_subtitle("Scrolling moves the content, not the scrollbar");
    mouse_exp.add_row(&nat_scroll);

    rows_box.append(&mouse_exp);

    // Keyboard (with in-app expander)
    let kbd_exp = adw::ExpanderRow::new();
    kbd_exp.set_title("Keyboard");
    kbd_exp.set_subtitle("Character repeat, layout shortcuts, hotkeys");
    kbd_exp.add_prefix(&gtk4::Image::from_icon_name("input-keyboard-symbolic"));
    kbd_exp.set_css_classes(&["win11-expander-row"]);

    let repeat_row = adw::ActionRow::new();
    repeat_row.set_title("Repeat delay");
    let rep_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 100.0, 1000.0, 50.0);
    rep_scale.set_value(400.0);
    rep_scale.set_size_request(180, -1);
    repeat_row.add_suffix(&rep_scale);
    kbd_exp.add_row(&repeat_row);

    rows_box.append(&kbd_exp);

    // Touchpad
    let touch_row = build_action_row("Touchpad", "Taps, gestures, scrolling, zooming", "input-touchpad-symbolic");
    rows_box.append(&touch_row);

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
