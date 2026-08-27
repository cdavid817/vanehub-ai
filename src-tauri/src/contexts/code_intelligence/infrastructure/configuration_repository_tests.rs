use super::SqliteCodeIntelligenceRepository;
use crate::contexts::code_intelligence::application::ports::{
    LspConfigurationRepository, WorkspaceTrustRepository,
};
use crate::contexts::code_intelligence::domain::configuration::{
    LanguageConfiguration, LspConfiguration, MAX_INITIALIZATION_OPTIONS_BYTES,
    MAX_STARTUP_ARGUMENTS, MAX_STARTUP_ARGUMENT_BYTES,
};
use crate::contexts::code_intelligence::domain::registry;
use crate::contexts::code_intelligence::domain::registry::LANGUAGE_DEFINITIONS;
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
    assert_eq!(configuration.languages.len(), LANGUAGE_DEFINITIONS.len());
    assert!(configuration
        .languages
        .values()
        .all(|language| !language.enabled));
}

#[test]
fn startup_arguments_round_trip_and_keep_unset_distinct_from_empty() {
    let (_directory, repository) = fixture("lsp config startup arguments");
    let mut configuration = LspConfiguration::default();
    configuration
        .languages
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .startup_arguments = Some(vec!["--log-file".to_owned(), "trace.log".to_owned()]);
    configuration
        .languages
        .get_mut(&registry::typescript().language_id())
        .expect("typescript")
        .startup_arguments = Some(Vec::new());
    repository
        .save_configuration(&configuration)
        .expect("save startup arguments");

    let reloaded = repository.load_configuration().expect("reload");
    assert_eq!(
        reloaded
            .language(&registry::rust().language_id())
            .expect("rust")
            .startup_arguments,
        Some(vec!["--log-file".to_owned(), "trace.log".to_owned()])
    );
    // Survives as an explicit empty list rather than collapsing back to "unset", which would
    // silently restore the registry default the user just cleared.
    assert_eq!(
        reloaded
            .language(&registry::typescript().language_id())
            .expect("typescript")
            .startup_arguments,
        Some(Vec::new())
    );

    let mut cleared = reloaded;
    cleared
        .languages
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .startup_arguments = None;
    repository
        .save_configuration(&cleared)
        .expect("save unset arguments");
    assert_eq!(
        repository
            .load_configuration()
            .expect("reload after unset")
            .language(&registry::rust().language_id())
            .expect("rust")
            .startup_arguments,
        None
    );
}

#[test]
fn a_stored_row_for_an_unregistered_language_neither_fails_the_load_nor_appears_in_it() {
    let (directory, repository) = fixture("lsp config unregistered language");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    database
        .connection()
        .expect("connection")
        .execute(
            "INSERT INTO lsp_language_configurations (language_id, enabled) VALUES ('ruby', 1)",
            [],
        )
        .expect("insert unregistered language row");

    let configuration = repository
        .load_configuration()
        .expect("load succeeds despite the unknown row");

    assert_eq!(configuration.languages.len(), LANGUAGE_DEFINITIONS.len());
    assert!(!configuration
        .languages
        .keys()
        .any(|language| language.as_str() == "ruby"));
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
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .enabled = true;
    valid
        .languages
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .executable_override = Some(absolute.to_string_lossy().into_owned());
    repository
        .save_configuration(&valid)
        .expect("save valid configuration");

    let mut invalid = valid.clone();
    invalid
        .languages
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .executable_override = Some("relative/rust-analyzer".into());
    assert!(repository.save_configuration(&invalid).is_err());

    assert_eq!(
        repository.load_configuration().expect("load preserved"),
        valid
    );
}

#[test]
fn a_configuration_naming_only_some_registered_languages_is_valid() {
    // This used to be rejected: the configuration had to name every supported language exactly
    // once. That requirement only held while the set was fixed. A build that registers a new
    // language must be able to read a configuration written before that language existed, so a
    // partial map is now the normal case rather than a corruption.
    let mut partial = LspConfiguration {
        enabled: true,
        languages: BTreeMap::new(),
    };
    partial.languages.insert(
        registry::rust().language_id(),
        LanguageConfiguration::default(),
    );
    assert!(partial.validate().is_ok());

    let empty = LspConfiguration {
        enabled: true,
        languages: BTreeMap::new(),
    };
    assert!(empty.validate().is_ok());
}

#[test]
fn configuration_validation_rejects_unbounded_non_object_options_and_bad_startup_arguments() {
    let mut non_object = LspConfiguration::default();
    non_object
        .languages
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .initialization_options = json!(["not", "an", "object"]);
    assert!(non_object.validate().is_err());

    let mut oversized = LspConfiguration::default();
    oversized
        .languages
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .initialization_options = json!({
        "value": "x".repeat(MAX_INITIALIZATION_OPTIONS_BYTES)
    });
    assert!(oversized.validate().is_err());

    let mut too_many_arguments = LspConfiguration::default();
    too_many_arguments
        .languages
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .startup_arguments = Some(vec!["--flag".to_owned(); MAX_STARTUP_ARGUMENTS + 1]);
    assert!(too_many_arguments.validate().is_err());

    let mut oversized_arguments = LspConfiguration::default();
    oversized_arguments
        .languages
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .startup_arguments = Some(vec!["x".repeat(MAX_STARTUP_ARGUMENT_BYTES + 1)]);
    assert!(oversized_arguments.validate().is_err());

    // A NUL would be truncated or rejected by the platform when the argument list is handed to a
    // process, so it is refused here where the reason can still be reported.
    let mut embedded_nul = LspConfiguration::default();
    embedded_nul
        .languages
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .startup_arguments = Some(vec!["--flag\0".to_owned()]);
    assert!(embedded_nul.validate().is_err());

    let mut empty_arguments = LspConfiguration::default();
    empty_arguments
        .languages
        .get_mut(&registry::rust().language_id())
        .expect("rust")
        .startup_arguments = Some(Vec::new());
    assert!(empty_arguments.validate().is_ok());
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
