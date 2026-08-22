//! Invariants of the manifest structure itself, independent of how it is decoded.
//!
//! These assert the things a decoder is entitled to rely on: which runtime kinds an external
//! package may name, that a contributed rule cannot express Allow, and that duplicate detection
//! sees every kind rather than whichever ones a later edit remembered to add.

use super::{
    ActivationEvent, AuthorizationRuleContribution, CapabilityRequest, ConfigurationContribution,
    ConnectorContribution, ContributedRuleEffect, ContributionKind, ContributionLocalId,
    ContributionManifest, ExtensionDependency, ExtensionId, ExtensionManifestV1,
    ExtensionRequirements, HookContribution, HookFailureMode, HookHandlerDeclaration,
    McpContribution, McpTransportDeclaration, ModePresetContribution, PortablePackagePath,
    PublisherId, RuntimeDeclaration, RuntimeKind, SkillContribution, SkillDependency,
    ToolContribution, TransformContribution, TrustProfile, VersionedExtensionManifest,
    ALL_CONTRIBUTION_KINDS, SUPPORTED_SCHEMA_VERSIONS,
};
use semver::{Version, VersionReq};

fn local(value: &str) -> ContributionLocalId {
    ContributionLocalId::parse(value).expect("valid local id")
}

fn path(value: &str) -> PortablePackagePath {
    PortablePackagePath::parse(value).expect("valid path")
}

/// One contribution of every kind, so a change that forgets a kind fails here rather than in an
/// adapter months later.
fn every_kind() -> ContributionManifest {
    ContributionManifest {
        tools: vec![ToolContribution {
            id: local("git_status"),
            display_name: "Git status".to_string(),
            description: None,
            input_schema: Some(path("schemas/in.json")),
            output_schema: Some(path("schemas/out.json")),
            handler: "tool.git_status".to_string(),
        }],
        skills: vec![SkillContribution {
            id: local("guarded-reviewer"),
            path: path("skills/guarded-reviewer/SKILL.md"),
        }],
        mcp_definitions: vec![McpContribution {
            id: local("acme-mcp"),
            display_name: "Acme".to_string(),
            transport: McpTransportDeclaration::Stdio {
                command: "acme-server".to_string(),
                args: vec!["--stdio".to_string()],
                env_keys: vec!["ACME_TOKEN".to_string()],
            },
        }],
        modes: vec![ModePresetContribution {
            id: local("guarded"),
            display_name: "Guarded".to_string(),
            strategy: "guardrails".to_string(),
            default_policy_template: Some("standard".to_string()),
            required_tool_groups: Vec::new(),
            required_skills: Vec::new(),
            required_hooks: vec![local("protect-force-push")],
        }],
        hooks: vec![HookContribution {
            id: local("protect-force-push"),
            event: "tool.before_execute".to_string(),
            matcher: vec![("tool_ids".to_string(), vec!["native.shell".to_string()])],
            handler: HookHandlerDeclaration::ExtensionRuntime {
                entry: "hook.protect_force_push".to_string(),
            },
            failure_mode: HookFailureMode::FailClosed,
            priority: 100,
        }],
        authorization_rules: vec![AuthorizationRuleContribution {
            id: local("force-push-ask"),
            operation: "git_operation".to_string(),
            matcher: vec![("command_regex".to_string(), vec![r"git\s+push".to_string()])],
            effect: ContributedRuleEffect::Ask,
            risk: "critical".to_string(),
            allowed_scopes: vec!["once".to_string()],
        }],
        connectors: vec![ConnectorContribution {
            id: local("github"),
            display_name: "GitHub".to_string(),
            connector_type: "cli".to_string(),
            driver: "connector.github".to_string(),
            auth_strategy: "external-cli".to_string(),
            capabilities: vec!["repository.read".to_string()],
        }],
        configuration: vec![ConfigurationContribution {
            id: local("settings"),
            schema: path("schemas/config.json"),
        }],
        transforms: vec![TransformContribution {
            id: local("prefix"),
            event: "prompt.after_assemble".to_string(),
            handler: "transform.prefix".to_string(),
        }],
    }
}

#[test]
fn declared_ids_reach_every_contribution_kind() {
    let manifest = every_kind();
    let declared = manifest.declared_ids();

    assert_eq!(declared.len(), ALL_CONTRIBUTION_KINDS.len());
    assert_eq!(manifest.total(), ALL_CONTRIBUTION_KINDS.len());

    let mut seen: Vec<ContributionKind> = declared.iter().map(|(kind, _)| *kind).collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen.len(),
        ALL_CONTRIBUTION_KINDS.len(),
        "a kind is missing from declared_ids and would escape duplicate detection"
    );
}

