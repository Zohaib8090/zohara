//! The Updates page: see what's new, pick what to update (or update
//! everything), and go back if an update didn't turn out well. All clicks.

use adw::prelude::*;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::time::Duration;

use crate::updates::{self, FlatpakInstalled, SnapStatus, Snapshot, Transaction, UpdateSet};

/// Set by a restore, which always needs a restart (unlike an update).
static FORCE_RESTART: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

struct Page {
    status_title: gtk4::Label,
    status_sub: gtk4::Label,
    spinner: gtk4::Spinner,
    update_all: gtk4::Button,
    update_sel: gtk4::Button,
    recheck: gtk4::Button,
    groups: gtk4::Box,
    history: gtk4::Box,
    set: RefCell<UpdateSet>,
    sel_zohara: RefCell<HashSet<String>>,
    sel_flatpak: RefCell<HashSet<String>>,
    sel_system: Cell<bool>,
    busy: Cell<bool>,
}

/// Runs `f` on a worker thread and hands its result to `done` on the UI thread.
fn background<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static, done: impl FnOnce(T) + 'static) {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    let mut done = Some(done);
    glib::timeout_add_local(Duration::from_millis(80), move || match rx.try_recv() {
        Ok(v) => {
            if let Some(d) = done.take() {
                d(v);
            }
            glib::ControlFlow::Break
        }
        Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(TryRecvError::Disconnected) => glib::ControlFlow::Break,
    });
}

pub fn build_page() -> gtk4::Widget {
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(820);
    clamp.set_margin_top(16);
    clamp.set_margin_bottom(24);
    clamp.set_margin_start(16);
    clamp.set_margin_end(16);
    let inner = gtk4::Box::new(gtk4::Orientation::Vertical, 16);

    let title = gtk4::Label::new(Some("Updates"));
    title.add_css_class("page-title");
    title.set_xalign(0.0);
    inner.append(&title);

    // Status card with the main buttons.
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    card.add_css_class("card");
    card.set_margin_bottom(4);
    let card_in = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    card_in.set_margin_top(16);
    card_in.set_margin_bottom(16);
    card_in.set_margin_start(18);
    card_in.set_margin_end(18);
    let head = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    let spinner = gtk4::Spinner::new();
    head.append(&spinner);
    let status_title = gtk4::Label::new(Some("Checking for updates…"));
    status_title.add_css_class("title-3");
    status_title.set_xalign(0.0);
    status_title.set_hexpand(true);
    head.append(&status_title);
    let recheck = gtk4::Button::from_icon_name("view-refresh-symbolic");
    recheck.set_tooltip_text(Some("Check again"));
    recheck.add_css_class("flat");
    head.append(&recheck);
    card_in.append(&head);
    let status_sub = gtk4::Label::new(None);
    status_sub.set_xalign(0.0);
    status_sub.set_wrap(true);
    status_sub.add_css_class("dim-label");
    card_in.append(&status_sub);
    let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    buttons.set_margin_top(6);
    let update_all = gtk4::Button::with_label("Update all");
    update_all.add_css_class("suggested-action");
    update_all.add_css_class("pill");
    update_all.set_sensitive(false);
    let update_sel = gtk4::Button::with_label("Update selected");
    update_sel.add_css_class("pill");
    update_sel.set_sensitive(false);
    buttons.append(&update_all);
    buttons.append(&update_sel);
    card_in.append(&buttons);
    card.append(&card_in);
    inner.append(&card);

    let groups = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    inner.append(&groups);
    let history = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    inner.append(&history);

    clamp.set_child(Some(&inner));
    scroll.set_child(Some(&clamp));

    let page = Rc::new(Page {
        status_title,
        status_sub,
        spinner,
        update_all: update_all.clone(),
        update_sel: update_sel.clone(),
        recheck: recheck.clone(),
        groups,
        history,
        set: RefCell::new(UpdateSet::default()),
        sel_zohara: RefCell::new(HashSet::new()),
        sel_flatpak: RefCell::new(HashSet::new()),
        sel_system: Cell::new(false),
        busy: Cell::new(false),
    });

    {
        let p = page.clone();
        recheck.connect_clicked(move |_| check(&p));
    }
    {
        let p = page.clone();
        update_all.connect_clicked(move |b| {
            let set = p.set.borrow().clone();
            let (sys, z, f) = (!set.system.is_empty(), set.zohara.iter().map(|u| u.name.clone()).collect(), set.flatpak.iter().map(|u| u.app_id.clone()).collect());
            install(&p, b.upcast_ref(), sys, z, f);
        });
    }
    {
        let p = page.clone();
        update_sel.connect_clicked(move |b| {
            let sys = p.sel_system.get() && !p.set.borrow().system.is_empty();
            let z = p.sel_zohara.borrow().iter().cloned().collect();
            let f = p.sel_flatpak.borrow().iter().cloned().collect();
            install(&p, b.upcast_ref(), sys, z, f);
        });
    }

    check(&page);
    load_history(&page);
    scroll.upcast()
}

