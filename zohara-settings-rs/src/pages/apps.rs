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
        .label("Apps")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Grouped Rows ──────────────────────────────────────────────────────────
    let rows_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    rows_box.set_css_classes(&["win11-card-group"]);

    // 1. Installed Apps (Expander with in-app search & package uninstall)
    let installed_exp = adw::ExpanderRow::new();
    installed_exp.set_title("Installed apps");
    installed_exp.set_subtitle("Uninstall and manage apps on your PC");
    installed_exp.add_prefix(&gtk4::Image::from_icon_name("application-x-executable-symbolic"));
    installed_exp.set_css_classes(&["win11-expander-row"]);

    let search_row = adw::ActionRow::new();
    let search_entry = gtk4::SearchEntry::builder()
        .placeholder_text("Search installed apps…")
        .hexpand(true)
        .build();
    search_row.add_suffix(&search_entry);
    installed_exp.add_row(&search_row);

    let loading_row = adw::ActionRow::new();
    loading_row.set_title("Scanning installed packages…");
    let spinner = gtk4::Spinner::new();
    spinner.start();
    loading_row.add_suffix(&spinner);
    installed_exp.add_row(&loading_row);

    let installed_exp_clone = installed_exp.clone();
    let loading_clone = loading_row.clone();
    glib::spawn_future_local(async move {
        let out = tokio::process::Command::new("pacman")
            .args(["-Qe"])
            .output()
            .await
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        installed_exp_clone.remove(&loading_clone);

        let mut count = 0;
        for line in out.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() { continue; }
            let pkg = parts[0];
            let ver = parts.get(1).copied().unwrap_or("");

            count += 1;
            if count > 20 { break; } // Show top 20 cleanly

            let row = adw::ActionRow::new();
            row.set_title(pkg);
            row.set_subtitle(&format!("Version {} • Native Arch Package", ver));
            row.add_prefix(&gtk4::Image::from_icon_name("application-x-executable-symbolic"));

            let uninst_btn = gtk4::Button::builder()
                .label("Uninstall")
                .css_classes(vec!["win11-danger-btn".to_string()])
                .build();
            let pkg_name = pkg.to_string();
            let row_clone = row.clone();
            uninst_btn.connect_clicked(move |btn| {
                // Confirm before doing anything destructive. The previous
                // implementation fired `pkexec pacman -R --noconfirm` on
                // click, which meant a single misclick could remove
                // essential system packages (glibc, systemd, zohara-settings
                // itself). The confirmation is the minimum safety net.
                let parent = btn.root().and_downcast::<gtk4::Window>();
                let dialog = adw::MessageDialog::builder()
                    .heading(&format!("Uninstall {}?", pkg_name))
                    .body("This will remove the package and any packages that depend on it. The change can be reverted with `pacman -S` if the package is still in the repositories.")
                    .transient_for(parent.as_ref().unwrap_or(&gtk4::Window::new()))
                    .build();
                dialog.add_response("cancel", "Cancel");
                dialog.add_response("uninstall", "Uninstall");
                dialog.set_response_appearance("uninstall", adw::ResponseAppearance::Destructive);
                dialog.set_default_response(Some("cancel"));

                let pkg_name = pkg_name.clone();
                let btn_w = btn.clone();
                let row_w = row_clone.clone();
                dialog.connect_response(None, move |d, resp| {
                    if resp == "uninstall" {
                        btn_w.set_sensitive(false);
                        row_w.set_subtitle("Uninstalling…");
                        // Use -Rs (remove + unneeded dependencies) instead of
                        // bare -R, and drop --noconfirm so pacman asks the
                        // user one more time at the polkit prompt. Belt and
                        // braces against accidental removal of system
                        // packages.
                        let _ = Command::new("pkexec")
                            .args(["pacman", "-Rs", &pkg_name])
                            .spawn();
                    }
                    d.close();
                });
                dialog.present();
            });
            row.add_suffix(&uninst_btn);
            installed_exp_clone.add_row(&row);
        }
    });

    rows_box.append(&installed_exp);

    // 2. Advanced App Settings
    let adv_row = build_action_row("Advanced app settings", "Choose where to get apps, archive apps, uninstall updates", "preferences-system-symbolic");
    rows_box.append(&adv_row);

    // 3. Default Apps
    let def_row = build_action_row("Default apps", "Defaults for file and link types, browser, mail", "preferences-desktop-default-applications-symbolic");
    rows_box.append(&def_row);

    // 4. Actions
    let act_row = build_action_row("Actions", "Zohara OS can recommend actions from these apps", "starred-symbolic");
    rows_box.append(&act_row);

    // 5. Offline Maps
    let map_row = build_action_row("Offline maps", "Downloads, storage location, map updates", "find-location-symbolic");
    rows_box.append(&map_row);

    // 6. Apps for Websites
    let web_row = build_action_row("Apps for websites", "Websites that can open in an app instead of a browser", "applications-internet-symbolic");
    rows_box.append(&web_row);

    // 7. Video Playback
    let vid_row = build_action_row("Video playback", "Video adjustments, HDR streaming, hardware acceleration", "video-x-generic-symbolic");
    rows_box.append(&vid_row);

    // 8. Startup (with In-App Startup Toggles)
    let startup_exp = adw::ExpanderRow::new();
    startup_exp.set_title("Startup");
    startup_exp.set_subtitle("Apps that start automatically when you sign in");
    startup_exp.add_prefix(&gtk4::Image::from_icon_name("system-run-symbolic"));
    startup_exp.set_css_classes(&["win11-expander-row"]);

    let apps = [
        ("Zohara Update Notifier", "High impact • Checks for OS upgrades at login", true),
        ("KDE Connect Daemon", "Low impact • Mobile device synchronization", false),
        ("PipeWire Audio Session", "High impact • Audio subsystem daemon", true),
    ];

    for (name, sub, active) in apps {
        let sw = adw::SwitchRow::new();
        sw.set_title(name);
        sw.set_subtitle(sub);
        sw.set_active(active);
        startup_exp.add_row(&sw);
    }
    rows_box.append(&startup_exp);

    // 9. Resume
    let res_row = build_action_row("Resume", "Continue work across devices", "document-open-recent-symbolic");
    rows_box.append(&res_row);

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
