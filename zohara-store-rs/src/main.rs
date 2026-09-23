pub mod app_info;
pub mod backend;
pub mod ui;
pub mod updates;

use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

fn main() {
    // Headless mode for a systemd user timer: check for updates and fire
    // desktop notifications without opening any window. This is what makes
    // "notify even when the Store isn't open" real, rather than only
    // checking when someone happens to launch the app.
    if std::env::args().any(|a| a == "--check-updates") {
        let pending = updates::check_for_updates();
        updates::notify_pending_updates(&pending);
        return;
    }

    let app = adw::Application::builder()
        .application_id("org.zohara.store")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
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
