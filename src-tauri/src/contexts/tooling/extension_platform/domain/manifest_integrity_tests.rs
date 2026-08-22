//! Whole-manifest integrity checks.
//!
//! Driven through the real parser and decoder, so a fixture that could not exist as a manifest
//! cannot be asserted about. Each test names the relationship being broken rather than the field,
//! because these are the failures a per-field decoder structurally cannot see.

use super::{
    check_integrity, global_ids, ExtensionManifestV1, ExtensionManifestV1Decoder, IntegrityReason,
    IntegrityViolation, VersionedExtensionManifest, EXTENSION_MANIFEST_YAML_LIMITS,
};
use semver::Version;
use vanehub_bounded_yaml::parse_block;

const HEADER: &str = "\
schema_version: 1
id: acme.git-guardian
display_name: Git Guardian
publisher: acme
version: 1.2.0
min_vanehub_version: \">=0.9.0\"
";

const WASM_RUNTIME: &str = "\
runtime:
  kind: wasm-module
  entry: runtime/guardian.wasm
";

fn manifest(extra: &str) -> ExtensionManifestV1 {
    let text = format!("{HEADER}{extra}");
    let document = parse_block(&text, EXTENSION_MANIFEST_YAML_LIMITS)
        .unwrap_or_else(|error| panic!("fixture should parse: {error:?}\n---\n{text}"));
    let VersionedExtensionManifest::V1(decoded) =
        ExtensionManifestV1Decoder::new(Version::parse("1.0.0").expect("version"))
            .decode(&document)
            .unwrap_or_else(|error| panic!("fixture should decode: {error}\n---\n{text}"));
    decoded
}

fn reasons(extra: &str) -> Vec<IntegrityReason> {
    check_integrity(&manifest(extra))
        .into_iter()
        .map(|violation| violation.reason)
        .collect()
}

// ---------------------------------------------------------------------------
// Clean manifests
// ---------------------------------------------------------------------------

#[test]
fn a_manifest_whose_references_all_resolve_has_no_violations() {
    let violations = check_integrity(&manifest(&format!(
        "{WASM_RUNTIME}\
activation_events:
  - onTool:git_status
  - onConnector:github
contributes:
  tools:
    git_status:
      display_name: Git status
      handler: tool.git_status
  connectors:
    github:
      display_name: GitHub
      type: cli
      driver: connector.github
      auth_strategy: external-cli
  mcp_definitions:
    acme:
      display_name: Acme
      transport:
        kind: stdio
        command: acme-server
  hooks:
    guard:
      event: tool.before_execute
      handler:
        kind: mcp-tool
        tool: acme/review
  modes:
    guarded:
      display_name: Guarded
      strategy: guardrails
      required_hooks: [guard]
"
    )));

    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn a_data_only_manifest_with_no_handlers_has_no_violations() {
    let violations = check_integrity(&manifest(
        "contributes:\n  skills:\n    reviewer:\n      path: skills/reviewer/SKILL.md\n",
    ));

    assert!(violations.is_empty(), "{violations:?}");
}

// ---------------------------------------------------------------------------
// Cross-contribution references
// ---------------------------------------------------------------------------

#[test]
fn a_mode_requiring_a_hook_the_manifest_does_not_contribute_is_reported() {
    let violations: Vec<IntegrityViolation> = check_integrity(&manifest(
        "contributes:\n  modes:\n    guarded:\n      display_name: Guarded\n      strategy: guardrails\n      required_hooks: [absent]\n",
    ));

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].reason, IntegrityReason::UnknownHookReference);
    assert_eq!(
        violations[0].field,
        "contributes.modes.guarded.required_hooks"
    );
    assert_eq!(violations[0].value, "absent");
}

#[test]
fn a_hook_routing_to_an_undeclared_mcp_server_is_reported() {
    let violations = check_integrity(&manifest(
        "contributes:\n  hooks:\n    guard:\n      event: e\n      handler:\n        kind: mcp-tool\n        tool: absent/review\n",
    ));

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].reason, IntegrityReason::UnknownMcpReference);
    assert_eq!(violations[0].field, "contributes.hooks.guard.handler.tool");
}

