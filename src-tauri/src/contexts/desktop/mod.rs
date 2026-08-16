//! Desktop settings, startup, window, tray, and floating-assistant behavior.

pub(crate) mod api;
mod api_update;
pub(crate) use api_update::{DesktopUpdateApi, UpdatePreferences, UpdateReceipt};
pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;
