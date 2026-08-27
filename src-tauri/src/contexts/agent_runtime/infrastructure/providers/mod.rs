mod compatibility;
mod invocation;
mod manifest;
mod output;
mod session_capture;

pub(crate) use crate::contexts::agent_runtime::application::ProviderPromptDelivery;
pub(crate) use invocation::{
    add_codex_output_capture_args, add_opencode_directory_args, build_interactive_invocation,
    build_invocation_with_role, message_override_selections, opencode_standard_permission_env_var,
    policy_override_selections, ProviderLaunchSegments, POLICY_TEMPLATE_GOVERNED_AGENT_IDS,
};
pub(crate) use output::{
    output_parser_for_format, BoundedProviderLines, ProviderOutputEvent, ProviderOutputFramer,
    ProviderOutputStream, ProviderReportedUsage, ProviderToolEvent, ProviderToolPhase,
    ProviderUsageOverlap,
};
pub(crate) use session_capture::{
    codex_session_root, find_codex_rollout_since, find_gemini_chat_session,
    find_opencode_session_since, opencode_database_path, prepare_provider_session_capture,
    ProviderSessionCapture, ProviderSessionDiscovery,
};

#[cfg(test)]
mod compatibility_tests;
#[cfg(test)]
mod session_capture_tests;
#[cfg(test)]
mod tests;

pub(crate) use compatibility::builtin_cli_provider_registry;
#[cfg(test)]
pub(crate) use invocation::build_invocation;
#[cfg(test)]
pub(crate) use output::output_parser_for;
