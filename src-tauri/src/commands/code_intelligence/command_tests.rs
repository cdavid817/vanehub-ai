use super::dto::{
    LspConfigurationDto, LspDiscoveryAvailabilityDto, LspLanguageConfigurationDto,
    LspSafeReasonCodeDto, LspServerTestInputDto, LspServerTestPhaseDto,
    LspServerTestPhaseStatusDto, LspWorkspaceTrustUpdateDto,
};
use crate::contexts::code_intelligence::api::CodeIntelligenceApi;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

fn fixture(label: &str) -> (TempDirectory, CodeIntelligenceApi) {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().join("data")).expect("database");
    let logging = Arc::new(UnifiedLoggingAdapter::new(directory.path().join("logs")));
    (
        directory,
        CodeIntelligenceApi::from_database(database, logging),
    )
}

fn unavailable_configuration(root: &Path) -> LspConfigurationDto {
    LspConfigurationDto {
        enabled: true,
        languages: vec![
            LspLanguageConfigurationDto {
                language: "rust".to_owned(),
                enabled: true,
                executable_override: Some(
                    root.join("missing-rust-analyzer")
                        .to_string_lossy()
                        .into_owned(),
                ),
                startup_arguments: None,
                initialization_options: json!({"cargo": {"allTargets": false}}),
            },
            LspLanguageConfigurationDto {
                language: "typescript_javascript".to_owned(),
                enabled: false,
                executable_override: Some(
                    root.join("missing-typescript-language-server")
                        .to_string_lossy()
                        .into_owned(),
                ),
                startup_arguments: None,
                initialization_options: json!({}),
            },
        ],
        // Ignored on the way in; the backend rebuilds it from the registry on the way out.
        descriptors: Vec::new(),
    }
}

#[test]
fn configuration_commands_round_trip_validated_values() {
    let (directory, api) = fixture("lsp-command-configuration");

    let initial = super::get_lsp_configuration::execute(&api).expect("initial configuration");
    assert!(!initial.enabled);
    assert_eq!(initial.languages.len(), 2);
    // Descriptors describe the build, not the saved settings, so they are present before anything
    // has been configured and are rebuilt on every read rather than round-tripped.
    assert_eq!(
        initial
            .descriptors
            .iter()
            .map(|descriptor| descriptor.language.as_str())
            .collect::<Vec<_>>(),
        vec!["rust", "typescript_javascript"]
    );

    let replacement = unavailable_configuration(directory.path());
    super::save_lsp_configuration::execute(&api, replacement.clone()).expect("save configuration");

    let saved = super::get_lsp_configuration::execute(&api).expect("saved configuration");
    assert_eq!(saved.enabled, replacement.enabled);
    assert_eq!(saved.languages, replacement.languages);
    assert_eq!(saved.descriptors, initial.descriptors);
}

#[test]
fn saving_a_configuration_for_an_unregistered_language_is_refused() {
    let (_directory, api) = fixture("lsp-command-unregistered-language");

    let rejected = LspConfigurationDto {
        enabled: true,
        languages: vec![LspLanguageConfigurationDto {
            language: "go".to_owned(),
            enabled: true,
            executable_override: None,
            startup_arguments: None,
            initialization_options: json!({}),
        }],
        descriptors: Vec::new(),
    };

    assert!(super::save_lsp_configuration::execute(&api, rejected).is_err());
    // The refusal must leave the stored configuration untouched rather than half-applied.
    let stored = super::get_lsp_configuration::execute(&api).expect("configuration after refusal");
    assert!(!stored.enabled);
    assert_eq!(stored.languages.len(), 2);
}

#[test]
fn workspace_trust_commands_canonicalize_update_and_list() {
    let (directory, api) = fixture("lsp-command-trust");
    let workspace = directory.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");

    let updated = super::update_lsp_workspace_trust::execute(
        &api,
        LspWorkspaceTrustUpdateDto {
            canonical_root: workspace.to_string_lossy().into_owned(),
            trusted: true,
        },
    )
    .expect("update trust");

    assert!(updated.trusted);
    assert_eq!(updated.revision, 1);
    assert_eq!(
        super::list_lsp_workspace_trust::execute(&api).expect("list trust"),
        vec![updated]
    );
}

#[tokio::test]
async fn discovery_and_server_test_return_safe_unavailable_results() {
    let (directory, api) = fixture("lsp-command-server-test");
    super::save_lsp_configuration::execute(&api, unavailable_configuration(directory.path()))
        .expect("save unavailable configuration");

    let discoveries =
        super::discover_lsp_servers::execute(&api).expect("discover configured servers");
    assert_eq!(discoveries.len(), 2);
    assert!(discoveries.iter().all(|result| {
        result.availability == LspDiscoveryAvailabilityDto::Unavailable
            && result.reason_code == Some(LspSafeReasonCodeDto::OverrideMissing)
    }));

    let result = super::test_lsp_server::execute(
        &api,
        LspServerTestInputDto {
            language: "rust".to_owned(),
        },
    )
    .await
    .expect("test unavailable server");
    let discovery = result
        .phases
        .iter()
        .find(|phase| phase.phase == LspServerTestPhaseDto::Discovery)
        .expect("discovery phase");
    assert_eq!(discovery.status, LspServerTestPhaseStatusDto::Failed);
    assert_eq!(
        discovery.reason_code,
        Some(LspSafeReasonCodeDto::ExecutableUnavailable)
    );
}

#[tokio::test]
async fn server_status_command_starts_empty_and_all_commands_are_registered() {
    let (_directory, api) = fixture("lsp-command-status");
    assert!(super::list_lsp_server_status::execute(&api)
        .await
        .expect("server statuses")
        .is_empty());

    let registry = concat!(
        include_str!("../core_registry.rs"),
        include_str!("../builtin_tool_registry.rs"),
        include_str!("../supplemental_registry.rs")
    );
    for command in [
        "get_lsp_configuration::get_lsp_configuration",
        "save_lsp_configuration::save_lsp_configuration",
        "list_lsp_workspace_trust::list_lsp_workspace_trust",
        "update_lsp_workspace_trust::update_lsp_workspace_trust",
        "discover_lsp_servers::discover_lsp_servers",
        "test_lsp_server::test_lsp_server",
        "list_lsp_server_status::list_lsp_server_status",
    ] {
        assert!(
            registry.contains(command),
            "missing registry entry: {command}"
        );
    }
}
