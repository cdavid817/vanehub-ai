//! Contract tests for `scripts/tools.json`.
//!
//! Every case reads the same JSON fixtures a Skill package would ship, so the checked-in contract
//! and the parser cannot drift apart the way an inline literal quietly can.

use super::{
    parse_manifest_bytes, BoundedSchemaError, DeclarativeFieldSource, SkillToolCapability,
    SkillToolDomainError, SkillToolImplementation, SkillToolManifest, DEFAULT_MANIFEST_LIMITS,
    DEFAULT_SKILL_TOOL_LIMITS, SUPPORTED_MANIFEST_VERSION,
};

const VALID_DECLARATIVE: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/valid-declarative.json");
const VALID_MODULE: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/valid-module.json");
const UNKNOWN_VERSION: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/adversarial/unknown-version.json");
const DUPLICATE_TOOL_IDS: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/adversarial/duplicate-tool-ids.json");
const TRAVERSAL_MODULE_PATH: &str = include_str!(
    "../../../../../tests/fixtures/skill-tools/adversarial/traversal-module-path.json"
);
const FORBIDDEN_KIND: &str = include_str!(
    "../../../../../tests/fixtures/skill-tools/adversarial/forbidden-implementation-kind.json"
);
const RECURSIVE_SCHEMA: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/adversarial/recursive-schema.json");
const OVERSIZED_SCHEMA: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/adversarial/oversized-schema.json");
const DEEP_SCHEMA: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/adversarial/deep-schema.json");
const LOOSENED_LIMITS: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/adversarial/loosened-limits.json");
const UNDECLARED_CAPABILITY: &str = include_str!(
    "../../../../../tests/fixtures/skill-tools/adversarial/undeclared-capability.json"
);
const TEMPLATE_EXPRESSION: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/adversarial/template-expression.json");
const HASH_MISMATCH: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/adversarial/hash-mismatch.json");

fn parse(fixture: &str) -> Result<SkillToolManifest, SkillToolDomainError> {
    parse_manifest_bytes(fixture.as_bytes(), &DEFAULT_MANIFEST_LIMITS)
}

#[test]
fn the_declarative_fixture_parses_into_normalized_ids_schemas_and_tightened_limits() {
    let manifest = parse(VALID_DECLARATIVE).expect("declarative manifest");
    assert_eq!(manifest.version, SUPPORTED_MANIFEST_VERSION);
    assert_eq!(manifest.owner.as_str(), "code-review");
    assert_eq!(
        manifest
            .tools
            .iter()
            .map(|tool| tool.id.as_str())
            .collect::<Vec<_>>(),
        vec!["diff-summary", "checklist"]
    );

    let diff = &manifest.tools[0];
    assert_eq!(diff.limits.wall_time_milliseconds, 3_000);
    assert_eq!(diff.limits.host_calls, 2);
    assert_eq!(
        diff.limits.memory_bytes,
        DEFAULT_SKILL_TOOL_LIMITS.memory_bytes
    );
    assert_eq!(
        diff.capabilities,
        vec![SkillToolCapability::Tool("read_file".to_string())]
    );
    assert_eq!(diff.input.metrics().depth, 2);

    let SkillToolImplementation::Declarative(declarative) = &diff.implementation else {
        panic!("expected a declarative implementation");
    };
    assert_eq!(declarative.target.operation(), "read_file");
    assert_eq!(
        declarative
            .template
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        vec!["encoding", "path"]
    );
    assert_eq!(
        declarative.template[0].source,
        DeclarativeFieldSource::Constant(serde_json::json!("utf-8"))
    );
    assert_eq!(
        declarative.template[1].source,
        DeclarativeFieldSource::Input("path".to_string())
    );
    assert!(manifest.declared_implementation_paths().is_empty());
}

#[test]
fn the_module_fixture_binds_a_contained_path_export_and_hash() {
    let manifest = parse(VALID_MODULE).expect("module manifest");
    let tool = &manifest.tools[0];
    assert_eq!(tool.limits.memory_bytes, 8 * 1024 * 1024);
    assert_eq!(tool.limits.fuel, 1_000_000);
    assert_eq!(tool.implementation.kind(), "wasm");

    let SkillToolImplementation::Module(module) = &tool.implementation else {
        panic!("expected a module implementation");
    };
    assert_eq!(module.path, "scripts/modules/complexity.wasm");
    assert_eq!(module.export, "score");
    assert!(module.content_hash.as_str().starts_with("sha256:"));
    assert_eq!(
        manifest.declared_implementation_paths(),
        vec!["scripts/modules/complexity.wasm".to_string()]
    );
}