#[test]
fn a_hook_routing_to_a_declared_mcp_server_is_accepted() {
    let violations = check_integrity(&manifest(
        "contributes:\n  mcp_definitions:\n    acme:\n      display_name: Acme\n      transport:\n        kind: stdio\n        command: acme-server\n  hooks:\n    guard:\n      event: e\n      handler:\n        kind: mcp-tool\n        tool: acme/review\n",
    ));

    assert!(violations.is_empty(), "{violations:?}");
}

// ---------------------------------------------------------------------------
// Handlers without a runtime
// ---------------------------------------------------------------------------

#[test]
fn a_handler_declared_without_a_runtime_is_reported_for_every_kind_that_needs_one() {
    // Installed, indexed, offered to the model, and then failing at the first call. Caught at
    // decode rather than at invocation.
    let found = reasons(
        "contributes:\n  tools:\n    t:\n      display_name: T\n      handler: tool.t\n  hooks:\n    h:\n      event: e\n      handler:\n        kind: extension-runtime\n        entry: hook.h\n  transforms:\n    x:\n      event: e\n      handler: transform.x\n",
    );

    assert_eq!(found.len(), 3);
    assert!(found
        .iter()
        .all(|reason| *reason == IntegrityReason::HandlerWithoutRuntime));
}

#[test]
fn the_same_handlers_are_accepted_once_a_runtime_is_declared() {
    let violations = check_integrity(&manifest(&format!(
        "{WASM_RUNTIME}contributes:\n  tools:\n    t:\n      display_name: T\n      handler: tool.t\n  transforms:\n    x:\n      event: e\n      handler: transform.x\n"
    )));

    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn an_mcp_backed_hook_needs_no_runtime_of_its_own() {
    // It calls a server, not the extension's own code, so a data-only package may contribute one.
    let violations = check_integrity(&manifest(
        "contributes:\n  mcp_definitions:\n    acme:\n      display_name: Acme\n      transport:\n        kind: stdio\n        command: acme-server\n  hooks:\n    h:\n      event: e\n      handler:\n        kind: mcp-tool\n        tool: acme/review\n",
    ));

    assert!(violations.is_empty(), "{violations:?}");
}

// ---------------------------------------------------------------------------
// Activation events
// ---------------------------------------------------------------------------

#[test]
fn an_activation_event_naming_an_undeclared_tool_or_connector_is_reported() {
    let found = reasons(&format!(
        "{WASM_RUNTIME}activation_events:\n  - onTool:absent\n  - onConnector:absent\n"
    ));

    assert_eq!(found.len(), 2);
    assert!(found
        .iter()
        .all(|reason| *reason == IntegrityReason::UnreachableActivationEvent));
}

#[test]
fn activation_events_naming_host_concepts_are_left_to_the_host() {
    // `onHook:<event-id>`, `onAgentMode:<mode-id>`, and `onCommand:<id>` name things the
    // application registers. Resolving them here would reject a correct manifest.
    let violations = check_integrity(&manifest(
        "activation_events:\n  - onHook:tool.before_execute\n  - onAgentMode:guardrails\n  - onCommand:vanehub.review\n  - onStartupFinished\n  - manual\n",
    ));

    assert!(violations.is_empty(), "{violations:?}");
}

// ---------------------------------------------------------------------------
// Path collisions
// ---------------------------------------------------------------------------

#[test]
fn two_declared_paths_differing_only_by_case_are_reported() {
    // Two entries, one file on macOS and Windows.
    let violations = check_integrity(&manifest(
        "contributes:\n  skills:\n    a:\n      path: skills/Reviewer.md\n    b:\n      path: skills/reviewer.md\n",
    ));

    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0].reason,
        IntegrityReason::CaseInsensitivePathCollision { .. }
    ));
}

