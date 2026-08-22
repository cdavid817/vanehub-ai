//! The decoder, driven end to end through the real parser.
//!
//! Tests supply manifest *text*, not a hand-built AST, so the two stages are exercised together
//! and a shape the parser cannot produce cannot be asserted about. Rejections are checked by field
//! path and code, because those are what an author acts on.

use super::{
    ContributedRuleEffect, DecodeReason, ExtensionManifestV1Decoder, HookFailureMode,
    HookHandlerDeclaration, McpTransportDeclaration, NetworkOrigin, RuntimeKind, TrustProfile,
    VersionedExtensionManifest, EXTENSION_MANIFEST_YAML_LIMITS, MAX_ACTIVATION_EVENTS,
    MAX_CONTRIBUTIONS_PER_KIND,
};
use semver::Version;
use vanehub_bounded_yaml::parse_block;

fn decode(text: &str) -> Result<VersionedExtensionManifest, (String, &'static str)> {
    let document = parse_block(text, EXTENSION_MANIFEST_YAML_LIMITS)
        .unwrap_or_else(|error| panic!("fixture should parse: {error:?}\n---\n{text}"));
    ExtensionManifestV1Decoder::new(Version::parse("1.0.0").expect("version"))
        .decode(&document)
        .map_err(|error| (error.field().to_string(), error.code()))
}

fn rejection(text: &str) -> (String, &'static str) {
    decode(text).expect_err("manifest should be rejected")
}

/// The smallest manifest that is complete. Every other fixture starts from this.
const MINIMAL: &str = "\
schema_version: 1
id: acme.git-guardian
display_name: Git Guardian
publisher: acme
version: 1.2.0
min_vanehub_version: \">=0.9.0\"
";

fn with(extra: &str) -> String {
    format!("{MINIMAL}{extra}")
}

// ---------------------------------------------------------------------------
// The whole document
// ---------------------------------------------------------------------------

