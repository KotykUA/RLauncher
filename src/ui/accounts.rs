//! Accounts management page.

use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::core::auth::AccountRecord;
use crate::ui::window::Ctx;

pub fn present(ctx: &Ctx) {
    let dialog = adw::Dialog::new();
    dialog.set_title("Accounts");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    // Header with add button
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title = gtk::Label::builder()
        .label("Accounts")
        .css_classes(["heading"])
        .halign(gtk::Align::Start)
        .hexpand(true)
        .build();
    let add_button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("Add Account")
        .build();
    header.append(&title);
    header.append(&add_button);

    // Accounts list
    let list = gtk::ListBox::builder()
        .css_classes(["boxed-list"])
        .selection_mode(gtk::SelectionMode::None)
        .build();

    let settings = ctx.state.settings.borrow();
    let accounts = settings.accounts.clone();
    let active_id = settings.active_account.clone();
    drop(settings);

    if accounts.is_empty() {
        let empty = adw::StatusPage::builder()
            .icon_name("avatar-default-symbolic")
            .title("No accounts")
            .description("Add an account to get started.")
            .build();
        content.append(&header);
        content.append(&empty);
    } else {
        content.append(&header);
        content.append(&list);

        for account in &accounts {
            let is_active = active_id.as_ref() == Some(&account.id());
            let row = build_account_row(account, is_active);
            list.append(&row);
        }
    }

    dialog.set_child(Some(&content));

    // Add offline account button handler
    let ctx_for_present = ctx.clone();
    let ctx = ctx.clone();
    add_button.connect_clicked(move |_| {
        let ctx = ctx.clone();
        show_add_offline_dialog(&ctx);
    });

    dialog.present(Some(&ctx_for_present.window));
}

fn build_account_row(account: &AccountRecord, is_active: bool) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(false);

    let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    box_.set_margin_top(12);
    box_.set_margin_bottom(12);
    box_.set_margin_start(12);
    box_.set_margin_end(12);

    let icon = gtk::Image::from_icon_name("avatar-default-symbolic");
    icon.set_pixel_size(32);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_halign(gtk::Align::Start);
    text.set_hexpand(true);

    let name = gtk::Label::builder()
        .label(account.username())
        .css_classes(if is_active { vec!["heading".to_string()] } else { vec![] })
        .halign(gtk::Align::Start)
        .build();

    let kind = gtk::Label::builder()
        .label(account.kind_label())
        .css_classes(["caption", "dim-label"])
        .halign(gtk::Align::Start)
        .build();

    text.append(&name);
    text.append(&kind);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);

    if is_active {
        let active_label = gtk::Label::builder()
            .label("Active")
            .css_classes(["success", "caption"])
            .build();
        actions.append(&active_label);
    } else {
        let activate_button = gtk::Button::builder()
            .icon_name("emblem-ok-symbolic")
            .tooltip_text("Set as active")
            .css_classes(["flat"])
            .build();
        actions.append(&activate_button);
    }

    let remove_button = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Remove account")
        .css_classes(["flat", "destructive-action"])
        .build();
    actions.append(&remove_button);

    box_.append(&icon);
    box_.append(&text);
    box_.append(&actions);
    row.set_child(Some(&box_));

    row
}

fn show_add_offline_dialog(ctx: &Ctx) {
    let dialog = adw::Dialog::new();
    dialog.set_title("Add Offline Account");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let label = gtk::Label::builder()
        .label("Username")
        .halign(gtk::Align::Start)
        .build();
    let entry = gtk::Entry::builder()
        .placeholder_text("Steve")
        .build();

    content.append(&label);
    content.append(&entry);

    // Button bar
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    button_box.set_halign(gtk::Align::End);
    let cancel_button = gtk::Button::builder()
        .label("Cancel")
        .build();
    let add_button = gtk::Button::builder()
        .label("Add")
        .css_classes(["suggested-action"])
        .build();
    button_box.append(&cancel_button);
    button_box.append(&add_button);
    content.append(&button_box);

    dialog.set_child(Some(&content));

    let dialog_clone = dialog.clone();
    let dialog_cancel = dialog.clone();
    let ctx_for_present = ctx.clone();
    let ctx = ctx.clone();
    add_button.connect_clicked(move |_| {
        let username = entry.text().trim().to_string();

        if username.is_empty() {
            ctx.toast("Please enter a username");
            return;
        }

        let account = AccountRecord::offline(&username);
        let account_id = account.id();
        let mut settings = ctx.state.settings.borrow_mut();
        settings.accounts.push(account);
        settings.active_account = Some(account_id);
        if let Err(err) = settings.save() {
            ctx.toast(&format!("Failed to save account: {err}"));
            return;
        }
        drop(settings);

        ctx.toast("Account added");
        dialog_clone.close();
    });

    cancel_button.connect_clicked(move |_| {
        dialog_cancel.close();
    });

    dialog.present(Some(&ctx_for_present.window));
}
