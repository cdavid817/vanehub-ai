use super::api::configuration_fingerprint;
use super::domain::registry;
use serde_json::json;
use std::path::Path;

fn fingerprint(arguments: &[&str]) -> String {
    let arguments = arguments
        .iter()
        .map(|argument| (*argument).to_string())
        .collect::<Vec<_>>();
    format!(
        "{:?}",
        configuration_fingerprint(
            registry::typescript(),
            Path::new("C:/tools/typescript-language-server.exe"),
            &arguments,
            &json!({}),
            1,
        )
        .expect("fingerprint")
    )
}

/// `lsp-server-management` requires a changed configuration to make the old fingerprint stale so
/// matching servers drain and restart. Startup arguments only became configurable here, so without
/// this they would be the one setting a user could change while their server kept running under
/// the old command line.
#[test]
fn changing_startup_arguments_changes_the_configuration_fingerprint() {
    let default = fingerprint(&["--stdio"]);

    assert_eq!(default, fingerprint(&["--stdio"]));
    assert_ne!(default, fingerprint(&[]));
    assert_ne!(default, fingerprint(&["--stdio", "--log-level=4"]));
    assert_ne!(default, fingerprint(&["--log-level=4", "--stdio"]));
}

/// Concatenating the arguments would let `["ab"]` and `["a", "b"]` hash the same, which would keep
/// a server running under a command line the user had actually changed.
#[test]
fn argument_boundaries_are_part_of_the_configuration_fingerprint() {
    assert_ne!(fingerprint(&["ab"]), fingerprint(&["a", "b"]));
    assert_ne!(
        fingerprint(&["--flag=value"]),
        fingerprint(&["--flag", "value"])
    );
}

#[test]
fn the_language_and_the_executable_are_part_of_the_configuration_fingerprint() {
    let typescript = fingerprint(&["--stdio"]);
    let rust = format!(
        "{:?}",
        configuration_fingerprint(
            registry::rust(),
            Path::new("C:/tools/typescript-language-server.exe"),
            &["--stdio".to_owned()],
            &json!({}),
            1,
        )
        .expect("fingerprint")
    );
    let other_executable = format!(
        "{:?}",
        configuration_fingerprint(
            registry::typescript(),
            Path::new("C:/other/typescript-language-server.exe"),
            &["--stdio".to_owned()],
            &json!({}),
            1,
        )
        .expect("fingerprint")
    );
    let other_trust_revision = format!(
        "{:?}",
        configuration_fingerprint(
            registry::typescript(),
            Path::new("C:/tools/typescript-language-server.exe"),
            &["--stdio".to_owned()],
            &json!({}),
            2,
        )
        .expect("fingerprint")
    );

    assert_ne!(typescript, rust);
    assert_ne!(typescript, other_executable);
    assert_ne!(typescript, other_trust_revision);
}
