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

    // ── Hero User Account Card ────────────────────────────────────────────────
    let hero_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 20);
    hero_card.set_css_classes(&["win11-hero-card"]);
    hero_card.set_margin_bottom(4);

    let user_name = std::env::var("USER").unwrap_or_else(|_| "zohaib".to_string());
    let display_name = format!("{} BAIG", user_name.to_uppercase());

    let avatar_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    avatar_box.set_css_classes(&["win11-avatar-circle-large"]);
    let avatar_icon = gtk4::Image::from_icon_name("avatar-default-symbolic");
    avatar_icon.set_pixel_size(44);
    avatar_box.append(&avatar_icon);
    hero_card.append(&avatar_box);

    let user_info_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    user_info_box.set_valign(gtk4::Align::Center);

    let name_lbl = gtk4::Label::builder()
        .label(&display_name)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-name".to_string()])
        .build();

    let email_lbl = gtk4::Label::builder()
        .label(&format!("{}@zohara.os", user_name))
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-device-sub".to_string()])
        .build();

    let admin_badge = gtk4::Label::builder()
        .label("Administrator")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-pill".to_string()])
        .build();

    user_info_box.append(&name_lbl);
    user_info_box.append(&email_lbl);
    user_info_box.append(&admin_badge);
    hero_card.append(&user_info_box);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    hero_card.append(&spacer);

    // Right badge
    let stor_badge = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    stor_badge.set_valign(gtk4::Align::Center);
    let stor_icon = gtk4::Image::from_icon_name("user-home-symbolic");
    stor_icon.set_pixel_size(24);
    stor_icon.set_css_classes(&["accent-cyan"]);
    let stor_lbl = gtk4::Label::builder()
        .label("Local Account\nActive session")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-badge-sub".to_string()])
        .build();
    stor_badge.append(&stor_icon);
    stor_badge.append(&stor_lbl);
    hero_card.append(&stor_badge);

    root_box.append(&hero_card);

    // ── Group Header: Account settings ────────────────────────────────────────
    let section_lbl = gtk4::Label::builder()
        .label("Account settings")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["win11-section-header".to_string()])
        .build();
    root_box.append(&section_lbl);

    // ── Grouped Rows ──────────────────────────────────────────────────────────
    let rows_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    rows_box.set_css_classes(&["win11-card-group"]);

    // 1. Your Info
    let info_row = build_action_row("Your info", "Profile photo, account display name", "avatar-default-symbolic");
    rows_box.append(&info_row);

    // 2. Sign-in Options (with In-App Password Change Dialog)
    let signin_exp = adw::ExpanderRow::new();
    signin_exp.set_title("Sign-in options");
    signin_exp.set_subtitle("Password, authentication security, lock timeout");
    signin_exp.add_prefix(&gtk4::Image::from_icon_name("dialog-password-symbolic"));
    signin_exp.set_css_classes(&["win11-expander-row"]);

    let chg_pass_row = adw::ActionRow::new();
    chg_pass_row.set_title("Account password");
    chg_pass_row.set_subtitle("Change your user account login password");
    let chg_btn = gtk4::Button::builder()
        .label("Change")
        .css_classes(vec!["win11-secondary-btn".to_string()])
        .build();
    chg_btn.connect_clicked(move |btn| {
        let parent = btn.root().and_downcast::<gtk4::Window>();
        let dialog = adw::MessageDialog::builder()
            .heading("Change Account Password")
            .body("Enter your new password below:")
            .transient_for(parent.as_ref().unwrap_or(&gtk4::Window::new()))
            .build();
        let pass_entry = gtk4::PasswordEntry::builder()
            .margin_top(8)
            .margin_bottom(8)
            .build();
        dialog.set_extra_child(Some(&pass_entry));
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("save", "Save Password");
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
    signin_exp.add_row(&chg_pass_row);
    rows_box.append(&signin_exp);

    // 3. Linked Devices
    let linked_row = build_action_row("Linked devices", "Find, repair, and manage devices that are signed in", "computer-symbolic");
    rows_box.append(&linked_row);

    // 4. Your Accounts
    let your_acc_row = build_action_row("Your accounts", "Add or manage accounts used in Zohara OS", "system-users-symbolic");
    rows_box.append(&your_acc_row);

    // 5. Family
    let fam_row = build_action_row("Family", "Manage your family group, edit account types and device permissions", "face-smile-symbolic");
    rows_box.append(&fam_row);

    // 6. Windows / System Backup
    let backup_row = build_action_row("System backup", "Back up your files, apps, preferences to restore them across devices", "drive-harddisk-symbolic");
    rows_box.append(&backup_row);

    // 7. Other Users (with In-App Add User)
    let other_exp = adw::ExpanderRow::new();
    other_exp.set_title("Other users");
    other_exp.set_subtitle("Device access, work or school users, guest account");
    other_exp.add_prefix(&gtk4::Image::from_icon_name("system-users-symbolic"));
    other_exp.set_css_classes(&["win11-expander-row"]);

    let add_user_row = adw::ActionRow::new();
    add_user_row.set_title("Add other user");
    add_user_row.set_subtitle("Create a standard or administrator user on this PC");
    let add_user_btn = gtk4::Button::builder()
        .label("Add account")
        .css_classes(vec!["win11-secondary-btn".to_string()])
        .build();
    add_user_btn.connect_clicked(move |btn| {
        let parent = btn.root().and_downcast::<gtk4::Window>();
        let dialog = adw::MessageDialog::builder()
            .heading("Create New User Account")
            .body("Enter username for the new account:")
            .transient_for(parent.as_ref().unwrap_or(&gtk4::Window::new()))
            .build();
        let name_entry = gtk4::Entry::builder().margin_top(8).margin_bottom(8).build();
        dialog.set_extra_child(Some(&name_entry));
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("create", "Create Account");
        dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);

        dialog.connect_response(None, move |d, resp| {
            if resp == "create" {
                let n = name_entry.text().trim().to_string();
                if !n.is_empty() {
                    let _ = Command::new("pkexec").args(["useradd", "-m", &n]).spawn();
                }
            }
            d.close();
        });
        dialog.present();
    });
    add_user_row.add_suffix(&add_user_btn);
    other_exp.add_row(&add_user_row);
    rows_box.append(&other_exp);

    // 8. Access Work or School
    let work_row = build_action_row("Access work or school", "Organization resources like email, apps, and network", "folder-remote-symbolic");
    rows_box.append(&work_row);

    root_box.append(&rows_box);
    scroll.set_child(Some(&root_box));
    scroll.upcast()
}

fn build_action_row(title: &str, subtitle: &str, icon_name: &str) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(title);
    row.set_subtitle(subtitle);
    row.add_prefix(&gtk4::Image::from_icon_name(icon_name));
    row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
    row.set_css_classes(&["win11-expander-row"]);
    row.set_activatable(true);
    row
}
