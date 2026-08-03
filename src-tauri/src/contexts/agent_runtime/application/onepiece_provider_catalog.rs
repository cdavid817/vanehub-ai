use super::{OnePieceProviderEndpoint, OnePieceProviderPreset};
use serde::Deserialize;
use std::collections::BTreeMap;

const CATALOG_JSON: &str = include_str!("../../../../../src/config/onepiece-provider-catalog.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogDocument {
    catalog_version: u32,
    providers: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogEntry {
    id: String,
    display_name: String,
    category: String,
    icon_key: String,
    provider: String,
    default_model_id: String,
    fallback_models: Vec<String>,
    api_key_url: String,
    docs_url: String,
    default_endpoint_type: String,
    endpoints: BTreeMap<String, EndpointEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct ModelDiscoveryEntry {
    strategy: String,
    url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EndpointEntry {
    base_url: String,
    interface_format: String,
    auth_strategy: String,
    source: String,
    model_discovery: ModelDiscoveryEntry,
}

fn catalog() -> CatalogDocument {
    serde_json::from_str(CATALOG_JSON)
        .unwrap_or_else(|error| panic!("embedded OnePiece provider catalog must be valid: {error}"))
}

fn to_preset(version: u32, entry: CatalogEntry) -> OnePieceProviderPreset {
    let endpoints = entry
        .endpoints
        .into_iter()
        .map(|(endpoint_type, endpoint)| OnePieceProviderEndpoint {
            endpoint_type,
            base_url: endpoint.base_url,
            interface_format: endpoint.interface_format,
            auth_strategy: endpoint.auth_strategy,
            source: endpoint.source,
            model_discovery_strategy: endpoint.model_discovery.strategy,
            model_discovery_url: endpoint.model_discovery.url,
        })
        .collect::<Vec<_>>();
    let default_endpoint = endpoints
        .iter()
        .find(|endpoint| endpoint.endpoint_type == entry.default_endpoint_type)
        .unwrap_or_else(|| panic!("provider {} must declare its default endpoint", entry.id));
    OnePieceProviderPreset {
        id: entry.id,
        catalog_version: version,
        display_name: entry.display_name,
        category: entry.category,
        icon_key: entry.icon_key,
        provider: entry.provider,
        default_model_id: entry.default_model_id,
        fallback_models: entry.fallback_models,
        interface_format: default_endpoint.interface_format.clone(),
        base_url: Some(default_endpoint.base_url.clone()),
        api_key_url: entry.api_key_url,
        docs_url: entry.docs_url,
        model_discovery_strategy: default_endpoint.model_discovery_strategy.clone(),
        default_endpoint_type: entry.default_endpoint_type,
        endpoints,
    }
}

pub(super) fn list() -> Vec<OnePieceProviderPreset> {
    let catalog = catalog();
    catalog
        .providers
        .into_iter()
        .map(|entry| to_preset(catalog.catalog_version, entry))
        .collect()
}

pub(super) fn resolve(provider_id: &str, endpoint_type: &str) -> Option<OnePieceProviderPreset> {
    let mut preset = list().into_iter().find(|entry| entry.id == provider_id)?;
    let endpoint = preset
        .endpoints
        .iter()
        .find(|endpoint| endpoint.endpoint_type == endpoint_type)?;
    preset.interface_format = endpoint.interface_format.clone();
    preset.base_url = Some(endpoint.base_url.clone());
    preset.model_discovery_strategy = endpoint.model_discovery_strategy.clone();
    preset.default_endpoint_type = endpoint.endpoint_type.clone();
    Some(preset)
}

#[cfg(test)]
pub(super) fn resolve_endpoint(
    provider_id: &str,
    endpoint_type: &str,
) -> Option<OnePieceProviderEndpoint> {
    list()
        .into_iter()
        .find(|entry| entry.id == provider_id)?
        .endpoints
        .into_iter()
        .find(|endpoint| endpoint.endpoint_type == endpoint_type)
}

pub(super) fn discovery_url(provider_id: &str, endpoint_type: &str) -> Option<String> {
    let preset = resolve(provider_id, endpoint_type)?;
    preset
        .endpoints
        .into_iter()
        .find(|endpoint| endpoint.endpoint_type == preset.default_endpoint_type)
        .and_then(|endpoint| endpoint.model_discovery_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn catalog_ids_are_unique_and_runtime_compatible() {
        let catalog = list();
        let ids = catalog
            .iter()
            .map(|preset| preset.id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), catalog.len());
        assert!(catalog.len() >= 20);
        assert!(catalog.iter().all(|preset| {
            preset.interface_format == "anthropic"
                || (preset.interface_format == "openai-compatible"
                    && preset
                        .base_url
                        .as_deref()
                        .is_some_and(|url| url.starts_with("https://")))
        }));
        assert!(catalog.iter().all(|preset| {
            !preset.icon_key.trim().is_empty()
                && !preset.fallback_models.is_empty()
                && preset.fallback_models.contains(&preset.default_model_id)
        }));
    }

    #[test]
    fn existing_preset_ids_and_endpoints_remain_stable() {
        assert_eq!(
            resolve("anthropic", "anthropic-messages").and_then(|entry| entry.base_url),
            Some("https://api.anthropic.com".to_string())
        );
        assert_eq!(
            resolve("openrouter", "openai-chat-completions").and_then(|entry| entry.base_url),
            Some("https://openrouter.ai/api/v1".to_string())
        );
        assert!(discovery_url("deepseek", "openai-chat-completions").is_some());
        assert_eq!(
            resolve_endpoint("deepseek", "anthropic-messages")
                .map(|endpoint| endpoint.auth_strategy),
            Some("bearer".to_string())
        );
    }
}
