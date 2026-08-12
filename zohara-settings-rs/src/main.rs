mod pages;
mod backend;

use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

fn main() {
    let app = adw::Application::builder()
        .application_id("os.zohara.Settings")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_decoration_layout(Some("icon:minimize,maximize,close"));
    }

    // Main navigation split view — sidebar + content (like Win11 Settings)
    let nav_split = adw::NavigationSplitView::new();
    nav_split.set_min_sidebar_width(240.0);
    nav_split.set_max_sidebar_width(280.0);

    // ── Sidebar ────────────────────────────────────────────────────────────────
    let sidebar_page = adw::NavigationPage::builder()
        .title("Settings")
        .build();

    let sidebar_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    // Header bar for the sidebar
    let sidebar_header = adw::HeaderBar::new();
    sidebar_header.set_show_end_title_buttons(false);

    // Search bar
    let search_entry = gtk4::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Find a setting"));
    search_entry.set_margin_start(12);
    search_entry.set_margin_end(12);
    search_entry.set_margin_top(8);
    search_entry.set_margin_bottom(8);

    // Nav list
    let nav_list = gtk4::ListBox::new();
    nav_list.set_css_classes(&["navigation-sidebar"]);
    nav_list.set_selection_mode(gtk4::SelectionMode::Single);

    let scroll = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .child(&nav_list)
        .build();

    sidebar_box.append(&sidebar_header);
    sidebar_box.append(&search_entry);
    sidebar_box.append(&scroll);
    sidebar_page.set_child(Some(&sidebar_box));
    nav_split.set_sidebar(Some(&sidebar_page));

    // ── Content area ───────────────────────────────────────────────────────────
    let content_nav = adw::NavigationView::new();
    let content_page = adw::NavigationPage::builder()
        .title("System")
        .build();

    // Start with System page
    let system_content = pages::system::build();
    content_page.set_child(Some(&system_content));
    content_nav.push(&content_page);

    let content_wrapper = adw::NavigationPage::builder()
        .title("Settings")
        .build();
    content_wrapper.set_child(Some(&content_nav));
    nav_split.set_content(Some(&content_wrapper));

    // ── Navigation items ───────────────────────────────────────────────────────
    struct NavItem {
        label: &'static str,
        icon:  &'static str,
    }

    let items = [
        NavItem { label: "System",              icon: "computer-symbolic"            },
        NavItem { label: "Network &amp; internet",  icon: "network-wireless-symbolic"    },
        NavItem { label: "Bluetooth &amp; devices", icon: "bluetooth-symbolic"           },
        NavItem { label: "Personalization",     icon: "preferences-desktop-symbolic" },
        NavItem { label: "Apps",                icon: "application-x-executable-symbolic" },
        NavItem { label: "Accounts",            icon: "system-users-symbolic"        },
        NavItem { label: "Gaming",              icon: "applications-games-symbolic"  },
        NavItem { label: "Time &amp; language",     icon: "preferences-system-time-symbolic" },
        NavItem { label: "Accessibility",       icon: "preferences-desktop-accessibility-symbolic" },
        NavItem { label: "Privacy &amp; security",  icon: "security-high-symbolic"       },
        NavItem { label: "Zohara Update",       icon: "system-software-update-symbolic" },
        NavItem { label: "Advanced (KDE)",      icon: "configure-symbolic"           },
    ];

    for item in &items {
        let row = adw::ActionRow::builder()
            .title(item.label)
            .activatable(true)
            .build();

        let icon = gtk4::Image::from_icon_name(item.icon);
        row.add_prefix(&icon);

        let chevron = gtk4::Image::from_icon_name("go-next-symbolic");
        chevron.set_css_classes(&["dim-label"]);
        row.add_suffix(&chevron);

        nav_list.append(&row);
    }

    // Handle row selection → swap content page
    let content_nav_clone = content_nav.clone();
    let content_page_clone = content_page.clone();
    nav_list.connect_row_activated(move |_, row| {
        let index = row.index();
        let (title, child): (&str, gtk4::Widget) = match index {
            0  => ("System",              pages::system::build().upcast()),
            1  => ("Network & internet",  pages::network::build().upcast()),
            2  => ("Bluetooth & devices", pages::bluetooth::build().upcast()),
            3  => ("Personalization",     pages::personalization::build().upcast()),
            4  => ("Apps",               pages::apps::build().upcast()),
            5  => ("Accounts",           pages::accounts::build().upcast()),
            6  => ("Gaming",             pages::gaming::build().upcast()),
            7  => ("Time & language",    pages::time_language::build().upcast()),
            8  => ("Accessibility",      pages::accessibility::build().upcast()),
            9  => ("Privacy & security", pages::privacy::build().upcast()),
            10 => ("Zohara Update",      pages::updates::build().upcast()),
            11 => ("Advanced (KDE)",     pages::advanced::build().upcast()),
            _  => return,
        };

        content_page_clone.set_title(title);
        content_page_clone.set_child(Some(&child));
    });

    // Select first item by default
    nav_list.select_row(nav_list.row_at_index(0).as_ref());

    // ── Window ─────────────────────────────────────────────────────────────────
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Settings")
        .default_width(1060)
        .default_height(740)
        .content(&nav_split)
        .build();

    window.present();
}
