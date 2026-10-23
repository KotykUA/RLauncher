//! Window shell: header bar, sidebar list, content stack, routing.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::config::instance::Instance;
use crate::state::AppState;
use crate::ui::{accounts, detail, new_instance, news, settings, widgets};

/// Everything a page needs to talk back to the shell.
#[derive(Clone)]
pub struct Ctx {
    pub window: adw::ApplicationWindow,
    pub state: Rc<AppState>,
    pub list: gtk::ListBox,
    pub stack: gtk::Stack,
    pub split: adw::NavigationSplitView,
    pub overlay: adw::ToastOverlay,
    /// Instance ids, indexed by row position in `list`.
    pub ids: Rc<RefCell<Vec<String>>>,
}

impl Ctx {
    pub fn toast(&self, message: &str) {
        widgets::toast(&self.overlay, message);
    }

    /// Rebuild the sidebar from `state.instances`.
    pub fn refresh_instances(&self) {
        self.state.reload_instances();

        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }

        let instances = self.state.instances.borrow().clone();
        let mut ids = Vec::with_capacity(instances.len());

        for instance in &instances {
            let row = build_row(instance);
            self.list.append(&row);
            ids.push(instance.id.clone());
        }

        *self.ids.borrow_mut() = ids;

        if instances.is_empty() {
            self.stack.set_visible_child_name("welcome");
        }
    }

    /// Show the detail page for an instance.
    pub fn show_instance(&self, id: &str) {
        let Some(instance) = self.state.instance(id) else {
            return;
        };

        let page_name = format!("detail-{id}");
        if let Some(old) = self.stack.child_by_name(&page_name) {
            self.stack.remove(&old);
        }

        let page = detail::build(self, instance);
        self.stack.add_named(&page, Some(&page_name));
        self.stack.set_visible_child_name(&page_name);
    }

    /// Select an instance by id, updating both list and content.
    pub fn select_instance(&self, id: &str) {
        let index = self
            .ids
            .borrow()
            .iter()
            .position(|candidate| candidate == id);

        if let Some(index) = index {
            if let Some(row) = self.list.row_at_index(index as i32) {
                self.list.select_row(Some(&row));
            }
        }

        self.show_instance(id);
    }
}

fn build_row(instance: &Instance) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();

    let outer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    outer.set_margin_top(8);
    outer.set_margin_bottom(8);
    outer.set_margin_start(12);
    outer.set_margin_end(12);

    // Avatar placeholder — downloading happens on the detail page to keep this cheap.
    let icon = gtk::Image::from_icon_name("application-x-executable-symbolic");
    icon.set_pixel_size(32);
    icon.add_css_class("dim-label");
    outer.append(&icon);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_valign(gtk::Align::Center);

    let name = gtk::Label::new(Some(&instance.name));
    name.set_xalign(0.0);
    name.add_css_class("body");

    let subtitle = gtk::Label::new(Some(&instance.subtitle()));
    subtitle.set_xalign(0.0);
    subtitle.add_css_class("caption");
    subtitle.add_css_class("dim-label");

    text.append(&name);
    text.append(&subtitle);

    outer.append(&text);
    row.set_child(Some(&outer));
    row.set_tooltip_text(Some(&instance.subtitle()));

    row
}

