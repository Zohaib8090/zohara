use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::process::{Command, Stdio};
use std::io::Write;

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
        .label("Accounts")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-page-title".to_string()])
        .build();
    root_box.append(&title_lbl);

    // ── Hero current-user card ──────────────────────────────────────────────
    let user_name = std::env::var("USER").unwrap_or_else(|_| "zohaib".to_string());
    let display_name = format!("{} BAIG", user_name.to_uppercase());

    let hero_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 20);
    hero_card.set_css_classes(&["win11-hero-card"]);
    hero_card.set_margin_bottom(4);

    let avatar_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    avatar_box.set_css_classes(&["win11-avatar-circle-large"]);
    let avatar_icon = gtk4::Image::from_icon_name("avatar-default-symbolic");
    avatar_icon.set_pixel_size(44);
    avatar_box.append(&avatar_icon);
    hero_card.append(&avatar_box);

    let user_info_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    user_info_box.set_valign(gtk4::Align::Center);
    user_info_box.append(
        &gtk4::Label::builder()
            .label(&display_name)
            .halign(gtk4::Align::Start)
            .css_classes(vec!["win11-device-name".to_string()])
            .build(),
    );
    user_info_box.append(
        &gtk4::Label::builder()
            .label(&format!("{}@zohara.os", user_name))
            .halign(gtk4::Align::Start)
            .css_classes(vec!["win11-device-sub".to_string()])
            .build(),
    );
    let admin_badge = gtk4::Label::builder()
        .label("Local account • Administrator")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-pill".to_string()])
        .build();
    user_info_box.append(&admin_badge);
    hero_card.append(&user_info_box);
    root_box.append(&hero_card);

    // ── Account settings group ───────────────────────────────────────────────
    let section_lbl = gtk4::Label::builder()
        .label("Account settings")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&section_lbl);

    let rows_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    rows_box.set_css_classes(&["win11-card-group"]);

    // 1. Change password (via pkexec chpasswd)
    let chg_pass_row = adw::ActionRow::new();
    chg_pass_row.set_title("Account password");
    chg_pass_row.set_subtitle("Change the password for the current account");
    chg_pass_row.add_prefix(&gtk4::Image::from_icon_name("dialog-password-symbolic"));
    let chg_btn = gtk4::Button::builder()
        .label("Change")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["win11-secondary-btn".to_string()])
        .build();
    chg_btn.connect_clicked(|btn| {
        let parent = btn.root().and_downcast::<gtk4::Window>();
        let dialog = adw::MessageDialog::builder()
            .heading("Change account password")
            .body("Enter your new password below. The password is set via pkexec chpasswd.")
            .transient_for(parent.as_ref().unwrap_or(&gtk4::Window::new()))
            .build();
        let pass_entry = gtk4::PasswordEntry::builder()
            .margin_top(8)
            .margin_bottom(8)
            .show_peek_icon(true)
            .build();
        dialog.set_extra_child(Some(&pass_entry));
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("save", "Save");
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.connect_response(None, move |d, resp| {
            if resp == "save" {
                let p = pass_entry.text().to_string();
                if !p.is_empty() {
                    let user = std::env::var("USER").unwrap_or_default();
                    if let Ok(mut child) = Command::new("pkexec")
                        .args(["chpasswd"])
                        .stdin(Stdio::piped())
                        .spawn()
                    {
                        if let Some(stdin) = child.stdin.as_mut() {
                            let _ = writeln!(stdin, "{}:{}", user, p);
                        }
                        let _ = child.wait();
                    }
                }
            }
            d.close();
        });
        dialog.present();
    });
    chg_pass_row.add_suffix(&chg_btn);
    rows_box.append(&chg_pass_row);

    // 2. Add other user (real pkexec useradd)
    let add_user_row = adw::ActionRow::new();
    add_user_row.set_title("Add another user");
    add_user_row.set_subtitle("Create a new standard account (administrator access can be granted after)");
    add_user_row.add_prefix(&gtk4::Image::from_icon_name("contact-new-symbolic"));
    let add_user_btn = gtk4::Button::builder()
        .label("Add account")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["win11-secondary-btn".to_string()])
        .build();
    add_user_btn.connect_clicked(|btn| {
        let parent = btn.root().and_downcast::<gtk4::Window>();
        let dialog = adw::MessageDialog::builder()
            .heading("Create new user account")
            .body("Enter a username (lowercase, no spaces). The account is created via pkexec useradd.")
            .transient_for(parent.as_ref().unwrap_or(&gtk4::Window::new()))
            .build();
        let name_entry = gtk4::Entry::builder()
            .placeholder_text("username")
            .margin_top(8)
            .margin_bottom(8)
            .build();
        dialog.set_extra_child(Some(&name_entry));
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("create", "Create account");
        dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);
        dialog.connect_response(None, move |d, resp| {
            if resp == "create" {
                let name = name_entry.text().trim().to_string();
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
                    // -m: create home, -s: set shell
                    let _ = Command::new("pkexec")
                        .args(["useradd", "-m", "-s", "/bin/bash", &name])
                        .status();
                }
            }
            d.close();
        });
        dialog.present();
    });
    add_user_row.add_suffix(&add_user_btn);
    rows_box.append(&add_user_row);

    // 3. Sign-in options
    let sign_row = adw::ExpanderRow::new();
    sign_row.set_title("Sign-in options");
    sign_row.set_subtitle("Password requirements, auto-login, lock-screen behaviour");
    sign_row.add_prefix(&gtk4::Image::from_icon_name("system-lock-screen-symbolic"));
    let auto_row = adw::SwitchRow::new();
    auto_row.set_title("Disable lock screen");
    auto_row.set_subtitle("When on, the screen will not lock after suspend (less secure)");
    sign_row.add_row(&auto_row);
    rows_box.append(&sign_row);

    // 4. Other users -- list all users from /etc/passwd with UID >= 1000
    let users_card = adw::PreferencesGroup::builder()
        .title("Other users on this device")
        .description("Standard and administrator accounts on this Zohara OS install.")
        .build();
    for user in list_users() {
        let row = adw::ActionRow::new();
        row.set_title(&user.username);
        row.set_subtitle(&user.detail);
        row.add_prefix(&gtk4::Image::from_icon_name(
            if user.is_admin { "avatar-default-symbolic" } else { "system-users-symbolic" },
        ));
        users_card.add(&row);
    }
    root_box.append(&rows_box);
    root_box.append(&users_card);

    scroll.set_child(Some(&root_box));
    scroll.upcast()
}