#[test]
fn declared_paths_reach_every_path_bearing_field() {
    let manifest = every_kind();
    let paths: Vec<&str> = manifest
        .declared_paths()
        .into_iter()
        .map(PortablePackagePath::as_str)
        .collect();

    assert!(paths.contains(&"schemas/in.json"));
    assert!(paths.contains(&"schemas/out.json"));
    assert!(paths.contains(&"skills/guarded-reviewer/SKILL.md"));
    assert!(paths.contains(&"schemas/config.json"));
    assert_eq!(paths.len(), 4);
}

#[test]
fn the_same_local_id_under_two_kinds_is_not_a_duplicate() {
    // A tool and a Skill may both be called `review`; two tools may not. Only a kind-aware view
    // can tell those apart, which is why `declared_ids` pairs them.
    let mut manifest = ContributionManifest::default();
    manifest.tools.push(ToolContribution {
        id: local("review"),
        display_name: "Review".to_string(),
        description: None,
        input_schema: None,
        output_schema: None,
        handler: "tool.review".to_string(),
    });
    manifest.skills.push(SkillContribution {
        id: local("review"),
        path: path("skills/review/SKILL.md"),
    });

    let declared = manifest.declared_ids();
    let mut pairs: Vec<(ContributionKind, &str)> = declared
        .iter()
        .map(|(kind, id)| (*kind, id.as_str()))
        .collect();
    let total = pairs.len();
    pairs.sort_unstable();
    pairs.dedup();
    assert_eq!(pairs.len(), total, "distinct kinds must not collide");
}

#[test]
fn an_external_package_may_not_select_the_builtin_runtime() {
    assert!(!RuntimeKind::Builtin.is_selectable_by_external_package());
    for kind in [
        RuntimeKind::WasmModule,
        RuntimeKind::Sidecar,
        RuntimeKind::WasmComponentReserved,
        RuntimeKind::None,
    ] {
        assert!(
            kind.is_selectable_by_external_package(),
            "{} should be selectable",
            kind.as_str()
        );
    }
}

