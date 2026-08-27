use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

/// Where the OTA manifest is published. The `create_update_bundle.sh` script
/// writes a `latest.json` next to the self-extracting bundle and the ISO build
/// uploads it to the GitHub Release, so this URL always points at the newest
/// published bundle's metadata.
const OTA_MANIFEST_URL: &str =
    "https://github.com/Zohaib8090/zohara/releases/latest/download/latest.json";

/// Local OS version, read from /etc/lsb-release (DISTRIB_RELEASE).
fn local_os_version() -> String {
    if let Ok(text) = std::fs::read_to_string("/etc/lsb-release") {
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("DISTRIB_RELEASE=") {
                return rest.trim().to_string();
            }
        }
    }
    String::new()
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

    // ── Page Title ──────────────────────────────────────────────────────────
    let title_lbl = gtk4::Label::builder()
        .label("Zohara Update")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Hero Update Status Banner ────────────────────────────────────────────
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
        .label("You're up to date")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-name".to_string()])
        .build();

    let last_check_lbl = gtk4::Label::builder()
        .label("Last checked: never")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-sub".to_string()])
        .build();

    info_box.append(&status_title);
    info_box.append(&last_check_lbl);
    hero_card.append(&info_box);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    hero_card.append(&spacer);

    // Right "Check for updates" Button
    let check_btn = gtk4::Button::builder()
        .label("Check for updates")
        .css_classes(vec!["win11-update-btn".to_string()])
        .valign(gtk4::Align::Center)
        .build();

    // Hidden "Download" button — revealed only when an OTA update is found.
    let download_btn = gtk4::Button::builder()
        .label("Download")
        .css_classes(vec!["win11-primary-btn".to_string()])
        .valign(gtk4::Align::Center)
        .build();
    download_btn.set_visible(false);

    hero_card.append(&check_btn);
    hero_card.append(&download_btn);
    root_box.append(&hero_card);

    // Shared store for the latest OTA download URL. The Download button's
    // handler is wired exactly once (below); the check handler only updates
    // this store and toggles visibility, so repeated checks never stack
    // click handlers.
    let ota_url_store: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    {
        let dl_store = ota_url_store.clone();
        download_btn.connect_clicked(move |_| {
            if let Some(url) = dl_store.borrow().clone() {
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            }
        });
    }

    // ── Section: More options ────────────────────────────────────────────────
    let more_lbl = gtk4::Label::builder()
        .label("More options")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&more_lbl);

    let more_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    more_box.set_css_classes(&["win11-card-group"]);

    // Latest updates toggle
    let fast_sw = adw::SwitchRow::new();
    fast_sw.set_title("Get the latest updates as soon as they're available");
    fast_sw.set_subtitle("Be among the first to get the latest non-security updates, fixes, and improvements");
    fast_sw.add_prefix(&gtk4::Image::from_icon_name("starred-symbolic"));
    fast_sw.set_active(false);
    fast_sw.set_css_classes(&["win11-expander-row"]);
    more_box.append(&fast_sw);

    // Pause updates row
    let pause_row = adw::ActionRow::new();
    pause_row.set_title("Pause updates");
    pause_row.set_subtitle("Select the duration to pause automatic updates");
    pause_row.add_prefix(&gtk4::Image::from_icon_name("media-playback-pause-symbolic"));
    let pause_combo = gtk4::DropDown::from_strings(&[
        "Pause for 1 week",
        "Pause for 2 weeks",
        "Pause for 3 weeks",
        "Pause for 4 weeks",
    ]);
    pause_combo.set_valign(gtk4::Align::Center);
    pause_row.add_suffix(&pause_combo);
    pause_row.set_css_classes(&["win11-expander-row"]);
    more_box.append(&pause_row);

    // Update History
    let hist_row = build_action_row(
        "Update history",
        "View installed packages and system upgrade logs",
        "document-open-recent-symbolic",
    );
    more_box.append(&hist_row);

    // Advanced Options
    let adv_row = build_action_row(
        "Advanced options",
        "Delivery optimization, optional updates, active hours, mirror selector",
        "preferences-system-symbolic",
    );
    more_box.append(&adv_row);

    // Zohara Insider Program — REMOVED. There is no insider program; the
    // previous UI implied one existed and clicking the row did nothing.
    root_box.append(&more_box);

    // ── Section: Related support ─────────────────────────────────────────────
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

    // ── Check-for-updates handler ──────────────────────────────────────────────
    let status_title_clone = status_title.clone();
    let last_check_clone = last_check_lbl.clone();
    let check_btn_clone = check_btn.clone();
    let download_btn_clone = download_btn.clone();
    let ota_store_clone = ota_url_store.clone();

    check_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        status_title_clone.set_text("Checking for updates…");
        last_check_clone.set_text("Synchronizing repositories…");
        download_btn_clone.set_visible(false);

        let title_c = status_title_clone.clone();
        let last_c = last_check_clone.clone();
        let check_c = check_btn_clone.clone();
        let dl_c = download_btn_clone.clone();
        let store_c = ota_store_clone.clone();

        glib::spawn_future_local(async move {
            // Step 1: ask the OTA manifest what the newest published Zohara OS
            // bundle is. We fetch it over the network with `curl` (present on
            // the ISO) rather than adding a Rust HTTP dependency. A failure
            // here is non-fatal: we just can't report an OS update.
            let (ota_available, ota_url) = fetch_ota_manifest().await;

            // Step 2: sync the package databases, then query for out-of-date
            // packages. "Check for updates" intentionally does NOT install.
            let sync = tokio::process::Command::new("pacman")
                .args(["-Sy"])
                .output()
                .await;

            if sync.is_err() || !sync.as_ref().unwrap().status.success() {
                let err_line = match sync {
                    Ok(out) => String::from_utf8_lossy(&out.stderr)
                        .lines()
                        .next()
                        .unwrap_or("pacman exited non-zero")
                        .to_string(),
                    Err(e) => e.to_string(),
                };
                title_c.set_text("Check failed");
                last_c.set_text(&format!("Error: {}", err_line));
                check_c.set_sensitive(true);
                return;
            }

            let qu = tokio::process::Command::new("pacman")
                .args(["-Qu"])
                .output()
                .await
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_default();

            let upgrades: Vec<&str> = qu.lines().filter(|l| !l.is_empty()).collect();
            let pkg_count = upgrades.len();

            let now = glib::DateTime::now_local()
                .and_then(|t| t.format("%Y-%m-%d %H:%M"))
                .unwrap_or_else(|_| "now".into());

            // Store the OTA URL and reveal the Download button if there is one.
            *store_c.borrow_mut() = ota_url.clone();
            dl_c.set_visible(ota_available && ota_url.is_some());

            // Compose the status banner from both channels.
            if ota_available {
                title_c.set_text("A Zohara OS update is available");
                last_c.set_text(&format!(
                    "Last checked: {}  •  {} package update{}",
                    now,
                    pkg_count,
                    if pkg_count == 1 { "" } else { "s" }
                ));
            } else if pkg_count == 0 {
                title_c.set_text("You're up to date");
                last_c.set_text(&format!("Last checked: {}", now));
            } else if pkg_count == 1 {
                title_c.set_text("1 package update available");
                last_c.set_text(&format!(
                    "Last checked: {}  •  {}",
                    now,
                    upgrades[0].split_whitespace().next().unwrap_or("(unknown)")
                ));
            } else {
                title_c.set_text(&format!("{} package updates available", pkg_count));
                last_c.set_text(&format!("Last checked: {}", now));
            }
            check_c.set_sensitive(true);
        });
    });

    scroll.set_child(Some(&root_box));
    scroll.upcast()
}

/// Fetch and parse `latest.json`. Returns (update_available, download_url).
/// `download_url` is only `Some` when an update is available AND a URL exists.
async fn fetch_ota_manifest() -> (bool, Option<String>) {
    let out = tokio::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "15", OTA_MANIFEST_URL])
        .output()
        .await;

    let stdout = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return (false, None),
    };

    let json: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => return (false, None),
    };

    let remote_version = json
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let url = json
        .get("download_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // If we can't publish a version, we don't know there's an update.
    if remote_version.is_empty() {
        return (false, None);
    }

    let local = local_os_version();
    // Available when local is unknown, or the remote version differs.
    let available = local.is_empty() || local != remote_version;
    (available, if available { url } else { None })
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
