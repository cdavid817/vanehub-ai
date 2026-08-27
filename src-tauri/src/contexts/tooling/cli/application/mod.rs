//! The source-aware CLI use cases.
//!
//! One module per concern. The flat `CliToolStatus` service this replaced lived beside these until
//! the Agent Runtime's launch path moved onto the environment snapshot; with no caller left it
//! went, along with its ports, its models, and its own SQLite repository.

pub(crate) mod environment_bulk;
pub(crate) mod environment_error;
pub(crate) mod environment_launch;
pub(crate) mod environment_planning;
pub(crate) mod environment_ports;
pub(crate) mod environment_refresh;
pub(crate) mod environment_service;
pub(crate) mod native_config;

#[cfg(test)]
#[path = "environment_readiness_tests.rs"]
mod environment_readiness_tests;
#[cfg(test)]
mod environment_service_fixtures;
#[cfg(test)]
mod environment_test_doubles;
