use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::process::Command;

fn kde_link_group(title: &str, items: &[(&str, &str, &str)]) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title(title);
    for (label, subtitle, kcm) in items {
        let row = adw::ActionRow::new();
        row.set_title(label);
        row.set_subtitle(subtitle);
        row.set_activatable(true);
        row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        let kcm = kcm.to_string();
        row.connect_activated(move |_| {
            let _ = Command::new("kcmshell6").arg(&kcm).spawn();
        });
        group.add(&row);
    }
    group
}

pub fn build() -> gtk4::Widget {
    let prefs_page = adw::PreferencesPage::new();
    prefs_page.add(&kde_link_group("Appearance", &[
        ("Global theme",      "Plasma themes, color scheme",     "kcm_lookandfeel"),
        ("Colors",            "Accent color and scheme",          "kcm_colors"),
        ("Icons",             "System icon theme",                "kcm_icons"),
        ("Cursors",           "Mouse cursor style",               "kcm_cursortheme"),
        ("Fonts",             "System-wide fonts",                "kcm_fonts"),
        ("Window decorations","Title bar and border style",       "kcm_kwindecoration"),
    ]));
    prefs_page.add(&kde_link_group("Desktop", &[
        ("Wallpaper",         "Desktop background image",         "kcm_wallpaper"),
        ("Desktop effects",   "Animations and compositing",       "kcm_kwin_effects"),
    ]));

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&prefs_page));
    toolbar_view.upcast()
}
