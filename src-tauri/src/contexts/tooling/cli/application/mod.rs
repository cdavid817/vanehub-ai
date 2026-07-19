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
    CliOperationPort, CliPackagePort, CliStatusRepository,
};
pub(crate) use service::{CliApplicationPorts, CliApplicationService};
