//! The Updates page: update the apps from Flathub (all, or the ones you pick) and go back if an update didn't turn out
//! well. All clicks. The operating system is updated in Zohara Settings, a separate program.

use adw::prelude::*;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::time::Duration;

use crate::updates::{self, FlatpakInstalled, UpdateSet};

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
    sel_flatpak: RefCell<HashSet<String>>,
    busy: Cell<bool>,
}

/// Runs `f` on a worker thread and hands its result to `done` on the UI thread.
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

/// One line of the "Installing and removing" list.
struct DownloadRow {
    id: u64,
    row: adw::ActionRow,
    icon: gtk4::Image,
    bar: gtk4::ProgressBar,
    dismiss: gtk4::Button,
}

/// The list of apps being installed or removed, hidden while there are none. Rows are updated in place as news
/// arrives, so the bars move smoothly instead of the list being rebuilt each time.
fn downloads_group() -> adw::PreferencesGroup {
    use crate::downloads::{self, State};
    let group = adw::PreferencesGroup::new();
    group.set_title("Installing and removing");
    group.set_description(Some("Apps you started from the Store. They run one after another."));
    group.set_visible(false);

    let rows: Rc<RefCell<Vec<DownloadRow>>> = Rc::new(RefCell::new(Vec::new()));
    let weak = group.downgrade();
    let render = move || {
        let Some(group) = weak.upgrade() else { return false };
        let entries = downloads::snapshot();
        let mut rows = rows.borrow_mut();
        // Lines for jobs that are gone.
        rows.retain(|r| {
            let keep = entries.iter().any(|e| e.id == r.id);
            if !keep {
                group.remove(&r.row);
            }
            keep
        });
        for e in &entries {
            if !rows.iter().any(|r| r.id == e.id) {
                let row = adw::ActionRow::new();
                row.set_subtitle_lines(2);
                let icon = gtk4::Image::from_icon_name("folder-download-symbolic");
                row.add_prefix(&icon);
                let bar = gtk4::ProgressBar::new();
                bar.set_valign(gtk4::Align::Center);
                bar.set_size_request(160, -1);
                row.add_suffix(&bar);
                let dismiss = gtk4::Button::with_label("Dismiss");
                dismiss.set_valign(gtk4::Align::Center);
                dismiss.add_css_class("flat");
                let id = e.id;
                dismiss.connect_clicked(move |_| downloads::dismiss(id));
                row.add_suffix(&dismiss);
                group.add(&row);
                rows.push(DownloadRow { id: e.id, row, icon, bar, dismiss });
            }
            let r = rows.iter().find(|r| r.id == e.id).expect("row exists");
            r.row.set_title(&glib::markup_escape_text(&e.title()));
            let failed = matches!(e.state, State::Failed(_));
            match &e.state {
                State::Failed(why) => {
                    r.row.set_subtitle(&glib::markup_escape_text(&format!("It didn't work. {why}")));
                    r.row.add_css_class("error");
                    r.icon.set_icon_name(Some("dialog-error-symbolic"));
                }
                _ => {
                    r.row.set_subtitle(&glib::markup_escape_text(&e.text));
                    r.row.remove_css_class("error");
                    r.icon.set_icon_name(Some("folder-download-symbolic"));
                }
            }
            r.bar.set_visible(!failed);
            r.dismiss.set_visible(failed);
            match (&e.state, e.fraction) {
                (State::Waiting, _) => r.bar.set_fraction(0.0),
                (_, Some(f)) => r.bar.set_fraction(f),
                (_, None) => r.bar.pulse(),
            }
        }
        group.set_visible(!entries.is_empty());
        true
    };
    render();
    downloads::on_change(render);
    group
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

    let title = gtk4::Label::new(Some("App updates"));
    title.add_css_class("page-title");
    title.set_xalign(0.0);
    inner.append(&title);

    // Apps being installed or removed right now (started from any page), however many.
    inner.append(&downloads_group());

    // The operating system is not updated here: say where it is, with a button.
    let sys_group = adw::PreferencesGroup::new();
    let sys_row = adw::ActionRow::new();
    sys_row.set_title("System updates are in Settings");
    sys_row.set_subtitle("The system, Zohara itself and apps installed from the Arch repositories (like Firefox) are updated in Settings, under Zohara Update");
    sys_row.add_prefix(&gtk4::Image::from_icon_name("emblem-system-symbolic"));
    let sys_btn = gtk4::Button::with_label("Open Zohara Update");
    sys_btn.set_valign(gtk4::Align::Center);
    sys_btn.connect_clicked(|_| {
        let _ = Command::new("zohara-settings").args(["--page", "Zohara Update"]).spawn();
    });
    sys_row.add_suffix(&sys_btn);
    sys_group.add(&sys_row);
    inner.append(&sys_group);

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
    let status_title = gtk4::Label::new(Some("Checking for app updates…"));
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
        sel_flatpak: RefCell::new(HashSet::new()),
        busy: Cell::new(false),
    });

    {
        let p = page.clone();
        recheck.connect_clicked(move |_| check(&p));
    }
    {
        let p = page.clone();
        update_all.connect_clicked(move |b| {
            let f = p.set.borrow().flatpak.iter().map(|u| u.app_id.clone()).collect();
            install(&p, b.upcast_ref(), f);
        });
    }
    {
        let p = page.clone();
        update_sel.connect_clicked(move |b| {
            let f = p.sel_flatpak.borrow().iter().cloned().collect();
            install(&p, b.upcast_ref(), f);
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
    page.status_title.set_text("Checking for app updates…");
    page.status_sub.set_text("");
    let p = page.clone();
    background(updates::check_all, move |set| {
        p.busy.set(false);
        p.spinner.stop();
        p.recheck.set_sensitive(true);
        // Everything starts ticked.
        *p.sel_flatpak.borrow_mut() = set.flatpak.iter().map(|u| u.app_id.clone()).collect();
        *p.set.borrow_mut() = set;
        render(&p);
    });
}

fn update_summary(page: &Rc<Page>) {
    let set = page.set.borrow();
    let n_sel = page.sel_flatpak.borrow().len();
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
        page.status_title.set_text("Your apps are up to date");
        page.status_sub.set_text("Apps from Flathub are on the latest versions.");
    } else if total == 0 {
        page.status_title.set_text("Couldn't check for app updates");
    } else {
        page.status_title.set_text(&format!("{total} app update{} available", if total == 1 { "" } else { "s" }));
        page.status_sub.set_text("Choose what to update, or update everything.");
    }
    if !set.errors.is_empty() {
        page.status_sub.set_text(&set.errors.join("\n"));
    }
    update_summary(page);

    if !set.flatpak.is_empty() {
        let g = adw::PreferencesGroup::new();
        g.set_title("Apps");
        for u in &set.flatpak {
            let (p, id) = (page.clone(), u.app_id.clone());
            let row = adw::ActionRow::new();
            row.set_title(&glib::markup_escape_text(&u.name));
            row.set_subtitle(&glib::markup_escape_text(&format!("From {}", if u.origin.is_empty() { "Flathub" } else { &u.origin })));
            let check = gtk4::CheckButton::new();
            check.set_active(true);
            check.set_valign(gtk4::Align::Center);
            check.connect_toggled(move |c| {
                if c.is_active() {
                    p.sel_flatpak.borrow_mut().insert(id.clone());
                } else {
                    p.sel_flatpak.borrow_mut().remove(&id);
                }
                update_summary(&p);
            });
            row.add_prefix(&check);
            row.set_activatable_widget(Some(&check));
            g.add(&row);
        }
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
    let close = gtk4::Button::with_label("Close");
    close.set_sensitive(false);
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
                    Ok(()) => status.set_text("Done."),
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

fn install(page: &Rc<Page>, from: &gtk4::Widget, flatpaks: Vec<String>) {
    if page.busy.get() {
        return;
    }
    run_job(page, from, "Updating", move |tx| updates::apply(&flatpaks, tx));
}

// ── Going back ─────────────────────────────────────────────────────────────

const MAX_CHOICES: usize = 5;

fn load_history(page: &Rc<Page>) {
    let p = page.clone();
    background(updates::flatpak_installed, move |apps| render_history(&p, apps));
}

fn render_history(page: &Rc<Page>, apps: Vec<FlatpakInstalled>) {
    while let Some(c) = page.history.first_child() {
        page.history.remove(&c);
    }
    if apps.is_empty() {
        return;
    }
    let g = adw::PreferencesGroup::new();
    g.set_title("Go back");
    g.set_description(Some("If an app update didn't turn out well, you can return to the version you had before."));
    let ex = adw::ExpanderRow::new();
    ex.set_title("Apps");
    ex.set_subtitle("Return an app to an earlier version");
    ex.add_prefix(&gtk4::Image::from_icon_name("view-app-grid-symbolic"));
    for app in &apps {
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
    page.history.append(&g);
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