// ── Checking ───────────────────────────────────────────────────────────────

fn check(page: &Rc<Page>) {
    if page.busy.get() {
        return;
    }
    page.busy.set(true);
    page.spinner.start();
    page.recheck.set_sensitive(false);
    page.update_all.set_sensitive(false);
    page.update_sel.set_sensitive(false);
    page.status_title.set_text("Checking for updates…");
    page.status_sub.set_text("");
    let p = page.clone();
    background(updates::check_all, move |set| {
        p.busy.set(false);
        p.spinner.stop();
        p.recheck.set_sensitive(true);
        // Everything starts ticked.
        *p.sel_zohara.borrow_mut() = set.zohara.iter().map(|u| u.name.clone()).collect();
        *p.sel_flatpak.borrow_mut() = set.flatpak.iter().map(|u| u.app_id.clone()).collect();
        p.sel_system.set(!set.system.is_empty());
        *p.set.borrow_mut() = set;
        render(&p);
    });
}

fn update_summary(page: &Rc<Page>) {
    let set = page.set.borrow();
    let n_sel = page.sel_zohara.borrow().len() + page.sel_flatpak.borrow().len() + if page.sel_system.get() { set.system.len() } else { 0 };
    let total = set.total();
    page.update_all.set_label(&format!("Update all ({total})"));
    page.update_all.set_sensitive(total > 0 && !page.busy.get());
    page.update_sel.set_label(&format!("Update selected ({n_sel})"));
    page.update_sel.set_sensitive(n_sel > 0 && n_sel < total && !page.busy.get());
}

fn render(page: &Rc<Page>) {
    while let Some(c) = page.groups.first_child() {
        page.groups.remove(&c);
    }
    let set = page.set.borrow().clone();
    let total = set.total();
    if total == 0 && set.errors.is_empty() {
        page.status_title.set_text("Everything is up to date");
        page.status_sub.set_text("Zohara, your system and your apps are on the latest versions.");
    } else if total == 0 {
        page.status_title.set_text("Couldn't check for updates");
    } else {
        page.status_title.set_text(&format!("{total} update{} available", if total == 1 { "" } else { "s" }));
        page.status_sub.set_text(if set.system_needs_restart() { "Restart your computer after the system update to finish it." } else { "Choose what to update, or update everything." });
    }
    if !set.errors.is_empty() {
        page.status_sub.set_text(&set.errors.join("\n"));
    }
    update_summary(page);

    let check_row = |title: &str, sub: &str, active: bool, on_toggle: Box<dyn Fn(bool)>| {
        let row = adw::ActionRow::new();
        row.set_title(&glib::markup_escape_text(title));
        row.set_subtitle(&glib::markup_escape_text(sub));
        let check = gtk4::CheckButton::new();
        check.set_active(active);
        check.set_valign(gtk4::Align::Center);
        check.connect_toggled(move |c| on_toggle(c.is_active()));
        row.add_prefix(&check);
        row.set_activatable_widget(Some(&check));
        row
    };

    if !set.zohara.is_empty() {
        let g = adw::PreferencesGroup::new();
        g.set_title("Zohara");
        g.set_description(Some("Settings, the Store and other Zohara apps. Each can be updated on its own."));
        for u in &set.zohara {
            let (p, name) = (page.clone(), u.name.clone());
            g.add(&check_row(
                &u.name,
                &format!("{} → {}", u.old, u.new),
                true,
                Box::new(move |on| {
                    if on {
                        p.sel_zohara.borrow_mut().insert(name.clone());
                    } else {
                        p.sel_zohara.borrow_mut().remove(&name);
                    }
                    update_summary(&p);
                }),
            ));
        }
        page.groups.append(&g);
    }

    if !set.flatpak.is_empty() {
        let g = adw::PreferencesGroup::new();
        g.set_title("Apps");
        for u in &set.flatpak {
            let (p, id) = (page.clone(), u.app_id.clone());
            g.add(&check_row(
                &u.name,
                &format!("From {}", if u.origin.is_empty() { "Flathub" } else { &u.origin }),
                true,
                Box::new(move |on| {
                    if on {
                        p.sel_flatpak.borrow_mut().insert(id.clone());
                    } else {
                        p.sel_flatpak.borrow_mut().remove(&id);
                    }
                    update_summary(&p);
                }),
            ));
        }
        page.groups.append(&g);
    }

    if !set.system.is_empty() {
        let g = adw::PreferencesGroup::new();
        g.set_title("System");
        let ex = adw::ExpanderRow::new();
        ex.set_title(&format!("System updates ({} packages)", set.system.len()));
        ex.set_subtitle(if set.system_needs_restart() { "Updated together to keep everything working. Needs a restart." } else { "Updated together to keep everything working" });
        let check = gtk4::CheckButton::new();
        check.set_active(true);
        check.set_valign(gtk4::Align::Center);
        let p = page.clone();
        check.connect_toggled(move |c| {
            p.sel_system.set(c.is_active());
            update_summary(&p);
        });
        ex.add_prefix(&check);
        for u in &set.system {
            let r = adw::ActionRow::new();
            r.set_title(&glib::markup_escape_text(&u.name));
            r.set_subtitle(&glib::markup_escape_text(&format!("{} → {}", u.old, u.new)));
            ex.add_row(&r);
        }
        g.add(&ex);
        page.groups.append(&g);
    }
}