pub fn build(app: &adw::Application, state: Rc<AppState>) -> adw::ApplicationWindow {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Minecraft")
        .default_width(1120)
        .default_height(740)
        .build();

    let header = adw::HeaderBar::new();

    let new_button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("New instance")
        .build();
    header.pack_start(&new_button);

    let menu = gio::Menu::new();
    menu.append(Some("Accounts"), Some("win.accounts"));
    menu.append(Some("News"), Some("win.news"));
    menu.append(Some("Preferences"), Some("win.preferences"));
    menu.append(Some("About"), Some("win.about"));

    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .build();
    header.pack_end(&menu_button);

    // --- content stack -----------------------------------------------------
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .hexpand(true)
        .vexpand(true)
        .build();

    let welcome = adw::StatusPage::builder()
        .icon_name("application-x-executable-symbolic")
        .title("No instance selected")
        .description("Create an instance to get started.")
        .build();
    stack.add_named(&welcome, Some("welcome"));
    stack.set_visible_child_name("welcome");

    // --- sidebar -----------------------------------------------------------
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["navigation-sidebar"])
        .build();

    let sidebar_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();

    let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar_box.append(&sidebar_scroll);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    footer.set_margin_top(6);
    footer.set_margin_bottom(6);
    footer.set_margin_start(6);
    footer.set_margin_end(6);
    footer.set_halign(gtk::Align::Center);

    let accounts_button = gtk::Button::builder()
        .icon_name("avatar-default-symbolic")
        .tooltip_text("Accounts")
        .css_classes(["flat"])
        .build();
    footer.append(&accounts_button);

    let news_button = gtk::Button::builder()
        .icon_name("news-symbolic")
        .tooltip_text("News")
        .css_classes(["flat"])
        .build();
    footer.append(&news_button);

    let prefs_button = gtk::Button::builder()
        .icon_name("emblem-system-symbolic")
        .tooltip_text("Preferences")
        .css_classes(["flat"])
        .build();
    footer.append(&prefs_button);

    sidebar_box.append(&footer);

    let split = adw::NavigationSplitView::new();
    split.set_sidebar(Some(&adw::NavigationPage::new(&sidebar_box, "Instances")));
    split.set_content(Some(&adw::NavigationPage::new(&stack, "Instances")));
    split.set_show_content(false);

    // --- assemble ----------------------------------------------------------
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&split));

    let overlay = adw::ToastOverlay::new();
    overlay.set_child(Some(&toolbar));

    window.set_content(Some(&overlay));

    // --- context -----------------------------------------------------------
    let ctx = Ctx {
        window: window.clone(),
        state: state.clone(),
        list: list.clone(),
        stack: stack.clone(),
        split: split.clone(),
        overlay: overlay.clone(),
        ids: Rc::new(RefCell::new(Vec::new())),
    };

    ctx.refresh_instances();

    // --- sidebar selection -------------------------------------------------
    {
        let ctx = ctx.clone();
        list.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let index = row.index() as usize;
            let id = ctx.ids.borrow().get(index).cloned();
            if let Some(id) = id {
                ctx.show_instance(&id);
                ctx.split.set_show_content(true);
            }
        });
    }

    // --- new instance ------------------------------------------------------
    {
        let ctx = ctx.clone();
        new_button.connect_clicked(move |_| {
            let ctx = ctx.clone();
            let ctx_clone = ctx.clone();
            new_instance::present(&ctx, move |id: String| {
                ctx_clone.refresh_instances();
                ctx_clone.select_instance(&id);
                ctx_clone.split.set_show_content(true);
            });
        });
    }

    // --- footer buttons ----------------------------------------------------
    {
        let ctx = ctx.clone();
        accounts_button.connect_clicked(move |_| {
            accounts::present(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        news_button.connect_clicked(move |_| {
            news::present(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        prefs_button.connect_clicked(move |_| {
            settings::present(&ctx);
        });
    }

    // --- actions (menu) ----------------------------------------------------
    register_action(&window, "accounts", {
        let ctx = ctx.clone();
        move || accounts::present(&ctx)
    });
    register_action(&window, "news", {
        let ctx = ctx.clone();
        move || news::present(&ctx)
    });
    register_action(&window, "preferences", {
        let ctx = ctx.clone();
        move || settings::present(&ctx)
    });
    register_action(&window, "about", {
        let window = window.clone();
        move || {
            let about = adw::AboutDialog::builder()
                .application_name("Minecraft Launcher")
                .application_icon(crate::utils::paths::APP_ID)
                .developer_name("Better DeepSeek")
                .version(env!("CARGO_PKG_VERSION"))
                .website("https://example.invalid/mc-launcher")
                .build();
            about.present(Some(&window));
        }
    });

    // Restore the last instance, if it still exists.
    if let Some(last) = state.settings.borrow().last_instance.clone() {
        if state.instance(&last).is_some() {
            ctx.select_instance(&last);
            split.set_show_content(true);
        }
    }

    window
}

fn register_action(window: &adw::ApplicationWindow, name: &str, callback: impl Fn() + 'static) {
    let action = gio::SimpleAction::new(name, None);
    action.connect_activate(move |_, _| callback());
    window.add_action(&action);
}

use gtk::gio;
