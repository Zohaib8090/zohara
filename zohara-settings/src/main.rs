mod system_manager;
mod network_manager;
mod bluetooth;
mod theme_manager;

use system_manager::SystemManager;
use network_manager::NetworkManager;
use bluetooth::BluetoothManager;
use theme_manager::ThemeManager;

use gtk4 as gtk;
use libadwaita as adw;

use adw::prelude::*;
use adw::{ActionRow, Application, ApplicationWindow, NavigationSplitView, PreferencesGroup, PreferencesPage};
use gtk::{ListBox, Label, SelectionMode, Stack, Widget};

fn main() {
    let app = Application::builder()
        .application_id("org.zohara.Settings")
        .build();

    app.connect_startup(|_| {
        adw::init();
    });

    app.connect_activate(build_ui);
    
    app.run();
}

fn build_ui(app: &Application) {
    let sys_mgr = SystemManager::new();
    let net_mgr = NetworkManager::new();
    // Start the async bluetooth worker on a dedicated OS thread.
    let bt_mgr = BluetoothManager::start(|| {
        // In a real build, call glib::idle_add_once here to queue a UI refresh.
        // The callback runs on the worker thread — only schedule a redraw, don't touch widgets.
    });
    let theme_mgr = ThemeManager::new();

    // The Stack that holds all the different settings pages
    let stack = Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    // 1. System Page
    let sys_page = PreferencesPage::new();
    let sys_group = PreferencesGroup::builder().title("System Specifications").build();
    
    let os_row = ActionRow::builder().title("OS Version").subtitle(sys_mgr.get_os_version()).build();
    let kernel_row = ActionRow::builder().title("Kernel").subtitle(sys_mgr.get_kernel_version()).build();
    let cpu_row = ActionRow::builder().title("CPU").subtitle(sys_mgr.get_cpu_model()).build();
    let ram_row = ActionRow::builder().title("RAM").subtitle(sys_mgr.get_memory_total()).build();
    
    sys_group.add(&os_row);
    sys_group.add(&kernel_row);
    sys_group.add(&cpu_row);
    sys_group.add(&ram_row);
    sys_page.add(&sys_group);
    
    stack.add_named(&sys_page, Some("system"));

    // 2. Network Page
    let net_page = PreferencesPage::new();
    let net_group = PreferencesGroup::builder().title("Network & Internet").build();
    let conn_row = ActionRow::builder().title("Current Connection").subtitle(net_mgr.get_current_connection_name()).build();
    net_group.add(&conn_row);
    net_page.add(&net_group);
    
    stack.add_named(&net_page, Some("network"));

    // 3. Bluetooth Page — reads from the live async cache
    let bt_page = PreferencesPage::new();
    let bt_group = PreferencesGroup::builder().title("Bluetooth & Devices").build();
    let bt_cache = bt_mgr.cache();
    let paired_count = bt_cache.devices.values().filter(|d| d.paired).count();
    let bt_status = if !bt_cache.bluez_available {
        "Bluetooth unavailable".to_string()
    } else if !bt_cache.adapter_powered {
        "Bluetooth is off".to_string()
    } else {
        format!("{} devices paired", paired_count)
    };
    let paired_row = ActionRow::builder().title("Paired Devices").subtitle(&bt_status).build();
    bt_group.add(&paired_row);
    bt_page.add(&bt_group);

    stack.add_named(&bt_page, Some("bluetooth"));

    // 4. Theme Page
    let theme_page = PreferencesPage::new();
    let theme_group = PreferencesGroup::builder().title("Personalization").build();
    let current_theme_row = ActionRow::builder().title("Current Theme").subtitle(theme_mgr.current_theme()).build();
    theme_group.add(&current_theme_row);
    theme_page.add(&theme_group);

    stack.add_named(&theme_page, Some("theme"));

    // Sidebar
    let sidebar_list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .css_classes(vec!["navigation-sidebar".to_string()])
        .build();

    let nav_sys = Label::builder().label("System").xalign(0.0).margin_start(12).margin_end(12).margin_top(12).margin_bottom(12).build();
    let nav_net = Label::builder().label("Network & internet").xalign(0.0).margin_start(12).margin_end(12).margin_top(12).margin_bottom(12).build();
    let nav_bt = Label::builder().label("Bluetooth & devices").xalign(0.0).margin_start(12).margin_end(12).margin_top(12).margin_bottom(12).build();
    let nav_theme = Label::builder().label("Personalization").xalign(0.0).margin_start(12).margin_end(12).margin_top(12).margin_bottom(12).build();

    sidebar_list.append(&nav_sys);
    sidebar_list.append(&nav_bt);
    sidebar_list.append(&nav_net);
    sidebar_list.append(&nav_theme);

    // Navigation Logic
    let stack_clone = stack.clone();
    sidebar_list.connect_row_selected(move |_, row| {
        if let Some(r) = row {
            let index = r.index();
            match index {
                0 => stack_clone.set_visible_child_name("system"),
                1 => stack_clone.set_visible_child_name("bluetooth"),
                2 => stack_clone.set_visible_child_name("network"),
                3 => stack_clone.set_visible_child_name("theme"),
                _ => {}
            }
        }
    });

    // Content container wrapper for the stack
    let content_page = adw::NavigationPage::builder()
        .title("Settings")
        .child(&stack)
        .build();

    // Sidebar wrapper
    let sidebar_page = adw::NavigationPage::builder()
        .title("Zohara")
        .child(&sidebar_list)
        .build();

    // Split view
    let split_view = NavigationSplitView::builder()
        .sidebar(&sidebar_page)
        .content(&content_page)
        .build();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Zohara Settings")
        .default_width(1024)
        .default_height(768)
        .content(&split_view)
        .build();

    window.present();
}