#[test]
fn runtime_kinds_round_trip_and_reject_the_unknown() {
    for kind in [
        RuntimeKind::Builtin,
        RuntimeKind::WasmModule,
        RuntimeKind::Sidecar,
        RuntimeKind::WasmComponentReserved,
        RuntimeKind::None,
    ] {
        assert_eq!(RuntimeKind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(RuntimeKind::parse("native"), None);
    assert_eq!(RuntimeKind::parse("wasm"), None);

    // Reserved, not silently treated as a module: the pinned engine has no component support, and
    // a package asking for one deserves to be told that rather than a parse failure.
    assert_ne!(
        RuntimeKind::parse("wasm-component"),
        Some(RuntimeKind::WasmModule)
    );
}

#[test]
fn only_runtime_bearing_kinds_require_an_entry() {
    assert!(RuntimeKind::WasmModule.requires_entry());
    assert!(RuntimeKind::Sidecar.requires_entry());
    assert!(RuntimeKind::Builtin.requires_entry());
    assert!(!RuntimeKind::None.requires_entry());
}

#[test]
fn trust_profiles_are_ordered_and_carry_the_documented_budgets() {
    assert!(TrustProfile::Strict < TrustProfile::Standard);
    assert!(TrustProfile::Standard < TrustProfile::Trusted);

    assert_eq!(TrustProfile::Strict.max_callback_seconds(), 5);
    assert_eq!(TrustProfile::Standard.max_callback_seconds(), 10);
    assert_eq!(TrustProfile::Trusted.max_callback_seconds(), 30);

    for profile in [
        TrustProfile::Strict,
        TrustProfile::Standard,
        TrustProfile::Trusted,
    ] {
        assert_eq!(TrustProfile::parse(profile.as_str()), Some(profile));
    }
    assert_eq!(TrustProfile::parse("full"), None);
}

#[test]
fn a_contributed_rule_cannot_express_allow() {
    // Not a validation rule that could be forgotten — `Allow` has no representation here, so a
    // manifest cannot carry one as far as the compiler.
    for effect in [ContributedRuleEffect::Ask, ContributedRuleEffect::Deny] {
        assert_eq!(ContributedRuleEffect::parse(effect.as_str()), Some(effect));
    }
    // No spelling of allow is representable, so the compiler enforces what a validation rule
    // would otherwise have to remember.
    assert_eq!(ContributedRuleEffect::parse("allow"), None);
    assert_eq!(ContributedRuleEffect::parse("Allow"), None);
}

#[test]
fn hook_failure_modes_round_trip() {
    for mode in [HookFailureMode::FailClosed, HookFailureMode::FailOpen] {
        assert_eq!(HookFailureMode::parse(mode.as_str()), Some(mode));
    }
    assert_eq!(HookFailureMode::parse("retry"), None);
}

#[test]
fn a_capability_request_reports_whether_it_asks_for_anything() {
    assert!(CapabilityRequest::default().is_empty());

    let request = CapabilityRequest {
        network_origins: vec!["https://api.github.com".to_string()],
        ..CapabilityRequest::default()
    };
    assert!(!request.is_empty());
}

#[test]
fn an_optional_dependency_is_distinguishable_from_a_required_one() {
    let required = ExtensionDependency {
        id: ExtensionId::parse("acme.base").expect("parse"),
        version: VersionReq::parse(">=1.0.0").expect("parse"),
        optional: false,
    };
    let optional = ExtensionDependency {
        optional: true,
        ..required.clone()
    };

    assert!(!required.optional);
    assert!(optional.optional);
    assert_ne!(required, optional);
}

#[test]
fn exactly_one_schema_version_is_supported_today() {
    assert_eq!(SUPPORTED_SCHEMA_VERSIONS, [1]);
}

#[test]
fn a_complete_manifest_assembles_from_domain_types_alone() {
    // The shape `ExtensionManifestV1Decoder` must produce. Built here so the struct is exercised
    // before the decoder exists, and so a field added without a decoder path is visible.
    let manifest = VersionedExtensionManifest::V1(ExtensionManifestV1 {
        id: ExtensionId::parse("acme.git-guardian").expect("parse"),
        display_name: "Git Guardian".to_string(),
        publisher: PublisherId::parse("acme").expect("parse"),
        version: Version::parse("1.2.0").expect("parse"),
        description: Some("Adds guarded Git tools.".to_string()),
        license: Some("Apache-2.0".to_string()),
        min_vanehub_version: VersionReq::parse(">=0.9.0").expect("parse"),
        runtime: RuntimeDeclaration {
            kind: RuntimeKind::WasmModule,
            entry: Some(path("runtime/git_guardian.wasm")),
            trust_profile: TrustProfile::Standard,
        },
        activation_events: vec![
            ActivationEvent::parse("onTool:git_status").expect("parse"),
            ActivationEvent::parse("manual").expect("parse"),
        ],
        requires: ExtensionRequirements {
            extensions: vec![ExtensionDependency {
                id: ExtensionId::parse("acme.base").expect("parse"),
                version: VersionReq::parse(">=1.0.0").expect("parse"),
                optional: false,
            }],
            skills: vec![SkillDependency {
                id: "code-reviewer".to_string(),
                version: VersionReq::parse(">=2.0.0, <3.0.0").expect("parse"),
                optional: true,
            }],
        },
        permissions: CapabilityRequest {
            network_origins: vec!["https://api.github.com".to_string()],
            secret_ids: vec!["github.token".to_string()],
            ..CapabilityRequest::default()
        },
        contributes: every_kind(),
    });

    assert_eq!(manifest.schema_version(), 1);
    assert_eq!(manifest.id().as_str(), "acme.git-guardian");
    assert_eq!(manifest.id().publisher().as_str(), "acme");

    let VersionedExtensionManifest::V1(inner) = &manifest;
    assert!(!inner.permissions.is_empty());
    assert_eq!(inner.contributes.total(), ALL_CONTRIBUTION_KINDS.len());
    // The declared runtime needs an entry, and one is present.
    assert!(inner.runtime.kind.requires_entry());
    assert!(inner.runtime.entry.is_some());
    // One event waits for the user; the other does not.
    assert_eq!(
        inner
            .activation_events
            .iter()
            .filter(|event| event.is_automatic())
            .count(),
        1
    );
}

#[test]
fn remote_transports_and_mcp_backed_hooks_are_representable() {
    // The variants the `every_kind` fixture does not use, so neither goes unexercised.
    let http = McpTransportDeclaration::Http {
        url: "https://mcp.example.com/rpc".to_string(),
        header_keys: vec!["Authorization".to_string()],
    };
    let McpTransportDeclaration::Http { url, header_keys } = &http else {
        panic!("expected an HTTP transport");
    };
    assert_eq!(url, "https://mcp.example.com/rpc");
    // Header *names* only. A value here would be a secret travelling inside a package.
    assert_eq!(header_keys, &["Authorization".to_string()]);

    let handler = HookHandlerDeclaration::McpTool {
        tool: "acme-mcp/review".to_string(),
    };
    assert_ne!(
        handler,
        HookHandlerDeclaration::ExtensionRuntime {
            entry: "acme-mcp/review".to_string(),
        }
    );
}