// ── Installing ─────────────────────────────────────────────────────────────

fn message(parent: &gtk4::Widget, title: &str, body: &str) {
    let d = adw::AlertDialog::new(Some(title), Some(body));
    d.add_response("ok", "OK");
    d.present(Some(parent));
}

/// Shows a window with live output while `work` runs, then reports the result.
fn run_job(page: &Rc<Page>, parent: &gtk4::Widget, title: &str, work: impl FnOnce(&Sender<String>) -> Result<(), String> + Send + 'static) {
    page.busy.set(true);
    update_summary(page);

    let dialog = adw::Dialog::new();
    dialog.set_title(title);
    dialog.set_content_width(620);
    dialog.set_can_close(false);
    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    body.set_margin_top(14);
    body.set_margin_bottom(16);
    body.set_margin_start(16);
    body.set_margin_end(16);
    let head = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    let spinner = gtk4::Spinner::new();
    spinner.start();
    head.append(&spinner);
    let status = gtk4::Label::new(Some("Working… this can take a few minutes."));
    status.set_xalign(0.0);
    status.set_wrap(true);
    status.set_hexpand(true);
    head.append(&status);
    body.append(&head);
    let log = gtk4::TextBuffer::new(None);
    let view = gtk4::TextView::with_buffer(&log);
    view.set_editable(false);
    view.set_monospace(true);
    view.set_cursor_visible(false);
    let sc = gtk4::ScrolledWindow::new();
    sc.set_min_content_height(240);
    sc.set_child(Some(&view));
    body.append(&sc);
    let actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    actions.set_halign(gtk4::Align::End);
    let restart = gtk4::Button::with_label("Restart now");
    restart.add_css_class("suggested-action");
    restart.set_visible(false);
    restart.connect_clicked(|_| {
        let _ = Command::new("systemctl").arg("reboot").spawn();
    });
    let close = gtk4::Button::with_label("Close");
    close.set_sensitive(false);
    actions.append(&restart);
    actions.append(&close);
    body.append(&actions);
    dialog.set_child(Some(&body));
    {
        let d = dialog.clone();
        close.connect_clicked(move |_| {
            d.set_can_close(true);
            d.close();
        });
    }
    dialog.present(Some(parent));

    let (tx, rx) = channel::<String>();
    let (done_tx, done_rx) = channel::<Result<(), String>>();
    std::thread::spawn(move || {
        let r = work(&tx);
        let _ = done_tx.send(r);
    });

    let page = page.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        while let Ok(line) = rx.try_recv() {
            let mut end = log.end_iter();
            log.insert(&mut end, &format!("{line}\n"));
            let mark = log.create_mark(None, &log.end_iter(), false);
            view.scroll_to_mark(&mark, 0.0, false, 0.0, 0.0);
            log.delete_mark(&mark);
        }
        match done_rx.try_recv() {
            Ok(result) => {
                spinner.stop();
                spinner.set_visible(false);
                match result {
                    Ok(()) => {
                        let must = FORCE_RESTART.swap(false, std::sync::atomic::Ordering::SeqCst) || updates::restart_pending();
                        status.set_text(if must { "Done. Restart your computer to finish." } else { "Done." });
                        restart.set_visible(must);
                    }
                    Err(e) => status.set_text(&format!("Something went wrong.\n{e}")),
                }
                close.set_sensitive(true);
                dialog.set_can_close(true);
                page.busy.set(false);
                check(&page);
                load_history(&page);
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => glib::ControlFlow::Break,
        }
    });
}

