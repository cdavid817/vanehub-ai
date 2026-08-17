//! Projects eligible Skill tools into provider definitions while retaining immutable dispatch keys.

use crate::contexts::agent_runtime::application::ToolDefinition;
use crate::contexts::tooling::skill_tools::application::{
    SkillToolApplicationError, SkillToolCatalogContext, SkillToolCatalogPort,
};
use crate::contexts::tooling::skill_tools::domain::SkillToolKey;
use std::collections::{HashMap, HashSet};

pub(crate) struct ResolvedSkillToolCatalog {
    pub(crate) definitions: Vec<ToolDefinition>,
    pub(crate) keys_by_name: HashMap<String, SkillToolKey>,
    pub(crate) generation: u64,
    pub(crate) lease: std::sync::Arc<dyn std::any::Any + Send + Sync>,
}

pub(crate) fn resolve_skill_tool_catalog(
    catalog: &dyn SkillToolCatalogPort,
    context: &SkillToolCatalogContext,
    existing_names: impl IntoIterator<Item = String>,
    interface_format: &str,
) -> Result<ResolvedSkillToolCatalog, SkillToolApplicationError> {
    if !matches!(interface_format, "anthropic" | "openai-compatible") {
        return Err(SkillToolApplicationError::HostDenied(
            "provider-interface".to_string(),
        ));
    }
    let mut names: HashSet<String> = existing_names.into_iter().collect();
    let mut definitions = Vec::new();
    let mut keys_by_name = HashMap::new();
    let snapshot = catalog.catalog_for(context)?;
    for entry in snapshot.entries {
        let canonical = entry.key.canonical_name()?;
        if canonical != entry.canonical_name || !provider_name_is_valid(&canonical) {
            return Err(SkillToolApplicationError::HostDenied(
                "canonical-tool-name".to_string(),
            ));
        }
        if !names.insert(canonical.clone()) || keys_by_name.contains_key(&canonical) {
            return Err(SkillToolApplicationError::HostDenied(
                "tool-name-collision".to_string(),
            ));
        }
        keys_by_name.insert(canonical.clone(), entry.key);
        definitions.push(ToolDefinition {
            name: canonical,
            description: entry.description,
            input_schema: entry.input_schema,
        });
    }
    Ok(ResolvedSkillToolCatalog {
        definitions,
        keys_by_name,
        generation: snapshot.generation,
        lease: snapshot.lease,
    })
}

fn provider_name_is_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

#[cfg(test)]
#[path = "skill_tool_catalog_adapter_tests.rs"]
mod tests;
