//! Zohara welcome: what you see on the live USB and the first time you sign in.
//! Live: install, migrate, or just try it. Installed: migrate, manage accounts
//! and updates (both live in Zohara Settings), or close.

use adw::prelude::*;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use zohara_welcome::{is_live, is_root};

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id("os.zohara.Welcome").build();
    app.connect_activate(build_ui);
    app.run_with_args::<&str>(&[])
}

/// Starts a program. If it exits with an error within two seconds, that error
/// is returned; still running after that means it launched fine.
fn launch(argv: Vec<String>) -> Result<(), String> {
    let (prog, args) = argv.split_first().ok_or("nothing to run")?;
    let mut child = Command::new(prog)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{prog}: {e}"))?;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                let mut err = String::new();
                if let Some(mut e) = child.stderr.take() {
                    let _ = std::io::Read::read_to_string(&mut e, &mut err);
                }
                // pkexec: 126 = the password prompt was dismissed
                if matches!(status.code(), Some(126) | Some(127)) {
                    return Err("The password prompt was cancelled.".into());
                }
                let err = err.trim();
                return Err(if err.is_empty() { format!("{prog} stopped with {status}") } else { err.to_string() });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(e.to_string()),
        }
    }
    // Keep waiting in the background so the child isn't left as a zombie.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn as_root(mut argv: Vec<&str>) -> Vec<String> {
    if !is_root() {
        argv.insert(0, "pkexec");
    }
    argv.into_iter().map(String::from).collect()
}

struct Action {
    title: &'static str,
    subtitle: &'static str,
    icon: &'static str,
    argv: Vec<String>,
    /// Close the welcome window once this has started.
    close_after: bool,
}

fn settings_page(page: &str) -> Vec<String> {
    vec!["zohara-settings".into(), "--page".into(), page.into()]
}

fn actions() -> Vec<Action> {
    let mut v = Vec::new();
    if is_live() {
        v.push(Action {
            title: "Install Zohara OS",
            subtitle: "Put Zohara on this computer",
            icon: "system-software-install-symbolic",
            argv: as_root(vec!["calamares"]),
            close_after: true,
        });
    }
    v.push(Action {
        title: "Migrate from another system",
        subtitle: "Move your files and apps from an old Linux install",
        icon: "document-save-symbolic",
        argv: vec!["zohara-migrate".into()],
        close_after: true,
    });
    if !is_live() {
        v.push(Action {
            title: "Manage users",
            subtitle: "Add and remove accounts",
            icon: "system-users-symbolic",
            argv: settings_page("Accounts"),
            close_after: false,
        });
        v.push(Action {
            title: "Update Zohara",
            subtitle: "Get the latest updates",
            icon: "system-software-update-symbolic",
            argv: settings_page("Zohara Update"),
            close_after: false,
        });
        v.push(Action {
            title: "Open Settings",
            subtitle: "Display, sound, network and more",
            icon: "preferences-system-symbolic",
            argv: vec!["zohara-settings".into()],
            close_after: false,
        });
    }
    v
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Welcome to Zohara")
        .default_width(460)
        .resizable(false)
        .build();

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 18);
    root.set_margin_top(12);
    root.set_margin_bottom(28);
    root.set_margin_start(28);
    root.set_margin_end(28);
    root.append(&gtk4::Label::builder().label("Welcome to Zohara").css_classes(vec!["title-1".to_string()]).build());
    root.append(&gtk4::Label::builder().label("What would you like to do?").css_classes(vec!["dim-label".to_string()]).build());

    let status = gtk4::Label::builder().css_classes(vec!["dim-label".to_string(), "caption".to_string()]).wrap(true).build();
    let group = adw::PreferencesGroup::new();
    for a in actions() {
        let row = adw::ActionRow::new();
        row.set_title(a.title);
        row.set_subtitle(a.subtitle);
        row.add_prefix(&gtk4::Image::from_icon_name(a.icon));
        row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        row.set_activatable(true);
        let (w, status, argv, close_after, title) = (window.clone(), status.clone(), Rc::new(a.argv), a.close_after, a.title);
        row.connect_activated(move |row| {
            status.set_text("Starting…");
            row.set_sensitive(false);
            let (tx, rx) = channel::<Result<(), String>>();
            let argv = argv.to_vec();
            std::thread::spawn(move || {
                let _ = tx.send(launch(argv));
            });
            let (w, status, row) = (w.clone(), status.clone(), row.clone());
            glib::timeout_add_local(Duration::from_millis(60), move || match rx.try_recv() {
                Ok(r) => {
                    row.set_sensitive(true);
                    status.set_text("");
                    match r {
                        Ok(()) => {
                            if close_after {
                                w.close();
                            }
                        }
                        Err(e) => {
                            let d = adw::AlertDialog::new(Some(&format!("Couldn't open {title}")), Some(&e));
                            d.add_response("ok", "OK");
                            d.present(Some(&w));
                        }
                    }
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(_) => glib::ControlFlow::Break,
            });
        });
        group.add(&row);
    }
    root.append(&group);
    root.append(&status);

    let close = gtk4::Button::with_label(if is_live() { "Try Zohara" } else { "Close" });
    close.add_css_class("pill");
    close.set_halign(gtk4::Align::Center);
    let w = window.clone();
    close.connect_clicked(move |_| w.close());
    root.append(&close);

    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&root));
    window.set_content(Some(&view));
    window.present();
}
