//! Where the variable-duration half of each command runs.
//!
//! The command returns an operation id the moment the work is queued. Every failure inside these
//! is already recorded on the operation by the service, so the join handle is dropped rather than
//! awaited -- there is nothing a caller could learn here that the operation does not already say.

use crate::contexts::tooling::cli::api::CliEnvironmentApi;
use crate::contexts::tooling::cli::application::environment_bulk::{
    PreparedCliBulkExecution, PreparedCliBulkPlanning, PreparedCliDoctor,
};
use crate::contexts::tooling::cli::application::environment_planning::{
    PreparedCliActionExecution, PreparedCliActionPlanning,
};
use crate::contexts::tooling::cli::application::environment_refresh::PreparedEnvironmentRefresh;

pub(super) fn spawn_refresh(api: CliEnvironmentApi, prepared: PreparedEnvironmentRefresh) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_refresh(prepared);
    });
}

pub(super) fn spawn_action_planning(api: CliEnvironmentApi, prepared: PreparedCliActionPlanning) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_action_planning(prepared);
    });
}

pub(super) fn spawn_action(api: CliEnvironmentApi, prepared: PreparedCliActionExecution) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_action(prepared);
    });
}

pub(super) fn spawn_bulk_planning(api: CliEnvironmentApi, prepared: PreparedCliBulkPlanning) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_bulk_planning(prepared);
    });
}

pub(super) fn spawn_bulk_action(api: CliEnvironmentApi, prepared: PreparedCliBulkExecution) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_bulk_action(prepared);
    });
}

pub(super) fn spawn_doctor(api: CliEnvironmentApi, prepared: PreparedCliDoctor) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_doctor(prepared);
    });
}
