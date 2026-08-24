use crate::contexts::tooling::cli_parameters::application::error::CliParameterApplicationError;
use crate::contexts::tooling::cli_parameters::application::ports::CliParameterCatalogPort;
use crate::contexts::tooling::cli_parameters::domain::catalog::CliParameterCatalog;
use crate::contexts::tooling::cli_parameters::domain::error::CliParameterDomainError;
use std::sync::{Arc, OnceLock};

/// The canonical registry. It is the single source of truth for native validation, native launch
/// projection, and the generated frontend contract.
const CANONICAL_CATALOG_SOURCE: &str = include_str!("../catalog/catalog.v2.json");

static CATALOG: OnceLock<Result<Arc<CliParameterCatalog>, String>> = OnceLock::new();

fn load() -> &'static Result<Arc<CliParameterCatalog>, String> {
    CATALOG.get_or_init(|| {
        CliParameterCatalog::parse(CANONICAL_CATALOG_SOURCE)
            .map(Arc::new)
            .map_err(|error| {
                error
                    .details
                    .get("reason")
                    .cloned()
                    .unwrap_or_else(|| error.code_str().to_string())
            })
    })
}

#[derive(Clone, Default)]
pub(crate) struct EmbeddedCliParameterCatalog;

impl CliParameterCatalogPort for EmbeddedCliParameterCatalog {
    /// Production never panics on a bad registry: it returns the structured catalog error and the
    /// settings page explains that the catalog needs repair. Tests and generation fail loudly.
    fn catalog(&self) -> Result<Arc<CliParameterCatalog>, CliParameterApplicationError> {
        match load() {
            Ok(catalog) => Ok(Arc::clone(catalog)),
            Err(reason) => Err(CliParameterDomainError::catalog_invalid(reason.clone()).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli_parameters::domain::catalog_validation::MANAGED_CLI_AGENT_IDS;
    use crate::contexts::tooling::cli_parameters::domain::definition::CliParameterOwnership;

    fn catalog() -> Arc<CliParameterCatalog> {
        EmbeddedCliParameterCatalog
            .catalog()
            .expect("the canonical registry must be valid")
    }

    #[test]
    fn the_canonical_registry_passes_every_invariant() {
        let catalog = catalog();
        assert_eq!(catalog.agent_ids(), MANAGED_CLI_AGENT_IDS.to_vec());
        assert_eq!(catalog.selection_schema_version, 2);
        assert!(!catalog.catalog_version.is_empty());
    }

    #[test]
    fn every_managed_cli_exposes_at_least_one_editable_parameter() {
        let catalog = catalog();
        for agent_id in MANAGED_CLI_AGENT_IDS {
            let editable = catalog.editable_definitions(agent_id).expect("known agent");
            assert!(
                !editable.is_empty(),
                "{agent_id} must expose editable parameters"
            );
            assert!(editable
                .iter()
                .all(|definition| definition.ownership == CliParameterOwnership::UserEditable));
        }
    }

    #[test]
    fn policy_governed_parameters_exist_but_are_never_editable() {
        let catalog = catalog();
        let governed = [
            ("claude-code", "permissionMode"),
            ("codex-cli", "sandbox"),
            ("codex-cli", "approvalPolicy"),
            ("gemini-cli", "approvalMode"),
            ("gemini-cli", "sandbox"),
            ("opencode", "agent"),
            ("opencode", "autoApprove"),
            ("antigravity-cli", "mode"),
            ("antigravity-cli", "sandbox"),
        ];
        for (agent_id, parameter_id) in governed {
            let definition = catalog
                .definition(agent_id, parameter_id)
                .expect("policy parameter must stay in the registry for the policy path");
            assert_eq!(definition.ownership, CliParameterOwnership::PolicyGoverned);
            assert!(catalog.editable_definition(agent_id, parameter_id).is_err());
        }
    }

    #[test]
    fn the_editable_registry_omits_reserved_and_dangerous_flags() {
        let catalog = catalog();
        let reserved = [
            "--output-format",
            "--resume",
            "--session",
            "--session-id",
            "--json",
            "--format",
            "--prompt",
            "--conversation",
            "-p",
        ];
        for agent_id in MANAGED_CLI_AGENT_IDS {
            for definition in catalog.definitions(agent_id).expect("known agent") {
                for flag in definition.renderer.flags() {
                    assert!(!reserved.contains(&flag), "{agent_id} exposes {flag}");
                    assert!(!flag.contains("dangerously"), "{agent_id} exposes {flag}");
                }
            }
        }
    }

    #[test]
    fn every_definition_carries_a_current_audit_record() {
        let catalog = catalog();
        for agent_id in MANAGED_CLI_AGENT_IDS {
            for definition in catalog.definitions(agent_id).expect("known agent") {
                assert!(definition.audit.source_url.starts_with("https://"));
                assert_eq!(definition.audit.reviewed_at.len(), 10);
                assert!(!definition.audit.note.trim().is_empty());
            }
        }
    }

    /// Task 5.10 — the exact flag spelling and argument slot each provider grammar expects.
    /// `codex-cli.ephemeral` is the only parameter whose slot straddles a subcommand, and every
    /// opencode parameter follows `run`, so none of them may claim the global slot.
    #[test]
    fn every_provider_declares_the_flag_and_slot_its_grammar_expects() {
        let catalog = catalog();
        let expectations: [(&str, &str, &str, &str); 12] = [
            ("claude-code", "model", "--model", "global"),
            ("claude-code", "effort", "--effort", "global"),
            (
                "claude-code",
                "screenReader",
                "--ax-screen-reader",
                "global",
            ),
            ("codex-cli", "model", "--model", "global"),
            ("codex-cli", "reasoningEffort", "--config", "global"),
            ("codex-cli", "ephemeral", "--ephemeral", "invocation"),
            ("gemini-cli", "model", "--model", "global"),
            ("gemini-cli", "extensions", "--extensions", "global"),
            (
                "gemini-cli",
                "includeDirectories",
                "--include-directories",
                "global",
            ),
            ("opencode", "model", "--model", "invocation"),
            ("opencode", "thinking", "--thinking", "invocation"),
            ("antigravity-cli", "effort", "--effort", "global"),
        ];
        for (agent_id, parameter_id, flag, slot) in expectations {
            let definition = catalog
                .definition(agent_id, parameter_id)
                .expect("declared parameter");
            assert!(
                definition.renderer.flags().contains(&flag),
                "{agent_id}.{parameter_id} must map to {flag}"
            );
            assert_eq!(
                serde_json::to_value(definition.renderer.slot())
                    .expect("slot")
                    .as_str()
                    .expect("slot string"),
                slot,
                "{agent_id}.{parameter_id} slot"
            );
        }
    }

    #[test]
    fn opencode_claims_no_global_slot_because_its_options_follow_the_run_subcommand() {
        let catalog = catalog();
        for definition in catalog.definitions("opencode").expect("opencode") {
            assert_eq!(
                serde_json::to_value(definition.renderer.slot())
                    .expect("slot")
                    .as_str(),
                Some("invocation"),
                "opencode.{} must not precede `run`",
                definition.id
            );
        }
        // Conversely, only the one codex parameter the `exec` grammar owns is an invocation token.
        let codex_invocation = catalog
            .definitions("codex-cli")
            .expect("codex")
            .iter()
            .filter(|definition| {
                serde_json::to_value(definition.renderer.slot())
                    .expect("slot")
                    .as_str()
                    == Some("invocation")
            })
            .map(|definition| definition.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(codex_invocation, ["ephemeral"]);
    }

    /// The registry is compiled into the binary with `include_str!`, so resolution cannot depend on
    /// a source tree being present next to the executable.
    #[test]
    fn the_registry_loads_independently_of_the_working_directory() {
        let directory = tempfile::tempdir().expect("tempdir");
        let original = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(directory.path()).expect("chdir");
        let parsed = CliParameterCatalog::parse(CANONICAL_CATALOG_SOURCE);
        std::env::set_current_dir(original).expect("restore cwd");
        assert!(parsed.is_ok());
    }

    #[test]
    fn parsing_is_cached_and_deterministic() {
        let first = catalog();
        let second = catalog();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn an_invalid_registry_is_a_structured_error_rather_than_a_panic() {
        let broken = CliParameterCatalog::parse("{\"catalogVersion\":\"\"}");
        let error = broken.expect_err("must reject");
        assert_eq!(error.code_str(), "CLI_PARAMETER_CATALOG_INVALID");
    }
}
