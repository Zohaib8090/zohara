use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

pub fn build() -> gtk4::Widget {
    let prefs_page = adw::PreferencesPage::new();

    // ── Wi-Fi ─────────────────────────────────────────────────────────────────
    let wifi_group = adw::PreferencesGroup::new();
    wifi_group.set_title("Wireless");

    let wifi_row = adw::SwitchRow::new();
    wifi_row.set_title("Wi-Fi");
    wifi_row.set_subtitle("Loading…");

    // Check current state async
    let wifi_row_clone = wifi_row.clone();
    glib::spawn_future_local(async move {
        match crate::backend::dbus::nm_wifi_enabled().await {
            Ok(enabled) => {
                wifi_row_clone.set_active(enabled);
                wifi_row_clone.set_subtitle(if enabled { "On" } else { "Off" });
            }
            Err(e) => wifi_row_clone.set_subtitle(&format!("Error: {}", e)),
        }
    });

    // Toggle Wi-Fi on switch change
    wifi_row.connect_active_notify(|row| {
        let enabled = row.is_active();
        glib::spawn_future_local(async move {
            let _ = crate::backend::dbus::nm_set_wifi(enabled).await;
        });
    });

    wifi_group.add(&wifi_row);

    // ── Available Networks ────────────────────────────────────────────────────
    let networks_group = adw::PreferencesGroup::new();
    networks_group.set_title("Available networks");

    let refresh_btn = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .css_classes(vec!["flat".to_string()])
        .build();
    networks_group.set_header_suffix(Some(&refresh_btn));

    // Scanning spinner row
    let scan_row = adw::ActionRow::new();
    scan_row.set_title("Scanning for networks…");
    let spinner = gtk4::Spinner::new();
    spinner.start();
    scan_row.add_suffix(&spinner);
    networks_group.add(&scan_row);

    let networks_group_clone = networks_group.clone();
    let scan_row_clone = scan_row.clone();

    let do_scan = move || {
        let networks_group = networks_group_clone.clone();
        let scan_row = scan_row_clone.clone();

        glib::spawn_future_local(async move {
            // Force a fresh scan and wait 3s for results
            let _ = tokio::process::Command::new("nmcli")
                .args(["dev", "wifi", "rescan"])
                .output()
                .await;
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            let result = tokio::process::Command::new("nmcli")
                .args(["-t", "-f", "SSID,SECURITY,SIGNAL,ACTIVE", "dev", "wifi", "list"])
                .output()
                .await;

            // Remove spinner row
            networks_group.remove(&scan_row);

            if let Ok(out) = result {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let mut seen = std::collections::HashSet::new();
                for line in stdout.lines().take(30) {
                    let parts: Vec<&str> = line.splitn(4, ':').collect();
                    if parts.len() < 4 { continue; }
                    let ssid = parts[0];
                    let security = parts[1];
                    let signal: u32 = parts[2].parse().unwrap_or(0);
                    let active = parts[3].trim() == "yes";

                    if ssid.is_empty() { continue; }
                    if !seen.insert(ssid.to_string()) { continue; } // deduplicate

                    let row = adw::ActionRow::new();
                    row.set_title(ssid);
                    row.set_subtitle(&format!("{} • {}% signal{}",
                        if security.is_empty() { "Open" } else { security },
                        signal,
                        if active { " • Connected" } else { "" }
                    ));
                    row.set_activatable(!active);

                    let icon_name = if signal > 70 { "network-wireless-signal-excellent-symbolic" }
                        else if signal > 40 { "network-wireless-signal-good-symbolic" }
                        else { "network-wireless-signal-weak-symbolic" };

                    let icon = if active {
                        let img = gtk4::Image::from_icon_name("network-wireless-connected-symbolic");
                        img.set_css_classes(&["accent"]);
                        img
                    } else {
                        gtk4::Image::from_icon_name(icon_name)
                    };
                    row.add_prefix(&icon);

                    if !active {
                        let connect_btn = gtk4::Button::builder()
                            .label("Connect")
                            .css_classes(vec!["suggested-action".to_string()])
                            .valign(gtk4::Align::Center)
                            .build();
                        let ssid_owned = ssid.to_string();
                        connect_btn.connect_clicked(move |_| {
                            let ssid = ssid_owned.clone();
                            glib::spawn_future_local(async move {
                                let _ = tokio::process::Command::new("nmcli")
                                    .args(["dev", "wifi", "connect", &ssid])
                                    .output()
                                    .await;
                            });
                        });
                        row.add_suffix(&connect_btn);
                    }

                    networks_group.add(&row);
                }
            }
        });
    };

    // Initial scan
    do_scan();

    // Refresh button re-scans
    let do_scan_for_btn = {
        let networks_group2 = networks_group.clone();
        move |_: &gtk4::Button| {
            // Clear existing rows (can't easily do this without tracking them)
            // Simple approach: spawn a new scan
            let networks_group = networks_group2.clone();
            glib::spawn_future_local(async move {
                let _ = tokio::process::Command::new("nmcli")
                    .args(["dev", "wifi", "rescan"])
                    .output()
                    .await;
            });
            let _ = networks_group.activate();
        }
    };
    refresh_btn.connect_clicked(do_scan_for_btn);

    // ── Ethernet ──────────────────────────────────────────────────────────────
    let eth_group = adw::PreferencesGroup::new();
    eth_group.set_title("Ethernet");

    let eth_row = adw::ActionRow::new();
    eth_row.set_title("Ethernet");
    eth_row.set_subtitle("Checking…");

    let eth_row_clone = eth_row.clone();
    glib::spawn_future_local(async move {
        let result = tokio::process::Command::new("nmcli")
            .args(["-t", "-f", "TYPE,STATE,DEVICE", "connection", "show", "--active"])
            .output()
            .await;
        if let Ok(out) = result {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let eth = stdout.lines().find(|l| l.contains("ethernet"));
            if let Some(line) = eth {
                let device = line.split(':').nth(2).unwrap_or("eth0");
                eth_row_clone.set_subtitle(&format!("Connected via {}", device));
            } else {
                eth_row_clone.set_subtitle("Not connected");
            }
        }
    });

    eth_group.add(&eth_row);

    prefs_page.add(&wifi_group);
    prefs_page.add(&networks_group);
    prefs_page.add(&eth_group);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&prefs_page));
    toolbar_view.upcast()
}
