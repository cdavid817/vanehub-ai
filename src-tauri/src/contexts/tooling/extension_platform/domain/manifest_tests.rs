//! Identity and activation-event invariants.
//!
//! Table-driven and bounded-combinatorial rather than randomised: the input space here is a
//! grammar, so enumerating its edges is both cheaper and more honest than sampling it. Nothing in
//! this file is a property-based test and none of it claims to be.

// Imported through the domain's published surface rather than from the child modules, so these
// exercise the same names `ExtensionManifestV1Decoder` and every later consumer will use.
use super::{
    is_external_contribution_id, ActivationEvent, ActivationTarget, ContributionGlobalId,
    ContributionKind, ContributionLocalId, ExtensionId, IdentifierKind, InstallationId,
    OperationWitness, PackageHash, PublisherId, RuntimeGenerationId, SnapshotId,
    ALL_CONTRIBUTION_KINDS, ALL_IDENTIFIER_KINDS, MAX_EXTENSION_ID_CHARACTERS,
    MIN_EXTENSION_ID_CHARACTERS,
};

// ---------------------------------------------------------------------------
// Extension and publisher ids
// ---------------------------------------------------------------------------

#[test]
fn accepted_extension_ids_are_exactly_two_lower_case_segments() {
    for value in [
        "a.b",
        "acme.git-guardian",
        "acme2.tool9",
        "a-b.c-d",
        "publisher.a-very-long-name-that-is-still-fine",
    ] {
        let id = ExtensionId::parse(value).unwrap_or_else(|_| panic!("{value} should parse"));
        assert_eq!(id.as_str(), value);
    }
}

#[test]
fn rejected_extension_ids_cover_every_way_the_grammar_can_break() {
    let cases = [
        ("", "empty"),
        ("ab", "shorter than the minimum"),
        ("acme", "no dot"),
        ("acme.", "empty name"),
        (".guardian", "empty publisher"),
        ("acme.tools.git", "two dots"),
        ("Acme.guardian", "upper case"),
        ("acme.Guardian", "upper case in the name"),
        ("acme_x.guardian", "underscore in a segment"),
        ("-acme.guardian", "leading dash"),
        ("acme-.guardian", "trailing dash"),
        ("acme.-guardian", "leading dash in the name"),
        ("acme.guardian-", "trailing dash in the name"),
        ("acme guardian.x", "whitespace"),
        ("acme.guardian/x", "path separator"),
        ("acme..guardian", "empty middle segment"),
    ];

    for (value, why) in cases {
        assert!(
            ExtensionId::parse(value).is_err(),
            "{value:?} should be rejected: {why}"
        );
    }
}

#[test]
fn extension_id_length_is_bounded_at_both_ends() {
    assert_eq!(MIN_EXTENSION_ID_CHARACTERS, 3);
    assert_eq!(MAX_EXTENSION_ID_CHARACTERS, 128);

    assert!(ExtensionId::parse("a.b").is_ok());
    assert!(ExtensionId::parse("a.").is_err());

    // Each half is separately bounded at 64, so the longest legal id is 64 + 1 + 64.
    let longest = format!("{}.{}", "a".repeat(64), "b".repeat(64));
    assert_eq!(longest.len(), MAX_EXTENSION_ID_CHARACTERS + 1);
    assert!(ExtensionId::parse(&longest).is_err());

    let at_limit = format!("{}.{}", "a".repeat(63), "b".repeat(64));
    assert_eq!(at_limit.len(), MAX_EXTENSION_ID_CHARACTERS);
    assert!(ExtensionId::parse(&at_limit).is_ok());
}

#[test]
fn an_extension_id_splits_into_the_halves_it_was_built_from() {
    let id = ExtensionId::parse("acme.git-guardian").expect("parse");
    assert_eq!(id.publisher().as_str(), "acme");
    assert_eq!(id.name(), "git-guardian");
}

#[test]
fn a_publisher_id_follows_the_same_segment_rule_as_an_extension_half() {
    for value in ["acme", "a", "acme-tools", "a1"] {
        assert!(PublisherId::parse(value).is_ok(), "{value} should parse");
    }
    for value in [
        "",
        "Acme",
        "acme.tools",
        "-acme",
        "acme-",
        "acme_tools",
        "acme ",
    ] {
        assert!(
            PublisherId::parse(value).is_err(),
            "{value} should be rejected"
        );
    }
}

#[test]
fn every_identifier_kind_has_a_distinct_stable_code() {
    // Callers branch on the code, so two kinds sharing one would make a diagnostic unactionable
    // without failing anything else.
    let mut codes: Vec<&str> = ALL_IDENTIFIER_KINDS
        .iter()
        .map(|kind| kind.code())
        .collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}

