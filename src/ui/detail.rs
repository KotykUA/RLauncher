//! Instance detail page: launch, live log, progress.

use std::rc::Rc;

use gtk::prelude::*;

use crate::config::instance::Instance;
use crate::core::{backend, java, skin};
use crate::ui::new_instance;
use crate::ui::widgets;
use crate::ui::window::Ctx;

enum Event {
    Log(String),
    Error(String),
    Started(u32),
    Exited(Option<i32>),
}

pub fn build(ctx: &Ctx, instance: Instance) -> gtk::Widget {
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    // --- header ------------------------------------------------------------
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 18);

    let avatar = gtk::Image::from_icon_name("application-x-executable-symbolic");
    avatar.set_pixel_size(72);

    // Load the active account's avatar in the background.
    if let Some(account) = ctx.state.active_account() {
        let key = account.skin_key();
        let (tx, rx) = async_channel::bounded::<Option<std::path::PathBuf>>(1);
        std::thread::spawn(move || {
            let _ = tx.send_blocking(skin::avatar_path(&key));
        });
        let avatar_weak = avatar.downgrade();
        glib::spawn_future_local(async move {
            if let Ok(Some(path)) = rx.recv().await {
                if let Some(avatar) = avatar_weak.upgrade() {
                    avatar.set_from_file(Some(&path));
                }
            }
        });
    }

    header.append(&avatar);

    let titles = gtk::Box::new(gtk::Orientation::Vertical, 4);
    titles.set_valign(gtk::Align::Center);

    let title = gtk::Label::new(Some(&instance.name));
    title.set_xalign(0.0);
    title.add_css_class("title-1");

    let subtitle = gtk::Label::new(Some(&instance.subtitle()));
    subtitle.set_xalign(0.0);
    subtitle.add_css_class("dim-label");

    titles.append(&title);
    titles.append(&subtitle);
    header.append(&titles);

    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    header.append(&spacer);

    // --- actions -----------------------------------------------------------
    let launch_button = gtk::Button::with_label("Play");
    launch_button.add_css_class("suggested-action");
    launch_button.add_css_class("pill");
    launch_button.set_size_request(140, 44);

    header.append(&launch_button);
    content.append(&header);

    // --- status line -------------------------------------------------------
    let status = gtk::Label::new(Some("Ready"));
    status.set_xalign(0.0);
    status.add_css_class("dim-label");
    content.append(&status);

    // --- log ---------------------------------------------------------------
    let log_view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();

    let log_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(220)
        .vexpand(true)
        .child(&log_view)
        .build();
    log_scroll.add_css_class("card");

    content.append(&log_scroll);

    // --- toolbar -----------------------------------------------------------
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    actions.set_halign(gtk::Align::Start);

    let edit_button = gtk::Button::with_label("Edit");
    let folder_button = gtk::Button::with_label("Open folder");
    let delete_button = gtk::Button::with_label("Delete");
    delete_button.add_css_class("destructive-action");

    actions.append(&edit_button);
    actions.append(&folder_button);
    actions.append(&delete_button);
    content.append(&actions);

    scroll.set_child(Some(&content));

    // --- behaviour ---------------------------------------------------------
    let append_log = {
        let buffer = log_view.buffer();
        move |text: &str| {
            let mut end = buffer.end_iter();
            buffer.insert(&mut end, &format!("{text}\n"));
        }
    };

    // Play
    {
        let ctx = ctx.clone();
        let instance = instance.clone();
        let launch_button = launch_button.clone();
        let status = status.clone();
        let append_log = append_log.clone();
        let stack = ctx.stack.clone();

        launch_button.clone().connect_clicked(move |button| {
            let Some(account) = ctx.state.active_account() else {
                widgets::alert(&ctx.window, "No account", "Add an account in the sidebar first.");
                return;
            };

            let settings = ctx.state.settings.borrow().clone();
            let java = backend::resolve_java(&instance, &settings);

            button.set_sensitive(false);
            status.set_text("Preparing…");
            append_log(&format!("--- {}", instance.name));

            if let Some(java) = java.as_ref() {
                if let Some(version) = java::probe_version(java) {
                    append_log(&format!("Java: {} ({version})", java.display()));
                }
            } else {
                append_log("Java: not found — the launcher will let mc-launcher-core decide.");
            }

            let (tx, rx) = async_channel::unbounded::<Event>();
            let worker_instance = instance.clone();

            std::thread::spawn(move || {
                let mut report = {
                    let tx = tx.clone();
                    move |line: &str| {
                        let _ = tx.send_blocking(Event::Log(line.to_string()));
                    }
                };

                let installed =
                    match backend::ensure_installed(&worker_instance, &settings, &mut report) {
                        Ok(installed) => installed,
                        Err(err) => {
                            let _ = tx.send_blocking(Event::Error(format!("{err:#}")));
                            return;
                        }
                    };

                let child = match backend::launch(
                    &worker_instance,
                    &account,
                    &settings,
                    &installed.version_id,
                    java.as_deref(),
                    &mut report,
                ) {
                    Ok(child) => child,
                    Err(err) => {
                        let _ = tx.send_blocking(Event::Error(format!("{err:#}")));
                        return;
                    }
                };

                let pid = child.id();
                let _ = tx.send_blocking(Event::Started(pid));

                let mut child = child;
                let code = child.wait().ok().and_then(|status| status.code());
                let _ = tx.send_blocking(Event::Exited(code));
            });

            let button = launch_button.clone();
            let status = status.clone();
            let append_log = append_log.clone();
            let ctx_after = ctx.clone();
            let stack = stack.clone();

            glib::spawn_future_local(async move {
                while let Ok(event) = rx.recv().await {
                    match event {
                        Event::Log(line) => {
                            append_log(&line);
                            status.set_text(&line);
                        }
                        Event::Started(pid) => {
                            status.set_text(&format!("Running (pid {pid})"));
                            append_log("Game started.");

                            if ctx_after.state.settings.borrow().close_on_launch {
                                ctx_after.window.close();
                            }
                        }
                        Event::Exited(code) => {
                            let summary = match code {
                                Some(0) => "Game exited normally.".to_string(),
                                Some(code) => format!("Game exited with code {code}."),
                                None => "Game process ended.".to_string(),
                            };
                            append_log(&summary);
                            status.set_text(&summary);
                            button.set_sensitive(true);
                        }
                        Event::Error(message) => {
                            append_log(&format!("ERROR: {message}"));
                            status.set_text("Failed");
                            widgets::alert(&ctx_after.window, "Launch failed", &message);
                            button.set_sensitive(true);
                        }
                    }
                }
            });

            // Keep `stack` alive for the duration of the closure.
            let _ = &stack;
        });
    }

    // Edit
    {
        let ctx = ctx.clone();
        let instance = instance.clone();
        edit_button.connect_clicked(move |_| {
            new_instance::present_edit(&ctx, instance.clone());
        });
    }

    // Open folder
    {
        let instance = instance.clone();
        let window = ctx.window.clone();
        folder_button.connect_clicked(move |_| {
            let uri = format!("file://{}", instance.dir().display());
            widgets::open_uri(&window, &uri);
        });
    }

    // Delete
    {
        let ctx = ctx.clone();
        let instance = instance.clone();
        let delete_button = delete_button.clone();
        delete_button.connect_clicked(move |_| {
            let ctx = ctx.clone();
            let ctx_clone = ctx.clone();
            let window = ctx.window.clone();
            let instance = instance.clone();
            widgets::confirm(
                &ctx.window,
                "Delete instance?",
                &format!(
                    "“{}” and its world, mods and configuration will be removed. This cannot be undone.",
                    instance.name
                ),
                "Delete",
                move || match instance.delete() {
                    Ok(()) => {
                        ctx_clone.refresh_instances();
                        ctx_clone.stack.set_visible_child_name("welcome");
                        ctx_clone.toast("Instance deleted");
                    }
                    Err(err) => widgets::alert(&window, "Delete failed", &format!("{err:#}")),
                },
            );
        });
    }

    // Remember the selection.
    {
        let mut settings = ctx.state.settings.borrow_mut();
        settings.last_instance = Some(instance.id.clone());
        drop(settings);
        ctx.state.persist_settings();
    }

    // Suppress unused-variable warnings for the Rc clone kept for the closure.
    let _ = Rc::strong_count(&Rc::new(()));

    scroll.upcast()
}

use gtk::glib;
