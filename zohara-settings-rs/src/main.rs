mod pages;
mod backend;

use gtk4::prelude::*;
use libadwaita as adw;

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn tokio_runtime() -> &'static Runtime {
    TOKIO_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to initialize Tokio runtime")
    })
}

// CSS is now loaded from data/win11.css via include_str! so the file can be
// edited with normal CSS tooling, and a future light-theme variant can be
// loaded conditionally. The macro embeds the file contents at compile time,
// so no runtime path lookup is needed.
const WIN11_CSS: &str = include_str!("../data/win11.css");

fn main() {
    // `--version` is checked before the GTK app is constructed, because it
    // doesn't need an event loop or a display. This is the version the
    // update page reads via `zohara-settings --version` and compares to
    // the manifest served at https://github.com/Zohaib8090/zohara/releases/
    // latest/download/latest.json.
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("zohara-settings {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let rt = tokio_runtime();
    let _rt_guard = rt.enter();

    let app = adw::Application::builder()
        .application_id("os.zohara.Settings")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

// ── Page registry (single source of truth) ────────────────────────────────────

struct PageDef {
    label: &'static str,
    icon:  &'static str,
}

static PAGES: &[PageDef] = &[
    PageDef { label: "Home",                 icon: "user-home-symbolic" },
    PageDef { label: "System",               icon: "computer-symbolic" },
    PageDef { label: "Bluetooth & devices",  icon: "bluetooth-symbolic" },
    PageDef { label: "Network & internet",   icon: "network-wireless-symbolic" },
    PageDef { label: "Personalization",      icon: "preferences-desktop-symbolic" },
    PageDef { label: "Apps",                 icon: "application-x-executable-symbolic" },
    PageDef { label: "Accounts",             icon: "system-users-symbolic" },
    PageDef { label: "Time & language",      icon: "preferences-system-time-symbolic" },
    PageDef { label: "Gaming",               icon: "applications-games-symbolic" },
    PageDef { label: "Accessibility",        icon: "preferences-desktop-accessibility-symbolic" },
    PageDef { label: "Privacy & security",   icon: "security-high-symbolic" },
    PageDef { label: "Windows Update",       icon: "system-software-update-symbolic" },
];

/// Build a page widget by its index into PAGES.
fn build_page(index: usize) -> gtk4::Widget {
    match index {
        0  => pages::home::build(),
        1  => pages::system::build(),
        2  => pages::bluetooth::build(),
        3  => pages::network::build(),
        4  => pages::personalization::build(),
        5  => pages::apps::build(),
        6  => pages::accounts::build(),
        7  => pages::time_language::build(),
        8  => pages::gaming::build(),
        9  => pages::accessibility::build(),
        10 => pages::privacy::build(),
        11 => pages::updates::build(),
        _  => unreachable!("Page index {} out of range", index),
    }
}

// ── UI ────────────────────────────────────────────────────────────────────────

fn build_ui(app: &adw::Application) {
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_decoration_layout(Some("icon:minimize,maximize,close"));
        // Respect the system light/dark preference instead of forcing dark.
        // The previous behaviour called set_gtk_application_prefer_dark_theme(true)
        // here, which made the Settings app ignore the user's Color mode toggle
        // on the Personalization page. The CSS in data/win11.css is dark-only,
        // so a future light theme would need to be loaded conditionally.
    }

    // Load custom Windows 11 Dark Acrylic CSS
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(WIN11_CSS);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let page_cache: Rc<RefCell<Vec<Option<gtk4::Widget>>>> =
        Rc::new(RefCell::new(vec![None; PAGES.len()]));

    // ── Main Layout Split ─────────────────────────────────────────────────────
    let root_h_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);

    // ── Left Navigation Sidebar ───────────────────────────────────────────────
    let sidebar_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    sidebar_box.set_css_classes(&["win11-sidebar"]);
    sidebar_box.set_size_request(260, -1);

    // 1. User Profile Header Card
    let user_name = std::env::var("USER").unwrap_or_else(|_| "zohaib".to_string());
    let user_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    user_card.set_css_classes(&["win11-user-card"]);

    let avatar_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    avatar_box.set_css_classes(&["win11-avatar-circle"]);
    let avatar_icon = gtk4::Image::from_icon_name("avatar-default-symbolic");
    avatar_icon.set_pixel_size(26);
    avatar_icon.set_valign(gtk4::Align::Center);
    avatar_icon.set_halign(gtk4::Align::Center);
    avatar_box.append(&avatar_icon);
    user_card.append(&avatar_box);

    let user_texts = gtk4::Box::new(gtk4::Orientation::Vertical, 1);
    user_texts.set_valign(gtk4::Align::Center);
    let u_name = gtk4::Label::builder()
        .label(&format!("{} baig", user_name))
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-user-name".to_string()])
        .build();
    let u_email = gtk4::Label::builder()
        .label(&format!("{}baig8090@outlook.com", user_name))
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-user-email".to_string()])
        .build();
    user_texts.append(&u_name);
    user_texts.append(&u_email);
    user_card.append(&user_texts);
    sidebar_box.append(&user_card);

    // 2. Navigation Items List
    let nav_list = gtk4::ListBox::new();
    nav_list.set_css_classes(&["win11-nav-list"]);
    nav_list.set_selection_mode(gtk4::SelectionMode::Single);

    for (i, page_def) in PAGES.iter().enumerate() {
        let row_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        let icon = gtk4::Image::from_icon_name(page_def.icon);
        icon.set_pixel_size(18);

        // Icon accent color styling
        match i {
            0 => icon.set_css_classes(&["accent-orange"]),
            1 => icon.set_css_classes(&["accent-blue"]),
            2 => icon.set_css_classes(&["accent-blue"]),
            3 => icon.set_css_classes(&["accent-blue"]),
            4 => icon.set_css_classes(&["accent-orange"]),
            5 => icon.set_css_classes(&["accent-blue"]),
            6 => icon.set_css_classes(&["accent-green"]),
            7 => icon.set_css_classes(&["accent-blue"]),
            8 => icon.set_css_classes(&["accent-purple"]),
            9 => icon.set_css_classes(&["accent-blue"]),
            10 => icon.set_css_classes(&["accent-blue"]),
            11 => icon.set_css_classes(&["accent-cyan"]),
            _ => (),
        }

        let lbl = gtk4::Label::builder()
            .label(page_def.label)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();

        row_box.append(&icon);
        row_box.append(&lbl);

        let row = gtk4::ListBoxRow::builder()
            .child(&row_box)
            .build();
        row.set_widget_name(&i.to_string());
        nav_list.append(&row);
    }

    let nav_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .child(&nav_list)
        .build();
    sidebar_box.append(&nav_scroll);
    root_h_box.append(&sidebar_box);

    // ── Right Main Content Area ───────────────────────────────────────────────
    let main_content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    main_content_box.set_hexpand(true);
    main_content_box.set_vexpand(true);

    // Centered Top Header Bar with Windows 11 Pill Search Bar
    let header = adw::HeaderBar::new();
    header.set_show_title(false);

    let search_entry = gtk4::SearchEntry::builder()
        .placeholder_text("Find a setting")
        .css_classes(vec!["win11-search".to_string()])
        .build();
    header.set_title_widget(Some(&search_entry));
    main_content_box.append(&header);

    // Content container
    let content_stack = gtk4::Stack::new();
    content_stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    content_stack.set_transition_duration(150);
    content_stack.set_vexpand(true);
    content_stack.set_hexpand(true);

    // Load initial page (Home, index 0)
    let home_widget = build_page(0);
    page_cache.borrow_mut()[0] = Some(home_widget.clone());
    content_stack.add_named(&home_widget, Some("page_0"));
    content_stack.set_visible_child_name("page_0");

    main_content_box.append(&content_stack);
    root_h_box.append(&main_content_box);

    // ── Search Filtering across Sidebar ───────────────────────────────────────
    let search_clone = search_entry.clone();
    nav_list.set_filter_func(move |row| {
        let q = search_clone.text().to_lowercase();
        if q.is_empty() { return true; }
        if let Ok(idx) = row.widget_name().parse::<usize>() {
            if idx < PAGES.len() {
                return PAGES[idx].label.to_lowercase().contains(&q);
            }
        }
        true
    });
    let nav_list_for_search = nav_list.clone();
    search_entry.connect_search_changed(move |_| nav_list_for_search.invalidate_filter());

    // ── Row Navigation Switching ──────────────────────────────────────────────
    let cache = page_cache.clone();
    let stack_clone = content_stack.clone();
    nav_list.connect_row_activated(move |_, row| {
        let idx: usize = match row.widget_name().parse() {
            Ok(i) if i < PAGES.len() => i,
            _ => return,
        };

        let page_tag = format!("page_{}", idx);
        let mut cache = cache.borrow_mut();
        if cache[idx].is_none() {
            let widget = build_page(idx);
            stack_clone.add_named(&widget, Some(&page_tag));
            cache[idx] = Some(widget);
        }
        stack_clone.set_visible_child_name(&page_tag);
    });

    if let Some(first_row) = nav_list.row_at_index(0) {
        nav_list.select_row(Some(&first_row));
    }

    // ── Window Setup ──────────────────────────────────────────────────────────
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Settings")
        .default_width(1120)
        .default_height(760)
        .content(&root_h_box)
        .css_classes(vec!["win11-window".to_string()])
        .build();

    window.present();
}