fn install(page: &Rc<Page>, from: &gtk4::Widget, system: bool, zohara: Vec<String>, flatpaks: Vec<String>) {
    if page.busy.get() {
        return;
    }
    run_job(page, from, "Updating", move |tx| updates::apply(system, &zohara, &flatpaks, tx));
}

// ── Going back ─────────────────────────────────────────────────────────────

struct HistoryData {
    last: Option<Transaction>,
    zohara: Vec<String>,
    flatpak: Vec<FlatpakInstalled>,
    snap: SnapStatus,
    /// `None`: they exist but listing them needs a password.
    snaps: Option<Vec<Snapshot>>,
}

fn load_history(page: &Rc<Page>) {
    let p = page.clone();
    background(
        || {
            let zohara = Command::new("pacman")
                .arg("-Qq")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).lines().filter(|l| l.starts_with("zohara-")).map(String::from).collect())
                .unwrap_or_default();
            let snap = updates::snapshot_status();
            let snaps = if snap.configured { updates::list_snapshots(false).ok() } else { Some(Vec::new()) };
            HistoryData { last: updates::last_update(), zohara, flatpak: updates::flatpak_installed(), snap, snaps }
        },
        move |h| render_history(&p, h),
    );
}

fn render_history(page: &Rc<Page>, h: HistoryData) {
    while let Some(c) = page.history.first_child() {
        page.history.remove(&c);
    }
    let g = adw::PreferencesGroup::new();
    g.set_title("Go back");
    g.set_description(Some("If an update didn't turn out well, you can return to the version you had before."));

    // Undo the last update.
    let undo = adw::ActionRow::new();
    undo.set_title("Undo the last update");
    match &h.last {
        Some(t) => undo.set_subtitle(&glib::markup_escape_text(&format!("{} package{} updated on {}", t.upgraded.len(), if t.upgraded.len() == 1 { "" } else { "s" }, t.when))),
        None => undo.set_subtitle("There's no recent update to undo"),
    }
    undo.add_prefix(&gtk4::Image::from_icon_name("edit-undo-symbolic"));
    let undo_btn = gtk4::Button::with_label("Undo…");
    undo_btn.set_valign(gtk4::Align::Center);
    undo_btn.set_sensitive(h.last.is_some());
    undo.add_suffix(&undo_btn);
    g.add(&undo);
    if let Some(t) = h.last.clone() {
        let p = page.clone();
        undo_btn.connect_clicked(move |b| {
            let (files, missing) = updates::files_to_undo(&t);
            let parent: gtk4::Widget = b.clone().upcast();
            if !missing.is_empty() {
                message(&parent, "Can't undo this update", &format!("These earlier versions are no longer saved on this computer:\n{}", missing.join("\n")));
                return;
            }
            let d = adw::AlertDialog::new(Some("Undo the last update?"), Some(&format!("{} package{} will go back to the version you had before. Your files and settings aren't touched.", files.len(), if files.len() == 1 { "" } else { "s" })));
            d.add_responses(&[("cancel", "Cancel"), ("undo", "Undo update")]);
            d.set_response_appearance("undo", adw::ResponseAppearance::Destructive);
            let (p2, parent2) = (p.clone(), parent.clone());
            d.connect_response(None, move |_, r| {
                if r == "undo" {
                    let files = files.clone();
                    run_job(&p2, &parent2, "Going back", move |tx| updates::downgrade(&files, tx));
                }
            });
            d.present(Some(&parent));
        });
    }

    // Zohara apps, one by one.
    if !h.zohara.is_empty() {
        let ex = adw::ExpanderRow::new();
        ex.set_title("Zohara apps");
        ex.set_subtitle("Return one of them to an earlier version");
        ex.add_prefix(&gtk4::Image::from_icon_name("system-software-install-symbolic"));
        for name in &h.zohara {
            let row = adw::ActionRow::new();
            row.set_title(&glib::markup_escape_text(name));
            row.set_subtitle(&glib::markup_escape_text(&updates::installed_version(name).unwrap_or_default()));
            let b = gtk4::Button::with_label("Go back…");
            b.set_valign(gtk4::Align::Center);
            let (p, n) = (page.clone(), name.clone());
            b.connect_clicked(move |b| pick_pacman_version(&p, b.upcast_ref(), &n));
            row.add_suffix(&b);
            ex.add_row(&row);
        }
        g.add(&ex);
    }

    // Flatpak apps.
    if !h.flatpak.is_empty() {
        let ex = adw::ExpanderRow::new();
        ex.set_title("Apps");
        ex.set_subtitle("Return an app to an earlier version");
        ex.add_prefix(&gtk4::Image::from_icon_name("view-app-grid-symbolic"));
        for app in &h.flatpak {
            let row = adw::ActionRow::new();
            row.set_title(&glib::markup_escape_text(&app.name));
            let b = gtk4::Button::with_label("Go back…");
            b.set_valign(gtk4::Align::Center);
            let (p, a) = (page.clone(), app.clone());
            b.connect_clicked(move |b| pick_flatpak_version(&p, b.upcast_ref(), &a));
            row.add_suffix(&b);
            ex.add_row(&row);
        }
        g.add(&ex);
    }
    page.history.append(&g);
    page.history.append(&restore_group(page, &h));
}