#[test]
fn a_rejection_names_the_identifier_that_failed() {
    assert_eq!(
        ExtensionId::parse("nope")
            .expect_err("rejected")
            .identifier(),
        IdentifierKind::Extension
    );
    assert_eq!(
        PublisherId::parse("Nope")
            .expect_err("rejected")
            .identifier(),
        IdentifierKind::Publisher
    );
    assert_eq!(
        PackageHash::parse("nope")
            .expect_err("rejected")
            .identifier(),
        IdentifierKind::PackageHash
    );
    assert_eq!(
        ActivationEvent::parse("nope")
            .expect_err("rejected")
            .identifier(),
        IdentifierKind::ActivationEvent
    );
}

#[test]
fn a_rejected_identifier_reports_a_code_and_a_bounded_value() {
    let error = ExtensionId::parse("NOT VALID").expect_err("should be rejected");
    assert_eq!(error.code(), "invalid_extension_id");
    assert_eq!(error.value(), "NOT VALID");

    let hostile = "!".repeat(4_000);
    let error = ExtensionId::parse(&hostile).expect_err("should be rejected");
    assert_eq!(error.value().chars().count(), MAX_EXTENSION_ID_CHARACTERS);
}

// ---------------------------------------------------------------------------
// Contribution ids
// ---------------------------------------------------------------------------

#[test]
fn contribution_local_ids_allow_dashes_and_underscores_inside_only() {
    for value in ["git_status", "guarded-reviewer", "a", "a1_b-c"] {
        assert!(
            ContributionLocalId::parse(value).is_ok(),
            "{value} should parse"
        );
    }
    for value in [
        "",
        "Git_status",
        "-lead",
        "lead-",
        "_lead",
        "lead_",
        "a b",
        "a.b",
        "a/b",
    ] {
        assert!(
            ContributionLocalId::parse(value).is_err(),
            "{value} should be rejected"
        );
    }
}

#[test]
fn every_contribution_kind_has_a_distinct_key_that_round_trips() {
    let mut keys: Vec<&str> = ALL_CONTRIBUTION_KINDS
        .iter()
        .map(|kind| kind.as_str())
        .collect();
    let total = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), total);

    for kind in ALL_CONTRIBUTION_KINDS {
        assert_eq!(ContributionKind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(ContributionKind::parse("provider"), None);
    assert_eq!(ContributionKind::parse(""), None);
}

#[test]
fn a_global_id_is_built_from_its_parts_for_every_kind_and_round_trips() {
    // Bounded combinatorial: every kind against a small set of ids, which is the whole cross
    // product that matters for this construction.
    let extension = ExtensionId::parse("acme.git-guardian").expect("parse");
    let locals = ["git_status", "guarded-reviewer", "a"];

    for kind in ALL_CONTRIBUTION_KINDS {
        for local in locals {
            let local = ContributionLocalId::parse(local).expect("parse");
            let global = ContributionGlobalId::new(&extension, kind, &local);

            assert_eq!(
                global.as_str(),
                format!(
                    "ext::{}::{}::{}",
                    extension.as_str(),
                    kind.as_str(),
                    local.as_str()
                )
            );
            assert_eq!(
                ContributionGlobalId::parse(global.as_str()).expect("round trip"),
                global
            );
        }
    }
}

#[test]
fn global_ids_from_different_parts_never_collide() {
    let one = ExtensionId::parse("acme.one").expect("parse");
    let two = ExtensionId::parse("acme.two").expect("parse");
    let local = ContributionLocalId::parse("shared").expect("parse");

    let mut ids: Vec<String> = Vec::new();
    for extension in [&one, &two] {
        for kind in ALL_CONTRIBUTION_KINDS {
            ids.push(
                ContributionGlobalId::new(extension, kind, &local)
                    .as_str()
                    .to_string(),
            );
        }
    }

    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), total, "namespacing must keep every id distinct");
}

#[test]
fn a_malformed_global_id_does_not_round_trip() {
    for value in [
        "",
        "ext::acme.one::tool",
        "ext::acme.one::tool::a::b",
        "other::acme.one::tool::a",
        "ext::ACME.one::tool::a",
        "ext::acme.one::provider::a",
        "ext::acme.one::tool::-bad",
    ] {
        assert!(
            ContributionGlobalId::parse(value).is_err(),
            "{value:?} should be rejected"
        );
    }
}

#[test]
fn only_a_prefixed_identifier_counts_as_external() {
    assert!(is_external_contribution_id("ext::acme.one::tool::a"));
    // The identities an extension must never occupy.
    for native in [
        "shell",
        "file",
        "remember",
        "mcp__server__tool",
        "extra",
        "ext",
        "ext:",
    ] {
        assert!(
            !is_external_contribution_id(native),
            "{native} must not read as external"
        );
    }
}

// ---------------------------------------------------------------------------
// Package hash and opaque identifiers
// ---------------------------------------------------------------------------

