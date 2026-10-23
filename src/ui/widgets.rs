//! Small widget helpers shared across pages.

use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

/// A settings-style row: bold title, dim subtitle.
pub fn action_row(title: &str, subtitle: &str) -> adw::ActionRow {
    adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .activatable(false)
        .build()
}

/// A row that shows a label and a value on the right.
pub fn value_row(title: &str, value: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).build();
    let label = gtk::Label::new(Some(value));
    label.add_css_class("dim-label");
    row.add_suffix(&label);
    row
}

pub fn page(title: &str) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    let _ = title;
    page
}

pub fn group(title: &str) -> adw::PreferencesGroup {
    adw::PreferencesGroup::builder().title(title).build()
}

/// Show a destructive confirmation. `on_confirm` runs only if the user agrees.
pub fn confirm(
    parent: &impl IsA<gtk::Widget>,
    heading: &str,
    body: &str,
    confirm_label: &str,
    on_confirm: impl Fn() + 'static,
) {
    let dialog = adw::MessageDialog::new(
        Some(parent),
        Some(heading),
        Some(body),
    );
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("confirm", confirm_label);
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    dialog.choose(None::<&gio::Cancellable>, move |response| {
        if response.as_str() == "confirm" {
            on_confirm();
        }
    });
}

/// Show a plain informational dialog.
pub fn alert(parent: &impl IsA<gtk::Widget>, heading: &str, body: &str) {
    let dialog = adw::MessageDialog::new(Some(parent), Some(heading), Some(body));
    dialog.add_response("ok", "OK");
    dialog.set_default_response(Some("ok"));
    dialog.set_close_response("ok");
    dialog.present();
}

pub fn toast(overlay: &adw::ToastOverlay, message: &str) {
    overlay.add_toast(adw::Toast::new(message));
}

/// Fire-and-forget URL opening.
pub fn open_uri(parent: &impl IsA<gtk::Widget>, uri: &str) {
    let launcher = gtk::UriLauncher::new(uri);
    launcher.launch(Some(parent), None::<&gio::Cancellable>, |_| {});
}

use gtk::gio;
