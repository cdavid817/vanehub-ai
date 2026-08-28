//! Desktop settings, startup, window, tray, and floating-assistant behavior.

pub(crate) mod api;
/// The pre-governance settings page against the dedicated policy.
#[cfg(test)]
mod api_personalization_tests;
mod api_update;
pub(crate) use api_update::{DesktopUpdateApi, UpdatePreferences, UpdateReceipt};
pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;
