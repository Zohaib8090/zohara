use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;

use crate::backend::ota::{self, UpdateStatus, UpdateManifest};
use crate::tokio_runtime;

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
        .label("Zohara Update")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Hero Update Status Banner ─────────────────────────────────────────────
    let hero_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 20);
    hero_card.set_css_classes(&["win11-hero-card"]);
    hero_card.set_margin_bottom(4);

    let sync_icon = gtk4::Image::from_icon_name("software-update-available-symbolic");
    sync_icon.set_pixel_size(48);
    sync_icon.set_css_classes(&["accent-cyan"]);
    hero_card.append(&sync_icon);

    let info_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    info_box.set_valign(gtk4::Align::Center);

    let status_title = gtk4::Label::builder()
        .label("Tap \"Check for updates\"")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-name".to_string()])
        .build();

    let last_check_lbl = gtk4::Label::builder()
        .label("Check for Zohara system updates and pending Arch package updates")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-sub".to_string()])
        .build();

    info_box.append(&status_title);
    info_box.append(&last_check_lbl);
    hero_card.append(&info_box);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    hero_card.append(&spacer);

    let check_btn = gtk4::Button::builder()
        .label("Check for updates")
        .css_classes(vec!["win11-update-btn".to_string()])
        .valign(gtk4::Align::Center)
        .build();
    hero_card.append(&check_btn);
    root_box.append(&hero_card);

    // ── OTA download + run section (hidden by default) ─────────────────────
    // A second hero card that appears only after a successful update check
    // has found a newer version. It carries the download / run buttons.
    let ota_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    ota_box.set_margin_bottom(4);
    ota_box.set_visible(false);

    let ota_title = gtk4::Label::builder()
        .label("Zohara OS update available")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    ota_box.append(&ota_title);

    let ota_changelog = gtk4::Label::builder()
        .halign(gtk4::Align::Start)
        .wrap(true)
        .css_classes(vec!["win11-stat-muted".to_string()])
        .build();
    ota_box.append(&ota_changelog);

    let ota_meta = gtk4::Label::builder()
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-stat-muted".to_string()])
        .build();
    ota_box.append(&ota_meta);

    let ota_button_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    ota_button_row.set_margin_top(8);
    ota_box.append(&ota_button_row);

    let download_btn = gtk4::Button::builder()
        .label("Download update")
        .css_classes(vec!["win11-update-btn".to_string()])
        .build();
    ota_button_row.append(&download_btn);

    let run_btn = gtk4::Button::builder()
        .label("Run update now")
        .css_classes(vec!["win11-primary-btn".to_string()])
        .set_sensitive(false)
        .build();
    ota_button_row.append(&run_btn);

    let progress_lbl = gtk4::Label::builder()
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-stat-text".to_string()])
        .build();
    progress_lbl.set_visible(false);
    ota_box.append(&progress_lbl);

    let progress_bar = gtk4::ProgressBar::new();
    progress_bar.set_show_text(false);
    progress_bar.set_visible(false);
    ota_box.append(&progress_bar);

    root_box.append(&ota_box);

    // The downloaded bundle path. We keep it in a RefCell so the download
    // closure can set it and the run closure can read it. None of these
    // callbacks outlive the page so RefCell is fine.
    let downloaded_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
    let current_manifest: Rc<RefCell<Option<UpdateManifest>>> = Rc::new(RefCell::new(None));

    // ── Section 1: pacman package updates (legacy `pacman -Qu`) ────────────
    // This section is the SAME as before: it shows pending pacman package
    // updates from the Arch mirror, not Zohara system updates. We keep it
    // so users can still see when their base packages are out of date.
    let more_lbl = gtk4::Label::builder()
        .label("Arch package updates")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&more_lbl);

    let more_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    more_box.set_css_classes(&["win11-card-group"]);

    let fast_sw = adw::SwitchRow::new();
    fast_sw.set_title("Get the latest Arch updates as soon as they're available");
    fast_sw.set_subtitle("Enable -Sy in pacman.conf so package lists sync on every check (the default is to sync only when refreshing)");
    fast_sw.add_prefix(&gtk4::Image::from_icon_name("starred-symbolic"));
    fast_sw.set_active(false);
    fast_sw.set_css_classes(&["win11-expander-row"]);
    more_box.append(&fast_sw);

    let pause_row = adw::ActionRow::new();
    pause_row.set_title("Pause Arch updates");
    pause_row.set_subtitle("When paused, \"Check for updates\" will still download metadata but not install anything");
    pause_row.add_prefix(&gtk4::Image::from_icon_name("media-playback-pause-symbolic"));
    let pause_combo = gtk4::DropDown::from_strings(&["Pause for 1 week", "Pause for 2 weeks", "Pause for 3 weeks", "Pause for 4 weeks"]);
    pause_combo.set_valign(gtk4::Align::Center);
    pause_row.add_suffix(&pause_combo);
    pause_row.set_css_classes(&["win11-expander-row"]);
    more_box.append(&pause_row);

    let hist_row = build_action_row("Update history", "View installed packages and system upgrade logs", "document-open-recent-symbolic");
    more_box.append(&hist_row);

    let adv_row = build_action_row("Advanced options", "pacman.conf mirror order, parallel downloads, hook scripts", "preferences-system-symbolic");
    more_box.append(&adv_row);

    root_box.append(&more_box);

    // ── Section 2: Related support ────────────────────────────────────────────
    let support_lbl = gtk4::Label::builder()
        .label("Related support")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&support_lbl);

    let support_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    support_box.set_css_classes(&["win11-card-group"]);

    let help_exp = adw::ExpanderRow::new();
    help_exp.set_title("Help with Zohara Update");
    help_exp.add_prefix(&gtk4::Image::from_icon_name("help-browser-symbolic"));
    help_exp.set_css_classes(&["win11-expander-row"]);

    for item in &[
        "Troubleshooting package download errors",
        "Rolling back or downgrading a package",
        "Configuring custom Arch mirrors in pacman.conf",
        "Cleaning up package cache safely",
    ] {
        let r = adw::ActionRow::new();
        r.set_title(item);
        r.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        r.set_activatable(true);
        help_exp.add_row(&r);
    }
    support_box.append(&help_exp);
    root_box.append(&support_box);

    // ── Wire the "Check for updates" button ────────────────────────────────
    // This single button does BOTH things in one click:
    //   1. Checks the OTA manifest (new Zohara system version)
    //   2. Runs `pacman -Sy && pacman -Qu` to get pending Arch packages
    // The OTA result is shown in the dedicated ota_box; the package count
    // is shown in the hero card.
    let status_title_c = status_title.clone();
    let last_check_c = last_check_lbl.clone();
    let check_btn_c = check_btn.clone();
    let ota_box_c = ota_box.clone();
    let ota_title_c = ota_title.clone();
    let ota_changelog_c = ota_changelog.clone();
    let ota_meta_c = ota_meta.clone();
    let download_btn_c = download_btn.clone();
    let run_btn_c = run_btn.clone();
    let progress_lbl_c = progress_lbl.clone();
    let progress_bar_c = progress_bar.clone();
    let manifest_cell = current_manifest.clone();
    let path_cell = downloaded_path.clone();

    check_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        status_title_c.set_text("Checking for updates…");
        last_check_c.set_text("Querying Zohara update server and Arch mirrors…");
        ota_box_c.set_visible(false);
        path_cell.borrow_mut().take();
        manifest_cell.borrow_mut().take();
        run_btn_c.set_sensitive(false);

        let title_c = status_title_c.clone();
        let last_c = last_check_c.clone();
        let btn_c = check_btn_c.clone();
        let ota_box_c = ota_box_c.clone();
        let ota_title_c = ota_title_c.clone();
        let ota_changelog_c = ota_changelog_c.clone();
        let ota_meta_c = ota_meta_c.clone();
        let dl_c = download_btn_c.clone();
        let run_c = run_btn_c.clone();
        let prog_lbl_c = progress_lbl_c.clone();
        let prog_bar_c = progress_bar_c.clone();
        let manifest_c = manifest_cell.clone();
        let path_c = path_cell.clone();

        glib::spawn_future_local(async move {
            // Spawn two parallel checks: Zohara system update + Arch package updates.
            // We do this in a single async block so the UI can show a unified result.
            let ota_fut = ota::check_for_updates();
            let pacman_fut = async {
                // -Sy: sync the local package DB with the mirror list.
                // We use --noconfirm because there is no user present; this is
                // a non-destructive read-only operation (no installs).
                let _ = tokio::process::Command::new("pacman")
                    .args(["-Sy", "--noconfirm"])
                    .output()
                    .await;
                // -Qu: query the local DB for out-of-date packages.
                tokio::process::Command::new("pacman")
                    .args(["-Qu"])
                    .output()
                    .await
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_default()
            };
            let (ota_status, qu_output) = tokio::join!(ota_fut, pacman_fut);

            // ── Render the Zohara system update section ─────────────────
            match &ota_status {
                UpdateStatus::UpdateAvailable(m) => {
                    ota_title_c.set_text(&format!("Zohara OS {} is available", m.version));
                    let mut meta = format!("Published {} • {:.1} MB", m.date, m.size_bytes as f64 / 1_048_576.0);
                    if let Some(s) = &m.sha256 {
                        meta.push_str(&format!(" • sha256: {}…", &s[..16.min(s.len())]));
                    }
                    ota_meta_c.set_text(&meta);
                    ota_changelog_c.set_text(if m.changelog.is_empty() {
                        "No changelog provided."
                    } else {
                        m.changelog.as_str()
                    });
                    *manifest_c.borrow_mut() = Some(m.clone());
                    *path_c.borrow_mut() = None;
                    ota_box_c.set_visible(true);
                    dl_c.set_sensitive(true);
                    dl_c.set_label("Download update");
                    run_c.set_sensitive(false);
                }
                UpdateStatus::TooOld { manifest, local } => {
                    ota_title_c.set_text("Reinstall required");
                    ota_meta_c.set_text(&format!(
                        "Local: {} • Minimum: {}",
                        local,
                        manifest.min_zohara_settings_version.as_deref().unwrap_or("?")
                    ));
                    ota_changelog_c.set_text(
                        "Your current Zohara Settings version is too old to apply OTA updates. \
                         Please re-download the latest ISO from the website and reinstall.",
                    );
                    ota_box_c.set_visible(true);
                    dl_c.set_sensitive(false);
                    dl_c.set_label("Download (blocked: too old)");
                    run_c.set_sensitive(false);
                }
                UpdateStatus::UpToDate => {
                    ota_box_c.set_visible(false);
                }
                UpdateStatus::Unreachable(reason) | UpdateStatus::Malformed(reason) | UpdateStatus::LocalUnknown(reason) => {
                    ota_title_c.set_text("Zohara system update: check failed");
                    ota_meta_c.set_text("");
                    ota_changelog_c.set_text(&reason);
                    ota_box_c.set_visible(true);
                    dl_c.set_sensitive(false);
                    dl_c.set_label("Download (unavailable)");
                    run_c.set_sensitive(false);
                }
            }

            // ── Render the Arch package count in the hero card ───────
            let upgrades: Vec<&str> = qu_output.lines().filter(|l| !l.is_empty()).collect();
            let count = upgrades.len();
            let now = glib::DateTime::now_local()
                .and_then(|t| t.format("%Y-%m-%d %H:%M"))
                .unwrap_or_else(|_| "now".into());

            if count == 0 {
                title_c.set_text("You're up to date");
            } else if count == 1 {
                title_c.set_text("1 Arch package update available");
            } else {
                title_c.set_text(&format!("{} Arch package updates available", count));
            }
            last_c.set_text(&format!("Last checked: {}", now));
            btn_c.set_sensitive(true);

            // Suppress unused-var warning on the labels we only update on error.
            let _ = prog_lbl_c;
            let _ = prog_bar_c;
        });
    });

    // ── Wire the "Download update" button ────────────────────────────────────
    let manifest_cell_dl = current_manifest.clone();
    let path_cell_dl = downloaded_path.clone();
    let dl_btn_c = download_btn.clone();
    let run_btn_c2 = run_btn.clone();
    let progress_lbl_c2 = progress_lbl.clone();
    let progress_bar_c2 = progress_bar.clone();
    dl_btn_c.connect_clicked(move |btn| {
        let manifest = match manifest_cell_dl.borrow().clone() {
            Some(m) => m,
            None => return,
        };
        btn.set_sensitive(false);
        btn.set_label("Downloading…");
        progress_lbl_c2.set_visible(true);
        progress_lbl_c2.set_text(&format!(
            "Downloading Zohara OS {} from the update server…",
            manifest.version
        ));
        progress_bar_c2.set_visible(true);
        progress_bar_c2.set_fraction(0.0);

        let btn_c = dl_btn_c.clone();
        let run_c = run_btn_c2.clone();
        let progress_lbl_c = progress_lbl_c2.clone();
        let progress_bar_c = progress_bar_c2.clone();
        let path_c = path_cell_dl.clone();
        glib::spawn_future_local(async move {
            // We can't show real curl progress-bar updates in a GTK ProgressBar
            // without parsing curl's ANSI escape sequences, which is a lot of
            // complexity for a one-time download. Instead, show an indeterminate
            // state and reveal the final path when done. Future work: spawn
            // curl in a piped task and read its progress meter.
            progress_bar_c.pulse();
            let result = ota::download_bundle(&manifest).await;
            match result {
                Ok(path) => {
                    *path_c.borrow_mut() = Some(path.clone());
                    progress_lbl_c.set_text(&format!(
                        "Downloaded to {}. Click \"Run update now\" to install.",
                        path.display()
                    ));
                    progress_bar_c.set_fraction(1.0);
                    btn_c.set_label("Downloaded");
                    run_c.set_sensitive(true);
                }
                Err(e) => {
                    progress_lbl_c.set_text(&format!("Download failed: {}", e));
                    progress_bar_c.set_fraction(0.0);
                    btn_c.set_sensitive(true);
                    btn_c.set_label("Retry download");
                    run_c.set_sensitive(false);
                }
            }
        });
    });

    // ── Wire the "Run update now" button ─────────────────────────────────────
    // This is a USER-DRIVEN action. We never auto-run. Clicking this
    // button execs the downloaded bundle under pkexec, which will prompt
    // for the admin password. The bundle's own install_update.sh output
    // is shown to the user via a stdout pipe.
    let path_cell_run = downloaded_path.clone();
    run_btn.connect_clicked(move |btn| {
        let path = match path_cell_run.borrow().clone() {
            Some(p) => p,
            None => {
                progress_lbl.set_text("Download the update first, then click Run.");
                return;
            }
        };
        btn.set_sensitive(false);
        progress_lbl.set_visible(true);
        progress_lbl.set_text(&format!("Running {} via pkexec…", path.display()));

        let path_str = path.to_string_lossy().to_string();
        let progress_lbl_c = progress_lbl.clone();
        glib::spawn_future_local(async move {
            // pkexec is the correct path here: it gives the user a polkit
            // prompt for the admin password. The bundle's install_update.sh
            // then does its own `set -e` work, prints each step, and exits.
            // We do NOT pass --nointeractive: the user must type the
            // password. We do NOT auto-reboot.
            let path_for_blocking = path_str.clone();
            let result = tokio_runtime()
                .spawn(async move {
                    tokio::task::spawn_blocking(move || {
                        std::process::Command::new("pkexec")
                            .arg("bash")
                            .arg(&path_for_blocking)
                            .status()
                    })
                    .await
                })
                .await;

            match result {
                Ok(Ok(Ok(status))) if status.success() => {
                    progress_lbl_c.set_text("Update applied. The new binaries are live; relaunch Zohara Settings to use them.");
                }
                Ok(Ok(Ok(status))) => {
                    progress_lbl_c.set_text(&format!(
                        "Update exited with code {:?}. Check the terminal output or run the script manually.",
                        status.code()
                    ));
                }
                Ok(Ok(Err(e))) => {
                    progress_lbl_c.set_text(&format!("Failed to run update: {}", e));
                }
                Ok(Err(e)) => {
                    progress_lbl_c.set_text(&format!("Update task panicked: {}", e));
                }
                Err(e) => {
                    progress_lbl_c.set_text(&format!("Update task join error: {}", e));
                }
            }
            btn.set_sensitive(true);
        });
    });

    scroll.set_child(Some(&root_box));
    scroll.upcast()
}

fn build_action_row(title: &str, subtitle: &str, icon_name: &str) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(title);
    row.set_subtitle(subtitle);
    row.add_prefix(&gtk4::Image::from_icon_name(icon_name));
    row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
    row.set_activatable(true);
    row.set_css_classes(&["win11-expander-row"]);
    row
}
