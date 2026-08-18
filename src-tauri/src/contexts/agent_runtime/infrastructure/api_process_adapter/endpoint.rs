//! Which endpoint a generation calls, with what credential, and what it is declared to support.
//!
//! All of it reads the same three sources in the same precedence — the request's frozen endpoint
//! Profile, the agent's stored Profile metadata, then the model catalog — which is why capability
//! and capacity resolution live together rather than beside their call sites.

use super::super::model_context_catalog;
use super::invocation::{wire_format_for, WireFormat};
use super::{failed_configuration, failed_non_retryable};
use crate::contexts::agent_runtime::application::{
    ApiAgentGateway, ApiCredentialPort, ApiProviderConfig, GenerationProcessEvent,
    GenerationProcessRequest, StoredEndpointProfileMetadata,
};
use crate::contexts::agent_runtime::domain::ContextCapacity;

/// Everything a generation needs to reach its provider, resolved once before anything is sent.
pub(super) struct ResolvedEndpoint {
    pub(super) provider_config: ApiProviderConfig,
    pub(super) endpoint_metadata: Option<StoredEndpointProfileMetadata>,
    pub(super) endpoint_capacity: Option<ContextCapacity>,
    pub(super) api_key: String,
    pub(super) wire_format: WireFormat,
}

/// A frozen endpoint Profile on the request wins over the agent's stored configuration for every
/// one of these, including the credential: a Profile-routed generation looks up
/// `onepiece-profile:<id>` first and only falls back to the agent's own key.
///
/// Each `Err` is the `GenerationProcessEvent` the caller returns unchanged. They are the same
/// seven events, built by the same three constructors from the same strings, in the same order as
/// when this lived inline — the caller's `Err(failure) => return failure` puts each back where its
/// `return` was.
// `result_large_err`: the same allowance `ask_user_question`, `request_plan_exit` and
// `execute_registered_native_tool` already carry. `GenerationProcessEvent` is what this module
// returns; boxing it here would put an allocation on a path whose whole job is to hand the event
// straight back.
#[allow(clippy::result_large_err)]
pub(super) fn resolve_endpoint(
    request: &GenerationProcessRequest,
    agent_id: &str,
    config: &dyn ApiAgentGateway,
    credentials: &dyn ApiCredentialPort,
) -> Result<ResolvedEndpoint, GenerationProcessEvent> {
    let provider_config = if let Some(profile) = request.endpoint_profile.as_ref() {
        ApiProviderConfig {
            source_provider_id: profile.source_provider_id.clone(),
            model_id: profile.model_id.clone(),
            interface_format: profile.interface_format.clone(),
            base_url: profile.base_url.clone(),
            auto_approve_tools: false,
        }
    } else {
        match config.provider_config(agent_id) {
            Ok(Some(config)) => config,
            Ok(None) => {
                return Err(failed_configuration(
                    agent_id,
                    "No model is configured for this agent.",
                ));
            }
            Err(error) => return Err(failed_non_retryable(&error.to_string())),
        }
    };
    let endpoint_metadata = if request.endpoint_profile.is_some() {
        None
    } else {
        match config.active_endpoint_profile_metadata(agent_id) {
            Ok(metadata) => metadata,
            Err(error) => return Err(failed_non_retryable(&error.to_string())),
        }
    };
    let endpoint_capacity = request
        .endpoint_profile
        .as_ref()
        .and_then(|profile| {
            let window = profile.context_window_tokens?;
            (profile.context_capacity_provenance != "unknown").then(|| ContextCapacity {
                context_window_tokens: window,
                maximum_output_tokens: Some(profile.reserved_output_tokens),
                metadata_revision: profile.context_capacity_provenance.clone(),
                source_identity: format!("endpoint-profile:{}", profile.profile_id),
            })
        })
        .or_else(|| {
            endpoint_metadata.as_ref().and_then(|metadata| {
                let window = metadata
                    .context_window_tokens
                    .and_then(|value| u64::try_from(value).ok())?;
                (metadata.context_capacity_provenance != "unknown").then(|| ContextCapacity {
                    context_window_tokens: window,
                    maximum_output_tokens: u64::try_from(metadata.reserved_output_tokens).ok(),
                    metadata_revision: metadata.context_capacity_provenance.clone(),
                    source_identity: format!("endpoint-profile:{}", metadata.profile_id),
                })
            })
        });
    let authentication_mode = if let Some(profile) = request.endpoint_profile.as_ref() {
        profile.authentication_mode.clone()
    } else {
        match config.api_endpoint_authentication_mode(agent_id) {
            Ok(mode) => mode,
            Err(error) => return Err(failed_non_retryable(&error.to_string())),
        }
    };
    let credential_id = request.endpoint_profile.as_ref().map_or_else(
        || agent_id.to_string(),
        |profile| format!("onepiece-profile:{}", profile.profile_id),
    );
    let fetched_credential = if authentication_mode == "required" {
        match credentials.fetch(&credential_id) {
            Ok(Some(key)) => Ok(Some(key)),
            Ok(None) if request.endpoint_profile.is_some() => credentials.fetch(agent_id),
            other => other,
        }
    } else {
        Ok(None)
    };
    let api_key = match fetched_credential {
        Ok(Some(key)) => key,
        Ok(None) if authentication_mode != "required" => String::new(),
        Ok(None) => {
            return Err(failed_configuration(
                agent_id,
                "No API key is stored for this agent.",
            ));
        }
        Err(error) => return Err(failed_non_retryable(&error.to_string())),
    };
    let wire_format = match wire_format_for(&provider_config) {
        Ok(wire_format) => wire_format,
        Err(message) => return Err(failed_configuration(agent_id, message)),
    };
    Ok(ResolvedEndpoint {
        provider_config,
        endpoint_metadata,
        endpoint_capacity,
        api_key,
        wire_format,
    })
}

/// Capability comes from reviewed catalog metadata, never from trying and seeing: a provider
/// that rejects an image-bearing request fails the whole generation after the user has already
/// waited, and the failure text varies by vendor (`add-agent-image-input` D3).
pub(super) fn resolve_image_support(
    request: &GenerationProcessRequest,
    endpoint_metadata: Option<&StoredEndpointProfileMetadata>,
    provider_config: &ApiProviderConfig,
) -> bool {
    request.endpoint_profile.as_ref().map_or_else(
        || {
            endpoint_metadata.map_or_else(
                || {
                    model_context_catalog::accepts_image_input(
                        provider_config.source_provider_id.as_deref(),
                        &provider_config.model_id,
                    )
                },
                |metadata| metadata.image_input_capability == "supported",
            )
        },
        |profile| profile.image_input_capability == "supported",
    )
}