#[test]
fn an_unknown_manifest_version_is_refused_instead_of_best_effort_parsed() {
    assert_eq!(
        parse(UNKNOWN_VERSION),
        Err(SkillToolDomainError::UnsupportedManifestVersion {
            supported: SUPPORTED_MANIFEST_VERSION,
            actual: 2
        })
    );
}

#[test]
fn duplicate_tool_ids_fail_closed_rather_than_letting_the_last_entry_win() {
    assert_eq!(
        parse(DUPLICATE_TOOL_IDS),
        Err(SkillToolDomainError::DuplicateToolId(
            "diff-summary".to_string()
        ))
    );
}

#[test]
fn a_traversing_module_path_is_rejected_before_any_filesystem_access() {
    assert!(matches!(
        parse(TRAVERSAL_MODULE_PATH),
        Err(SkillToolDomainError::UnsafeImplementationPath(_))
    ));
    for path in [
        "modules/x.wasm",
        "scripts/modules/x.py",
        "scripts/modules/nested/x.wasm",
        "scripts/modules/.hidden.wasm",
        "scripts/modules/C:x.wasm",
        "scripts/modules/x%2e.wasm",
        "scripts/modules/.wasm",
        "scripts/modules/UPPER.wasm",
    ] {
        let manifest = VALID_MODULE.replace("scripts/modules/complexity.wasm", path);
        assert!(
            matches!(
                parse(&manifest),
                Err(SkillToolDomainError::UnsafeImplementationPath(_))
            ),
            "{path} must be refused"
        );
    }
}

#[test]
fn host_scripts_and_native_binaries_are_refused_as_implementation_kinds() {
    assert_eq!(
        parse(FORBIDDEN_KIND),
        Err(SkillToolDomainError::ForbiddenImplementationKind(
            "python".to_string()
        ))
    );
    for kind in ["Shell", "POWERSHELL", "batch", "node", "exec", "dylib"] {
        let manifest =
            FORBIDDEN_KIND.replace("\"kind\": \"python\"", &format!("\"kind\": \"{kind}\""));
        assert_eq!(
            parse(&manifest),
            Err(SkillToolDomainError::ForbiddenImplementationKind(
                kind.to_ascii_lowercase()
            ))
        );
    }
    let unknown = FORBIDDEN_KIND.replace("\"kind\": \"python\"", "\"kind\": \"quantum\"");
    assert_eq!(
        parse(&unknown),
        Err(SkillToolDomainError::UnknownImplementationKind(
            "quantum".to_string()
        ))
    );
}

#[test]
fn schema_budgets_reject_recursive_oversized_and_deep_declarations() {
    assert_eq!(
        parse(RECURSIVE_SCHEMA),
        Err(SkillToolDomainError::Schema(
            BoundedSchemaError::UnsupportedCombinator("$ref".to_string())
        ))
    );
    assert!(matches!(
        parse(OVERSIZED_SCHEMA),
        Err(SkillToolDomainError::Schema(
            BoundedSchemaError::TooManyProperties { maximum: 32, .. }
        ))
    ));
    assert!(matches!(
        parse(DEEP_SCHEMA),
        Err(SkillToolDomainError::Schema(BoundedSchemaError::TooDeep {
            maximum: 6,
            ..
        }))
    ));
}

#[test]
fn a_manifest_may_not_raise_a_central_ceiling() {
    assert_eq!(
        parse(LOOSENED_LIMITS),
        Err(SkillToolDomainError::LimitAboveCeiling {
            limit: "wallTimeMs"
        })
    );
}

#[test]
fn a_declarative_target_must_be_covered_by_a_declared_capability() {
    assert_eq!(
        parse(UNDECLARED_CAPABILITY),
        Err(SkillToolDomainError::UndeclaredCapability(
            "tool:run_command".to_string()
        ))
    );
}

#[test]
fn templates_reject_conditionals_expressions_and_unprojected_inputs() {
    assert!(matches!(
        parse(TEMPLATE_EXPRESSION),
        Err(SkillToolDomainError::InvalidTemplate(_))
    ));
    for template in [
        r#"{ "path": "${input.path}" }"#,
        r#"{ "path": { "$each": "path" } }"#,
        r#"{ "path": { "$shell": "cat $PATH" } }"#,
        r#"{ "path": { "$input": "undeclared" } }"#,
        r#"{ "path": { "$input": 7 } }"#,
        r#"{ "path": [] }"#,
    ] {
        let manifest = VALID_DECLARATIVE.replace(
            "{\n          \"path\": { \"$input\": \"path\" },\n          \"encoding\": { \"$const\": \"utf-8\" }\n        }",
            template,
        );
        assert!(
            matches!(
                parse(&manifest),
                Err(SkillToolDomainError::InvalidTemplate(_))
            ),
            "{template} must be refused"
        );
    }
}

