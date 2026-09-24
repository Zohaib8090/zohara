//! Zohara migration wizard.
//!
//! The window runs as the normal user. The work itself runs in a second copy
//! of this program started through pkexec (`zohara-migrate --worker DEVICE`),
//! which prints one JSON event per line. Closing the window or pressing Stop
//! closes the helper's input, which makes it stop after the current file.

use adw::prelude::*;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use serde::Deserialize;
use std::cell::RefCell;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;
use zohara_welcome::migrate::{run_job, Event, Report};
use zohara_welcome::{is_live, is_root};

fn main() -> glib::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--worker") {
        std::process::exit(worker(&args[2..]));
    }

    let app = adw::Application::builder().application_id("os.zohara.Migrate").build();
    app.connect_activate(build_ui);
    app.run_with_args::<&str>(&[])
}

// ── Privileged helper ──────────────────────────────────────────────────────

fn worker(args: &[String]) -> i32 {
    let Some(dev) = args.first() else {
        eprintln!("usage: zohara-migrate --worker DEVICE [--delete-old-system]");
        return 2;
    };
    if !is_root() {
        eprintln!("the migration helper must run as root");
        return 2;
    }
    let delete = args.iter().any(|a| a == "--delete-old-system");

    // Stdin closing (window closed, Stop pressed) cancels after the current file.
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            let mut sink = Vec::new();
            let _ = std::io::Read::read_to_end(&mut std::io::stdin(), &mut sink);
            cancel.store(true, Ordering::Relaxed);
        });
    }
    let emit = |e: Event| {
        if let Ok(line) = serde_json::to_string(&e) {
            println!("{line}");
        }
    };
    let report = run_job(dev, delete, &emit, &cancel);
    emit(Event::Done(report));
    0
}

// ── Window ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct LsblkOut {
    blockdevices: Vec<Dev>,
}

#[derive(Deserialize)]
struct Dev {
    path: String,
    fstype: Option<String>,
    size: Option<String>,
    label: Option<String>,
    model: Option<String>,
    #[serde(default)]
    mountpoints: Vec<Option<String>>,
    #[serde(default)]
    children: Vec<Dev>,
}

struct Part {
    path: String,
    title: String,
    subtitle: String,
}

/// Unmounted ext4/btrfs/xfs partitions: where an old Linux system could be.
fn candidates() -> Vec<Part> {
    fn collect(devs: &[Dev], disk: &str, out: &mut Vec<Part>) {
        for d in devs {
            let mounted = d.mountpoints.iter().flatten().any(|m| !m.is_empty());
            if matches!(d.fstype.as_deref(), Some("ext4" | "btrfs" | "xfs")) && !mounted {
                let fs = d.fstype.clone().unwrap_or_default();
                let label = d.label.clone().unwrap_or_default();
                out.push(Part {
                    path: d.path.clone(),
                    title: if label.is_empty() { d.path.clone() } else { format!("{label} ({})", d.path) },
                    subtitle: format!("{} · {fs}{}", d.size.clone().unwrap_or_default(), if disk.is_empty() { String::new() } else { format!(" · on {disk}") }),
                });
            }
            let name = d.model.as_deref().map(str::trim).filter(|m| !m.is_empty()).unwrap_or(disk);
            collect(&d.children, name, out);
        }
    }
    let Ok(o) = Command::new("lsblk").args(["-J", "-o", "PATH,FSTYPE,SIZE,LABEL,MODEL,MOUNTPOINTS"]).output() else { return Vec::new() };
    let Ok(parsed) = serde_json::from_slice::<LsblkOut>(&o.stdout) else { return Vec::new() };
    let mut out = Vec::new();
    collect(&parsed.blockdevices, "", &mut out);
    out
}

struct Ui {
    window: adw::ApplicationWindow,
    stack: gtk4::Stack,
    bar: gtk4::ProgressBar,
    status: gtk4::Label,
    log: gtk4::TextBuffer,
    log_view: gtk4::TextView,
    log_lines: RefCell<u32>,
    stop: gtk4::Button,
    child: RefCell<Option<Child>>,
    got_done: RefCell<bool>,
    // results widgets
    result_title: gtk4::Label,
    result_box: gtk4::Box,
}

fn page_root() -> gtk4::Box {
    let b = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    b.set_margin_top(24);
    b.set_margin_bottom(24);
    b.set_margin_start(28);
    b.set_margin_end(28);
    b
}

fn heading(text: &str) -> gtk4::Label {
    gtk4::Label::builder().label(text).halign(gtk4::Align::Start).css_classes(vec!["title-2".to_string()]).wrap(true).build()
}

