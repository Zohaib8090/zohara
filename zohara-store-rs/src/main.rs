pub mod app_info;
pub mod backend;
pub mod ui;

use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

fn main() {
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
