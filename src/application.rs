//! Application object, CSS loading and process entry.

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::state::AppState;
use crate::ui;
use crate::utils::paths;

const APP_ID: &str = paths::APP_ID;

pub fn run() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::empty())
        .build();

    app.connect_startup(|_| load_css());

    app.connect_activate(|app| {
        // Guard against a second window being created when the app is re-activated.
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }

        let state = AppState::load();
        ui::window::build(app, state).present();
    });

    app.run()
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../data/style.css"));

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

use gtk::gio;
