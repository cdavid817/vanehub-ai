//! Adapters for the source-aware CLI environment.
//!
//! Everything here is I/O: a process, SQLite, a package manager, an installer download, or the
//! filesystem. Policy lives in the application layer, and nothing in that layer names a module
//! from this one -- `bootstrap` does the naming.

pub(crate) mod environment_discovery;
pub(crate) mod environment_gateway;
pub(crate) mod environment_probe;
pub(crate) mod environment_repository;
pub(crate) mod environment_runtime_adapters;
/// Referenced by `platform::database::migrations`, so it has a production caller already.
pub(crate) mod environment_schema;
mod environment_serde;
mod native_config_reader;
pub(crate) mod npm_source;
pub(crate) mod vendor_downloader;
pub(crate) mod vendor_source;
pub(crate) mod winget_source;

#[cfg(test)]
#[path = "environment_platform_tests.rs"]
mod environment_platform_tests;
#[cfg(test)]
#[path = "environment_source_matrix_tests.rs"]
mod environment_source_matrix_tests;

pub(crate) use native_config_reader::NativeConfigReader;
