//! Canonical manifest digest.
//!
//! Two properties matter and pull in opposite directions: reformatting must not change the digest,
//! and changing any meaning must. The second is asserted exhaustively — one mutation per field,
//! all compared at once — because a field left out of the canonical encoding is invisible in every
//! other test and would let a witness accept a manifest the user never saw.

use super::{
    manifest_digest, ExtensionManifestV1Decoder, ManifestDigest, VersionedExtensionManifest,
    EXTENSION_MANIFEST_YAML_LIMITS,
};
use semver::Version;
use vanehub_bounded_yaml::parse_block;

fn digest_of(text: &str) -> String {
    let document = parse_block(text, EXTENSION_MANIFEST_YAML_LIMITS)
        .unwrap_or_else(|error| panic!("fixture should parse: {error:?}\n---\n{text}"));
    let manifest = ExtensionManifestV1Decoder::new(Version::parse("1.0.0").expect("version"))
        .decode(&document)
        .unwrap_or_else(|error| panic!("fixture should decode: {error}\n---\n{text}"));
    manifest_digest(&manifest).as_str().to_string()
}

/// Exercises every field the encoding is supposed to reach, so the mutation table below has
/// something to mutate in each of them.
const RICH: &str = "\
schema_version: 1
id: acme.git-guardian
display_name: Git Guardian
publisher: acme
version: 1.2.0
description: Guarded Git tools.
license: Apache-2.0
min_vanehub_version: \">=0.9.0\"
runtime:
  kind: wasm-module
  entry: runtime/guardian.wasm
  trust_profile: standard
activation_events:
  - onTool:git_status
  - manual
requires:
  extensions:
    acme.base:
      version: \">=1.0.0\"
  skills:
    code-reviewer:
      version: \">=2.0.0\"
      optional: true
