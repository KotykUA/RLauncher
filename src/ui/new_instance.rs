//! Create / edit an instance.

use std::rc::Rc;

use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::config::instance::{Instance, LoaderKind};
use crate::core::versions;
use crate::ui::widgets;
use crate::ui::window::Ctx;

/// Create a new instance. `
