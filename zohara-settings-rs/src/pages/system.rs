use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::process::Command;

pub fn build() -> gtk4::Widget {
    let prefs_page = adw::PreferencesPage::new();
    prefs_page.set_title("System");
    prefs_page.set_icon_name(Some("computer-symbolic"));

    // ── Power & Battery ───────────────────────────────────────────────────────
    let power_group = adw::PreferencesGroup::new();
    power_group.set_title("Power & battery");

    let profile_row = adw::ComboRow::new();
    profile_row.set_title("Power mode");
    profile_row.set_subtitle("Choose a power profile to balance performance and energy use");
    let profiles = gtk4::StringList::new(&["Performance", "Balanced", "Battery Saver"]);
    profile_row.set_model(Some(&profiles));
    profile_row.set_selected(1); // Balanced default

    profile_row.connect_selected_notify(|row| {
        let profile = match row.selected() {
            0 => "performance",
            2 => "power-saver",
            _ => "balanced",
        };
        // Use power-profiles-daemon via D-Bus (non-blocking fire-and-forget)
        glib::spawn_future_local(async move {
            let _ = tokio::process::Command::new("powerprofilesctl")
                .arg("set")
                .arg(profile)
                .output()
                .await;
        });
    });

    let battery_row = adw::ActionRow::new();
    battery_row.set_title("Battery");
    battery_row.set_subtitle("Loading…");

    // Load battery info async
    let battery_row_clone = battery_row.clone();
    glib::spawn_future_local(async move {
        match crate::backend::dbus::upower_battery().await {
            Ok((pct, charging)) => {
                let status = if charging { "Charging" } else { "Discharging" };
                battery_row_clone.set_subtitle(&format!("{:.0}% — {}", pct, status));
            }
            Err(_) => {
                battery_row_clone.set_subtitle("No battery detected (desktop / AC only)");
            }
        }
    });

    power_group.add(&profile_row);
    power_group.add(&battery_row);

    // ── Display ───────────────────────────────────────────────────────────────
    let display_group = adw::PreferencesGroup::new();
    display_group.set_title("Display");

    let display_row = adw::ActionRow::new();
    display_row.set_title("Display settings");
    display_row.set_subtitle("Resolution, refresh rate, and arrangement (KDE)");
    display_row.set_activatable(true);
    display_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
    display_row.connect_activated(|_| {
        let _ = Command::new("kcmshell6").arg("kcm_kscreen").spawn();
    });

    let sound_row = adw::ActionRow::new();
    sound_row.set_title("Sound");
    sound_row.set_subtitle("Volume, output, and input devices (KDE)");
    sound_row.set_activatable(true);
    sound_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
    sound_row.connect_activated(|_| {
        let _ = Command::new("kcmshell6").arg("kcm_pulseaudio").spawn();
    });

    display_group.add(&display_row);
    display_group.add(&sound_row);

    // ── About ─────────────────────────────────────────────────────────────────
    let about_group = adw::PreferencesGroup::new();
    about_group.set_title("About this device");

    // Collect system info synchronously (these are instant reads)
    let os_version = read_os_version();
    let kernel = read_kernel();
    let cpu = read_cpu();
    let ram = read_ram();
    let gpu = read_gpu();

    for (title, value) in &[
        ("Zohara OS", os_version.as_str()),
        ("Kernel", kernel.as_str()),
        ("Processor", cpu.as_str()),
        ("Installed RAM", ram.as_str()),
        ("Graphics", gpu.as_str()),
    ] {
        let row = adw::ActionRow::new();
        row.set_title(title);
        row.set_subtitle(value);
        about_group.add(&row);
    }

    let copy_row = adw::ActionRow::new();
    copy_row.set_title("Copy device specifications");
    copy_row.set_activatable(true);
    copy_row.add_suffix(&gtk4::Image::from_icon_name("edit-copy-symbolic"));

    let specs = format!(
        "OS: {}\nKernel: {}\nCPU: {}\nRAM: {}\nGPU: {}",
        os_version, kernel, cpu, ram, gpu
    );
    copy_row.connect_activated(move |row| {
        let display = row.display();
        display.clipboard().set_text(&specs);
    });
    about_group.add(&copy_row);

    prefs_page.add(&power_group);
    prefs_page.add(&display_group);
    prefs_page.add(&about_group);

    // Wrap in a scrolled window inside a ToolbarView for the header
    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&prefs_page));

    toolbar_view.upcast()
}

fn read_os_version() -> String {
    let out = Command::new("bash")
        .args(["-c", "grep PRETTY_NAME /etc/os-release | cut -d= -f2 | tr -d '\"'"])
        .output()
        .ok();
    out.and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Zohara OS".to_string())
}

fn read_kernel() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn read_cpu() -> String {
    let out = Command::new("bash")
        .args(["-c", "grep 'model name' /proc/cpuinfo | head -1 | cut -d: -f2"])
        .output()
        .ok();
    out.and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string())
}

fn read_ram() -> String {
    let out = Command::new("bash")
        .args(["-c", "grep MemTotal /proc/meminfo | awk '{printf \"%.1f GiB\", $2/1048576}'"])
        .output()
        .ok();
    out.and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn read_gpu() -> String {
    let out = Command::new("bash")
        .args(["-c", "lspci | grep -i 'vga\\|3d\\|display' | head -1 | sed 's/.*: //'"])
        .output()
        .ok();
    out.and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown".to_string())
}
