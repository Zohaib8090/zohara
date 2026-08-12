use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::process::Command;

pub fn build() -> gtk4::Widget {
    let prefs_page = adw::PreferencesPage::new();

    let tz_group = adw::PreferencesGroup::new();
    tz_group.set_title("Date & time");

    let tz_row = adw::ActionRow::new();
    tz_row.set_title("Timezone");
    let tz = Command::new("bash")
        .args(["-c", "timedatectl show --property=Timezone --value"])
        .output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    tz_row.set_subtitle(&tz);
    tz_row.set_activatable(true);
    tz_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
    tz_row.connect_activated(|_| {
        let _ = Command::new("kcmshell6").arg("kcm_regionandlang").spawn();
    });
    tz_group.add(&tz_row);

    let ntp_row = adw::SwitchRow::new();
    ntp_row.set_title("Synchronize time automatically");
    ntp_row.set_subtitle("Using NTP");
    let ntp_on = Command::new("bash")
        .args(["-c", "timedatectl show --property=NTP --value"])
        .output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "yes")
        .unwrap_or(false);
    ntp_row.set_active(ntp_on);
    ntp_row.connect_active_notify(|row| {
        let on = row.is_active();
        glib::spawn_future_local(async move {
            let _ = tokio::process::Command::new("pkexec")
                .args(["timedatectl", "set-ntp", if on { "true" } else { "false" }])
                .output().await;
        });
    });
    tz_group.add(&ntp_row);

    let lang_group = adw::PreferencesGroup::new();
    lang_group.set_title("Language & keyboard");

    let lang_row = adw::ActionRow::new();
    lang_row.set_title("Language & formats");
    lang_row.set_subtitle("Locale, date format, keyboard layout");
    lang_row.set_activatable(true);
    lang_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
    lang_row.connect_activated(|_| {
        let _ = Command::new("kcmshell6").arg("kcm_regionandlang").spawn();
    });
    lang_group.add(&lang_row);

    prefs_page.add(&tz_group);
    prefs_page.add(&lang_group);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&prefs_page));
    toolbar_view.upcast()
}
