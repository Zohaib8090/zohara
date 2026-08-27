use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::process::Command;

pub fn build() -> gtk4::Widget {
    let prefs_page = adw::PreferencesPage::new();

    let group = adw::PreferencesGroup::new();
    group.set_title("Advanced");
    group.set_description(Some("Open the full desktop environment settings panel for advanced configuration not covered by Zohara Settings."));

    let open_row = adw::ActionRow::new();
    open_row.set_title("Open Advanced Settings");
    open_row.set_subtitle("Full desktop configuration panel");
    open_row.set_activatable(true);
    open_row.add_prefix(&gtk4::Image::from_icon_name("preferences-system-symbolic"));
    open_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
    open_row.connect_activated(|_| {
        // The real binary on Arch Linux is `systemsettings` (lowercase); the
        // KDE6 variant is also exposed as `systemsettings`. Spawn it and
        // ignore failure silently if the DE panel isn't installed.
        let _ = Command::new("systemsettings").spawn();
    });
    group.add(&open_row);

    prefs_page.add(&group);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&prefs_page));
    toolbar_view.upcast()
}
