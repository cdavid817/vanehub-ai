pub(crate) mod artifacts;
mod artifacts_query;
pub(crate) mod change_set_review;
pub(crate) mod delegation;
pub(crate) mod get_builtin_tool_readiness;
pub(crate) mod manual_delegation;
pub(crate) mod operations;

pub(super) fn require_feature(capability: &str, mode: &str) -> Result<(), String> {
    use crate::contexts::agent_runtime::application::OnePieceToolFeatureGates;

    OnePieceToolFeatureGates::from_environment()
        .enabled(capability, mode)
        .then_some(())
        .ok_or_else(|| "feature_disabled".to_owned())
}
