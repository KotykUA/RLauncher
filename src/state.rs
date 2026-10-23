//! Shared, single-threaded application state.

use std::cell::RefCell;
use std::rc::Rc;

use crate::config::instance::Instance;
use crate::config::settings::Settings;

pub struct AppState {
    pub settings: RefCell<Settings>,
    pub instances: RefCell<Vec<Instance>>,
}

impl AppState {
    pub fn load() -> Rc<Self> {
        Rc::new(Self {
            settings: RefCell::new(Settings::load()),
            instances: RefCell::new(Instance::list()),
        })
    }

    /// Re-read the instance directory. Cheap enough to call on every mutation.
    pub fn reload_instances(&self) {
        *self.instances.borrow_mut() = Instance::list();
    }

    pub fn persist_settings(&self) {
        if let Err(err) = self.settings.borrow().save() {
            tracing::warn!("failed to persist settings: {err:#}");
        }
    }

    pub fn active_account(&self) -> Option<crate::core::auth::AccountRecord> {
        let settings = self.settings.borrow();
        let active_id = settings.active_account.as_ref()?;
        settings
            .accounts
            .iter()
            .find(|account| &account.id() == active_id)
            .cloned()
    }

    pub fn instance(&self, id: &str) -> Option<Instance> {
        self.instances
            .borrow()
            .iter()
            .find(|instance| instance.id == id)
            .cloned()
    }
}
