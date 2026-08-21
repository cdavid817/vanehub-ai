use crate::contexts::tooling::extension_platform::api::ExtensionPlatformApi;
use crate::contexts::tooling::extension_platform::application::{
    DefaultPrerequisites, FeatureGateService, NoForcedDisables,
};
use crate::contexts::tooling::extension_platform::infrastructure::{
    FeatureGateSystemClock, SqliteFeatureGateAuditSink, SqliteFeatureGateRepository,
};
use crate::platform::database::NativeDatabase;
use std::sync::Arc;

/// Assembles the capability-gate service.
///
/// Construction reads persisted desired state once and publishes the initial snapshot. A failed
/// read is not an error here: the service starts from the all-disabled snapshot, which is the
/// correct answer when gate state is unknown.
pub(crate) fn assemble_extension_platform_api(database: NativeDatabase) -> ExtensionPlatformApi {
    let database = Arc::new(database);
    ExtensionPlatformApi::new(Arc::new(FeatureGateService::new(
        Arc::new(SqliteFeatureGateRepository::new(Arc::clone(&database))),
        Arc::new(SqliteFeatureGateAuditSink::new(database)),
        Arc::new(NoForcedDisables),
        Arc::new(DefaultPrerequisites),
        Arc::new(FeatureGateSystemClock),
    )))
}
