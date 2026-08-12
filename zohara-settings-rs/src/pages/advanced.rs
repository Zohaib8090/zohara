use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::process::Command;

pub fn build() -> gtk4::Widget {
    let prefs_page = adw::PreferencesPage::new();

    let group = adw::PreferencesGroup::new();
    group.set_title("KDE System Settings");
    group.set_description(Some("Open the full KDE System Settings panel for advanced configuration not covered by Zohara Settings."));

    let open_row = adw::ActionRow::new();
    open_row.set_title("Open KDE System Settings");
    open_row.set_subtitle("Full plasma configuration panel");
    open_row.set_activatable(true);
    open_row.add_prefix(&gtk4::Image::from_icon_name("preferences-system-symbolic"));
    open_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
    open_row.connect_activated(|_| {
        let _ = Command::new("systemsettings6").spawn();
    });
    group.add(&open_row);

    prefs_page.add(&group);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&prefs_page));
    toolbar_view.upcast()
}