fn build_ui(app: &adw::Application) {
    let stack = gtk4::Stack::new();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Zohara Migration")
        .default_width(700)
        .default_height(580)
        .build();
    let header = adw::HeaderBar::new();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&stack));
    window.set_content(Some(&view));

    if is_live() {
        stack.add_named(&live_page(&window), Some("live"));
        window.present();
        return;
    }

    let log = gtk4::TextBuffer::new(None);
    let log_view = gtk4::TextView::with_buffer(&log);
    let ui = Rc::new(Ui {
        window: window.clone(),
        stack: stack.clone(),
        bar: gtk4::ProgressBar::new(),
        status: gtk4::Label::new(Some("Starting…")),
        log,
        log_view,
        log_lines: RefCell::new(0),
        stop: gtk4::Button::with_label("Stop"),
        child: RefCell::new(None),
        got_done: RefCell::new(false),
        result_title: heading(""),
        result_box: gtk4::Box::new(gtk4::Orientation::Vertical, 12),
    });
    stack.add_named(&select_page(&ui), Some("select"));
    stack.add_named(&progress_page(&ui), Some("progress"));
    stack.add_named(&results_page(&ui), Some("results"));

    // Closing the window stops the helper after its current file.
    let ui2 = ui.clone();
    window.connect_close_request(move |_| {
        if let Some(mut c) = ui2.child.borrow_mut().take() {
            drop(c.stdin.take());
        }
        glib::Propagation::Proceed
    });
    window.present();
}

fn live_page(window: &adw::ApplicationWindow) -> gtk4::Box {
    let root = page_root();
    root.append(&heading("Install Zohara first"));
    root.append(
        &gtk4::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .label(
                "You're on the live USB. Install Zohara on your disk, start it, then open this tool again. It will:\n\n\
                 •  move your files one at a time, checking each with a SHA-256 checksum\n\
                 •  never overwrite anything already on the new system\n\
                 •  install the matching apps for the Debian/Ubuntu ones you had\n\
                 •  optionally remove the old system afterwards, only if every file moved",
            )
            .build(),
    );
    let go = gtk4::Button::with_label("Open the installer");
    go.add_css_class("suggested-action");
    go.add_css_class("pill");
    go.set_halign(gtk4::Align::Start);
    let w = window.clone();
    go.connect_clicked(move |_| {
        let _ = if is_root() { Command::new("calamares").spawn() } else { Command::new("pkexec").arg("calamares").spawn() };
        w.close();
    });
    root.append(&go);
    root
}

fn select_page(ui: &Rc<Ui>) -> gtk4::Widget {
    let root = page_root();
    root.append(&heading("Move from your old system"));
    root.append(
        &gtk4::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .label(
                "Choose the disk that holds your old Linux system. Your files from its /home folder are moved here one at a time and \
                 checked as they go, so this works even when the disk is nearly full. Files already here are never overwritten.",
            )
            .build(),
    );

    let parts = candidates();
    let group = adw::PreferencesGroup::new();
    group.set_title("Partitions");
    let selected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    if parts.is_empty() {
        let r = adw::ActionRow::new();
        r.set_title("No other Linux partitions found");
        r.set_subtitle("The old system's disk must be connected and not in use. Only ext4, btrfs and xfs are supported.");
        group.add(&r);
    }
    let mut first: Option<gtk4::CheckButton> = None;
    for p in &parts {
        let r = adw::ActionRow::new();
        r.set_title(&glib::markup_escape_text(&p.title));
        r.set_subtitle(&glib::markup_escape_text(&p.subtitle));
        let check = gtk4::CheckButton::new();
        check.set_valign(gtk4::Align::Center);
        match &first {
            Some(f) => check.set_group(Some(f)),
            None => first = Some(check.clone()),
        }
        let (sel, path) = (selected.clone(), p.path.clone());
        check.connect_toggled(move |c| {
            if c.is_active() {
                *sel.borrow_mut() = Some(path.clone());
            }
        });
        r.add_prefix(&check);
        r.set_activatable_widget(Some(&check));
        group.add(&r);
    }
    root.append(&group);

    let opts = adw::PreferencesGroup::new();
    let delete = adw::SwitchRow::new();
    delete.set_title("Remove the old system afterwards");
    delete.set_subtitle("Frees its space. Only happens if every file moved successfully. Off by default");
    opts.add(&delete);
    root.append(&opts);

    let start = gtk4::Button::with_label("Start moving");
    start.add_css_class("suggested-action");
    start.add_css_class("pill");
    start.set_halign(gtk4::Align::Start);
    let ui2 = ui.clone();
    start.connect_clicked(move |b| {
        let Some(dev) = selected.borrow().clone() else {
            let d = adw::AlertDialog::new(Some("Choose a partition"), Some("Select the disk that holds your old system first."));
            d.add_response("ok", "OK");
            d.present(Some(b));
            return;
        };
        let del = delete.is_active();
        let body = format!(
            "Your files from {dev} will be moved to this computer, one at a time, and checked as they go.{}",
            if del { "\n\nIf every file moves successfully, the old system's files will then be permanently deleted." } else { "" }
        );
        let d = adw::AlertDialog::new(Some("Start moving?"), Some(&body));
        d.add_responses(&[("cancel", "Cancel"), ("go", "Start")]);
        d.set_response_appearance("go", if del { adw::ResponseAppearance::Destructive } else { adw::ResponseAppearance::Suggested });
        let ui3 = ui2.clone();
        d.connect_response(None, move |_, r| {
            if r == "go" {
                start_job(&ui3, &dev, del);
            }
        });
        d.present(Some(b));
    });
    root.append(&start);

    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_child(Some(&root));
    scroll.upcast()
}

