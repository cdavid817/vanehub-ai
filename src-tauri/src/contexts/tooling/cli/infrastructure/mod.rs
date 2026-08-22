mod candidates;
mod detection_adapter;
// Source-aware discovery, built alongside the detection adapter it replaces. Task group 9 wires it
// into bootstrap; task group 13 deletes the old one.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "assembled in bootstrap by task group 9; remove with that group"
    )
)]
pub(crate) mod environment_discovery;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "assembled in bootstrap by task group 9; remove with that group"
    )
)]
pub(crate) mod environment_gateway;
#[cfg(test)]
#[path = "environment_platform_tests.rs"]
mod environment_platform_tests;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "assembled in bootstrap by task group 9; remove with that group"
    )
)]
pub(crate) mod environment_probe;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "assembled in bootstrap by task group 9; remove with that group"
    )
)]
pub(crate) mod environment_repository;
// Referenced by `platform::database::migrations`, so it has a production caller already.
pub(crate) mod environment_schema;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "assembled in bootstrap by task group 9; remove with that group"
    )
)]
mod environment_serde;
mod executable_locator;
mod native_config_reader;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "assembled in bootstrap by task group 9; remove with that group"
    )
)]
pub(crate) mod npm_source;
mod package_adapter;
mod process_adapter;
mod runtime_adapters;
mod sqlite_repository;
mod support;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "assembled in bootstrap by task group 9; remove with that group"
    )
)]
pub(crate) mod vendor_source;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "assembled in bootstrap by task group 9; remove with that group"
    )
)]
pub(crate) mod winget_source;

pub(crate) use detection_adapter::CliDetectionAdapter;
pub(crate) use executable_locator::CliExecutableLocatorAdapter;
pub(crate) use native_config_reader::NativeConfigReader;
pub(crate) use package_adapter::CliPackageAdapter;
pub(crate) use runtime_adapters::{
    CliMutationAdapter, CliOperationAdapter, SystemCliClock, UnifiedCliLoggingAdapter,
};
pub(crate) use sqlite_repository::SqliteCliStatusRepository;
