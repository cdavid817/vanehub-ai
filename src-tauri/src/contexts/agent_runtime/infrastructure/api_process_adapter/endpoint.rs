//! Which endpoint a generation calls, with what credential, and what it is declared to support.
//!
//! All of it reads the same three sources in the same precedence — the request's frozen endpoint
//! Profile, the agent's stored Profile metadata, then the model catalog — which is why capability
//! and capacity resolution live together rather than beside their call sites.

use super::super::model_context_catalog;
use crate::contexts::agent_runtime::application::{
    ApiProviderConfig, GenerationProcessRequest, StoredEndpointProfileMetadata,
};

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