#[test]
fn a_package_hash_is_exactly_sixty_four_lower_case_hex_characters() {
    let valid = "0123456789abcdef".repeat(4);
    assert_eq!(valid.len(), 64);
    assert_eq!(
        PackageHash::parse(&valid).expect("parse").as_str(),
        valid.as_str()
    );

    for value in [
        "",
        &valid[..63],
        &format!("{valid}0"),
        &"0123456789ABCDEF".repeat(4),
        &"0123456789abcdeg".repeat(4),
        &" ".repeat(64),
    ] {
        assert!(
            PackageHash::parse(value).is_err(),
            "{value:?} should be rejected"
        );
    }
}

#[test]
fn opaque_identifiers_share_one_rule_and_stay_separate_types() {
    // The macro exists so this rule cannot drift between them; this asserts it did not.
    for value in ["a", "op-1", "gen_2", "A1"] {
        // Round-trip rather than merely accept: the value is stored as written, so a constructor
        // that trimmed or lower-cased would silently make two distinct snapshots compare equal.
        assert_eq!(
            SnapshotId::parse(value).expect("parse").as_str(),
            value,
            "{value} should round-trip"
        );
        assert_eq!(InstallationId::parse(value).expect("parse").as_str(), value);
        assert_eq!(
            RuntimeGenerationId::parse(value).expect("parse").as_str(),
            value
        );
        assert_eq!(
            OperationWitness::parse(value).expect("parse").as_str(),
            value
        );
    }
    for value in ["", "a b", "a.b", "a/b", &"a".repeat(129)] {
        assert!(
            SnapshotId::parse(value).is_err(),
            "{value:?} should be rejected"
        );
        assert!(
            InstallationId::parse(value).is_err(),
            "{value:?} should be rejected"
        );
        assert!(
            RuntimeGenerationId::parse(value).is_err(),
            "{value:?} should be rejected"
        );
        assert!(
            OperationWitness::parse(value).is_err(),
            "{value:?} should be rejected"
        );
    }

    assert_eq!(
        SnapshotId::parse("").expect_err("rejected").code(),
        "invalid_snapshot_id"
    );
    assert_eq!(
        InstallationId::parse("").expect_err("rejected").code(),
        "invalid_installation_id"
    );
    assert_eq!(
        RuntimeGenerationId::parse("").expect_err("rejected").code(),
        "invalid_runtime_generation_id"
    );
    assert_eq!(
        OperationWitness::parse("").expect_err("rejected").code(),
        "invalid_operation_witness"
    );
}

// ---------------------------------------------------------------------------
// Activation events
// ---------------------------------------------------------------------------

#[test]
fn every_documented_activation_event_parses_and_round_trips() {
    let cases = [
        "onStartupFinished",
        "onSessionStart",
        "manual",
        "onAgentMode:guardrails",
        "onTool:git_status",
        "onHook:tool.before_execute",
        "onConnector:github",
        "onCommand:vanehub.review",
    ];

    for value in cases {
        let event =
            ActivationEvent::parse(value).unwrap_or_else(|_| panic!("{value} should parse"));
        assert_eq!(event.to_manifest_value(), value);
    }
}

#[test]
fn an_unknown_or_malformed_activation_event_is_rejected() {
    for value in [
        "",
        "onStartup",
        "startupFinished",
        "onTool",
        "onTool:",
        "onTool:a b",
        "onTool:a:b",
        "onUnknown:x",
        "Manual",
        &format!("onTool:{}", "a".repeat(200)),
    ] {
        assert!(
            ActivationEvent::parse(value).is_err(),
            "{value:?} should be rejected"
        );
    }

    assert_eq!(
        ActivationEvent::parse("onUnknown:x")
            .expect_err("rejected")
            .code(),
        "invalid_activation_event"
    );
}

#[test]
fn only_manual_activation_waits_for_the_user() {
    let automatic = [
        "onStartupFinished",
        "onSessionStart",
        "onAgentMode:guardrails",
        "onTool:git_status",
        "onHook:tool.before_execute",
        "onConnector:github",
        "onCommand:review",
    ];
    for value in automatic {
        let event = ActivationEvent::parse(value).expect("parse");
        assert!(event.is_automatic(), "{value} activates without the user");
    }
    assert!(!ActivationEvent::parse("manual")
        .expect("parse")
        .is_automatic());
}

#[test]
fn only_parameterised_events_carry_a_target() {
    assert_eq!(
        ActivationEvent::parse("onTool:git_status")
            .expect("parse")
            .target()
            .map(ActivationTarget::as_str),
        Some("git_status")
    );
    for value in ["onStartupFinished", "onSessionStart", "manual"] {
        assert!(
            ActivationEvent::parse(value)
                .expect("parse")
                .target()
                .is_none(),
            "{value} has no target"
        );
    }
}
