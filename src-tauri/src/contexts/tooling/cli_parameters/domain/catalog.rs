use super::compatibility::CliParameterSupport;
use super::definition::{CliLaunchScope, CliParameterDefinition};
use super::diagnostic::{CliParameterDiagnostic, CliParameterDiagnosticCode};
use super::error::CliParameterDomainError;
use super::rendering::CliArgumentSegments;
use super::selection::CliParameterSelectionMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CliParameterAgentCatalog {
    pub(crate) agent_id: String,
    pub(crate) parameters: Vec<CliParameterDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CliParameterCatalog {
    pub(crate) catalog_version: String,
    pub(crate) selection_schema_version: u32,
    pub(crate) agents: Vec<CliParameterAgentCatalog>,
}

impl CliParameterCatalog {
    /// Parses and fully validates the canonical registry. Callers never see a partially valid
    /// catalog: an invariant failure is a `CLI_PARAMETER_CATALOG_INVALID` error.
    pub(crate) fn parse(source: &str) -> Result<Self, CliParameterDomainError> {
        let mut catalog: Self = serde_json::from_str(source)
            .map_err(|error| CliParameterDomainError::catalog_invalid(error.to_string()))?;
        for agent in &mut catalog.agents {
            let agent_id = agent.agent_id.clone();
            for parameter in &mut agent.parameters {
                parameter.agent_id = agent_id.clone();
            }
        }
        super::catalog_validation::validate(&catalog)?;
        Ok(catalog)
    }

    pub(crate) fn agent_ids(&self) -> Vec<&str> {
        self.agents
            .iter()
            .map(|agent| agent.agent_id.as_str())
            .collect()
    }

    pub(crate) fn contains_agent(&self, agent_id: &str) -> bool {
        self.agents.iter().any(|agent| agent.agent_id == agent_id)
    }

    pub(crate) fn definitions(
        &self,
        agent_id: &str,
    ) -> Result<&[CliParameterDefinition], CliParameterDomainError> {
        self.agents
            .iter()
            .find(|agent| agent.agent_id == agent_id)
            .map(|agent| agent.parameters.as_slice())
            .ok_or_else(|| CliParameterDomainError::unknown_agent(agent_id))
    }

    /// The only view the settings page and its transport DTOs ever receive.
    pub(crate) fn editable_definitions(
        &self,
        agent_id: &str,
    ) -> Result<Vec<&CliParameterDefinition>, CliParameterDomainError> {
        Ok(self
            .definitions(agent_id)?
            .iter()
            .filter(|definition| definition.is_user_editable())
            .collect())
    }

    #[cfg(test)]
    pub(crate) fn definition(
        &self,
        agent_id: &str,
        parameter_id: &str,
    ) -> Result<&CliParameterDefinition, CliParameterDomainError> {
        self.definitions(agent_id)?
            .iter()
            .find(|definition| definition.id == parameter_id)
            .ok_or_else(|| CliParameterDomainError::unknown_parameter(agent_id, parameter_id))
    }

    #[cfg(test)]
    pub(crate) fn editable_definition(
        &self,
        agent_id: &str,
        parameter_id: &str,
    ) -> Result<&CliParameterDefinition, CliParameterDomainError> {
        let definition = self.definition(agent_id, parameter_id)?;
        if !definition.is_user_editable() {
            return Err(CliParameterDomainError::unknown_parameter(
                agent_id,
                parameter_id,
            ));
        }
        Ok(definition)
    }
}

/// Deterministic projection of resolved selections into placement segments. Catalog order is the
/// only ordering input, so the same profile always renders the same token sequence.
pub(crate) fn render_segments(
    agent_id: &str,
    definitions: &[CliParameterDefinition],
    selections: &CliParameterSelectionMap,
    scope: CliLaunchScope,
    support: &BTreeMap<String, CliParameterSupport>,
) -> (CliArgumentSegments, Vec<CliParameterDiagnostic>) {
    let mut segments = CliArgumentSegments::default();
    let mut diagnostics = Vec::new();
    for definition in definitions {
        let Some(selection) = selections.get(&definition.id) else {
            continue;
        };
        let Some(value) = selection.as_value() else {
            continue;
        };
        if !definition.applies_to(scope) {
            continue;
        }
        if let Some(state) = support.get(&definition.id) {
            if state.blocks_launch() {
                let code = match state {
                    CliParameterSupport::UnsupportedPlatform { .. } => {
                        CliParameterDiagnosticCode::UnsupportedPlatform
                    }
                    _ => CliParameterDiagnosticCode::UnsupportedByActiveVersion,
                };
                diagnostics.push(
                    CliParameterDiagnostic::new(code, agent_id, Some(definition.id.clone()))
                        .with_detail("scope", format!("{scope:?}").to_lowercase()),
                );
                continue;
            }
        }
        for token in definition.renderer.render(&definition.id, value) {
            segments.push(token);
        }
    }
    (segments, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::super::compatibility::CliParameterSupport;
    use super::super::selection::CliParameterSelection;
    use super::super::testing::{boolean_definition, custom_text_definition};
    use super::*;

    fn definitions() -> Vec<CliParameterDefinition> {
        let mut model = custom_text_definition();
        model.launch_scopes = vec![CliLaunchScope::Interactive, CliLaunchScope::Chat];
        let mut ephemeral = boolean_definition();
        ephemeral.id = "ephemeral".to_string();
        ephemeral.launch_scopes = vec![CliLaunchScope::Chat];
        vec![model, ephemeral]
    }

    #[test]
    fn inherited_selections_emit_no_token() {
        let selections = CliParameterSelectionMap::from([
            ("model".to_string(), CliParameterSelection::Inherit),
            ("ephemeral".to_string(), CliParameterSelection::Inherit),
        ]);
        let (segments, diagnostics) = render_segments(
            "codex-cli",
            &definitions(),
            &selections,
            CliLaunchScope::Chat,
            &BTreeMap::new(),
        );
        assert!(segments.is_empty());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn scope_filters_definitions_without_losing_the_selection() {
        let selections = CliParameterSelectionMap::from([
            ("model".to_string(), CliParameterSelection::text("sonnet")),
            (
                "ephemeral".to_string(),
                CliParameterSelection::boolean(true),
            ),
        ]);
        let definitions = definitions();
        let (chat, _) = render_segments(
            "codex-cli",
            &definitions,
            &selections,
            CliLaunchScope::Chat,
            &BTreeMap::new(),
        );
        assert_eq!(chat.global_values(), ["--model", "sonnet", "--search"]);
        let (interactive, _) = render_segments(
            "codex-cli",
            &definitions,
            &selections,
            CliLaunchScope::Interactive,
            &BTreeMap::new(),
        );
        assert_eq!(interactive.global_values(), ["--model", "sonnet"]);
    }

    #[test]
    fn an_unsupported_selection_is_omitted_with_a_diagnostic() {
        let selections = CliParameterSelectionMap::from([(
            "model".to_string(),
            CliParameterSelection::text("sonnet"),
        )]);
        let support = BTreeMap::from([(
            "model".to_string(),
            CliParameterSupport::UnsupportedVersion {
                installed_version: "1.0.0".to_string(),
                required_range: ">= 2.0.0".to_string(),
            },
        )]);
        let (segments, diagnostics) = render_segments(
            "claude-code",
            &definitions(),
            &selections,
            CliLaunchScope::Chat,
            &support,
        );
        assert!(segments.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            CliParameterDiagnosticCode::UnsupportedByActiveVersion
        );
    }

    #[test]
    fn an_unknown_version_still_renders_the_saved_value() {
        let selections = CliParameterSelectionMap::from([(
            "model".to_string(),
            CliParameterSelection::text("sonnet"),
        )]);
        let support = BTreeMap::from([(
            "model".to_string(),
            CliParameterSupport::UnknownVersion {
                required_range: None,
            },
        )]);
        let (segments, diagnostics) = render_segments(
            "claude-code",
            &definitions(),
            &selections,
            CliLaunchScope::Chat,
            &support,
        );
        assert_eq!(segments.global_values(), ["--model", "sonnet"]);
        assert!(diagnostics.is_empty());
    }
}
