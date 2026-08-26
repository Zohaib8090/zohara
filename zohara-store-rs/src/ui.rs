use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use glib;
use std::rc::Rc;
use std::cell::RefCell;

use crate::app_info::{get_curated_apps, AppCategory, AppInfo};
use crate::backend::{self, InstalledCache};

const CSS: &str = r#"
.nav-bar {
    background: alpha(@headerbar_bg_color, 0.95);
    border-bottom: 1px solid alpha(@borders, 0.4);
    padding: 0 12px;
}
.nav-pill {
    border-radius: 20px;
    padding: 5px 18px;
    font-weight: 600;
    font-size: 13px;
    min-height: 0;
    border: none;
    background: transparent;
    transition: all 150ms ease;
}
.nav-pill:checked {
    background: @accent_color;
    color: white;
}
.search-row {
    background: alpha(@card_bg_color, 0.7);
    border-bottom: 1px solid alpha(@borders, 0.3);
    padding: 8px 20px;
}
.banner-card {
    border-radius: 20px;
    padding: 36px 40px;
    min-height: 200px;
}
.banner-blue { background: linear-gradient(135deg, #1a73e8 0%, #0d47a1 100%); }
.banner-purple { background: linear-gradient(135deg, #7c3aed 0%, #4c1d95 100%); }
.banner-teal { background: linear-gradient(135deg, #0891b2 0%, #065f46 100%); }
.banner-orange { background: linear-gradient(135deg, #ea580c 0%, #7c2d12 100%); }
.banner-title {
    font-size: 26px;
    font-weight: 800;
    color: white;
    letter-spacing: -0.5px;
}
.banner-subtitle {
    font-size: 14px;
    color: alpha(white, 0.8);
    margin-top: 4px;
}
.banner-btn {
    border-radius: 20px;
    padding: 6px 22px;
    font-weight: 700;
    background: white;
    color: #1a1a2e;
    border: none;
    margin-top: 18px;
}
.banner-btn:hover { background: alpha(white, 0.9); }
.section-label {
    font-size: 18px;
    font-weight: 800;
    letter-spacing: -0.3px;
}
.see-all-btn {
    font-size: 12px;
    font-weight: 600;
    color: @accent_color;
    background: transparent;
    border: none;
    padding: 4px 8px;
}
.app-tile {
    border-radius: 16px;
    padding: 16px 14px;
    background: @card_bg_color;
    min-width: 152px;
    max-width: 152px;
    border: 1px solid alpha(@borders, 0.4);
    transition: all 180ms ease;
}
.app-tile:hover {
    background: alpha(@accent_color, 0.06);
    border-color: alpha(@accent_color, 0.3);
}
.tile-name {
    font-weight: 700;
    font-size: 13px;
}
.tile-pub {
    font-size: 11px;
    color: alpha(@foreground_color, 0.55);
}
.tile-free {
    font-size: 11px;
    color: @success_color;
    font-weight: 700;
}
.tile-btn {
    border-radius: 14px;
    padding: 4px 14px;
    font-size: 12px;
    font-weight: 600;
    min-height: 0;
}
.cat-pill {
    border-radius: 14px;
    padding: 4px 12px;
    font-size: 11px;
    font-weight: 700;
    background: alpha(#3584e4, 0.18);
    color: #1c71d8;
    border: 1px solid alpha(#3584e4, 0.35);
    min-height: 0;
    transition: all 150ms ease;
}
.cat-pill:checked, .cat-pill:hover {
    background: #3584e4;
    color: #ffffff;
    border-color: #3584e4;
}
.list-row-icon { margin: 6px 4px; }
.rating-label {
    font-size: 11px;
    color: alpha(@foreground_color, 0.55);
    font-weight: 600;
}
.page-title {
    font-size: 22px;
    font-weight: 800;
    letter-spacing: -0.4px;
}
"#;

pub fn build() -> gtk4::Widget {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display, &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let tv = adw::ToolbarView::new();

    // ── HeaderBar ────────────────────────────────────────────────────────────
    let header = adw::HeaderBar::new();
    header.set_show_title(false);

    // App icon on the left
    let app_icon = gtk4::Image::from_icon_name("system-software-install");
    app_icon.set_pixel_size(22);
    app_icon.set_margin_end(4);
    header.pack_start(&app_icon);

    let app_label = gtk4::Label::new(Some("Zohara Store"));
    app_label.add_css_class("title-4");
    header.pack_start(&app_label);

    // Nav tabs in center
    let home_btn = gtk4::ToggleButton::with_label("Home");
    home_btn.add_css_class("nav-pill");
    home_btn.set_active(true);

    let apps_btn = gtk4::ToggleButton::with_label("Apps");
    apps_btn.add_css_class("nav-pill");
    apps_btn.set_group(Some(&home_btn));

    let games_btn = gtk4::ToggleButton::with_label("Games");
    games_btn.add_css_class("nav-pill");
    games_btn.set_group(Some(&home_btn));

    let nav = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    nav.set_hexpand(true);
    nav.set_halign(gtk4::Align::Center);
    nav.append(&home_btn);
    nav.append(&apps_btn);
    nav.append(&games_btn);
    header.set_title_widget(Some(&nav));

    // Search toggle
    let search_toggle = gtk4::ToggleButton::new();
    search_toggle.set_icon_name("system-search-symbolic");
    search_toggle.set_tooltip_text(Some("Search (Ctrl+F)"));
    header.pack_end(&search_toggle);
    tv.add_top_bar(&header);

    // ── Search Bar ───────────────────────────────────────────────────────────
    let search_bar = gtk4::SearchBar::new();
    search_bar.add_css_class("search-row");
    let search_entry = gtk4::SearchEntry::new();
    search_entry.set_hexpand(true);
    search_entry.set_placeholder_text(Some("Search apps, games, and more…"));
    search_bar.set_child(Some(&search_entry));
    search_toggle
        .bind_property("active", &search_bar, "search-mode-enabled")
        .bidirectional().sync_create().build();
    tv.add_top_bar(&search_bar);

    // ── Installed cache — 2 subprocess calls total, done here once ───────────
    let cache = Rc::new(InstalledCache::load());

    // ── Stack ────────────────────────────────────────────────────────────────
    let stack = gtk4::Stack::new();
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.set_transition_duration(150);

    let home_page = build_home_page(cache.clone());
    let apps_page = build_browse_page(false, cache.clone());
    let games_page = build_browse_page(true, cache.clone());
    let (search_page, search_list_box) = build_search_page();

    stack.add_named(&home_page,   Some("home"));
    stack.add_named(&apps_page,   Some("apps"));
    stack.add_named(&games_page,  Some("games"));
    stack.add_named(&search_page, Some("search"));

    // Nav wiring
    let s = stack.clone(); home_btn.connect_toggled(move |b| { if b.is_active() { s.set_visible_child_name("home"); } });
    let s = stack.clone(); apps_btn.connect_toggled(move |b| { if b.is_active() { s.set_visible_child_name("apps"); } });
    let s = stack.clone(); games_btn.connect_toggled(move |b| { if b.is_active() { s.set_visible_child_name("games"); } });

    // Search wiring
    let stack_s = stack.clone();
    let hb = home_btn.clone();
    let slb = search_list_box.clone();
    
    search_entry.connect_search_changed(move |entry| {
        let text = entry.text().to_string();
        let query = text.trim().to_string();
        
        if query.len() < 2 {
            if query.is_empty() {
                hb.set_active(true);
                stack_s.set_visible_child_name("home");
            }
            return;
        }
        
        stack_s.set_visible_child_name("search");
        
        let slb_c = slb.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        
        std::thread::spawn(move || {
            let results = backend::search_apps(&query);
            let fresh_cache = InstalledCache::load();
            let _ = tx.send((results, fresh_cache));
        });
        
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            if let Ok((results, fresh_cache)) = rx.try_recv() {
                populate_list(&slb_c, results, &fresh_cache);
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    });

    tv.set_content(Some(&stack));
    tv.upcast()
}

// ── Home Page ─────────────────────────────────────────────────────────────────
fn build_home_page(cache: Rc<InstalledCache>) -> gtk4::Widget {
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let page = gtk4::Box::new(gtk4::Orientation::Vertical, 36);
    page.set_margin_top(28);
    page.set_margin_bottom(40);
    page.set_margin_start(28);
    page.set_margin_end(28);

    page.append(&build_banner_carousel());
    page.append(&build_tile_section("🔥  Editor's Picks", picks_apps(), cache.clone()));
    page.append(&build_tile_section("🎮  Top Games", top_games(), cache.clone()));
    page.append(&build_tile_section("🛠️  Developer Tools", dev_apps(), cache.clone()));

    scroll.set_child(Some(&page));
    scroll.upcast()
}

fn build_banner_carousel() -> gtk4::Widget {
    let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 10);

    // Main big banner
    let big = build_banner(
        "Mozilla Firefox",
        "The fast, private browser loved by millions",
        "banner-blue",
        "web-browser",
        "firefox",
    );
    big.set_height_request(220);
    outer.append(&big);

    // Row of 3 smaller banners
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    for (name, sub, css, icon, id) in [
        ("Steam", "Access thousands of PC games", "banner-purple", "applications-games", "steam"),
        ("GIMP", "Professional image editing", "banner-teal", "image-x-generic", "gimp"),
        ("VSCodium", "Code. Build. Deploy.", "banner-orange", "text-editor", "vscodium"),
    ] {
        let card = build_banner(name, sub, css, icon, id);
        card.set_hexpand(true);
        card.set_height_request(130);
        row.append(&card);
    }
    outer.append(&row);
    outer.upcast()
}

fn build_banner(name: &str, subtitle: &str, css: &str, icon: &str, pkg_id: &str) -> gtk4::Box {
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    card.add_css_class("banner-card");
    card.add_css_class(css);

    let ico = gtk4::Image::from_icon_name(icon);
    ico.set_pixel_size(52);
    ico.set_halign(gtk4::Align::Start);
    card.append(&ico);

    let t = gtk4::Label::new(Some(name));
    t.add_css_class("banner-title");
    t.set_xalign(0.0);
    card.append(&t);

    let s = gtk4::Label::new(Some(subtitle));
    s.add_css_class("banner-subtitle");
    s.set_xalign(0.0);
    card.append(&s);

    let btn = gtk4::Button::with_label("Get Now");
    btn.add_css_class("banner-btn");
    btn.set_halign(gtk4::Align::Start);
    let pkg = pkg_id.to_string();
    btn.connect_clicked(move |_| {
        // Find the app and trigger install
        if let Some(app) = get_curated_apps().into_iter().find(|a| a.id == pkg) {
            let source = app.source.clone();
            let pkg_name = app.package_name.clone();
            std::thread::spawn(move || {
                backend::install_app(&source, &pkg_name);
            });
        }
    });
    card.append(&btn);
    card
}

fn build_tile_section(title: &str, apps: Vec<AppInfo>, cache: Rc<InstalledCache>) -> gtk4::Widget {
    let section = gtk4::Box::new(gtk4::Orientation::Vertical, 14);

    // Header row
    let hdr = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    hdr.set_hexpand(true);
    let heading = gtk4::Label::new(Some(title));
    heading.add_css_class("section-label");
    heading.set_xalign(0.0);
    heading.set_hexpand(true);
    let see_all = gtk4::Button::with_label("See all");
    see_all.add_css_class("see-all-btn");
    hdr.append(&heading);
    hdr.append(&see_all);
    section.append(&hdr);

    let hscroll = gtk4::ScrolledWindow::new();
    hscroll.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
    let cards = gtk4::Box::new(gtk4::Orientation::Horizontal, 14);
    cards.set_margin_bottom(6);

    for app in apps {
        cards.append(&build_app_tile(app, &cache));
    }

    hscroll.set_child(Some(&cards));
    section.append(&hscroll);
    section.upcast()
}

fn build_app_tile(app: AppInfo, cache: &InstalledCache) -> gtk4::Widget {
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    card.add_css_class("app-tile");

    let icon = create_app_icon(&app.icon_name, &app.id, 56);
    icon.set_halign(gtk4::Align::Center);
    icon.set_margin_bottom(4);
    card.append(&icon);

    let name = gtk4::Label::new(Some(&app.name));
    name.add_css_class("tile-name");
    name.set_halign(gtk4::Align::Center);
    name.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name.set_max_width_chars(14);
    name.set_justify(gtk4::Justification::Center);
    card.append(&name);

    let pub_lbl = gtk4::Label::new(Some(&app.publisher));
    pub_lbl.add_css_class("tile-pub");
    pub_lbl.set_halign(gtk4::Align::Center);
    pub_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    pub_lbl.set_max_width_chars(16);
    card.append(&pub_lbl);

    // Stars + rating
    let stars = format!("★ {:.1}", app.rating);
    let rating_lbl = gtk4::Label::new(Some(&stars));
    rating_lbl.add_css_class("rating-label");
    rating_lbl.set_halign(gtk4::Align::Center);
    card.append(&rating_lbl);

    let free_lbl = gtk4::Label::new(Some("Free"));
    free_lbl.add_css_class("tile-free");
    free_lbl.set_halign(gtk4::Align::Center);
    card.append(&free_lbl);

    let installed = cache.is_installed(&app.source, &app.package_name);
    let btn = gtk4::Button::with_label(if installed { "Remove" } else { "Get" });
    btn.add_css_class("tile-btn");
    if installed {
        btn.add_css_class("destructive-action");
    } else {
        btn.add_css_class("suggested-action");
    }
    btn.set_halign(gtk4::Align::Center);
    card.append(&btn);
    card.upcast()
}

// ── Browse Pages (Apps / Games) ───────────────────────────────────────────────
fn build_browse_page(games_only: bool, cache: Rc<InstalledCache>) -> gtk4::Widget {
    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    // Category pills
    let pills_scroll = gtk4::ScrolledWindow::new();
    pills_scroll.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
    pills_scroll.set_margin_top(16);
    pills_scroll.set_margin_bottom(12);
    pills_scroll.set_margin_start(24);
    pills_scroll.set_margin_end(24);

    let pills = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let all_btn = gtk4::ToggleButton::with_label("All");
    all_btn.add_css_class("cat-pill");
    all_btn.set_active(true);
    pills.append(&all_btn);

    let categories: Vec<(&str, AppCategory)> = if games_only {
        vec![("Arcade", AppCategory::Games)]
    } else {
        vec![
            ("Browsers", AppCategory::Browser),
            ("Development", AppCategory::Development),
            ("Multimedia", AppCategory::Multimedia),
            ("Communication", AppCategory::Communication),
            ("Graphics", AppCategory::Graphics),
            ("Productivity", AppCategory::Productivity),
            ("Utilities", AppCategory::Utilities),
        ]
    };

    for (label, _cat) in &categories {
        let btn = gtk4::ToggleButton::with_label(label);
        btn.add_css_class("cat-pill");
        btn.set_group(Some(&all_btn));
        pills.append(&btn);
    }
    pills_scroll.set_child(Some(&pills));
    vbox.append(&pills_scroll);

    // List
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(820);
    clamp.set_margin_bottom(24);
    clamp.set_margin_start(16);
    clamp.set_margin_end(16);

    let list_box = gtk4::ListBox::new();
    list_box.set_widget_name("main-listbox");
    list_box.add_css_class("boxed-list");
    list_box.set_selection_mode(gtk4::SelectionMode::None);

    let apps = get_curated_apps()
        .into_iter()
        .filter(|a| if games_only { a.category.is_game() } else { !a.category.is_game() })
        .collect();
    populate_list(&list_box, apps, &cache);

    clamp.set_child(Some(&list_box));
    scroll.set_child(Some(&clamp));
    vbox.append(&scroll);
    vbox.upcast()
}

// ── Search Results Page ───────────────────────────────────────────────────────
fn build_search_page() -> (gtk4::Widget, gtk4::ListBox) {
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(820);
    clamp.set_margin_top(16);
    clamp.set_margin_bottom(24);
    clamp.set_margin_start(16);
    clamp.set_margin_end(16);

    let inner = gtk4::Box::new(gtk4::Orientation::Vertical, 14);

    let lbl = gtk4::Label::new(Some("Search Results"));
    lbl.add_css_class("page-title");
    lbl.set_xalign(0.0);
    inner.append(&lbl);

    let list_box = gtk4::ListBox::new();
    list_box.set_widget_name("main-listbox");
    list_box.add_css_class("boxed-list");
    list_box.set_selection_mode(gtk4::SelectionMode::None);

    let placeholder = adw::ActionRow::new();
    placeholder.set_title("Start typing to search…");
    list_box.append(&placeholder);

    inner.append(&list_box);
    clamp.set_child(Some(&inner));
    scroll.set_child(Some(&clamp));
    (scroll.upcast(), list_box)
}

// ── Populate List ─────────────────────────────────────────────────────────────
fn populate_list(list_box: &gtk4::ListBox, apps: Vec<AppInfo>, cache: &InstalledCache) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
    if apps.is_empty() {
        let row = adw::ActionRow::new();
        row.set_title("No results found");
        row.set_subtitle("Try a different search term");
        list_box.append(&row);
        return;
    }
    for app in apps {
        list_box.append(&create_list_row(app, cache));
    }
}

fn create_list_row(app: AppInfo, cache: &InstalledCache) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(&app.name);
    row.set_subtitle(&format!("{} • {}  ★ {:.1}", app.publisher, app.description, app.rating));

    let icon = create_app_icon(&app.icon_name, &app.id, 44);
    icon.add_css_class("list-row-icon");
    row.add_prefix(&icon);

    let cat_lbl = gtk4::Label::new(Some(app.category.label()));
    cat_lbl.add_css_class("cat-pill");
    cat_lbl.set_valign(gtk4::Align::Center);

    let installed = cache.is_installed(&app.source, &app.package_name);
    let btn = gtk4::Button::new();
    btn.set_valign(gtk4::Align::Center);
    btn.add_css_class("pill");
    set_btn_label(&btn, installed);

    let pbar = gtk4::ProgressBar::new();
    pbar.set_valign(gtk4::Align::Center);
    pbar.set_pulse_step(0.1);
    pbar.set_visible(false);

    let suffix = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    suffix.set_valign(gtk4::Align::Center);
    suffix.append(&cat_lbl);
    suffix.append(&pbar);
    suffix.append(&btn);
    row.add_suffix(&suffix);

    let state = Rc::new(RefCell::new(AppState { installed, working: false }));
    let app_c = app.clone();
    let btn_c = btn.clone();
    let pbar_c = pbar.clone();
    let state_c = state.clone();

    btn.connect_clicked(move |_| {
        let mut s = state_c.borrow_mut();
        if s.working { return; }
        let was = s.installed;
        s.working = true;
        drop(s);

        btn_c.set_sensitive(false);
        btn_c.set_label(if was { "Removing…" } else { "Installing…" });
        pbar_c.set_visible(true);

        let (tx, rx) = std::sync::mpsc::channel();
        let app_a = app_c.clone();

        std::thread::spawn(move || {
            let ok = if was {
                backend::remove_app(&app_a.source, &app_a.package_name)
            } else {
                backend::install_app(&app_a.source, &app_a.package_name)
            };
            let _ = tx.send(ok);
        });

        let btn_a = btn_c.clone();
        let pbar_a = pbar_c.clone();
        let st_a = state_c.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            match rx.try_recv() {
                Ok(ok) => {
                    let mut s = st_a.borrow_mut();
                    s.working = false;
                    if ok { s.installed = !was; }
                    let now = s.installed;
                    drop(s);

                    pbar_a.set_visible(false);
                    btn_a.set_sensitive(true);
                    set_btn_label(&btn_a, now);
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    pbar_a.pulse();
                    glib::ControlFlow::Continue
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let mut s = st_a.borrow_mut();
                    s.working = false;
                    drop(s);

                    pbar_a.set_visible(false);
                    btn_a.set_sensitive(true);
                    set_btn_label(&btn_a, was);
                    glib::ControlFlow::Break
                }
            }
        });
    });

    row
}

// ── Curated subsets ───────────────────────────────────────────────────────────
fn picks_apps() -> Vec<AppInfo> {
    get_curated_apps().into_iter().filter(|a| {
        matches!(a.id.as_str(), "firefox" | "vlc" | "gimp" | "discord" | "libreoffice" | "obsidian")
    }).collect()
}

fn top_games() -> Vec<AppInfo> {
    get_curated_apps().into_iter().filter(|a| a.category.is_game()).collect()
}

fn dev_apps() -> Vec<AppInfo> {
    get_curated_apps().into_iter().filter(|a| {
        matches!(a.category, crate::app_info::AppCategory::Development)
            || matches!(a.id.as_str(), "vscodium" | "git" | "htop")
    }).collect()
}

fn create_app_icon(icon_name: &str, app_id: &str, size: i32) -> gtk4::Image {
    let display = gtk4::gdk::Display::default();
    let icon_theme = display.map(|d| gtk4::IconTheme::for_display(&d));

    let mapped = icon_for(app_id);
    let candidates = [
        icon_name,
        mapped,
        app_id,
        "system-run",
        "package-x-generic",
        "preferences-other",
    ];

    if let Some(theme) = icon_theme {
        for name in candidates {
            if !name.is_empty() && theme.has_icon(name) {
                let img = gtk4::Image::from_icon_name(name);
                img.set_pixel_size(size);
                return img;
            }
        }
    }

    let img = gtk4::Image::from_icon_name("system-run");
    img.set_pixel_size(size);
    img
}

fn icon_for(id: &str) -> &'static str {
    match id {
        "firefox"          => "firefox",
        "chromium"         => "chromium",
        "vscodium"         => "vscodium",
        "git"              => "git",
        "spotify"          => "spotify-client",
        "discord"          => "discord",
        "telegram-desktop" => "telegram",
        "vlc"              => "vlc",
        "mpv"              => "mpv",
        "gimp"             => "gimp",
        "inkscape"         => "inkscape",
        "krita"            => "krita",
        "steam"            => "steam",
        "lutris"           => "lutris",
        "heroic"           => "heroic",
        "supertuxkart"     => "supertuxkart",
        "0ad"              => "0ad",
        "libreoffice"      => "libreoffice-main",
        "obsidian"         => "obsidian",
        "htop"             => "htop",
        "timeshift"        => "timeshift",
        _                  => "system-run",
    }
}

struct AppState { installed: bool, working: bool }

fn set_btn_label(btn: &gtk4::Button, installed: bool) {
    if installed {
        btn.set_label("Remove");
        btn.remove_css_class("suggested-action");
        btn.add_css_class("destructive-action");
    } else {
        btn.set_label("Get");
        btn.remove_css_class("destructive-action");
        btn.add_css_class("suggested-action");
    }
}
