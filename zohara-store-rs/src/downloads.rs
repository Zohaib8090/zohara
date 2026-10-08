//! Everything the Store is installing or removing right now, so the Updates page can show it however many apps were
//! started. The package manager does one job at a time (see `backend::PACKAGE_LOCK`), so apps started together show
//! as "Waiting" until it is their turn.
//!
//! Lives on the UI thread only (GTK is single-threaded): the Get/Remove buttons report in, the Updates page and the
//! tab badge listen.

use std::cell::RefCell;

use crate::backend::Progress;

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Started, the package manager is busy with another app.
    Waiting,
    Working,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: u64,
    pub name: String,
    pub removing: bool,
    pub state: State,
    pub text: String,
    /// 0..1 when known; `None` when all that is known is "still working".
    pub fraction: Option<f64>,
}

impl Entry {
    /// "Installing Firefox" / "Removing Firefox".
    pub fn title(&self) -> String {
        format!("{} {}", if self.removing { "Removing" } else { "Installing" }, self.name)
    }
}

#[derive(Default)]
struct Tracker {
    next: u64,
    entries: Vec<Entry>,
    listeners: Vec<Box<dyn Fn() -> bool>>,
}

thread_local! {
    static TRACKER: RefCell<Tracker> = RefCell::new(Tracker::default());
}

/// Calls every listener; one that returns false is dropped. Listeners may add listeners.
fn notify() {
    let mut ls = TRACKER.with(|t| std::mem::take(&mut t.borrow_mut().listeners));
    ls.retain(|f| f());
    TRACKER.with(|t| {
        let mut t = t.borrow_mut();
        let added = std::mem::take(&mut t.listeners);
        t.listeners = ls;
        t.listeners.extend(added);
    });
}

/// `f` runs whenever the list changes; return false from it to stop listening (its window is gone).
pub fn on_change(f: impl Fn() -> bool + 'static) {
    TRACKER.with(|t| t.borrow_mut().listeners.push(Box::new(f)));
}

/// A job was started for `name`. Returns its id for the other calls.
pub fn start(name: &str, removing: bool) -> u64 {
    let id = TRACKER.with(|t| {
        let mut t = t.borrow_mut();
        t.next += 1;
        let id = t.next;
        t.entries.push(Entry {
            id,
            name: name.to_string(),
            removing,
            state: State::Waiting,
            text: "Waiting for the other installs to finish…".into(),
            fraction: Some(0.0),
        });
        id
    });
    notify();
    id
}

/// News from the job: it has its turn now, and how far it is.
pub fn progress(id: u64, p: &Progress) {
    let changed = TRACKER.with(|t| {
        match t.borrow_mut().entries.iter_mut().find(|e| e.id == id) {
            Some(e) => {
                e.state = State::Working;
                e.text = p.text.clone();
                e.fraction = p.fraction;
                true
            }
            None => false,
        }
    });
    if changed {
        notify();
    }
}

/// The job ended. A finished one disappears; a failed one stays, with the reason, until dismissed.
pub fn finish(id: u64, result: Result<(), String>) {
    TRACKER.with(|t| {
        let mut t = t.borrow_mut();
        match result {
            Ok(()) => t.entries.retain(|e| e.id != id),
            Err(why) => {
                if let Some(e) = t.entries.iter_mut().find(|e| e.id == id) {
                    e.state = State::Failed(why);
                    e.fraction = None;
                }
            }
        }
    });
    notify();
}

pub fn dismiss(id: u64) {
    TRACKER.with(|t| t.borrow_mut().entries.retain(|e| e.id != id));
    notify();
}

/// Jobs the Updates page lists, oldest first.
pub fn snapshot() -> Vec<Entry> {
    TRACKER.with(|t| t.borrow().entries.clone())
}

/// Jobs that are waiting or running (not the failed ones), for the tab badge.
pub fn active_count() -> usize {
    TRACKER.with(|t| t.borrow().entries.iter().filter(|e| !matches!(e.state, State::Failed(_))).count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn reset() {
        TRACKER.with(|t| *t.borrow_mut() = Tracker::default());
    }

    #[test]
    fn several_apps_queue_then_run_one_after_another() {
        reset();
        let a = start("Firefox", false);
        let b = start("VLC", false);
        let c = start("GIMP", true);
        assert_eq!(active_count(), 3);
        assert!(snapshot().iter().all(|e| e.state == State::Waiting));

        progress(a, &Progress { fraction: Some(0.4), text: "Downloading".into() });
        let s = snapshot();
        assert_eq!(s[0].state, State::Working);
        assert_eq!(s[0].fraction, Some(0.4));
        assert_eq!(s[1].state, State::Waiting);
        assert_eq!(s[0].title(), "Installing Firefox");
        assert_eq!(s[2].title(), "Removing GIMP");

        finish(a, Ok(()));
        assert_eq!(snapshot().iter().map(|e| e.id).collect::<Vec<_>>(), vec![b, c]);
    }

    #[test]
    fn a_failure_stays_with_its_reason_until_dismissed_and_is_not_counted_as_active() {
        reset();
        let a = start("Firefox", false);
        finish(a, Err("No space left".into()));
        assert_eq!(snapshot()[0].state, State::Failed("No space left".into()));
        assert_eq!(active_count(), 0);
        dismiss(a);
        assert!(snapshot().is_empty());
    }

    #[test]
    fn listeners_hear_changes_and_can_stop_listening() {
        reset();
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        on_change(move || {
            c.set(c.get() + 1);
            c.get() < 2
        });
        let a = start("A", false);
        progress(a, &Progress { fraction: None, text: "x".into() });
        finish(a, Ok(()));
        assert_eq!(calls.get(), 2, "dropped after it returned false");
    }

    #[test]
    fn news_about_a_job_that_is_gone_is_ignored() {
        reset();
        progress(99, &Progress { fraction: Some(1.0), text: "x".into() });
        assert!(snapshot().is_empty());
    }
}
