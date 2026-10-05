//! The Store's browsing screens: Home, Apps, Games and search. (The Updates page is `updates_ui.rs`.)
//!
//! Every app is shown by the same card (`app_card`): icon, name, publisher, a two-line description and a Get/Remove
//! button; clicking the card opens the details dialog. All buttons for one app stay in step through the shared
//! `InstalledCache`. Search runs on an in-memory copy of the catalog, loaded once in the background, so typing never
//! starts a program or waits on one.

use adw::prelude::*;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::app_info::{get_curated_apps, AppCategory, AppInfo, AppSource};
use crate::backend::{self, InstalledCache};

type Cache = Rc<InstalledCache>;

/// Most results shown for one search; more than this is a longer list than anyone reads.
const SEARCH_LIMIT: usize = 40;

const CSS: &str = r#"
.nav-pill {
    border-radius: 20px;
    padding: 5px 18px;
    font-weight: 600;
    min-height: 0;
    border: none;
    background: transparent;
}
.nav-pill:checked { background: @accent_bg_color; color: @accent_fg_color; }
.search-row { padding: 6px 20px; }

.hero {
    border-radius: 20px;
    padding: 32px 36px;
    min-height: 190px;
    color: white;
}
.hero-blue { background: linear-gradient(135deg, #1a73e8 0%, #0d47a1 100%); }
.hero-purple { background: linear-gradient(135deg, #7c3aed 0%, #4c1d95 100%); }
.hero-teal { background: linear-gradient(135deg, #0891b2 0%, #065f46 100%); }
.hero-orange { background: linear-gradient(135deg, #ea580c 0%, #7c2d12 100%); }
.hero-title { font-size: 26px; font-weight: 800; color: white; }
.hero-sub { font-size: 14px; color: alpha(white, 0.85); }
.hero-btn, .hero-btn.suggested-action, .hero-btn.destructive-action {
    background: white; color: #1a1a2e; border: none; border-radius: 20px; padding: 6px 24px; font-weight: 700;
}

.section-title { font-size: 19px; font-weight: 800; }
.section-sub { font-size: 13px; opacity: 0.65; }
.page-title { font-size: 24px; font-weight: 800; }

.app-card {
    background: @card_bg_color;
    color: @card_fg_color;
    border-radius: 14px;
    padding: 14px 16px;
    border: 1px solid alpha(currentColor, 0.1);
    transition: border-color 150ms ease, background 150ms ease;
}
.app-card:hover { border-color: alpha(@accent_bg_color, 0.7); background: mix(@card_bg_color, @accent_bg_color, 0.07); }
.card-name { font-weight: 700; font-size: 15px; }
.card-pub { font-size: 12px; opacity: 0.6; }
.card-desc { font-size: 13px; opacity: 0.85; }
.card-meta { font-size: 12px; opacity: 0.6; }
.card-btn { border-radius: 16px; padding: 4px 18px; min-height: 0; font-weight: 600; }

.chip {
    border-radius: 16px;
    padding: 4px 14px;
    min-height: 0;
    font-size: 13px;
    font-weight: 600;
    background: alpha(currentColor, 0.07);
    border: none;
}
.chip:checked { background: @accent_bg_color; color: @accent_fg_color; }
.badge {
    font-size: 11px; font-weight: 700; padding: 2px 10px; border-radius: 10px;
    background: alpha(currentColor, 0.1);
}
"#;

// ── Window ────────────────────────────────────────────────────────────────────

pub fn build() -> gtk4::Widget {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    let tv = adw::ToolbarView::new();

    // ── Header: title, navigation tabs, search ──
    let header = adw::HeaderBar::new();
    let app_icon = gtk4::Image::from_icon_name("system-software-install");
    app_icon.set_pixel_size(22);
    header.pack_start(&app_icon);
    let app_label = gtk4::Label::new(Some("Zohara Store"));
    app_label.add_css_class("title-4");
    header.pack_start(&app_label);

    let home_btn = nav_pill("Home", None);
    home_btn.set_active(true);
    let apps_btn = nav_pill("Apps", Some(&home_btn));
    let games_btn = nav_pill("Games", Some(&home_btn));
    let updates_btn = nav_pill("Updates", Some(&home_btn));
    let nav = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    nav.set_halign(gtk4::Align::Center);
    for b in [&home_btn, &apps_btn, &games_btn, &updates_btn] {
        nav.append(b);
    }
    header.set_title_widget(Some(&nav));

    let search_toggle = gtk4::ToggleButton::new();
    search_toggle.set_icon_name("system-search-symbolic");
    search_toggle.set_tooltip_text(Some("Search (Ctrl+F)"));
    header.pack_end(&search_toggle);
    tv.add_top_bar(&header);

    let search_bar = gtk4::SearchBar::new();
    search_bar.add_css_class("search-row");
    let search_entry = gtk4::SearchEntry::new();
    search_entry.set_hexpand(true);
    search_entry.set_search_delay(200);
    search_entry.set_placeholder_text(Some("Search apps, games, and more…"));
    search_bar.set_child(Some(&search_entry));
    search_toggle.bind_property("active", &search_bar, "search-mode-enabled").bidirectional().sync_create().build();
    tv.add_top_bar(&search_bar);

    // Ctrl+F opens the search box from anywhere in the window.
    {
        let st = search_toggle.clone();
        let trigger = gtk4::ShortcutTrigger::parse_string("<Control>f");
        let action = gtk4::CallbackAction::new(move |_, _| {
            st.set_active(true);
            glib::Propagation::Stop
        });
        let sc = gtk4::ShortcutController::new();
        sc.set_scope(gtk4::ShortcutScope::Global);
        if let Some(t) = trigger {
            sc.add_shortcut(gtk4::Shortcut::new(Some(t), Some(action)));
        }
        tv.add_controller(sc);
    }

    // ── Pages ──
    let cache: Cache = Rc::new(InstalledCache::load());
    let stack = gtk4::Stack::new();
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.set_transition_duration(120);

    // Sections' "See all" buttons switch tabs through the pills, so the highlighted tab stays right.
    let go_apps: Rc<dyn Fn()> = { let b = apps_btn.clone(); Rc::new(move || b.set_active(true)) };
    let go_games: Rc<dyn Fn()> = { let b = games_btn.clone(); Rc::new(move || b.set_active(true)) };

    stack.add_named(&build_home_page(&cache, go_apps, go_games), Some("home"));
    stack.add_named(&build_browse_page(false, &cache), Some("apps"));
    stack.add_named(&build_browse_page(true, &cache), Some("games"));
    let search_page = SearchPage::new();
    stack.add_named(&search_page.widget, Some("search"));
    stack.add_named(&crate::updates_ui::build_page(), Some("updates"));

    // The tab to return to when the search box is emptied.
    let last_tab = Rc::new(RefCell::new(String::from("home")));
    for (btn, name) in [(&home_btn, "home"), (&apps_btn, "apps"), (&games_btn, "games"), (&updates_btn, "updates")] {
        let (s, last) = (stack.clone(), last_tab.clone());
        btn.connect_toggled(move |b| {
            if b.is_active() {
                *last.borrow_mut() = name.to_string();
                s.set_visible_child_name(name);
            }
        });
    }
    // `zohara-store --page updates` (from the update notification) opens the Updates page.
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--page") {
        if args.get(i + 1).map(String::as_str) == Some("updates") {
            updates_btn.set_active(true);
        }
    }

    // ── Search: the catalog is read once in the background, then every search is in memory ──
    let catalog: Rc<RefCell<Vec<AppInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let catalog_ready = Rc::new(Cell::new(false));
    let curated = Rc::new(get_curated_apps());
    let search_page = Rc::new(search_page);

    let run_search: Rc<dyn Fn()> = {
        let (entry, stack, last, page, cache) = (search_entry.clone(), stack.clone(), last_tab.clone(), search_page.clone(), cache.clone());
        let (catalog, ready, curated) = (catalog.clone(), catalog_ready.clone(), curated.clone());
        Rc::new(move || {
            let query = entry.text().trim().to_string();
            if query.is_empty() {
                stack.set_visible_child_name(&last.borrow());
                return;
            }
            if query.chars().count() < 2 {
                return;
            }
            let results = backend::search_catalog(&curated, &catalog.borrow(), &query, SEARCH_LIMIT);
            page.show(&query, results, &cache, ready.get());
            stack.set_visible_child_name("search");
        })
    };
    {
        let run = run_search.clone();
        search_entry.connect_search_changed(move |_| run());
    }
    {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(backend::load_catalog());
        });
        let (catalog, ready, run) = (catalog.clone(), catalog_ready.clone(), run_search.clone());
        glib::timeout_add_local(std::time::Duration::from_millis(250), move || match rx.try_recv() {
            Ok(all) => {
                *catalog.borrow_mut() = all;
                ready.set(true);
                run(); // results typed before the catalog arrived get filled in now
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(_) => {
                ready.set(true);
                glib::ControlFlow::Break
            }
        });
    }

    tv.set_content(Some(&stack));
    tv.upcast()
}

fn nav_pill(label: &str, group: Option<&gtk4::ToggleButton>) -> gtk4::ToggleButton {
    let b = gtk4::ToggleButton::with_label(label);
    b.add_css_class("nav-pill");
    if let Some(g) = group {
        b.set_group(Some(g));
    }
    b
}

// ── Home ──────────────────────────────────────────────────────────────────────

fn build_home_page(cache: &Cache, go_apps: Rc<dyn Fn()>, go_games: Rc<dyn Fn()>) -> gtk4::Widget {
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(1100);
    clamp.set_tightening_threshold(900);

    let page = gtk4::Box::new(gtk4::Orientation::Vertical, 30);
    page.set_margin_top(24);
    page.set_margin_bottom(40);
    page.set_margin_start(24);
    page.set_margin_end(24);

    page.append(&hero_carousel(cache));
    page.append(&section(
        "Editor's picks",
        "Good places to start",
        picks_apps(),
        cache,
        Some(go_apps.clone()),
    ));
    page.append(&section("Top games", "Free games to try", top_games(), cache, Some(go_games)));
    page.append(&section("For developers", "Editors, version control and tools", dev_apps(), cache, Some(go_apps)));

    clamp.set_child(Some(&page));
    scroll.set_child(Some(&clamp));
    scroll.upcast()
}

/// A few featured apps, one per slide, with dots underneath; the slides turn by themselves every few seconds.
fn hero_carousel(cache: &Cache) -> gtk4::Widget {
    let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    let carousel = adw::Carousel::new();
    carousel.set_allow_scroll_wheel(false);

    let all = get_curated_apps();
    let mut slides = 0;
    for (id, css) in [("firefox", "hero-blue"), ("steam", "hero-purple"), ("gimp", "hero-teal"), ("vscodium", "hero-orange")] {
        let Some(app) = all.iter().find(|a| a.id == id) else { continue };
        let slide = gtk4::Box::new(gtk4::Orientation::Horizontal, 20);
        slide.set_hexpand(true);
        slide.add_css_class("hero");
        slide.add_css_class(css);
        let icon = create_app_icon(&app.icon_name, &app.id, 84);
        icon.set_valign(gtk4::Align::Center);
        slide.append(&icon);
        let col = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        col.set_valign(gtk4::Align::Center);
        col.set_hexpand(true);
        let t = gtk4::Label::new(Some(&app.name));
        t.add_css_class("hero-title");
        t.set_xalign(0.0);
        let s = gtk4::Label::new(Some(&app.description));
        s.add_css_class("hero-sub");
        s.set_xalign(0.0);
        s.set_wrap(true);
        let btn = install_button(app, cache);
        btn.add_css_class("hero-btn");
        btn.set_halign(gtk4::Align::Start);
        btn.set_margin_top(10);
        col.append(&t);
        col.append(&s);
        col.append(&btn);
        slide.append(&col);
        carousel.append(&slide);
        slides += 1;
    }
    outer.append(&carousel);
    if slides > 1 {
        let dots = adw::CarouselIndicatorDots::new();
        dots.set_carousel(Some(&carousel));
        outer.append(&dots);
        let weak = carousel.downgrade();
        glib::timeout_add_seconds_local(7, move || {
            let Some(c) = weak.upgrade() else { return glib::ControlFlow::Break };
            let n = c.n_pages();
            if n > 1 && c.is_mapped() {
                let next = (c.position().round() as u32 + 1) % n;
                c.scroll_to(&c.nth_page(next), true);
            }
            glib::ControlFlow::Continue
        });
    }
    outer.upcast()
}

fn section(title: &str, subtitle: &str, apps: Vec<AppInfo>, cache: &Cache, see_all: Option<Rc<dyn Fn()>>) -> gtk4::Widget {
    let sec = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    let hdr = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let texts = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    texts.set_hexpand(true);
    let h = gtk4::Label::new(Some(title));
    h.add_css_class("section-title");
    h.set_xalign(0.0);
    let sub = gtk4::Label::new(Some(subtitle));
    sub.add_css_class("section-sub");
    sub.set_xalign(0.0);
    texts.append(&h);
    texts.append(&sub);
    hdr.append(&texts);
    if let Some(go) = see_all {
        let b = gtk4::Button::with_label("See all");
        b.add_css_class("flat");
        b.set_valign(gtk4::Align::Center);
        b.connect_clicked(move |_| go());
        hdr.append(&b);
    }
    sec.append(&hdr);
    sec.append(&app_grid(&apps, cache));
    sec.upcast()
}

// ── Cards ─────────────────────────────────────────────────────────────────────

fn app_grid(apps: &[AppInfo], cache: &Cache) -> gtk4::FlowBox {
    let grid = gtk4::FlowBox::new();
    grid.set_selection_mode(gtk4::SelectionMode::None);
    grid.set_homogeneous(true);
    grid.set_min_children_per_line(1);
    grid.set_max_children_per_line(3);
    grid.set_column_spacing(14);
    grid.set_row_spacing(14);
    grid.set_valign(gtk4::Align::Start);
    for a in apps {
        grid.append(&app_card(a, cache));
    }
    grid
}

/// One app: icon, name, publisher, a two-line description, rating and the Get/Remove button. Clicking anywhere else
/// on the card opens the details dialog.
fn app_card(app: &AppInfo, cache: &Cache) -> gtk4::Widget {
    let card = gtk4::Box::new(gtk4::Orientation::Horizontal, 14);
    card.add_css_class("app-card");
    card.set_widget_name(app.category.label()); // read by the category chips on the Apps page
    card.set_cursor_from_name(Some("pointer"));

    let icon = create_app_icon(&app.icon_name, &app.id, 56);
    icon.set_valign(gtk4::Align::Start);
    card.append(&icon);

    let col = gtk4::Box::new(gtk4::Orientation::Vertical, 3);
    col.set_hexpand(true);
    let name = gtk4::Label::new(Some(&app.name));
    name.add_css_class("card-name");
    name.set_xalign(0.0);
    name.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    col.append(&name);

    let pub_lbl = gtk4::Label::new(Some(&app.publisher));
    pub_lbl.add_css_class("card-pub");
    pub_lbl.set_xalign(0.0);
    pub_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    col.append(&pub_lbl);

    let desc = gtk4::Label::new(Some(if app.description.is_empty() { "No description available." } else { &app.description }));
    desc.add_css_class("card-desc");
    desc.set_xalign(0.0);
    desc.set_yalign(0.0);
    desc.set_wrap(true);
    desc.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    desc.set_lines(2);
    desc.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    desc.set_width_chars(24);
    desc.set_max_width_chars(40);
    desc.set_margin_top(4);
    desc.set_vexpand(true);
    col.append(&desc);

    let foot = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    foot.set_margin_top(8);
    let meta = gtk4::Label::new(Some(&card_meta(app)));
    meta.add_css_class("card-meta");
    meta.set_hexpand(true);
    meta.set_xalign(0.0);
    foot.append(&meta);
    foot.append(&install_button(app, cache));
    col.append(&foot);
    card.append(&col);

    let click = gtk4::GestureClick::new();
    let (weak, app2, cache2) = (card.downgrade(), app.clone(), cache.clone());
    click.connect_released(move |_, _, _, _| {
        if let Some(c) = weak.upgrade() {
            show_details(c.upcast_ref(), &app2, &cache2);
        }
    });
    card.add_controller(click);
    card.upcast()
}

fn card_meta(app: &AppInfo) -> String {
    let src = match app.source {
        AppSource::Pacman => "Arch",
        AppSource::Flatpak => "Flathub",
    };
    if app.rating > 0.0 {
        format!("★ {:.1}  ·  {src}", app.rating)
    } else {
        src.to_string()
    }
}

fn show_details(from: &gtk4::Widget, app: &AppInfo, cache: &Cache) {
    let dialog = adw::Dialog::new();
    dialog.set_title(&app.name);
    dialog.set_content_width(520);

    let bx = gtk4::Box::new(gtk4::Orientation::Vertical, 14);
    for set in [gtk4::Widget::set_margin_top, gtk4::Widget::set_margin_bottom, gtk4::Widget::set_margin_start, gtk4::Widget::set_margin_end] {
        set(bx.upcast_ref(), 24);
    }
    let top = gtk4::Box::new(gtk4::Orientation::Horizontal, 16);
    top.append(&create_app_icon(&app.icon_name, &app.id, 72));
    let titles = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    titles.set_valign(gtk4::Align::Center);
    titles.set_hexpand(true);
    let t = gtk4::Label::new(Some(&app.name));
    t.add_css_class("title-1");
    t.set_xalign(0.0);
    t.set_wrap(true);
    let p = gtk4::Label::new(Some(&app.publisher));
    p.add_css_class("card-pub");
    p.set_xalign(0.0);
    titles.append(&t);
    titles.append(&p);
    top.append(&titles);
    let btn = install_button(app, cache);
    btn.set_valign(gtk4::Align::Center);
    top.append(&btn);
    bx.append(&top);

    let desc = gtk4::Label::new(Some(if app.description.is_empty() { "No description available." } else { &app.description }));
    desc.set_wrap(true);
    desc.set_xalign(0.0);
    desc.set_selectable(true);
    bx.append(&desc);

    let info = adw::PreferencesGroup::new();
    let rows: [(&str, String); 3] = [
        ("Source", match app.source { AppSource::Pacman => "Arch Linux software sources".into(), AppSource::Flatpak => "Flathub".into() }),
        ("Package", app.package_name.clone()),
        ("Category", app.category.label().to_string()),
    ];
    for (k, v) in rows {
        let r = adw::ActionRow::new();
        r.set_title(k);
        r.set_subtitle(&glib::markup_escape_text(&v));
        r.set_subtitle_selectable(true);
        info.add(&r);
    }
    bx.append(&info);

    let sc = gtk4::ScrolledWindow::new();
    sc.set_propagate_natural_height(true);
    sc.set_max_content_height(560);
    sc.set_child(Some(&bx));
    dialog.set_child(Some(&sc));
    dialog.present(Some(from));
}

// ── Install / remove button ───────────────────────────────────────────────────

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

/// The Get/Remove button for an app. It reads the shared cache, runs the install or removal on a thread (the package
/// manager queues operations, see `backend::PACKAGE_LOCK`), and on success updates the cache, which re-labels every
/// other button for the same app. A failure is explained in a dialog.
fn install_button(app: &AppInfo, cache: &Cache) -> gtk4::Button {
    let btn = gtk4::Button::new();
    btn.add_css_class("card-btn");
    btn.set_valign(gtk4::Align::Center);
    set_btn_label(&btn, cache.is_installed(&app.source, &app.package_name));

    let working = Rc::new(Cell::new(false));
    {
        let (weak, working, app, cache2) = (btn.downgrade(), working.clone(), app.clone(), cache.clone());
        cache.on_change(move || match weak.upgrade() {
            Some(b) => {
                if !working.get() {
                    set_btn_label(&b, cache2.is_installed(&app.source, &app.package_name));
                }
                true
            }
            None => false,
        });
    }

    let (app, cache) = (app.clone(), cache.clone());
    btn.connect_clicked(move |b| {
        if working.get() {
            return;
        }
        let was = cache.is_installed(&app.source, &app.package_name);
        working.set(true);
        b.set_sensitive(false);
        b.set_label(if was { "Removing…" } else { "Installing…" });

        let (tx, rx) = std::sync::mpsc::channel();
        let a = app.clone();
        std::thread::spawn(move || {
            let res = if was { backend::remove_app(&a.source, &a.package_name) } else { backend::install_app(&a.source, &a.package_name) };
            let _ = tx.send(res);
        });

        let (b, working, app, cache) = (b.clone(), working.clone(), app.clone(), cache.clone());
        glib::timeout_add_local(std::time::Duration::from_millis(150), move || match rx.try_recv() {
            Ok(res) => {
                working.set(false);
                b.set_sensitive(true);
                match res {
                    Ok(()) => cache.set_installed(&app.source, &app.package_name, !was),
                    Err(why) => {
                        set_btn_label(&b, was);
                        let title = if was { format!("Couldn't remove {}", app.name) } else { format!("Couldn't install {}", app.name) };
                        show_error(b.upcast_ref(), &title, &why);
                    }
                }
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                working.set(false);
                b.set_sensitive(true);
                set_btn_label(&b, was);
                glib::ControlFlow::Break
            }
        });
    });
    btn
}

/// Tells the person why an install or removal didn't happen, instead of the button just quietly going back to how it was.
fn show_error(from: &gtk4::Widget, title: &str, why: &str) {
    let d = adw::AlertDialog::new(Some(title), Some(why));
    d.add_response("ok", "OK");
    d.set_default_response(Some("ok"));
    d.present(Some(from));
}

// ── Apps / Games ──────────────────────────────────────────────────────────────

fn build_browse_page(games_only: bool, cache: &Cache) -> gtk4::Widget {
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(1100);
    clamp.set_tightening_threshold(900);
    let page = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    page.set_margin_top(24);
    page.set_margin_bottom(40);
    page.set_margin_start(24);
    page.set_margin_end(24);

    let title = gtk4::Label::new(Some(if games_only { "Games" } else { "Apps" }));
    title.add_css_class("page-title");
    title.set_xalign(0.0);
    page.append(&title);

    let mut apps: Vec<AppInfo> = get_curated_apps().into_iter().filter(|a| a.category.is_game() == games_only).collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Category chips (only categories that have apps in them). Picking one hides the cards of the others.
    let mut cats: Vec<AppCategory> = Vec::new();
    for a in &apps {
        if !cats.contains(&a.category) {
            cats.push(a.category.clone());
        }
    }
    let grid = app_grid(&apps, cache);
    let selected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    {
        let sel = selected.clone();
        grid.set_filter_func(move |child| match (&*sel.borrow(), child.child()) {
            (None, _) => true,
            (Some(want), Some(card)) => card.widget_name() == want.as_str(),
            _ => true,
        });
    }
    if cats.len() > 1 {
        let chips = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let all = gtk4::ToggleButton::with_label("All");
        all.add_css_class("chip");
        all.set_active(true);
        {
            let (sel, grid) = (selected.clone(), grid.clone());
            all.connect_toggled(move |b| {
                if b.is_active() {
                    *sel.borrow_mut() = None;
                    grid.invalidate_filter();
                }
            });
        }
        chips.append(&all);
        for c in cats {
            let b = gtk4::ToggleButton::with_label(c.label());
            b.add_css_class("chip");
            b.set_group(Some(&all));
            let (sel, grid, label) = (selected.clone(), grid.clone(), c.label().to_string());
            b.connect_toggled(move |b| {
                if b.is_active() {
                    *sel.borrow_mut() = Some(label.clone());
                    grid.invalidate_filter();
                }
            });
            chips.append(&b);
        }
        let cs = gtk4::ScrolledWindow::new();
        cs.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
        cs.set_child(Some(&chips));
        page.append(&cs);
    }
    page.append(&grid);

    let hint = gtk4::Label::new(Some("Looking for something else? Use the search button at the top: it searches all of Arch Linux's software and Flathub."));
    hint.add_css_class("section-sub");
    hint.set_xalign(0.0);
    hint.set_wrap(true);
    page.append(&hint);

    clamp.set_child(Some(&page));
    scroll.set_child(Some(&clamp));
    scroll.upcast()
}

// ── Search results ────────────────────────────────────────────────────────────

struct SearchPage {
    widget: gtk4::Widget,
    title: gtk4::Label,
    note: gtk4::Label,
    results: gtk4::Box,
    empty: adw::StatusPage,
}

impl SearchPage {
    fn new() -> Self {
        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        let clamp = adw::Clamp::new();
        clamp.set_maximum_size(1100);
        clamp.set_tightening_threshold(900);
        let page = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        page.set_margin_top(24);
        page.set_margin_bottom(40);
        page.set_margin_start(24);
        page.set_margin_end(24);

        let title = gtk4::Label::new(None);
        title.add_css_class("page-title");
        title.set_xalign(0.0);
        title.set_wrap(true);
        let note = gtk4::Label::new(None);
        note.add_css_class("section-sub");
        note.set_xalign(0.0);
        let results = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let empty = adw::StatusPage::new();
        empty.set_icon_name(Some("edit-find-symbolic"));
        empty.set_title("No apps found");
        empty.set_description(Some("Try another word, or part of the name."));
        empty.set_visible(false);

        page.append(&title);
        page.append(&note);
        page.append(&results);
        page.append(&empty);
        clamp.set_child(Some(&page));
        scroll.set_child(Some(&clamp));
        SearchPage { widget: scroll.upcast(), title, note, results, empty }
    }

    fn show(&self, query: &str, results: Vec<AppInfo>, cache: &Cache, catalog_ready: bool) {
        self.title.set_text(&format!("Results for “{query}”"));
        while let Some(c) = self.results.first_child() {
            self.results.remove(&c);
        }
        let n = results.len();
        self.empty.set_visible(n == 0 && catalog_ready);
        self.note.set_text(&match (n, catalog_ready) {
            (0, false) => "Loading the full catalog…".to_string(),
            (_, false) => format!("{n} from the Store's picks. Loading the full catalog…"),
            (n, true) if n >= SEARCH_LIMIT => format!("Showing the best {n} matches"),
            (1, true) => "1 app".to_string(),
            (n, true) => format!("{n} apps"),
        });
        if n > 0 {
            self.results.append(&app_grid(&results, cache));
        }
    }
}

// ── Curated subsets ───────────────────────────────────────────────────────────

fn picks_apps() -> Vec<AppInfo> {
    get_curated_apps()
        .into_iter()
        .filter(|a| matches!(a.id.as_str(), "firefox" | "vlc" | "gimp" | "discord" | "libreoffice" | "obsidian"))
        .collect()
}

fn top_games() -> Vec<AppInfo> {
    get_curated_apps().into_iter().filter(|a| a.category.is_game()).collect()
}

fn dev_apps() -> Vec<AppInfo> {
    get_curated_apps()
        .into_iter()
        .filter(|a| matches!(a.category, AppCategory::Development) || matches!(a.id.as_str(), "vscodium" | "git" | "htop"))
        .collect()
}

// ── Icons ─────────────────────────────────────────────────────────────────────

fn create_app_icon(icon_name: &str, app_id: &str, size: i32) -> gtk4::Image {
    let theme = gtk4::gdk::Display::default().map(|d| gtk4::IconTheme::for_display(&d));
    let candidates = [icon_name, icon_for(app_id), app_id, "application-x-executable", "package-x-generic"];
    let name = theme
        .and_then(|t| candidates.into_iter().find(|n| !n.is_empty() && t.has_icon(n)))
        .unwrap_or("application-x-executable");
    let img = gtk4::Image::from_icon_name(name);
    img.set_pixel_size(size);
    img
}

fn icon_for(id: &str) -> &'static str {
    match id {
        "firefox" => "firefox",
        "chromium" => "chromium",
        "vscodium" => "vscodium",
        "git" => "git",
        "spotify" => "spotify-client",
        "discord" => "discord",
        "telegram-desktop" => "telegram",
        "vlc" => "vlc",
        "mpv" => "mpv",
        "gimp" => "gimp",
        "inkscape" => "inkscape",
        "krita" => "krita",
        "steam" => "steam",
        "lutris" => "lutris",
        "heroic" => "heroic",
        "supertuxkart" => "supertuxkart",
        "0ad" => "0ad",
        "libreoffice" => "libreoffice-main",
        "obsidian" => "obsidian",
        "htop" => "htop",
        "timeshift" => "timeshift",
        _ => "application-x-executable",
    }
}