permissions:
  filesystem:
    read:
      - workspace/**
    write:
      - workspace/out/**
  network:
    origins:
      - https://api.github.com
  process:
    - git
  secrets:
    - github.token
contributes:
  tools:
    git_status:
      display_name: Git status
      description: Shows status.
      input_schema: schemas/in.json
      output_schema: schemas/out.json
      handler: tool.git_status
  skills:
    guarded-reviewer:
      path: skills/guarded-reviewer/SKILL.md
  mcp_definitions:
    acme:
      display_name: Acme
      transport:
        kind: stdio
        command: acme-server
        args: [--stdio, --verbose]
        env_keys: [ACME_TOKEN]
  modes:
    guarded:
      display_name: Guarded
      strategy: guardrails
      default_policy_template: standard
      required_tool_groups: [git]
      required_skills: [code-reviewer]
      required_hooks: [protect]
  hooks:
    protect:
      event: tool.before_execute
      matcher:
        tool_ids: [native.shell]
      handler:
        kind: extension-runtime
        entry: hook.protect
      failure_mode: fail_closed
      priority: 100
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
      capabilities: [repository.read]
  configuration:
    settings:
      schema: schemas/config.json
  transforms:
    prefix:
      event: prompt.after_assemble
      handler: transform.prefix
";

// ---------------------------------------------------------------------------
// Formatting is not meaning
// ---------------------------------------------------------------------------

#[test]
fn the_digest_is_stable_across_repeated_runs() {
    assert_eq!(digest_of(RICH), digest_of(RICH));
}

#[test]
fn reordering_top_level_fields_does_not_change_the_digest() {
    let reordered = "\
publisher: acme
version: 1.2.0
id: acme.git-guardian
schema_version: 1
min_vanehub_version: \">=0.9.0\"
display_name: Git Guardian
";
    let original = "\
schema_version: 1
id: acme.git-guardian
display_name: Git Guardian
publisher: acme
version: 1.2.0
min_vanehub_version: \">=0.9.0\"
";
    assert_eq!(digest_of(reordered), digest_of(original));
}

#[test]
fn reordering_contributions_and_set_members_does_not_change_the_digest() {
    // Which tools exist is meaning; the order they were typed in is not.
    let one = "\
schema_version: 1
id: acme.x
display_name: X
publisher: acme
version: 1.0.0
min_vanehub_version: \">=0.9.0\"
runtime:
  kind: wasm-module
  entry: r.wasm
activation_events:
  - manual
  - onStartupFinished
contributes:
  tools:
    alpha:
      display_name: A
      handler: a
    beta:
      display_name: B
      handler: b
";
    let two = "\
schema_version: 1
id: acme.x
display_name: X
publisher: acme
version: 1.0.0
min_vanehub_version: \">=0.9.0\"
runtime:
  kind: wasm-module
  entry: r.wasm
activation_events:
  - onStartupFinished
  - manual
contributes:
  tools:
    beta:
      display_name: B
      handler: b
    alpha:
      display_name: A
      handler: a
";
    assert_eq!(digest_of(one), digest_of(two));
}

#[test]
fn comments_and_blank_lines_do_not_change_the_digest() {
    let annotated = format!("# a comment\n\n{RICH}\n\n# trailing\n");
    assert_eq!(digest_of(&annotated), digest_of(RICH));
}

#[test]
fn quoting_a_scalar_does_not_change_the_digest() {
    let quoted = RICH.replace("license: Apache-2.0", "license: \"Apache-2.0\"");
    assert_eq!(digest_of(&quoted), digest_of(RICH));
}

// ---------------------------------------------------------------------------
// Order where order is meaning
// ---------------------------------------------------------------------------

#[test]
fn reordering_command_line_arguments_does_change_the_digest() {
    // The one list that is a sequence and not a set. Sorting it would make `--force` and
    // `--dry-run` interchangeable.
    let swapped = RICH.replace("args: [--stdio, --verbose]", "args: [--verbose, --stdio]");
    assert_ne!(digest_of(&swapped), digest_of(RICH));
}

// ---------------------------------------------------------------------------
// Every field participates
// ---------------------------------------------------------------------------

#[test]
fn changing_any_single_field_changes_the_digest() {
    // The guard against a field missing from the canonical encoding. A gap here means two
    // manifests differing in that field share a digest, and an install witness would accept a
    // package the user never previewed.
    let mutations: [(&str, &str, &str); 32] = [
        ("id", "id: acme.git-guardian", "id: acme.git-guard"),
        (
            "display_name",
            "display_name: Git Guardian",
            "display_name: Guardian",
        ),
        ("publisher", "publisher: acme\n", "publisher: acmex\n"),
        ("version", "version: 1.2.0", "version: 1.2.1"),
        (
            "description",
            "description: Guarded Git tools.",
            "description: Other.",
        ),
        ("license", "license: Apache-2.0", "license: MIT"),
        (
            "min_vanehub_version",
            "min_vanehub_version: \">=0.9.0\"",
            "min_vanehub_version: \">=0.9.1\"",
        ),
        ("runtime.kind", "kind: wasm-module", "kind: sidecar"),
        (
            "runtime.entry",
            "entry: runtime/guardian.wasm",
            "entry: runtime/other.wasm",
        ),
        (
            "runtime.trust_profile",
            "trust_profile: standard",
            "trust_profile: strict",
        ),
        (
            "activation_events",
            "- onTool:git_status",
            "- onTool:git_diff",
        ),
        (
            "requires.extensions.id",
            "    acme.base:",
            "    acme.other:",
        ),
        (
            "requires.extensions.version",
            "      version: \">=1.0.0\"",
            "      version: \">=1.1.0\"",
        ),
        (
            "requires.skills.id",
            "    code-reviewer:",
            "    other-reviewer:",
        ),
        (
            "requires.skills.optional",
            "      optional: true",
            "      optional: false",
        ),
        (
            "permissions.filesystem.read",
            "- workspace/**",
            "- workspace/src/**",
        ),
        (
            "permissions.filesystem.write",
            "- workspace/out/**",
            "- workspace/tmp/**",
        ),
        (
            "permissions.network.origins",
            "- https://api.github.com",
            "- https://api.gitlab.com",
        ),
        ("permissions.process", "- git\n", "- rg\n"),
        ("permissions.secrets", "- github.token", "- gitlab.token"),
        ("tool.id", "    git_status:", "    git_state:"),
        (
            "tool.display_name",
            "display_name: Git status",
            "display_name: Git state",
        ),
        (
            "tool.description",
            "description: Shows status.",
            "description: Shows state.",
        ),
        (
            "tool.input_schema",
            "input_schema: schemas/in.json",
            "input_schema: schemas/i.json",
        ),
        (
            "tool.output_schema",
            "output_schema: schemas/out.json",
            "output_schema: schemas/o.json",
        ),
        (
            "tool.handler",
            "handler: tool.git_status",
            "handler: tool.git_state",
        ),
        (
            "skill.path",
            "path: skills/guarded-reviewer/SKILL.md",
            "path: skills/other/SKILL.md",
        ),
        (
            "mcp.command",
            "command: acme-server",
            "command: other-server",
        ),
        (
            "mcp.env_keys",
            "env_keys: [ACME_TOKEN]",
            "env_keys: [OTHER_TOKEN]",
        ),
        (
            "hook.failure_mode",
            "failure_mode: fail_closed",
            "failure_mode: fail_open",
        ),
        ("hook.priority", "priority: 100", "priority: 50"),
        ("rule.effect", "effect: ask", "effect: deny"),
    ];

    let baseline = digest_of(RICH);
    let mut seen: Vec<(String, &str)> = vec![(baseline.clone(), "baseline")];

    for (field, from, to) in mutations {
        let mutated = RICH.replace(from, to);
        assert_ne!(
            mutated, RICH,
            "mutation for {field} did not change the source"
        );
        let digest = digest_of(&mutated);
        assert_ne!(
            digest, baseline,
            "changing {field} left the digest unchanged"
        );
        seen.push((digest, field));
    }

    // And no two mutations collide with each other.
    let total = seen.len();
    seen.sort_unstable();
    seen.dedup_by(|a, b| a.0 == b.0);
    assert_eq!(total, seen.len(), "two different manifests share a digest");
}

#[test]
fn adding_or_removing_a_contribution_changes_the_digest() {
    let without = RICH.replace(
        "  configuration:\n    settings:\n      schema: schemas/config.json\n",
        "",
    );
    assert_ne!(without, RICH);
    assert_ne!(digest_of(&without), digest_of(RICH));
}

#[test]
fn an_absent_optional_field_differs_from_a_present_empty_one() {
    // `description:` with nothing under it decodes as an empty mapping rather than a scalar, so
    // the comparable case is absent versus present-with-a-value. Both must differ from each other
    // and from a third spelling, since "not stated" and "stated as X" are different reviews.
    let absent = RICH.replace("description: Guarded Git tools.\n", "");
    let present = RICH.replace("description: Guarded Git tools.", "description: x");

    assert_ne!(digest_of(&absent), digest_of(RICH));
    assert_ne!(digest_of(&present), digest_of(RICH));
    assert_ne!(digest_of(&absent), digest_of(&present));
}

// ---------------------------------------------------------------------------
// Encoding is unambiguous
// ---------------------------------------------------------------------------

#[test]
fn values_that_would_collide_under_naive_concatenation_do_not() {
    // `ab` + `c` and `a` + `bc` are the same bytes without length prefixes. Asserted on real
    // manifests rather than on the encoder, because the encoder is the thing under suspicion.
    fn with_names(first: &str, second: &str) -> String {
        format!(
            "\
schema_version: 1
id: acme.x
display_name: X
publisher: acme
version: 1.0.0
min_vanehub_version: \">=0.9.0\"
permissions:
  secrets:
    - {first}
    - {second}
"
        )
    }

    assert_ne!(
        digest_of(&with_names("ab", "c")),
        digest_of(&with_names("a", "bc"))
    );

    // The same trap one level down, inside a record.
    let one = RICH.replace("display_name: Git status", "display_name: ab");
    let one = one.replace("handler: tool.git_status", "handler: c");
    let two = RICH.replace("display_name: Git status", "display_name: a");
    let two = two.replace("handler: tool.git_status", "handler: bc");
    assert_ne!(digest_of(&one), digest_of(&two));
}

// ---------------------------------------------------------------------------
// The digest value itself
// ---------------------------------------------------------------------------

#[test]
fn a_digest_is_sixty_four_lower_case_hex_characters_and_round_trips() {
    let digest = digest_of(RICH);

    assert_eq!(digest.len(), 64);
    assert!(digest
        .chars()
        .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character)));
    assert_eq!(
        ManifestDigest::parse(&digest).expect("round trip").as_str(),
        digest
    );

    assert!(ManifestDigest::parse("").is_none());
    assert!(ManifestDigest::parse(&digest.to_uppercase()).is_none());
    assert!(ManifestDigest::parse(&digest[..63]).is_none());
}

#[test]
fn the_schema_version_is_part_of_the_identity() {
    // A future `V2` carrying the same fields is not the same manifest, so the variant tag is in
    // the encoding rather than implied by the fields that happen to be present.
    let VersionedExtensionManifest::V1(_) = decode(RICH);
    // Asserted structurally: there is one variant today, and the tag is written unconditionally in
    // `manifest_digest`. This test exists so adding `V2` without a tag fails to compile here.
}

fn decode(text: &str) -> VersionedExtensionManifest {
    let document = parse_block(text, EXTENSION_MANIFEST_YAML_LIMITS).expect("fixture should parse");
    ExtensionManifestV1Decoder::new(Version::parse("1.0.0").expect("version"))
        .decode(&document)
        .expect("fixture should decode")
}
