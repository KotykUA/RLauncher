//! Minecraft news page.

use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::core::news;
use crate::ui::window::Ctx;

pub fn present(ctx: &Ctx) {
    let dialog = adw::Dialog::new();
    dialog.set_title("Minecraft News");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    // Header with refresh button
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title = gtk::Label::builder()
        .label("Minecraft News")
        .css_classes(["heading"])
        .halign(gtk::Align::Start)
        .hexpand(true)
        .build();
    let refresh_button = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh")
        .css_classes(["flat"])
        .build();
    header.append(&title);
    header.append(&refresh_button);

    // News list
    let list = gtk::ListBox::builder()
        .css_classes(["boxed-list"])
        .selection_mode(gtk::SelectionMode::None)
        .vexpand(true)
        .build();

    content.append(&header);
    content.append(&list);

    dialog.set_child(Some(&content));

    // Load news
    let ctx = ctx.clone();
    let list_clone = list.clone();
    load_news(&list_clone, &ctx);

    // Refresh button handler
    let ctx_clone = ctx.clone();
    let list_clone2 = list.clone();
    refresh_button.connect_clicked(move |_| {
        load_news(&list_clone2, &ctx_clone);
    });

    dialog.present(Some(&ctx.window));
}

fn load_news(list: &gtk::ListBox, _ctx: &Ctx) {
    // Clear existing items
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    // Show loading state
    let loading = gtk::Label::builder()
        .label("Loading news…")
        .css_classes(["dim-label"])
        .build();
    list.append(&loading);

    // Load news synchronously for simplicity
    let entries = news::fetch_news(10);

    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    match entries {
        Ok(entries) => {
            if entries.is_empty() {
                let empty = adw::StatusPage::builder()
                    .icon_name("news-symbolic")
                    .title("No news available")
                    .build();
                list.append(&empty);
            } else {
                for entry in entries {
                    let row = build_news_row(&entry);
                    list.append(&row);
                }
            }
        }
        Err(err) => {
            let error = adw::StatusPage::builder()
                .icon_name("network-offline-symbolic")
                .title("Failed to load news")
                .description(&format!("{err:#}"))
                .build();
            list.append(&error);
        }
    }
}

fn build_news_row(entry: &news::NewsEntry) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(true);

    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 6);
    box_.set_margin_top(12);
    box_.set_margin_bottom(12);
    box_.set_margin_start(12);
    box_.set_margin_end(12);

    let title = gtk::Label::builder()
        .label(&entry.title)
        .css_classes(["heading"])
        .halign(gtk::Align::Start)
        .wrap(true)
        .build();

    let body = gtk::Label::builder()
        .label(&entry.body)
        .css_classes(["body", "dim-label"])
        .halign(gtk::Align::Start)
        .wrap(true)
        .build();

    box_.append(&title);
    box_.append(&body);
    row.set_child(Some(&box_));

    row
}
