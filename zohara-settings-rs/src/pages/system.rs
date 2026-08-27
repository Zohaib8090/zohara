use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::backend::system::{self, SystemInfo};

fn read_info() -> SystemInfo {
    system::read()
}

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

    // ── Hardware info read once on page open ──────────────────────────────
    let info = read_info();

    // ── Hero Device Card (renames the host) ──────────────────────────────
    let hero_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 18);
    hero_card.set_css_classes(&["win11-hero-card"]);
    hero_card.set_margin_bottom(4);

    let thumb_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    thumb_box.set_css_classes(&["win11-device-thumb"]);
    let thumb_icon = gtk4::Image::from_icon_name("computer-symbolic");
    thumb_icon.set_pixel_size(42);
    thumb_box.append(&thumb_icon);
    hero_card.append(&thumb_box);

    let info_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    info_box.set_valign(gtk4::Align::Center);
    let host_lbl = gtk4::Label::builder()
        .label(&info.hostname)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-name".to_string()])
        .build();
    let model_lbl = gtk4::Label::builder()
        .label(&info.cpu_model)
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
    root_box.append(&hero_card);

    // ── Cards group: About + Specs + Migration + Utilities ────────────────
    let rows_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    rows_box.set_margin_top(8);

    // 1) About card
    let about_card = adw::PreferencesGroup::builder()
        .title("About this device")
        .build();
    about_card.add(&make_row("Device name", &info.hostname));
    about_card.add(&make_row("OS", &info.os_pretty));
    about_card.add(&make_row(
        "Kernel",
        &format!("linux-zen {}", info.kernel),
    ));
    about_card.add(&make_row(
        "Processor",
        &format!("{} ({} logical cores)", info.cpu_model, info.cpu_cores),
    ));
    let host_for_rename = host_lbl.clone();
    about_card.add(&make_row_with_action(
        "Rename device",
        "Change the hostname shown on the network",
        "Rename",
        move |_| {
            let parent = rename_btn.root().and_downcast::<gtk4::Window>();
            let dialog = adw::MessageDialog::builder()
                .heading("Rename your PC")
                .body("Enter a new name for this device:")
                .transient_for(parent.as_ref().unwrap_or(&gtk4::Window::new()))
                .build();
            let entry = gtk4::Entry::builder()
                .text(&*host_for_rename.text())
                .margin_top(8)
                .margin_bottom(8)
                .build();
            dialog.set_extra_child(Some(&entry));
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("rename", "Save");
            dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
            let host_for_dialog = host_for_rename.clone();
            dialog.connect_response(None, move |d, resp| {
                if resp == "rename" {
                    let new_name = entry.text().trim().to_string();
                    if !new_name.is_empty() {
                        let _ = std::process::Command::new("hostnamectl")
                            .arg("set-hostname")
                            .arg(&new_name)
                            .status();
                        host_for_dialog.set_text(&new_name);
                    }
                }
                d.close();
            });
            dialog.present();
        },
    ));
    rows_box.append(&about_card);

    // 2) Hardware specs
    let ram_total_h = system::human_bytes(info.ram_total_bytes);
    let ram_used_h = system::human_bytes(info.ram_used_bytes);
    let ram_pct = if info.ram_total_bytes == 0 {
        0.0
    } else {
        info.ram_used_bytes as f64 / info.ram_total_bytes as f64
    };
    let disk_total_h = system::human_bytes(info.disk_total_bytes);
    let disk_used_h = system::human_bytes(info.disk_used_bytes);
    let disk_pct = if info.disk_total_bytes == 0 {
        0.0
    } else {
        info.disk_used_bytes as f64 / info.disk_total_bytes as f64
    };

    let specs_card = adw::PreferencesGroup::builder()
        .title("Hardware")
        .build();
    specs_card.add(&make_row(
        "Memory",
        &format!(
            "{} / {} ({:.0}%)",
            ram_used_h,
            ram_total_h,
            ram_pct * 100.0
        ),
    ));
    specs_card.add(&make_row(
        "Storage",
        &format!(
            "{} / {} ({:.0}%) on {}",
            disk_used_h,
            disk_total_h,
            disk_pct * 100.0,
            info.disk_mount
        ),
    ));
    rows_box.append(&specs_card);

    // 3) Migration tools
    let mig_card = adw::PreferencesGroup::builder()
        .title("Migration")
        .description("Bring in users and packages from another Linux install on this disk.")
        .build();
    mig_card.add(&{
        let row = adw::ActionRow::new();
        row.set_title("Import from another Linux install");
        row.set_subtitle("Detect Fedora / Ubuntu / Debian installations on other partitions");
        row.add_prefix(&gtk4::Image::from_icon_name("system-update-symbolic"));
        let btn = gtk4::Button::builder()
            .label("Scan now")
            .valign(gtk4::Align::Center)
            .css_classes(vec!["win11-secondary-btn".to_string()])
            .build();
        btn.connect_clicked(|_| {
            // The actual scan lives in /usr/local/bin/zohara-migrate (the
            // PyQt5 helper from the live ISO). If the script is not
            // installed, the spawn just fails silently -- we don't
            // want to throw an error dialog for a missing optional tool.
            let _ = std::process::Command::new("zohara-migrate")
                .arg("--scan")
                .spawn();
        });
        row.add_suffix(&btn);
        row.set_activatable(false);
        row
    });
    rows_box.append(&mig_card);

    // 4) Zohara utilities -- surface the still-installed .pyqt5 helpers
    // so users have one place to find them. Each button just spawns the
    // corresponding /usr/local/bin/<cmd> -- real Python-side logic lives
    // there.
    let tools_card = adw::PreferencesGroup::builder()
        .title("Zohara utilities")
        .description("First-run welcome, user manager, and offline package cache cleaner.")
        .build();
    for (label, sub, icon, cmd) in &[
        (
            "Welcome",
            "Show the first-run tour",
            "preferences-system-login-symbolic",
            "zohara-welcome",
        ),
        (
            "User manager",
            "Add or remove user accounts",
            "system-users-symbolic",
            "zohara-usermgr",
        ),
        (
            "Package cache cleaner",
            "Remove old versions from /var/cache/pacman/pkg",
            "edit-clear-symbolic",
            "zohara-cleanup-cache",
        ),
    ] {
        let row = adw::ActionRow::new();
        row.set_title(label);
        row.set_subtitle(sub);
        row.add_prefix(&gtk4::Image::from_icon_name(icon));
        let btn = gtk4::Button::builder()
            .label("Open")
            .valign(gtk4::Align::Center)
            .css_classes(vec!["win11-secondary-btn".to_string()])
            .build();
        let cmd_str = cmd.to_string();
        btn.connect_clicked(move |_| {
            let _ = std::process::Command::new(&cmd_str).spawn();
        });
        row.add_suffix(&btn);
        row.set_activatable(false);
        tools_card.add(&row);
    }
    rows_box.append(&tools_card);

    root_box.append(&rows_box);
    scroll.set_child(Some(&root_box));
    scroll.upcast()
}

fn make_row(title: &str, value: &str) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(title);
    row.set_subtitle(value);
    row.set_activatable(false);
    row
}

fn make_row_with_action<F: Fn(&adw::ActionRow) + 'static>(
    title: &str,
    sub: &str,
    action_label: &str,
    f: F,
) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(title);
    row.set_subtitle(sub);
    let btn = gtk4::Button::builder()
        .label(action_label)
        .valign(gtk4::Align::Center)
        .css_classes(vec!["win11-secondary-btn".to_string()])
        .build();
    let row_for_cb = row.clone();
    btn.connect_clicked(move |_| f(&row_for_cb));
    row.add_suffix(&btn);
    row
}
