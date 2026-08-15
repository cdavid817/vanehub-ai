use crate::contexts::agent_runtime::domain::ContextCapacity;
use serde::Deserialize;

const CATALOG_JSON: &str =
    include_str!("../../../../../src/config/onepiece-model-context-catalog.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelContextCatalog {
    catalog_version: String,
    entries: Vec<ModelContextEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelContextEntry {
    provider_id: String,
    model_id: String,
    context_window_tokens: u64,
    maximum_output_tokens: Option<u64>,
    /// Whether this model accepts image input (`add-agent-image-input`). Required per entry
    /// rather than defaulted: a default would silently claim support for the next model someone
    /// adds to the catalog, and a provider rejecting an image-bearing request fails the whole
    /// generation after the user has already waited.
    accepts_image_input: bool,
    metadata_revision: String,
    source_identity: String,
}

/// Whether the reviewed catalog says this model accepts images. An identifier absent from the
/// catalog is treated as unsupported rather than assumed capable -- `onepiece-native-agent`
/// already establishes that capabilities are not inferred from unknown model identifiers.
pub(crate) fn accepts_image_input(provider_id: Option<&str>, model_id: &str) -> bool {
    let Some(provider_id) = provider_id else {
        return false;
    };
    let Ok(catalog) = serde_json::from_str::<ModelContextCatalog>(CATALOG_JSON) else {
        return false;
    };
    if catalog.catalog_version != "onepiece-model-context-catalog-v1" {
        return false;
    }
    catalog
        .entries
        .into_iter()
        .find(|entry| entry.provider_id == provider_id && entry.model_id == model_id)
        .is_some_and(|entry| entry.accepts_image_input)
}

pub(crate) fn resolve_capacity(
    provider_id: Option<&str>,
    model_id: &str,
) -> Option<ContextCapacity> {
    let provider_id = provider_id?;
    let catalog: ModelContextCatalog = serde_json::from_str(CATALOG_JSON).ok()?;
    if catalog.catalog_version != "onepiece-model-context-catalog-v1" {
        return None;
    }
    catalog
        .entries
        .into_iter()
        .find(|entry| entry.provider_id == provider_id && entry.model_id == model_id)
        .map(|entry| ContextCapacity {
            context_window_tokens: entry.context_window_tokens,
            maximum_output_tokens: entry.maximum_output_tokens,
            metadata_revision: entry.metadata_revision,
            source_identity: entry.source_identity,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_has_unique_sorted_exact_keys_and_safe_metadata() {
        let catalog: ModelContextCatalog = serde_json::from_str(CATALOG_JSON).expect("catalog");
        assert_eq!(catalog.catalog_version, "onepiece-model-context-catalog-v1");
        let keys: Vec<_> = catalog
            .entries
            .iter()
            .map(|entry| format!("{}:{}", entry.provider_id, entry.model_id))
            .collect();
        assert_eq!(keys.iter().collect::<HashSet<_>>().len(), keys.len());
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
        assert!(catalog.entries.iter().all(|entry| {
            !entry.provider_id.contains(char::is_whitespace)
                && !entry.model_id.contains(char::is_whitespace)
                && entry.context_window_tokens > 0
                && entry.maximum_output_tokens.is_none_or(|value| value > 0)
                && !entry.metadata_revision.is_empty()
                && !entry.source_identity.is_empty()
        }));
    }

    #[test]
    fn lookup_is_exact_and_unknown_models_stay_unknown() {
        assert_eq!(
            resolve_capacity(Some("openai"), "gpt-5.4")
                .expect("known")
                .context_window_tokens,
            1_050_000
        );
        assert!(resolve_capacity(Some("OpenAI"), "gpt-5.4").is_none());
        assert!(resolve_capacity(Some("openai"), "GPT-5.4").is_none());
        assert!(resolve_capacity(None, "gpt-5.4").is_none());
    }
}
