use super::SqliteCodeIntelligenceRepository;
use crate::contexts::code_intelligence::application::ports::{
    LspConfigurationRepository, WorkspaceTrustRepository,
};
use crate::contexts::code_intelligence::domain::configuration::{
    LanguageConfiguration, LspConfiguration, MAX_INITIALIZATION_OPTIONS_BYTES,
};
use crate::contexts::code_intelligence::domain::models::LanguageFamily;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use serde_json::json;
use std::collections::BTreeMap;

fn fixture(label: &str) -> (TempDirectory, SqliteCodeIntelligenceRepository) {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    (directory, SqliteCodeIntelligenceRepository::new(database))
}

#[test]
fn fresh_configuration_is_disabled_for_every_supported_language() {
    let (_directory, repository) = fixture("lsp config defaults");
    let configuration = repository.load_configuration().expect("load defaults");

    assert!(!configuration.enabled);
    assert_eq!(configuration.languages.len(), 2);
    assert!(configuration
        .languages
        .values()
        .all(|language| !language.enabled));
}

#[test]
fn invalid_configuration_does_not_replace_the_last_valid_configuration() {
    let (_directory, repository) = fixture("lsp config atomic validation");
    let absolute = std::env::current_dir()
        .expect("current directory")
        .join("rust-analyzer");
    let mut valid = LspConfiguration {
        enabled: true,
        ..LspConfiguration::default()
    };
    valid
        .languages
        .get_mut(&LanguageFamily::Rust)
        .expect("rust")
        .enabled = true;
    valid
        .languages
        .get_mut(&LanguageFamily::Rust)
        .expect("rust")
        .executable_override = Some(absolute.to_string_lossy().into_owned());
    repository
        .save_configuration(&valid)
        .expect("save valid configuration");

    let mut invalid = valid.clone();
    invalid
        .languages
        .get_mut(&LanguageFamily::Rust)
        .expect("rust")
        .executable_override = Some("relative/rust-analyzer".into());
    assert!(repository.save_configuration(&invalid).is_err());

    assert_eq!(
        repository.load_configuration().expect("load preserved"),
        valid
    );
}

#[test]
fn configuration_validation_rejects_missing_languages_and_unbounded_non_object_options() {
    let mut missing = LspConfiguration {
        enabled: true,
        languages: BTreeMap::new(),
    };
    missing
        .languages
        .insert(LanguageFamily::Rust, LanguageConfiguration::default());
    assert!(missing.validate().is_err());

    let mut non_object = LspConfiguration::default();
    non_object
        .languages
        .get_mut(&LanguageFamily::Rust)
        .expect("rust")
        .initialization_options = json!(["not", "an", "object"]);
    assert!(non_object.validate().is_err());

    let mut oversized = LspConfiguration::default();
    oversized
        .languages
        .get_mut(&LanguageFamily::Rust)
        .expect("rust")
        .initialization_options = json!({
        "value": "x".repeat(MAX_INITIALIZATION_OPTIONS_BYTES)
    });
    assert!(oversized.validate().is_err());
}

#[test]
fn trust_repository_canonicalizes_roots_and_advances_revisions() {
    let (workspace, repository) = fixture("lsp workspace trust");
    let trusted = repository
        .set_workspace_trust(workspace.path(), true)
        .expect("trust workspace");
    let revoked = repository
        .set_workspace_trust(workspace.path(), false)
        .expect("revoke workspace");

    assert!(trusted.is_trusted());
    assert_eq!(
        std::path::Path::new(trusted.canonical_root()),
        std::fs::canonicalize(workspace.path())
            .expect("canonical workspace")
            .as_path()
    );
    assert_eq!(trusted.revision(), 1);
    assert!(!revoked.is_trusted());
    assert_eq!(revoked.revision(), 2);
    assert_eq!(
        repository.list_workspace_trust().expect("list trust"),
        vec![revoked]
    );
}

#[test]
fn trust_repository_rejects_relative_and_missing_roots() {
    let (_directory, repository) = fixture("lsp invalid workspace trust");

    assert!(repository
        .set_workspace_trust(std::path::Path::new("relative"), true)
        .is_err());
    assert!(repository
        .set_workspace_trust(std::path::Path::new("Z:/missing/vanehub-workspace"), true)
        .is_err());
    assert!(repository
        .list_workspace_trust()
        .expect("no invalid trust persisted")
        .is_empty());
}