fn progress_page(ui: &Rc<Ui>) -> gtk4::Box {
    let root = page_root();
    root.append(&heading("Moving your files…"));
    root.append(&ui.bar);
    ui.status.set_halign(gtk4::Align::Start);
    root.append(&ui.status);
    ui.log_view.set_editable(false);
    ui.log_view.set_monospace(true);
    ui.log_view.set_cursor_visible(false);
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&ui.log_view));
    root.append(&scroll);
    let ui2 = ui.clone();
    ui.stop.connect_clicked(move |b| {
        b.set_sensitive(false);
        ui2.status.set_text("Stopping after the current file…");
        if let Some(c) = ui2.child.borrow_mut().as_mut() {
            drop(c.stdin.take());
        }
    });
    ui.stop.set_halign(gtk4::Align::Start);
    root.append(&ui.stop);
    root
}

fn results_page(ui: &Rc<Ui>) -> gtk4::Widget {
    let root = page_root();
    root.append(&ui.result_title);
    ui.result_box.set_vexpand(true);
    root.append(&ui.result_box);
    let reboot = gtk4::Button::with_label("Restart now");
    reboot.add_css_class("pill");
    reboot.set_halign(gtk4::Align::Start);
    reboot.connect_clicked(|_| {
        let _ = Command::new("systemctl").arg("reboot").spawn();
    });
    let close = gtk4::Button::with_label("Close");
    let w = ui.window.clone();
    close.connect_clicked(move |_| w.close());
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    row.append(&close);
    row.append(&reboot);
    root.append(&row);
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_child(Some(&root));
    scroll.upcast()
}

// ── Running the helper ─────────────────────────────────────────────────────

fn start_job(ui: &Rc<Ui>, dev: &str, delete: bool) {
    ui.bar.set_fraction(0.0);
    ui.log.set_text("");
    *ui.log_lines.borrow_mut() = 0;
    *ui.got_done.borrow_mut() = false;
    ui.stop.set_sensitive(true);
    ui.status.set_text("Waiting for your password…");
    ui.stack.set_visible_child_name("progress");

    let exe = std::env::current_exe().unwrap_or_else(|_| "/usr/local/bin/zohara-migrate".into());
    let mut cmd = if is_root() { Command::new(&exe) } else {
        let mut c = Command::new("pkexec");
        c.arg(&exe);
        c
    };
    cmd.args(["--worker", dev]);
    if delete {
        cmd.arg("--delete-old-system");
    }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return show_results(ui, fatal_report(format!("Couldn't start the migration helper: {e}"))),
    };
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    *ui.child.borrow_mut() = Some(child);

    let (tx, rx) = channel::<Event>();
    let (etx, erx) = channel::<String>();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(ev) = serde_json::from_str::<Event>(&line) {
                let _ = tx.send(ev);
            }
        }
    });
    std::thread::spawn(move || {
        let mut text = String::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            text.push_str(&line);
            text.push('\n');
        }
        let _ = etx.send(text);
    });
    poll(ui.clone(), rx, erx);
}

fn fatal_report(msg: String) -> Report {
    Report { fatal: Some(msg), ..Default::default() }
}