#[test]
fn the_design_document_example_decodes() {
    // Keyed collections throughout, matching the shape decided in `design.md`.
    let manifest = with(
        "\
description: Adds guarded Git tools, Hooks, rules, and a GitHub connection.
license: Apache-2.0
runtime:
  kind: wasm-module
  entry: runtime/git_guardian.wasm
  trust_profile: standard
activation_events:
  - onTool:git_status
  - onHook:tool.before_execute
  - onConnector:github
requires:
  skills:
    code-reviewer:
      version: \">=2.0.0, <3.0.0\"
permissions:
  filesystem:
    read:
      - workspace/**
  network:
    origins:
      - https://api.github.com
  secrets:
    - github.token
contributes:
  tools:
    git_status:
      display_name: Git status
      input_schema: schemas/git-status-input.json
      output_schema: schemas/git-status-output.json
      handler: tool.git_status
  skills:
    guarded-reviewer:
      path: skills/guarded-reviewer/SKILL.md
  hooks:
    protect-force-push:
      event: tool.before_execute
      matcher:
        tool_ids: [native.shell]
      handler:
        kind: extension-runtime
        entry: hook.protect_force_push
      failure_mode: fail_closed
  authorization_rules:
    force-push-ask:
      operation: git_operation
      matcher:
        command_regex: \"git push --force\"
      effect: ask
      risk: critical
      allowed_scopes: [once]
  connectors:
    github:
      display_name: GitHub
      type: cli
      driver: connector.github
      auth_strategy: external-cli
      capabilities: [repository.read, pull_request.read]
",
    );

    let VersionedExtensionManifest::V1(decoded) = decode(&manifest).expect("should decode");

    assert_eq!(decoded.id.as_str(), "acme.git-guardian");
    assert_eq!(decoded.publisher.as_str(), "acme");
    assert_eq!(decoded.version, Version::parse("1.2.0").expect("version"));
    assert_eq!(decoded.display_name, "Git Guardian");
    assert_eq!(decoded.license.as_deref(), Some("Apache-2.0"));

    assert_eq!(decoded.runtime.kind, RuntimeKind::WasmModule);
    assert_eq!(decoded.runtime.trust_profile, TrustProfile::Standard);
    assert_eq!(
        decoded.runtime.entry.as_ref().map(|path| path.as_str()),
        Some("runtime/git_guardian.wasm")
    );

    assert_eq!(decoded.activation_events.len(), 3);
    assert_eq!(decoded.requires.skills.len(), 1);
    assert_eq!(decoded.requires.skills[0].id, "code-reviewer");
    // Silence means required, so an author cannot ship a missing piece by omission.
    assert!(!decoded.requires.skills[0].optional);

    let origins: Vec<&str> = decoded
        .permissions
        .network_origins
        .iter()
        .map(NetworkOrigin::as_str)
        .collect();
    assert_eq!(origins, ["https://api.github.com"]);
    assert_eq!(decoded.permissions.secret_ids, ["github.token"]);
    assert!(decoded.permissions.process_commands.is_empty());

    assert_eq!(decoded.contributes.tools.len(), 1);
    assert_eq!(decoded.contributes.tools[0].id.as_str(), "git_status");
    assert_eq!(decoded.contributes.skills.len(), 1);
    assert_eq!(decoded.contributes.hooks.len(), 1);
    assert_eq!(
        decoded.contributes.hooks[0].failure_mode,
        HookFailureMode::FailClosed
    );
    assert_eq!(
        decoded.contributes.hooks[0].handler,
        HookHandlerDeclaration::ExtensionRuntime {
            entry: "hook.protect_force_push".to_string()
        }
    );
    assert_eq!(
        decoded.contributes.authorization_rules[0].effect,
        ContributedRuleEffect::Ask
    );
    assert_eq!(decoded.contributes.connectors.len(), 1);
}

#[test]
fn a_manifest_with_no_runtime_and_no_contributions_decodes() {
    let VersionedExtensionManifest::V1(decoded) = decode(MINIMAL).expect("should decode");

    assert_eq!(decoded.runtime.kind, RuntimeKind::None);
    assert!(decoded.runtime.entry.is_none());
    // Absent trust profile is the tightest one; a looser default would grant authority nobody
    // wrote down.
    assert_eq!(decoded.runtime.trust_profile, TrustProfile::Strict);
    assert!(decoded.permissions.is_empty());
    assert_eq!(decoded.contributes.total(), 0);
    assert!(decoded.activation_events.is_empty());
}

// ---------------------------------------------------------------------------
// Unknown fields
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_field_is_refused_at_every_level_rather_than_ignored() {
    // The property that makes explicit reading worth its verbosity. Each of these is a key some
    // author wrote meaning something; ignoring one silently installs a package whose stated
    // intent nobody honoured.
    let cases = [
        ("surprise: yes\n", "surprise"),
        (
            "runtime:\n  kind: none\n  surprise: yes\n",
            "runtime.surprise",
        ),
        (
            "permissions:\n  surprise:\n    - a\n",
            "permissions.surprise",
        ),
        (
            "permissions:\n  filesystem:\n    surprise:\n      - a\n",
            "permissions.filesystem.surprise",
        ),
        (
            "requires:\n  skills:\n    a:\n      version: \"1\"\n      surprise: yes\n",
            "requires.skills.a.surprise",
        ),
        ("contributes:\n  surprise:\n    a:\n      b: c\n", "contributes.surprise"),
        (
            "contributes:\n  tools:\n    t:\n      display_name: T\n      handler: h\n      surprise: yes\n",
            "contributes.tools.t.surprise",
        ),
        (
            "contributes:\n  hooks:\n    h:\n      event: e\n      handler:\n        kind: mcp-tool\n        tool: t\n        surprise: yes\n",
            "contributes.hooks.h.handler.surprise",
        ),
    ];

    for (extra, expected_field) in cases {
        let (field, code) = rejection(&with(extra));
        assert_eq!(code, "unknown_field", "for {extra:?}");
        assert_eq!(field, expected_field, "for {extra:?}");
    }
}

// ---------------------------------------------------------------------------
// Schema and application version
// ---------------------------------------------------------------------------

#[test]
fn a_future_schema_version_is_incompatible_rather_than_partially_read() {
    let manifest = MINIMAL.replace("schema_version: 1", "schema_version: 2");
    let (field, code) = rejection(&manifest);

    assert_eq!(field, "schema_version");
    assert_eq!(code, "unsupported_schema_version");
}

#[test]
fn a_schema_version_is_checked_before_anything_else_in_the_document() {
    // A future manifest may contain fields this build has no reader for. Reporting those as
    // unknown would send the author fixing keys that are correct for the schema they targeted.
    let manifest = "schema_version: 9\nsurprise: yes\n";
    let (field, code) = rejection(manifest);

    assert_eq!(field, "schema_version");
    assert_eq!(code, "unsupported_schema_version");
}

#[test]
fn a_non_numeric_schema_version_is_not_reported_as_an_unsupported_number() {
    let manifest = MINIMAL.replace("schema_version: 1", "schema_version: one");
    let (field, code) = rejection(&manifest);

    assert_eq!(field, "schema_version");
    assert_eq!(code, "expected_scalar");
}

#[test]
fn a_manifest_requiring_a_newer_application_is_rejected_with_both_versions() {
    let manifest = MINIMAL.replace(
        "min_vanehub_version: \">=0.9.0\"",
        "min_vanehub_version: \">=2.0.0\"",
    );
    let (field, code) = rejection(&manifest);

    assert_eq!(field, "min_vanehub_version");
    assert_eq!(code, "incompatible_application_version");
}

// ---------------------------------------------------------------------------
// The list-of-records shape
// ---------------------------------------------------------------------------

#[test]
fn a_list_of_records_is_named_rather_than_reported_as_a_type_error() {
    // The shape an author arriving from another extension ecosystem writes first. "expects a
    // mapping" would not tell them what to do; this says which form to use.
    let manifest = with("contributes:\n  tools:\n    - git_status\n");
    let (field, code) = rejection(&manifest);

    assert_eq!(field, "contributes.tools");
    assert_eq!(code, "list_of_records");
}

#[test]
fn duplicate_contribution_ids_are_unrepresentable() {
    // Not a decoder rule: the parser rejects duplicate keys, and the id *is* the key. This is the
    // reason the format is keyed rather than a list.
    let manifest = with(
        "contributes:\n  tools:\n    t:\n      display_name: A\n      handler: a\n    t:\n      display_name: B\n      handler: b\n",
    );
    let parsed = parse_block(&manifest, EXTENSION_MANIFEST_YAML_LIMITS);

    let error = parsed.expect_err("duplicate ids should not reach the decoder");
    assert_eq!(error.code(), "duplicate_key");
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

#[test]
fn an_external_package_cannot_name_the_builtin_runtime() {
    let (field, code) = rejection(&with("runtime:\n  kind: builtin\n  entry: a.wasm\n"));

    assert_eq!(field, "runtime.kind");
    assert_eq!(code, "not_permitted");
}

#[test]
fn the_component_model_is_refused_by_name_rather_than_as_a_broken_module() {
    let (field, code) = rejection(&with("runtime:\n  kind: wasm-component\n  entry: a.wasm\n"));

    assert_eq!(field, "runtime.kind");
    assert_eq!(code, "not_permitted");
}

#[test]
fn a_runtime_that_needs_an_entry_must_declare_one_and_one_that_does_not_must_not() {
    let (field, code) = rejection(&with("runtime:\n  kind: wasm-module\n"));
    assert_eq!(field, "runtime.entry");
    assert_eq!(code, "missing_field");

    let (field, code) = rejection(&with("runtime:\n  kind: none\n  entry: a.wasm\n"));
    assert_eq!(field, "runtime.entry");
    assert_eq!(code, "not_permitted");
}

#[test]
fn a_runtime_entry_is_validated_as_a_portable_path() {
    let (field, code) = rejection(&with(
        "runtime:\n  kind: wasm-module\n  entry: \"..\\\\escape.wasm\"\n",
    ));

    assert_eq!(field, "runtime.entry");
    assert_eq!(code, "invalid_package_path");
}

#[test]
fn an_unknown_trust_profile_is_refused_with_the_accepted_set() {
    let (field, code) = rejection(&with(
        "runtime:\n  kind: wasm-module\n  entry: a.wasm\n  trust_profile: full\n",
    ));

    assert_eq!(field, "runtime.trust_profile");
    assert_eq!(code, "unknown_value");
}

// ---------------------------------------------------------------------------
// Identifiers, versions, and paths
// ---------------------------------------------------------------------------

#[test]
fn a_malformed_identifier_names_the_field_that_held_it() {
    let manifest = MINIMAL.replace("id: acme.git-guardian", "id: NotValid");
    let (field, code) = rejection(&manifest);
    assert_eq!(field, "id");
    assert_eq!(code, "invalid_identifier");

    let (field, code) = rejection(&with(
        "contributes:\n  tools:\n    Bad_Id:\n      display_name: T\n      handler: h\n",
    ));
    assert_eq!(field, "contributes.tools.Bad_Id");
    assert_eq!(code, "invalid_identifier");
}

#[test]
fn a_malformed_version_is_distinguished_from_a_malformed_requirement() {
    let manifest = MINIMAL.replace("version: 1.2.0", "version: one");
    let (field, code) = rejection(&manifest);
    assert_eq!(field, "version");
    assert_eq!(code, "invalid_version");

    let manifest = MINIMAL.replace(
        "min_vanehub_version: \">=0.9.0\"",
        "min_vanehub_version: nonsense",
    );
    let (field, code) = rejection(&manifest);
    assert_eq!(field, "min_vanehub_version");
    assert_eq!(code, "invalid_version_requirement");
}

#[test]
fn a_contribution_path_is_validated_where_it_appears() {
    let (field, code) = rejection(&with(
        "contributes:\n  skills:\n    s:\n      path: /absolute/SKILL.md\n",
    ));

    assert_eq!(field, "contributes.skills.s.path");
    assert_eq!(code, "invalid_package_path");
}

#[test]
fn a_requested_origin_is_validated_and_canonicalized_at_decode() {
    let manifest =
        with("permissions:\n  network:\n    origins:\n      - \"HTTPS://API.GitHub.com:443\"\n");
    let VersionedExtensionManifest::V1(decoded) = decode(&manifest).expect("should decode");
    // Stored as the canonical origin, so the value the broker matches on cannot differ from the
    // one a reviewer approved by case or by a default port.
    assert_eq!(
        decoded.permissions.network_origins[0].as_str(),
        "https://api.github.com"
    );

    for (origins, why) in [
        ("      - \"https://*.github.com\"", "wildcard"),
        ("      - \"https://api.github.com/repos\"", "path"),
        ("      - \"https://user:token@api.github.com\"", "userinfo"),
        ("      - \"http://api.github.com\"", "plaintext remote"),
        ("      - \"file:///etc/passwd\"", "unsupported scheme"),
    ] {
        let (field, code) = rejection(&with(&format!(
            "permissions:\n  network:\n    origins:\n{origins}\n"
        )));
        assert_eq!(field, "permissions.network.origins", "for {why}");
        assert_eq!(code, "invalid_network_origin", "for {why}");
    }
}

#[test]
fn a_missing_required_field_names_it() {
    let manifest = MINIMAL.replace("publisher: acme\n", "");
    let (field, code) = rejection(&manifest);

    assert_eq!(field, "publisher");
    assert_eq!(code, "missing_field");
}

#[test]
fn an_activation_event_outside_the_catalog_is_refused() {
    let (field, code) = rejection(&with("activation_events:\n  - onSomething:x\n"));

    assert_eq!(field, "activation_events");
    assert_eq!(code, "invalid_identifier");
}

// ---------------------------------------------------------------------------
// Contribution specifics
// ---------------------------------------------------------------------------

#[test]
fn a_contributed_rule_asking_for_allow_is_refused_with_the_accepted_set() {
    let (field, code) = rejection(&with(
        "contributes:\n  authorization_rules:\n    r:\n      operation: git_operation\n      effect: allow\n      risk: low\n",
    ));

    assert_eq!(field, "contributes.authorization_rules.r.effect");
    assert_eq!(code, "unknown_value");
}

#[test]
fn a_hook_handler_kind_configured_locally_is_not_accepted_from_a_package() {
    // Command, HTTP, prompt, and Agent handlers are things an operator configures, not things a
    // downloaded package brings with it.
    for kind in ["command", "http", "prompt", "agent"] {
        let (field, code) = rejection(&with(&format!(
            "contributes:\n  hooks:\n    h:\n      event: e\n      handler:\n        kind: {kind}\n",
        )));
        assert_eq!(field, "contributes.hooks.h.handler.kind", "for {kind}");
        assert_eq!(code, "unknown_value", "for {kind}");
    }
}

#[test]
fn a_hook_defaults_to_failing_closed() {
    let manifest = with(
        "contributes:\n  hooks:\n    h:\n      event: e\n      handler:\n        kind: mcp-tool\n        tool: t\n",
    );
    let VersionedExtensionManifest::V1(decoded) = decode(&manifest).expect("should decode");

    assert_eq!(
        decoded.contributes.hooks[0].failure_mode,
        HookFailureMode::FailClosed
    );
    assert_eq!(decoded.contributes.hooks[0].priority, 0);
}

#[test]
fn an_mcp_transport_carries_key_names_and_never_values() {
    let manifest = with(
        "contributes:\n  mcp_definitions:\n    server:\n      display_name: Acme\n      transport:\n        kind: stdio\n        command: acme-server\n        args: [--stdio]\n        env_keys: [ACME_TOKEN]\n",
    );
    let VersionedExtensionManifest::V1(decoded) = decode(&manifest).expect("should decode");

    let McpTransportDeclaration::Stdio {
        command,
        args,
        env_keys,
    } = &decoded.contributes.mcp_definitions[0].transport
    else {
        panic!("expected a stdio transport");
    };
    assert_eq!(command, "acme-server");
    assert_eq!(args, &["--stdio".to_string()]);
    // A name, not a value. The manifest has no representation for a secret.
    assert_eq!(env_keys, &["ACME_TOKEN".to_string()]);
}

#[test]
fn an_unknown_mcp_transport_kind_is_refused() {
    let (field, code) = rejection(&with(
        "contributes:\n  mcp_definitions:\n    s:\n      display_name: S\n      transport:\n        kind: websocket\n        url: wss://example.com\n",
    ));

    assert_eq!(field, "contributes.mcp_definitions.s.transport.kind");
    assert_eq!(code, "unknown_value");
}

#[test]
fn a_matcher_accepts_a_single_value_or_a_list_and_refuses_a_mapping() {
    let manifest = with(
        "contributes:\n  hooks:\n    h:\n      event: e\n      matcher:\n        tool_ids: native.shell\n        risks: [high, critical]\n      handler:\n        kind: mcp-tool\n        tool: t\n",
    );
    let VersionedExtensionManifest::V1(decoded) = decode(&manifest).expect("should decode");

    let matcher = &decoded.contributes.hooks[0].matcher;
    assert_eq!(matcher.len(), 2);
    assert_eq!(
        matcher[0],
        ("tool_ids".to_string(), vec!["native.shell".to_string()])
    );
    assert_eq!(
        matcher[1],
        (
            "risks".to_string(),
            vec!["high".to_string(), "critical".to_string()]
        )
    );

    let (field, code) = rejection(&with(
        "contributes:\n  hooks:\n    h:\n      event: e\n      matcher:\n        nested:\n          deeper: x\n      handler:\n        kind: mcp-tool\n        tool: t\n",
    ));
    assert_eq!(field, "contributes.hooks.h.matcher.nested");
    assert_eq!(code, "expected_scalar_sequence");
}

// ---------------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------------

#[test]
fn a_collection_past_its_ceiling_names_the_collection() {
    let events = (0..=MAX_ACTIVATION_EVENTS)
        .map(|index| format!("  - onTool:t{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (field, code) = rejection(&with(&format!("activation_events:\n{events}\n")));

    assert_eq!(field, "activation_events");
    assert_eq!(code, "too_many_entries");
}

#[test]
fn a_contribution_collection_past_its_ceiling_names_the_kind() {
    // Bounds how many contributions one package can inject into the registry at once.
    let tools = (0..=MAX_CONTRIBUTIONS_PER_KIND)
        .map(|index| format!("    t{index}:\n      display_name: T\n      handler: h"))
        .collect::<Vec<_>>()
        .join("\n");
    let (field, code) = rejection(&with(&format!("contributes:\n  tools:\n{tools}\n")));

    assert_eq!(field, "contributes.tools");
    assert_eq!(code, "too_many_entries");
}

#[test]
fn a_rejection_carries_the_structured_reason_not_only_a_code() {
    // The declared version travels with the error, so a caller can report "this package targets
    // schema 2" rather than only "unsupported".
    let document = parse_block(
        &MINIMAL.replace("schema_version: 1", "schema_version: 7"),
        EXTENSION_MANIFEST_YAML_LIMITS,
    )
    .expect("fixture should parse");
    let error = ExtensionManifestV1Decoder::new(Version::parse("1.0.0").expect("version"))
        .decode(&document)
        .expect_err("should be rejected");

    assert_eq!(
        error.reason(),
        &DecodeReason::UnsupportedSchemaVersion { declared: 7 }
    );
    // The message names the field and what to do, not just a code.
    assert!(error.to_string().contains("schema_version"));
    assert!(error.to_string().contains('7'));
}

#[test]
fn the_manifest_profile_is_separate_from_the_skill_config_profile() {
    // Roomier where a manifest needs it, and a distinct value so that a later edit to one is
    // visibly not an edit to the other.
    assert_eq!(EXTENSION_MANIFEST_YAML_LIMITS.max_bytes, 64 * 1_024);
    assert_eq!(EXTENSION_MANIFEST_YAML_LIMITS.max_nodes, 2_048);
    assert_eq!(EXTENSION_MANIFEST_YAML_LIMITS.max_sequence_items, 64);
}