/// Whole-system restore points (Btrfs snapshots).
fn restore_group(page: &Rc<Page>, h: &HistoryData) -> adw::PreferencesGroup {
    let g = adw::PreferencesGroup::new();
    g.set_title("Restore the whole system");
    g.set_description(Some("A restore point is saved automatically before and after every update. Restoring one puts apps and system files back as they were; your own files in Home aren't touched."));

    let info = |title: &str, sub: &str, icon: &str| {
        let r = adw::ActionRow::new();
        r.set_title(title);
        r.set_subtitle(sub);
        r.add_prefix(&gtk4::Image::from_icon_name(icon));
        r
    };
    if !h.snap.supported {
        g.add(&info("System restore is off", "This computer wasn't installed with Btrfs, which restore points need. A fresh install with Btrfs (the default) turns it on.", "dialog-information-symbolic"));
        return g;
    }
    if !h.snap.configured {
        g.add(&info("Setting up system restore…", "It finishes the next time you restart.", "emblem-synchronizing-symbolic"));
        return g;
    }

    let ex = adw::ExpanderRow::new();
    ex.set_title("Restore points");
    ex.add_prefix(&gtk4::Image::from_icon_name("document-open-recent-symbolic"));
    g.add(&ex);
    match &h.snaps {
        Some(list) => fill_restore_points(page, &ex, list),
        None => {
            ex.set_subtitle("Show them to choose one");
            let show = gtk4::Button::with_label("Show…");
            show.set_valign(gtk4::Align::Center);
            let (p, ex2) = (page.clone(), ex.clone());
            show.connect_clicked(move |b| {
                b.set_sensitive(false);
                let (p, ex, b) = (p.clone(), ex2.clone(), b.clone());
                background(|| updates::list_snapshots(true), move |r| {
                    b.set_sensitive(true);
                    match r {
                        Ok(list) => {
                            b.set_visible(false);
                            fill_restore_points(&p, &ex, &list);
                        }
                        Err(e) => ex.set_subtitle(&glib::markup_escape_text(&e)),
                    }
                });
            });
            ex.add_suffix(&show);
        }
    }
    g
}