#[test]
fn a_malformed_hash_is_refused_and_a_well_formed_one_is_only_a_claim() {
    // A syntactically valid hash parses; whether the bytes match is integrity verification's job,
    // which is why this fixture is usable input for the discovery-side mismatch test.
    let manifest = parse(HASH_MISMATCH).expect("well-formed hash claim");
    assert_eq!(manifest.tools.len(), 1);

    for hash in ["md5:abc", "sha256:xyz", "sha256:", "abc123"] {
        let broken = HASH_MISMATCH.replace(
            "sha256:5555555555555555555555555555555555555555555555555555555555555555",
            hash,
        );
        assert!(
            matches!(
                parse(&broken),
                Err(SkillToolDomainError::InvalidContentHash(_))
            ),
            "{hash} must be refused"
        );
    }
}

#[test]
fn structural_manifest_failures_are_reported_by_field() {
    assert_eq!(
        parse_manifest_bytes(b"not json", &DEFAULT_MANIFEST_LIMITS),
        Err(SkillToolDomainError::ManifestNotAnObject)
    );
    assert_eq!(
        parse_manifest_bytes(b"[]", &DEFAULT_MANIFEST_LIMITS),
        Err(SkillToolDomainError::ManifestNotAnObject)
    );
    assert_eq!(
        parse(r#"{ "version": 1, "skillId": "code-review", "tools": [] }"#),
        Err(SkillToolDomainError::EmptyToolList)
    );
    assert_eq!(
        parse(r#"{ "version": 1, "tools": [] }"#),
        Err(SkillToolDomainError::MissingField("skillId"))
    );
    assert_eq!(
        parse(r#"{ "version": 1, "skillId": "code-review", "tools": [], "extra": true }"#),
        Err(SkillToolDomainError::UnknownField("extra".to_string()))
    );
    assert_eq!(
        parse(r#"{ "version": 1, "skillId": "Code Review", "tools": [] }"#),
        Err(SkillToolDomainError::InvalidOwnerId(
            "Code Review".to_string()
        ))
    );

    let oversized = vec![b' '; DEFAULT_MANIFEST_LIMITS.maximum_manifest_bytes as usize + 1];
    assert!(matches!(
        parse_manifest_bytes(&oversized, &DEFAULT_MANIFEST_LIMITS),
        Err(SkillToolDomainError::TooLarge { .. })
    ));
}

#[test]
fn tool_counts_capabilities_and_descriptions_stay_inside_the_contract_budget() {
    let entry = |index: usize| {
        format!(
            r#"{{ "id": "tool-{index}", "description": "Tool {index}", "capabilities": [],
                  "input": {{ "type": "object", "properties": {{}} }},
                  "output": {{ "type": "object" }},
                  "implementation": {{ "kind": "wasm", "module": "scripts/modules/m{index}.wasm",
                    "export": "run", "hash": "sha256:{}" }} }}"#,
            "8".repeat(64)
        )
    };
    let entries = (0..=DEFAULT_MANIFEST_LIMITS.maximum_tools)
        .map(entry)
        .collect::<Vec<_>>()
        .join(",");
    assert!(matches!(
        parse(&format!(
            r#"{{ "version": 1, "skillId": "code-review", "tools": [{entries}] }}"#
        )),
        Err(SkillToolDomainError::TooManyTools { maximum: 16, .. })
    ));

    let capabilities = (0..=DEFAULT_MANIFEST_LIMITS.maximum_capabilities)
        .map(|index| format!("\"tool:op_{index}\""))
        .collect::<Vec<_>>()
        .join(",");
    let too_many = VALID_MODULE.replace(
        "\"capabilities\": []",
        &format!("\"capabilities\": [{capabilities}]"),
    );
    assert!(matches!(
        parse(&too_many),
        Err(SkillToolDomainError::TooManyCapabilities { maximum: 8, .. })
    ));

    let duplicated = VALID_MODULE.replace(
        "\"capabilities\": []",
        "\"capabilities\": [\"tool:read_file\", \"tool:read_file\"]",
    );
    assert_eq!(
        parse(&duplicated),
        Err(SkillToolDomainError::DuplicateCapability(
            "tool:read_file".to_string()
        ))
    );

    let malformed = VALID_MODULE.replace("\"capabilities\": []", "\"capabilities\": [\"fs:read\"]");
    assert_eq!(
        parse(&malformed),
        Err(SkillToolDomainError::UnknownCapability(
            "fs:read".to_string()
        ))
    );

    let long_description = VALID_MODULE.replace(
        "Scores a function body without touching the host.",
        &"x".repeat(DEFAULT_MANIFEST_LIMITS.maximum_description_characters + 1),
    );
    assert!(matches!(
        parse(&long_description),
        Err(SkillToolDomainError::DescriptionTooLong { maximum: 512, .. })
    ));
}
