use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::plugin_integrations::api::PluginIntegrationApi;
use crate::contexts::tooling::plugin_integrations::application::PluginIntegrationApplicationService;
use crate::contexts::tooling::plugin_integrations::infrastructure::{
    GitHubCliToolAdapter, SystemPluginIntegrationClock, UnifiedPluginIntegrationLoggingAdapter,
};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn assemble_plugin_integration_api(fallback_log_dir: PathBuf) -> PluginIntegrationApi {
    let logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_dir));
    PluginIntegrationApi::new(PluginIntegrationApplicationService::new(
        Arc::new(GitHubCliToolAdapter),
        Arc::new(UnifiedPluginIntegrationLoggingAdapter::new(logging)),
        Arc::new(SystemPluginIntegrationClock),
    ))
}