fn fill_restore_points(page: &Rc<Page>, ex: &adw::ExpanderRow, list: &[Snapshot]) {
    ex.set_subtitle(&format!("{} saved", list.len()));
    if list.is_empty() {
        let r = adw::ActionRow::new();
        r.set_title("None yet");
        ex.add_row(&r);
    }
    for s in list.iter().take(30) {
        let row = adw::ActionRow::new();
        row.set_title(&glib::markup_escape_text(&s.title()));
        row.set_subtitle(&glib::markup_escape_text(&s.date));
        let b = gtk4::Button::with_label("Restore…");
        b.set_valign(gtk4::Align::Center);
        let (p, n, title) = (page.clone(), s.number, s.title());
        b.connect_clicked(move |b| {
            let d = adw::AlertDialog::new(
                Some("Restore the system to this point?"),
                Some(&format!("“{title}”. Apps and system files go back as they were then, and you'll restart your computer. Your own files in Home aren't touched. Anything installed or changed since will be undone.")),
            );
            d.add_responses(&[("cancel", "Cancel"), ("restore", "Restore")]);
            d.set_response_appearance("restore", adw::ResponseAppearance::Destructive);
            let (p2, parent) = (p.clone(), b.clone().upcast::<gtk4::Widget>());
            d.connect_response(None, move |_, r| {
                if r == "restore" {
                    run_job(&p2, &parent, "Restoring", move |tx| {
                        let res = updates::restore_snapshot(n, tx);
                        if res.is_ok() {
                            FORCE_RESTART.store(true, std::sync::atomic::Ordering::SeqCst);
                        }
                        res
                    });
                }
            });
            d.present(Some(b));
        });
        row.add_suffix(&b);
        ex.add_row(&row);
    }
}

const MAX_CHOICES: usize = 5;

fn pick_pacman_version(page: &Rc<Page>, from: &gtk4::Widget, name: &str) {
    let (name2, from2, page2) = (name.to_string(), from.clone(), page.clone());
    from.set_sensitive(false);
    let from_btn = from.clone();
    background(
        {
            let n = name.to_string();
            move || updates::older_versions(&n)
        },
        move |versions| {
            from_btn.set_sensitive(true);
            if versions.is_empty() {
                message(&from2, "No earlier version saved", &format!("{name2} has no earlier version stored on this computer."));
                return;
            }
            let versions: Vec<_> = versions.into_iter().take(MAX_CHOICES).collect();
            let d = adw::AlertDialog::new(Some(&format!("Go back to which version of {name2}?")), Some("Choose the version to return to."));
            d.add_response("cancel", "Cancel");
            for (i, v) in versions.iter().enumerate() {
                d.add_response(&format!("v{i}"), &v.version);
            }
            let (page3, from3) = (page2.clone(), from2.clone());
            d.connect_response(None, move |_, r| {
                if let Some(i) = r.strip_prefix('v').and_then(|s| s.parse::<usize>().ok()) {
                    if let Some(v) = versions.get(i) {
                        let files = vec![v.path.clone()];
                        run_job(&page3, &from3, "Going back", move |tx| updates::downgrade(&files, tx));
                    }
                }
            });
            d.present(Some(&from2));
        },
    );
}

fn pick_flatpak_version(page: &Rc<Page>, from: &gtk4::Widget, app: &FlatpakInstalled) {
    let (app2, from2, page2) = (app.clone(), from.clone(), page.clone());
    from.set_sensitive(false);
    let from_btn = from.clone();
    background(
        {
            let a = app.clone();
            move || updates::flatpak_history(&a)
        },
        move |res| {
            from_btn.set_sensitive(true);
            let commits = match res {
                Ok(c) if !c.is_empty() => c,
                Ok(_) => return message(&from2, "No earlier version available", &format!("{} has no earlier version to go back to.", app2.name)),
                Err(e) => return message(&from2, "Couldn't get the versions", &e),
            };
            let commits: Vec<_> = commits.into_iter().take(MAX_CHOICES).collect();
            let d = adw::AlertDialog::new(Some(&format!("Go back to which version of {}?", app2.name)), Some("Choose the version to return to."));
            d.add_response("cancel", "Cancel");
            for (i, c) in commits.iter().enumerate() {
                let label = if c.subject.is_empty() { c.date.clone() } else { format!("{} ({})", c.subject, c.date) };
                d.add_response(&format!("v{i}"), &label);
            }
            let (page3, from3, id) = (page2.clone(), from2.clone(), app2.app_id.clone());
            d.connect_response(None, move |_, r| {
                if let Some(i) = r.strip_prefix('v').and_then(|s| s.parse::<usize>().ok()) {
                    if let Some(c) = commits.get(i) {
                        let (id, hash) = (id.clone(), c.hash.clone());
                        run_job(&page3, &from3, "Going back", move |tx| updates::flatpak_go_back(&id, &hash, tx));
                    }
                }
            });
            d.present(Some(&from2));
        },
    );
}