#[test]
fn two_declared_paths_differing_only_by_unicode_composition_are_reported() {
    // `é` as one code point versus `e` plus a combining accent. Distinct bytes, one file after
    // macOS normalizes on write.
    let precomposed = "caf\u{e9}.md";
    let decomposed = "cafe\u{301}.md";
    assert_ne!(precomposed, decomposed);

    let violations = check_integrity(&manifest(&format!(
        "contributes:\n  configuration:\n    a:\n      schema: {precomposed}\n    b:\n      schema: {decomposed}\n"
    )));

    assert_eq!(violations.len(), 1, "{violations:?}");
    assert!(matches!(
        violations[0].reason,
        IntegrityReason::UnicodePathCollision { .. }
    ));
}

#[test]
fn a_case_collision_is_not_also_reported_as_a_unicode_collision() {
    // Both normalizations fold the pair together; reporting twice would make one mistake look
    // like two and leave a publisher unsure what to change.
    let violations = check_integrity(&manifest(
        "contributes:\n  skills:\n    a:\n      path: skills/A.md\n    b:\n      path: skills/a.md\n",
    ));

    assert_eq!(violations.len(), 1);
}

#[test]
fn distinct_paths_do_not_collide() {
    let violations = check_integrity(&manifest(&format!(
        "{WASM_RUNTIME}contributes:\n  skills:\n    a:\n      path: skills/one.md\n    b:\n      path: skills/two.md\n  configuration:\n    c:\n      schema: schemas/config.json\n"
    )));

    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn the_runtime_entry_participates_in_collision_detection() {
    let violations = check_integrity(&manifest(
        "runtime:\n  kind: wasm-module\n  entry: shared/Thing.wasm\ncontributes:\n  configuration:\n    c:\n      schema: shared/thing.wasm\n",
    ));

    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0].reason,
        IntegrityReason::CaseInsensitivePathCollision { .. }
    ));
}

// ---------------------------------------------------------------------------
// Global ids
// ---------------------------------------------------------------------------

#[test]
fn global_ids_are_derived_for_every_contribution_and_are_distinct() {
    let decoded = manifest(&format!(
        "{WASM_RUNTIME}contributes:\n  tools:\n    shared:\n      display_name: T\n      handler: h\n  skills:\n    shared:\n      path: skills/s.md\n"
    ));
    let ids: Vec<String> = global_ids(&decoded)
        .iter()
        .map(|id| id.as_str().to_string())
        .collect();

    // Same local id under two kinds, distinct once namespaced — which is why the kind is part of
    // the global id rather than only the extension.
    assert_eq!(
        ids,
        vec![
            "ext::acme.git-guardian::tool::shared",
            "ext::acme.git-guardian::skill::shared",
        ]
    );
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

#[test]
fn every_violation_is_reported_rather_than_only_the_first() {
    // A publisher fixing a manifest wants the list. Nothing here executes anything to find the
    // next one, so stopping early buys no safety and costs a round trip per mistake.
    let found = reasons(
        "contributes:\n  modes:\n    m:\n      display_name: M\n      strategy: s\n      required_hooks: [absent]\n  tools:\n    t:\n      display_name: T\n      handler: h\n",
    );

    assert_eq!(found.len(), 2);
    assert!(found.contains(&IntegrityReason::UnknownHookReference));
    assert!(found.contains(&IntegrityReason::HandlerWithoutRuntime));
}

#[test]
fn every_integrity_reason_has_a_distinct_code() {
    let reasons = [
        IntegrityReason::UnknownHookReference,
        IntegrityReason::UnknownMcpReference,
        IntegrityReason::CaseInsensitivePathCollision {
            other: String::new(),
        },
        IntegrityReason::UnicodePathCollision {
            other: String::new(),
        },
        IntegrityReason::HandlerWithoutRuntime,
        IntegrityReason::UnreachableActivationEvent,
    ];

    let mut codes: Vec<&str> = reasons.iter().map(IntegrityReason::code).collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}