struct UserEntry {
    username: String,
    detail: String,
    is_admin: bool,
}

fn list_users() -> Vec<UserEntry> {
    let mut out = Vec::new();
    let Ok(content) = std::fs::read_to_string("/etc/passwd") else {
        return out;
    };
    let admins: Vec<u32> = std::fs::read_to_string("/etc/group")
        .ok()
        .map(|s| {
            s.lines()
                .filter(|l| l.starts_with("wheel:") || l.starts_with("sudo:"))
                .filter_map(|l| l.split(':').nth(3))
                .flat_map(|members| members.split(','))
                .filter_map(|name| {
                    std::fs::read_to_string("/etc/passwd").ok().and_then(|passwd| {
                        passwd.lines()
                            .find(|l| l.starts_with(&format!("{}:", name)))
                            .and_then(|l| l.split(':').nth(2))
                            .and_then(|uid| uid.parse::<u32>().ok())
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    for line in content.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 7 {
            continue;
        }
        let Ok(uid) = parts[2].parse::<u32>() else { continue };
        if uid < 1000 || parts[0] == "nobody" {
            continue;
        }
        let is_admin = admins.contains(&uid);
        let detail = if is_admin {
            format!("UID {} • Administrator", uid)
        } else {
            format!("UID {} • Standard", uid)
        };
        out.push(UserEntry {
            username: parts[0].to_string(),
            detail,
            is_admin,
        });
    }
    out
}
