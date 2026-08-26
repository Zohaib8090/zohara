use gtk4::prelude::*;
use gtk4::gio;
use libadwaita as adw;
use adw::prelude::*;
use std::process::Command;


pub fn build() -> gtk4::Widget {
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    root_box.set_margin_start(28);
    root_box.set_margin_end(28);
    root_box.set_margin_top(20);
    root_box.set_margin_bottom(32);

    // ── Page Title ────────────────────────────────────────────────────────────
    let title_lbl = gtk4::Label::builder()
        .label("Personalization")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Hero Preview Banner (Mockup Preview on Left + 6 Themes on Right) ──────
    let hero_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 20);
    hero_box.set_margin_bottom(8);

    // Left: Big Desktop Window Preview Mockup
    let preview_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    preview_box.set_size_request(240, 140);
    preview_box.set_css_classes(&["win11-theme-preview-large"]);
    hero_box.append(&preview_box);

    // Right: 6-Theme Selection Grid
    let themes_container = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    let select_lbl = gtk4::Label::builder()
        .label("Select a theme to apply")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-card-title".to_string()])
        .build();
    themes_container.append(&select_lbl);

    let themes_grid = gtk4::Grid::builder()
        .column_spacing(8)
        .row_spacing(8)
        .column_homogeneous(true)
        .build();

    let theme_presets = [
        ("Zohara Dark Bloom", "#1a162b", "#7c3aed"),
        ("Windows 11 Light", "#e0e7ff", "#0078d4"),
        ("Deep Nebula", "#0a0e17", "#0ea5e9"),
        ("Nordic Night", "#2e3440", "#88c0d0"),
        ("Sunset Amber", "#26130d", "#f97316"),
        ("Emerald Forest", "#062016", "#10b981"),
    ];

    for (i, (name, bg, accent)) in theme_presets.iter().enumerate() {
        let btn = gtk4::Button::builder()
            .css_classes(vec!["win11-theme-thumb".to_string()])
            .tooltip_text(*name)
            .build();
        let thumb = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        thumb.set_size_request(76, 48);

        let custom_css = format!(
            "box {{ background: linear-gradient(135deg, {} 0%, #08080c 100%); border-radius: 8px; border: 2px solid {}; }}",
            bg, accent
        );
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(&custom_css);
        thumb.style_context().add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);

        btn.set_child(Some(&thumb));
        let col = (i % 3) as i32;
        let row = (i / 3) as i32;
        themes_grid.attach(&btn, col, row, 1, 1);
    }
    themes_container.append(&themes_grid);
    hero_box.append(&themes_container);
    root_box.append(&hero_box);

    // ── Grouped Rows (100% In-App Standalone) ─────────────────────────────────
    let rows_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    rows_box.set_css_classes(&["win11-card-group"]);

    // 1. Background (with In-App Wallpapers)
    let bg_exp = adw::ExpanderRow::new();
    bg_exp.set_title("Background");
    bg_exp.set_subtitle("Background image, color, slideshow");
    bg_exp.add_prefix(&gtk4::Image::from_icon_name("preferences-desktop-wallpaper-symbolic"));
    bg_exp.set_css_classes(&["win11-expander-row"]);

    let choose_file_row = adw::ActionRow::new();
    choose_file_row.set_title("Choose a photo");
    let browse_btn = gtk4::Button::builder()
        .label("Browse photos")
        .css_classes(vec!["win11-secondary-btn".to_string()])
        .build();
    let choose_file_clone = choose_file_row.clone();
    browse_btn.connect_clicked(move |btn| {
        let parent = btn.root().and_downcast::<gtk4::Window>();
        let file_dialog = gtk4::FileDialog::new();
        file_dialog.set_title("Select Wallpaper Image");
        let choose_clone = choose_file_clone.clone();
        file_dialog.open(parent.as_ref(), None::<&gio::Cancellable>, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    let path_str = path.to_string_lossy().to_string();
                    let _ = Command::new("plasma-apply-wallpaperimage").arg(&path_str).spawn();
                    choose_clone.set_subtitle(&format!("Applied: {}", path.file_name().unwrap_or_default().to_string_lossy()));
                }
            }
        });
    });
    choose_file_row.add_suffix(&browse_btn);
    bg_exp.add_row(&choose_file_row);
    rows_box.append(&bg_exp);

    // 2. Colors (with In-App Accent Colors)
    let colors_exp = adw::ExpanderRow::new();
    colors_exp.set_title("Colors");
    colors_exp.set_subtitle("Accent color, transparency effects, color theme");
    colors_exp.add_prefix(&gtk4::Image::from_icon_name("preferences-desktop-color-symbolic"));
    colors_exp.set_css_classes(&["win11-expander-row"]);

    let mode_row = adw::ComboRow::new();
    mode_row.set_title("Choose your mode");
    let mode_list = gtk4::StringList::new(&["Dark (Recommended)", "Light", "Custom"]);
    mode_row.set_model(Some(&mode_list));
    mode_row.set_selected(0);
    colors_exp.add_row(&mode_row);

    let trans_row = adw::SwitchRow::new();
    trans_row.set_title("Transparency effects");
    trans_row.set_subtitle("Windows and surfaces appear translucent");
    trans_row.set_active(true);
    colors_exp.add_row(&trans_row);

    rows_box.append(&colors_exp);

    // 3. Themes
    let themes_row = build_action_row("Themes", "Install, create, and manage desktop themes", "applications-graphics-symbolic");
    rows_box.append(&themes_row);

    // 4. Dynamic Lighting
    let lighting_row = build_action_row("Dynamic Lighting", "Connected RGB devices, effects, app settings", "weather-clear-symbolic");
    rows_box.append(&lighting_row);

    // 5. Lock Screen
    let lock_row = build_action_row("Lock screen", "Lock screen images, apps, timeout status", "system-lock-screen-symbolic");
    rows_box.append(&lock_row);

    // 6. Text Input
    let text_row = build_action_row("Text input", "Touch keyboard, voice typing, emoji and more", "input-keyboard-symbolic");
    rows_box.append(&text_row);

    // 7. Start
    let start_row = build_action_row("Start", "Recent apps and items, folders, start menu layout", "view-app-grid-symbolic");
    rows_box.append(&start_row);

    // 8. Taskbar (with In-App Alignment Toggle)
    let taskbar_exp = adw::ExpanderRow::new();
    taskbar_exp.set_title("Taskbar");
    taskbar_exp.set_subtitle("Taskbar behaviors, system pins, alignment");
    taskbar_exp.add_prefix(&gtk4::Image::from_icon_name("view-paged-symbolic"));
    taskbar_exp.set_css_classes(&["win11-expander-row"]);

    let align_row = adw::ComboRow::new();
    align_row.set_title("Taskbar alignment");
    let align_list = gtk4::StringList::new(&["Center (Windows 11 style)", "Left (Classic style)"]);
    align_row.set_model(Some(&align_list));
    align_row.set_selected(0);
    taskbar_exp.add_row(&align_row);

    rows_box.append(&taskbar_exp);

    // 9. Fonts
    let fonts_row = build_action_row("Fonts", "Font family, font sizes, ClearType text", "preferences-desktop-font-symbolic");
    rows_box.append(&fonts_row);

    root_box.append(&rows_box);
    scroll.set_child(Some(&root_box));
    scroll.upcast()
}

fn build_action_row(title: &str, subtitle: &str, icon_name: &str) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(title);
    row.set_subtitle(subtitle);
    row.add_prefix(&gtk4::Image::from_icon_name(icon_name));
    row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
    row.set_css_classes(&["win11-expander-row"]);
    row.set_activatable(true);
    row
}
