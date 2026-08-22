// The source-aware use cases are built alongside the flat `CliToolStatus` service they replace.
// Task group 9 moves the Tauri commands onto them and task group 13 deletes what is left.
//
// `not(test)` makes the attribute double as a coverage gate: an item no test touches fails the
// test build rather than passing unnoticed.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI Tauri commands in task group 9; remove with that group"
    )
)]
pub(crate) mod environment_bulk;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI Tauri commands in task group 9; remove with that group"
    )
)]
pub(crate) mod environment_error;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI Tauri commands in task group 9; remove with that group"
    )
)]
pub(crate) mod environment_planning;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI Tauri commands in task group 9; remove with that group"
    )
)]
pub(crate) mod environment_ports;
#[cfg(test)]
#[path = "environment_readiness_tests.rs"]
mod environment_readiness_tests;
pub(crate) mod environment_refresh;
pub(crate) mod environment_service;
#[cfg(test)]
mod environment_service_fixtures;
#[cfg(test)]
mod environment_test_doubles;

mod error;
mod models;
mod ports;
mod service;

#[cfg(test)]
mod tests;

pub(crate) use error::CliApplicationError;
pub(crate) use models::{
    CliDetectionResult, CliLogCategory, CliLogEvent, CliLogLevel, CliOperationRequest,
    CliOperationResult, CliOperationType, CliToolStatus, PreparedCliInstall, PreparedCliRefresh,
    PreparedCliUpgradeAll, StartedCliOperation,
};
pub(crate) use ports::{
    CliClockPort, CliDetectionPort, CliExecutableLocatorPort, CliLoggingPort, CliMutationPort,
    CliOperationPort, CliPackagePort, CliStatusRepository, NativeConfigPort,
};
pub(crate) use service::{CliApplicationPorts, CliApplicationService};
