use super::language_id::{LanguageIdError, LspLanguageId};
use super::registry::{
    definition, definition_for_extension, definition_for_server, HostPlatform, LANGUAGE_DEFINITIONS,
};
use std::collections::BTreeSet;

/// The compiler no longer proves that every language is handled, because the enum it could match
/// exhaustively is gone. This is what replaces that guarantee: a language that reaches the runtime
/// without the data some consumer needs fails here rather than failing a user with an unexplained
/// unavailable server.
#[test]
fn every_registered_language_declares_everything_its_consumers_need() {
    assert!(!LANGUAGE_DEFINITIONS.is_empty());
    for definition in LANGUAGE_DEFINITIONS {
        let id = definition.id;
        assert!(
            LspLanguageId::new(id).is_ok(),
            "{id}: language id must satisfy its own validation"
        );
        assert!(
            !definition.server_id.is_empty(),
            "{id}: server id must not be empty"
        );
        assert!(
            !definition.executables.is_empty(),
            "{id}: discovery needs at least one candidate executable"
        );
        assert!(
            !definition.root_markers.is_empty(),
            "{id}: project-root detection needs at least one marker"
        );
        assert!(
            !definition.extensions.is_empty(),
            "{id}: document admission needs at least one extension mapping"
        );
        assert!(
            !definition.fixture_files.is_empty(),
            "{id}: the isolated server test needs a minimal project"
        );
        assert!(
            !definition.platforms.is_empty(),
            "{id}: a language supported nowhere cannot be offered anywhere"
        );
        for (extension, language_id) in definition.extensions {
            assert!(
                !extension.is_empty() && !language_id.is_empty(),
                "{id}: extension mappings must be populated on both sides"
            );
            assert!(
                !extension.starts_with('.'),
                "{id}: extensions are compared against Path::extension output, which omits the dot"
            );
        }
    }
}

#[test]
fn registry_declared_ids_round_trip_through_the_validated_identifier() {
    // `trusted` skips validation in release builds, so the values this repository actually passes
    // to it are asserted here. A registry id that stopped being valid would otherwise only fail as
    // a debug assertion on whoever ran the app next.
    for definition in LANGUAGE_DEFINITIONS {
        let trusted = definition.language_id();
        assert_eq!(trusted.as_str(), definition.id);
        assert_eq!(
            LspLanguageId::new(definition.id).expect("registry id validates"),
            trusted
        );
        assert!(trusted == *definition.id);
    }
}

#[test]
fn language_ids_server_ids_and_extensions_are_unique_across_the_registry() {
    let mut language_ids = BTreeSet::new();
    let mut server_ids = BTreeSet::new();
    let mut extensions = BTreeSet::new();
    for definition in LANGUAGE_DEFINITIONS {
        assert!(
            language_ids.insert(definition.id),
            "duplicate language id {}",
            definition.id
        );
        assert!(
            server_ids.insert(definition.server_id),
            "duplicate server id {}",
            definition.server_id
        );
        for (extension, _) in definition.extensions {
            // Extension lookup returns the first match, so a contested extension would resolve by
            // declaration order and silently route a file to the wrong server.
            assert!(
                extensions.insert(*extension),
                "extension {extension} is claimed by more than one language"
            );
        }
    }
}

#[test]
fn the_registry_still_declares_the_two_languages_the_previous_enum_hard_coded() {
    let rust = definition("rust").expect("rust is registered");
    assert_eq!(rust.server_id, "rust_analyzer");
    assert_eq!(rust.executables, &["rust-analyzer"]);
    assert!(rust.default_startup_arguments.is_empty());
    assert_eq!(rust.root_markers, &["Cargo.toml"]);
    assert_eq!(rust.language_id_for_extension("rs"), Some("rust"));

    let typescript = definition("typescript_javascript").expect("typescript is registered");
    assert_eq!(typescript.server_id, "typescript_language_server");
    assert_eq!(typescript.executables, &["typescript-language-server"]);
    assert_eq!(typescript.default_startup_arguments, &["--stdio"]);
    assert_eq!(
        typescript.root_markers,
        &["tsconfig.json", "jsconfig.json", "package.json"]
    );
    for (extension, expected) in [
        ("ts", "typescript"),
        ("tsx", "typescriptreact"),
        ("js", "javascript"),
        ("mjs", "javascript"),
        ("cjs", "javascript"),
        ("jsx", "javascriptreact"),
    ] {
        assert_eq!(
            typescript.language_id_for_extension(extension),
            Some(expected)
        );
    }
}

#[test]
fn lookups_resolve_registered_values_and_reject_everything_else() {
    assert!(definition("go").is_none());
    assert!(definition_for_server("gopls").is_none());
    assert!(definition_for_extension("go").is_none());

    let (owner, language_id) = definition_for_extension("tsx").expect("tsx is admitted");
    assert_eq!(owner.id, "typescript_javascript");
    assert_eq!(language_id, "typescriptreact");
    assert_eq!(
        definition_for_server("rust_analyzer").map(|found| found.id),
        Some("rust")
    );
}

#[test]
fn every_registered_language_supports_the_platform_the_tests_run_on() {
    // Not a property of the registry in general -- a language may legitimately declare fewer
    // platforms -- but true of both current entries, so a build where it stopped being true is a
    // change someone made rather than one that drifted in.
    for definition in LANGUAGE_DEFINITIONS {
        assert!(
            definition.supports_host(),
            "{}: expected support for the host platform",
            definition.id
        );
        assert!(definition.platforms.contains(&HostPlatform::current()));
    }
}

#[test]
fn language_id_validation_rejects_what_would_break_a_key_or_a_localization_lookup() {
    assert!(LspLanguageId::new("rust").is_ok());
    assert!(LspLanguageId::new("typescript_javascript").is_ok());
    assert!(LspLanguageId::new("cpp17").is_ok());

    assert_eq!(LspLanguageId::new(""), Err(LanguageIdError::Empty));
    assert_eq!(
        LspLanguageId::new("Rust"),
        Err(LanguageIdError::UnsupportedCharacter)
    );
    assert_eq!(
        LspLanguageId::new("c++"),
        Err(LanguageIdError::UnsupportedCharacter)
    );
    assert_eq!(
        LspLanguageId::new(" rust"),
        Err(LanguageIdError::UnsupportedCharacter)
    );
    assert_eq!(
        LspLanguageId::new("rust.analyzer"),
        Err(LanguageIdError::UnsupportedCharacter)
    );
    assert!(matches!(
        LspLanguageId::new("a".repeat(65)),
        Err(LanguageIdError::TooLong { length: 65 })
    ));
}
