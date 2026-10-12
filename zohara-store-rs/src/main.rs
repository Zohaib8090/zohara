pub mod app_info;
pub mod appimage;
pub mod aur;
pub mod community_updates;
pub mod backend;
pub mod downloads;
pub mod library;
pub mod requests;
pub mod ui;
pub mod updates;
pub mod updates_ui;

use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

fn main() {
    // Headless mode for a systemd user timer: check for app updates and fire
    // desktop notifications without opening any window. This is what makes
    // "notify even when the Store isn't open" real, rather than only
    // checking when someone happens to launch the app.
    if std::env::args().any(|a| a == "--check-updates") {
        let found = updates::check_all();
        updates::notify_pending(&found);
        return;
    }

    // One Store window: a second launch (a browser's Install button, "Open with", the update notification) hands its
    // arguments to the window that is already open instead of opening another.
    let app = adw::Application::builder()
        .application_id("org.zohara.store")
        .flags(gtk4::gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_activate(build_ui);
    app.connect_command_line(|app, cmd| {
        let args: Vec<String> = cmd.arguments().iter().skip(1).map(|a| a.to_string_lossy().into_owned()).collect();
        app.activate();
        for r in requests::parse_args(&args) {
            ui::handle_request(r);
        }
        0.into()
    });
    app.run();
}

fn build_ui(app: &adw::Application) {
    // Already open: just bring it to the front.
    if let Some(w) = app.active_window() {
        w.present();
        return;
    }
    // Force standard window decorations
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_decoration_layout(Some("icon:minimize,maximize,close"));
    }

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Software Store")
        .default_width(1100)
        .default_height(750)
        .build();

    let content = ui::build();
    window.set_content(Some(&content));
    window.present();
}