fn poll(ui: Rc<Ui>, rx: Receiver<Event>, erx: Receiver<String>) {
    glib::timeout_add_local(Duration::from_millis(80), move || {
        loop {
            match rx.try_recv() {
                Ok(Event::Progress { pct, msg }) => {
                    ui.bar.set_fraction(pct as f64 / 100.0);
                    ui.status.set_text(&msg);
                }
                Ok(Event::File { name, status }) => log_line(&ui, &format!("{status}  {name}")),
                Ok(Event::Done(r)) => {
                    *ui.got_done.borrow_mut() = true;
                    reap(&ui);
                    show_results(&ui, r);
                    return glib::ControlFlow::Break;
                }
                Err(TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        // The helper's output ended without a result.
        let code = ui.child.borrow_mut().take().and_then(|mut c| c.wait().ok()).and_then(|s| s.code());
        let err = erx.recv_timeout(Duration::from_millis(500)).unwrap_or_default();
        let msg = match code {
            Some(126) | Some(127) => "The password prompt was cancelled, so nothing was changed.".to_string(),
            _ if err.trim().is_empty() => "The migration helper stopped unexpectedly. Files already moved are safe in /home; the rest are still on the old disk.".to_string(),
            _ => format!("The migration helper stopped unexpectedly:\n{}", err.trim()),
        };
        show_results(&ui, fatal_report(msg));
        glib::ControlFlow::Break
    });
}

fn reap(ui: &Rc<Ui>) {
    if let Some(mut c) = ui.child.borrow_mut().take() {
        drop(c.stdin.take());
        let _ = c.wait();
    }
}

fn log_line(ui: &Rc<Ui>, line: &str) {
    let mut n = ui.log_lines.borrow_mut();
    *n += 1;
    // Keep the view light on very large migrations.
    if *n > 2000 {
        let mut start = ui.log.start_iter();
        let mut end = start;
        end.forward_lines(500);
        ui.log.delete(&mut start, &mut end);
        *n -= 500;
    }
    let mut end = ui.log.end_iter();
    ui.log.insert(&mut end, &format!("{line}\n"));
    let mark = ui.log.create_mark(None, &ui.log.end_iter(), false);
    ui.log_view.scroll_to_mark(&mark, 0.0, false, 0.0, 0.0);
    ui.log.delete_mark(&mark);
}

fn section(title: &str, lines: &[String], css: &str) -> gtk4::Box {
    let b = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    b.append(&gtk4::Label::builder().label(title).halign(gtk4::Align::Start).css_classes(vec!["heading".to_string()]).build());
    let tv = gtk4::TextView::new();
    tv.set_editable(false);
    tv.set_monospace(true);
    tv.set_wrap_mode(gtk4::WrapMode::WordChar);
    tv.add_css_class(css);
    tv.buffer().set_text(&lines.join("\n"));
    let sc = gtk4::ScrolledWindow::new();
    sc.set_min_content_height(90);
    sc.set_max_content_height(160);
    sc.set_propagate_natural_height(true);
    sc.set_child(Some(&tv));
    b.append(&sc);
    b
}

fn show_results(ui: &Rc<Ui>, r: Report) {
    while let Some(c) = ui.result_box.first_child() {
        ui.result_box.remove(&c);
    }
    if let Some(f) = &r.fatal {
        ui.result_title.set_text("Migration didn't finish");
        ui.result_box.append(&gtk4::Label::builder().label(f).wrap(true).xalign(0.0).selectable(true).build());
    } else if r.cancelled {
        ui.result_title.set_text("Stopped");
    } else if r.failed.is_empty() {
        ui.result_title.set_text("All done");
    } else {
        ui.result_title.set_text("Done, with some files left behind");
    }

    ui.result_box.append(&gtk4::Label::builder().xalign(0.0).wrap(true).label(format!(
        "{} files moved  ·  {} apps installed  ·  {} apps without a match  ·  {} files couldn't be moved{}",
        r.files_moved, r.installed.len(), r.unmapped.len(), r.failed.len(),
        if r.system_deleted { "\nThe old system's files were removed." } else { "" }
    )).build());
    if !r.failed.is_empty() {
        ui.result_box.append(&section("Kept on the old disk (couldn't be moved)", &r.failed, "error"));
    }
    if !r.notes.is_empty() {
        ui.result_box.append(&section("Good to know", &r.notes, "dim-label"));
    }
    if !r.unmapped.is_empty() {
        ui.result_box.append(&section("Apps with no match on Zohara (look for them in Zohara Store)", &r.unmapped, "warning"));
    }
    ui.stack.set_visible_child_name("results");
}
