//! Preferences/settings page.

use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::config::settings::Settings;
use crate::ui::widgets;
use crate::ui::window::Ctx;

pub fn present(ctx: &Ctx) {
    let dialog = adw::Dialog::new();
    dialog.set_title("Preferences");

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_width_request(500);

    let page = adw::PreferencesPage::new();

    // General section
    let general_group = widgets::group("General");

    // Java path
    let java_row = adw::ActionRow::builder()
        .title("Java Path")
        .subtitle("Path to Java executable (optional)")
        .build();
    let java_entry = gtk::Entry::builder()
        .placeholder_text("/usr/bin/java")
        .build();
    let browse_button = gtk::Button::builder()
        .icon_name("folder-open-symbolic")
        .tooltip_text("Browse")
        .css_classes(["flat"])
        .build();
    java_row.add_suffix(&java_entry);
    java_row.add_suffix(&browse_button);
    general_group.add(&java_row);

    // Default memory
    let memory_row = adw::ActionRow::builder()
        .title("Default Memory")
        .subtitle("Default memory allocation for new instances (MB)")
        .build();
    let memory_adj = gtk::Adjustment::new(4096.0, 512.0, 32768.0, 512.0, 1024.0, 0.0);
    let memory_spin = gtk::SpinButton::builder()
        .adjustment(&memory_adj)
        .build();
    memory_row.add_suffix(&memory_spin);
    general_group.add(&memory_row);

    page.add(&general_group);

    // Launcher section
    let launcher_group = widgets::group("Launcher");

    // Keep launcher open
    let keep_open_row = adw::ActionRow::builder()
        .title("Keep Launcher Open")
        .subtitle("Keep the launcher open when Minecraft is running")
        .build();
    let keep_open_switch = gtk::Switch::builder()
        .active(true)
        .build();
    keep_open_row.add_suffix(&keep_open_switch);
    keep_open_row.set_activatable_widget(Some(&keep_open_switch));
    launcher_group.add(&keep_open_row);

    // Close after game
    let close_row = adw::ActionRow::builder()
        .title("Close After Game")
        .subtitle("Close the launcher when Minecraft exits")
        .build();
    let close_switch = gtk::Switch::builder()
        .active(false)
        .build();
    close_row.add_suffix(&close_switch);
    close_row.set_activatable_widget(Some(&close_switch));
    launcher_group.add(&close_row);

    page.add(&launcher_group);

    // About section
    let about_group = widgets::group("About");

    let version_row = widgets::value_row(
        "Version",
        env!("CARGO_PKG_VERSION"),
    );
    about_group.add(&version_row);

    page.add(&about_group);

    content.append(&page);

    // Button bar
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    button_box.set_halign(gtk::Align::End);
    let cancel_button = gtk::Button::builder()
        .label("Cancel")
        .build();
    let save_button = gtk::Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    button_box.append(&cancel_button);
    button_box.append(&save_button);
    content.append(&button_box);

    scrolled.set_child(Some(&content));
    dialog.set_child(Some(&scrolled));

    let dialog_clone = dialog.clone();
    let dialog_cancel = dialog.clone();
    let ctx_for_present = ctx.clone();
    let ctx = ctx.clone();

    // Load current settings
    let settings = ctx.state.settings.borrow();
    if let Some(java_path) = &settings.java_path {
        java_entry.set_text(java_path);
    }
    memory_spin.set_value(settings.memory_mb as f64);
    keep_open_switch.set_active(!settings.close_on_launch);
    close_switch.set_active(settings.close_on_launch);
    drop(settings);

    save_button.connect_clicked(move |_| {
        let java_path = java_entry.text().trim().to_string();
        let java_path = if java_path.is_empty() { None } else { Some(java_path) };
        let memory = memory_spin.value() as u32;
        let _keep_open = keep_open_switch.is_active();
        let close_on_launch = close_switch.is_active();

        let mut settings = ctx.state.settings.borrow_mut();
        settings.java_path = java_path;
        settings.memory_mb = memory;
        settings.close_on_launch = close_on_launch;

        if let Err(err) = settings.save() {
            ctx.toast(&format!("Failed to save settings: {err}"));
            return;
        }
        drop(settings);

        ctx.toast("Settings saved");
        dialog_clone.close();
    });

    cancel_button.connect_clicked(move |_| {
        dialog_cancel.close();
    });

    dialog.present(Some(&ctx_for_present.window));
}
