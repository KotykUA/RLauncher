//! Create / edit an instance.

use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::config::instance::{Instance, LoaderKind};
use crate::ui::window::Ctx;

/// Create a new instance.
pub fn present(ctx: &Ctx, on_create: impl Fn(String) + 'static) {
    let dialog = adw::Dialog::new();
    dialog.set_title("New Instance");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    // Name entry
    let name_label = gtk::Label::builder()
        .label("Instance Name")
        .halign(gtk::Align::Start)
        .build();
    let name_entry = gtk::Entry::builder()
        .placeholder_text("My Minecraft Instance")
        .build();

    // Version dropdown
    let version_label = gtk::Label::builder()
        .label("Minecraft Version")
        .halign(gtk::Align::Start)
        .build();
    let version_entry = gtk::Entry::builder()
        .placeholder_text("1.20.1")
        .text("1.20.1")
        .build();

    // Loader dropdown
    let loader_label = gtk::Label::builder()
        .label("Loader")
        .halign(gtk::Align::Start)
        .build();
    let loader_combo = gtk::StringList::new(&LoaderKind::ALL.map(|l| l.label()));
    let loader_dropdown = gtk::DropDown::builder()
        .model(&loader_combo)
        .selected(0)
        .build();

    content.append(&name_label);
    content.append(&name_entry);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&version_label);
    content.append(&version_entry);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&loader_label);
    content.append(&loader_dropdown);

    // Button bar
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    button_box.set_halign(gtk::Align::End);
    let cancel_button = gtk::Button::builder()
        .label("Cancel")
        .build();
    let create_button = gtk::Button::builder()
        .label("Create")
        .css_classes(["suggested-action"])
        .build();
    button_box.append(&cancel_button);
    button_box.append(&create_button);
    content.append(&button_box);

    dialog.set_child(Some(&content));

    let dialog_clone = dialog.clone();
    let dialog_cancel = dialog.clone();
    let ctx_for_present = ctx.clone();
    let ctx = ctx.clone();
    create_button.connect_clicked(move |_| {
        let name = name_entry.text().trim().to_string();
        let version = version_entry.text().trim().to_string();
        let loader_idx = loader_dropdown.selected() as usize;
        let loader = LoaderKind::ALL[loader_idx];

        if name.is_empty() {
            ctx.toast("Please enter an instance name");
            return;
        }

        if version.is_empty() {
            ctx.toast("Please enter a Minecraft version");
            return;
        }

        let instance = Instance::new(&name, &version, loader);
        if let Err(err) = instance.save() {
            ctx.toast(&format!("Failed to create instance: {err}"));
            return;
        }

        on_create(instance.id);
        dialog_clone.close();
    });

    cancel_button.connect_clicked(move |_| {
        dialog_cancel.close();
    });

    dialog.present(Some(&ctx_for_present.window));
}

/// Edit an existing instance.
pub fn present_edit(ctx: &Ctx, instance: Instance) {
    let dialog = adw::Dialog::new();
    dialog.set_title(&format!("Edit {}", instance.name));

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    // Name entry
    let name_label = gtk::Label::builder()
        .label("Instance Name")
        .halign(gtk::Align::Start)
        .build();
    let name_entry = gtk::Entry::builder()
        .text(&instance.name)
        .build();

    // Memory slider
    let memory_label = gtk::Label::builder()
        .label("Memory (MB)")
        .halign(gtk::Align::Start)
        .build();
    let memory_adj = gtk::Adjustment::new(instance.memory_mb as f64, 512.0, 32768.0, 512.0, 1024.0, 0.0);
    let memory_spin = gtk::SpinButton::builder()
        .adjustment(&memory_adj)
        .build();

    // JVM args
    let jvm_label = gtk::Label::builder()
        .label("JVM Arguments")
        .halign(gtk::Align::Start)
        .build();
    let jvm_entry = gtk::Entry::builder()
        .text(&instance.jvm_args)
        .placeholder_text("-Xmx4G")
        .build();

    content.append(&name_label);
    content.append(&name_entry);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&memory_label);
    content.append(&memory_spin);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&jvm_label);
    content.append(&jvm_entry);

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

    dialog.set_child(Some(&content));

    let dialog_clone = dialog.clone();
    let dialog_cancel = dialog.clone();
    let ctx_for_present = ctx.clone();
    let ctx = ctx.clone();
    let instance_id = instance.id.clone();
    save_button.connect_clicked(move |_| {
        let name = name_entry.text().trim().to_string();
        let memory = memory_spin.value() as u32;
        let jvm_args = jvm_entry.text().trim().to_string();

        if name.is_empty() {
            ctx.toast("Please enter an instance name");
            return;
        }

        match Instance::load(&instance_id) {
            Ok(mut inst) => {
                inst.name = name;
                inst.memory_mb = memory;
                inst.jvm_args = jvm_args;
                if let Err(err) = inst.save() {
                    ctx.toast(&format!("Failed to save instance: {err}"));
                    return;
                }
                ctx.refresh_instances();
                ctx.toast("Instance saved");
            }
            Err(err) => {
                ctx.toast(&format!("Failed to load instance: {err}"));
                return;
            }
        }

        dialog_clone.close();
    });

    cancel_button.connect_clicked(move |_| {
        dialog_cancel.close();
    });

    dialog.present(Some(&ctx_for_present.window));
}
