//! The source-aware CLI environment command surface.
//!
//! One file per command, each thin: validate, hand off, return an operation id. Nothing here
//! contains policy -- policy lives in the application service, so a rule cannot hold in the UI path
//! and not in a test or a future caller.

pub(crate) mod background;
pub(crate) mod dto;
pub(crate) mod error;
pub(crate) mod execute_cli_action;
pub(crate) mod execute_cli_bulk_action;
pub(crate) mod get_cli_action_plan;
pub(crate) mod get_cli_bulk_action_plan;
pub(crate) mod list_cli_environments;
mod mapper;
pub(crate) mod prepare_cli_action;
pub(crate) mod prepare_cli_bulk_action;
pub(crate) mod refresh_cli_environment;
pub(crate) mod run_cli_doctor;
