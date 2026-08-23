//! What an identity admits, and the four it exists to refuse.

use super::{
    registered_connector_failures, ConnectorGlobalId, ConnectorIdentifierKind,
    ConnectorIdentityError, ConnectorSnapshotRef, ConnectorTarget, CredentialHandle, DisplayLabel,
    InstanceId, OwnerExtensionId, PublicConfiguration, TargetKind, ALL_TARGET_KINDS,
    GLOBAL_TARGET_KEY,
};

#[test]
fn a_display_label_keeps_what_the_person_typed() {
    // Normalising the stored label would rewrite the user's own words back at them.
    let label = DisplayLabel::parse("  Acme  Production  ").expect("label");

    assert_eq!(
        label.as_str(),
        "Acme  Production",
        "only the ends are trimmed"
    );
}

#[test]
fn uniqueness_is_decided_on_a_normalised_key_the_label_is_not() {
    // `Acme Prod` and `acme  prod` in one list is how a credential gets attached to the wrong
    // instance. Casing and whitespace runs are the confusable pairs people actually produce.
    let typed = DisplayLabel::parse("Acme Prod").expect("label");
    let sloppy = DisplayLabel::parse("acme   PROD").expect("label");
    let tabbed = DisplayLabel::parse("Acme\tProd").expect("label");

    assert_eq!(typed.key(), sloppy.key());
    assert_eq!(typed.key(), tabbed.key());
    assert_eq!(typed.key().as_str(), "acme prod");
    assert_ne!(
        typed.as_str(),
        sloppy.as_str(),
        "the labels themselves stay distinct"
    );
}

#[test]
fn two_genuinely_different_labels_keep_different_keys() {
    let one = DisplayLabel::parse("Acme Prod").expect("label");
    let other = DisplayLabel::parse("Acme Staging").expect("label");

    assert_ne!(one.key(), other.key());
}

#[test]
fn a_label_is_bounded_and_carries_no_control_characters() {
    assert!(DisplayLabel::parse("").is_err());
    assert!(DisplayLabel::parse("   ").is_err());
    assert!(DisplayLabel::parse("has\0nul").is_err());
    assert!(DisplayLabel::parse("has\nnewline").is_err());
    assert!(DisplayLabel::parse(&"a".repeat(97)).is_err());
    assert!(DisplayLabel::parse(&"a".repeat(96)).is_ok());
}

#[test]
fn a_credential_handle_does_not_print_itself() {
    // A handle is not a secret, but printing one turns every log line into a map of which
    // credential-store entries exist.
    let handle = CredentialHandle::parse("cred-01hxyz").expect("handle");

    assert_eq!(format!("{handle:?}"), "CredentialHandle(<redacted>)");
    assert!(!format!("{handle:?}").contains("01hxyz"));
    assert_eq!(handle.expose_for_storage(), "cred-01hxyz");
}

#[test]
fn a_public_configuration_refuses_secret_shaped_keys() {
    // A name check at the one boundary where the name is reliable: a field the definition declared
    // *public* is by construction not the one called `api_key`. It catches the specific mistake
    // this column invites -- pasting a token into the visible settings form.
    for smuggled in [
        "api_key",
        "token",
        "password",
        "client_secret",
        "authorization",
        "private_key",
        "refresh_token",
    ] {
        let error = PublicConfiguration::of(&[(smuggled, "value")]).expect_err(smuggled);
        assert_eq!(
            error.code(),
            "secret_shaped_public_configuration",
            "{smuggled:?}"
        );
    }

    // What it is for.
    assert!(PublicConfiguration::of(&[
        ("base_url", "https://example.test"),
        ("workspace_id", "42"),
    ])
    .is_ok());
}

#[test]
fn a_configuration_does_not_depend_on_the_order_a_form_submitted_it() {
    let forward = PublicConfiguration::of(&[("b_field", "2"), ("a_field", "1")]).expect("config");
    let reversed = PublicConfiguration::of(&[("a_field", "1"), ("b_field", "2")]).expect("config");

    assert_eq!(forward, reversed);
    assert_eq!(forward.as_str(), "a_field=1\nb_field=2");
}

#[test]
fn a_configuration_exposes_its_entries_in_the_canonical_order() {
    let held = PublicConfiguration::of(&[("workspace_id", "42"), ("base_url", "https://x.test")])
        .expect("config");

    assert_eq!(
        held.entries(),
        &[
            ("base_url".to_string(), "https://x.test".to_string()),
            ("workspace_id".to_string(), "42".to_string()),
        ]
    );
    assert!(PublicConfiguration::empty().entries().is_empty());
}

#[test]
fn a_configuration_round_trips_through_its_stored_form() {
    let written =
        PublicConfiguration::of(&[("base_url", "https://example.test/a=b"), ("depth", "3")])
            .expect("config");

    assert_eq!(
        PublicConfiguration::parse(&written.as_str()).expect("round trip"),
        written,
        "a value containing '=' survives, because only the first one separates"
    );
    assert_eq!(
        PublicConfiguration::parse("").expect("empty"),
        PublicConfiguration::empty()
    );
}

#[test]
fn a_configuration_key_and_value_are_bounded_and_carry_no_nul() {
    assert!(
        PublicConfiguration::of(&[("Base_URL", "x")]).is_err(),
        "keys are lower_snake_case"
    );
    assert!(PublicConfiguration::of(&[("", "x")]).is_err());
    assert!(PublicConfiguration::of(&[("_leading", "x")]).is_err());
    assert!(PublicConfiguration::of(&[("field", "has\0nul")]).is_err());
    assert!(PublicConfiguration::of(&[("field", &"a".repeat(2_049))]).is_err());
    assert!(PublicConfiguration::parse("no_equals_sign").is_err());
}

#[test]
fn the_global_target_has_exactly_one_spelling() {
    // A nullable target column would let SQLite treat every NULL as distinct and admit unlimited
    // global bindings for one instance, each invisible to the others.
    let global = ConnectorTarget::global();

    assert_eq!(global.kind(), TargetKind::Global);
    assert_eq!(global.key(), GLOBAL_TARGET_KEY);
    assert!(GLOBAL_TARGET_KEY.is_empty());
    assert_eq!(
        ConnectorTarget::parse("global", GLOBAL_TARGET_KEY).expect("global"),
        global
    );
    assert_eq!(
        ConnectorTarget::parse("global", "somewhere")
            .expect_err("global carries no key")
            .code(),
        "invalid_connector_target_key"
    );
    assert_eq!(
        ConnectorTarget::scoped(TargetKind::Global, "")
            .expect_err("global is not constructible as scoped")
            .code(),
        "invalid_connector_target_key"
    );
}

#[test]
fn a_scoped_target_must_say_what_it_is_scoped_to() {
    assert!(ConnectorTarget::scoped(TargetKind::Project, "").is_err());
    assert!(ConnectorTarget::scoped(TargetKind::Project, "has\0nul").is_err());
    assert!(ConnectorTarget::scoped(TargetKind::Project, "d:/work/repo").is_ok());
    assert!(ConnectorTarget::scoped(TargetKind::Agent, "claude-code").is_ok());
    assert!(ConnectorTarget::scoped(TargetKind::Session, "session-1").is_ok());
}

#[test]
fn a_target_kind_this_build_does_not_know_is_refused_rather_than_defaulted() {
    assert_eq!(
        ConnectorTarget::parse("workspace", "x")
            .expect_err("unknown kind")
            .code(),
        "invalid_connector_target_kind"
    );
}

#[test]
fn every_target_round_trips_through_the_two_columns_it_is_stored_as() {
    let targets = [
        ConnectorTarget::global(),
        ConnectorTarget::scoped(TargetKind::Project, "d:/work/repo").expect("project"),
        ConnectorTarget::scoped(TargetKind::Agent, "claude-code").expect("agent"),
        ConnectorTarget::scoped(TargetKind::Session, "session-1").expect("session"),
    ];

    for target in targets {
        assert_eq!(
            ConnectorTarget::parse(target.kind().as_str(), target.key()).expect("round trip"),
            target
        );
    }
    for kind in ALL_TARGET_KINDS.iter().copied() {
        assert_eq!(TargetKind::parse(kind.as_str()), Some(kind));
    }
}

#[test]
fn a_namespaced_contribution_id_is_a_connector_global_id() {
    for accepted in ["ext::acme.mailer::smtp", "vanehub.github", "a"] {
        assert!(ConnectorGlobalId::parse(accepted).is_ok(), "{accepted}");
    }
    for rejected in [
        "",
        "Ext::Acme",
        ":leading",
        "trailing.",
        "has space",
        "has\0nul",
    ] {
        assert_eq!(
            ConnectorGlobalId::parse(rejected)
                .expect_err(rejected)
                .code(),
            "invalid_connector_global_id",
            "{rejected:?}"
        );
    }
}

#[test]
fn an_owner_extension_is_validated_as_text_and_never_resolved_here() {
    assert!(OwnerExtensionId::parse("acme.mailer").is_ok());
    for rejected in ["", "Acme.Mailer", ".leading", "has space", "has_underscore"] {
        assert!(OwnerExtensionId::parse(rejected).is_err(), "{rejected:?}");
    }
}

#[test]
fn a_rejection_carries_the_offending_value_but_cannot_be_made_unbounded_by_it() {
    let hostile = "A".repeat(10_000);

    let error: ConnectorIdentityError = ConnectorGlobalId::parse(&hostile).expect_err("reject");

    assert_eq!(error.kind, ConnectorIdentifierKind::ConnectorGlobal);
    assert!(!error.value.is_empty());
    assert!(error.value.len() <= 160);
}

#[test]
fn an_opaque_reference_is_validated_as_text_and_never_resolved_here() {
    assert!(ConnectorSnapshotRef::parse("snap-01HXYZ").is_ok());
    assert!(InstanceId::parse("instance-01HXYZ").is_ok());
    for rejected in ["", "has space", "has/slash", "has:colon"] {
        assert!(
            ConnectorSnapshotRef::parse(rejected).is_err(),
            "{rejected:?}"
        );
        assert!(InstanceId::parse(rejected).is_err(), "{rejected:?}");
    }
}

#[test]
fn no_two_failures_in_this_subdomain_share_a_code() {
    let codes = registered_connector_failures();
    let total = codes.len();

    let mut unique = codes;
    unique.sort_unstable();
    unique.dedup();

    assert_eq!(
        unique.len(),
        total,
        "two failures present the same code; a caller branching on it cannot tell them apart"
    );
}

#[test]
fn every_failure_code_is_lower_snake_case_and_bounded() {
    for code in registered_connector_failures() {
        assert!(!code.is_empty());
        assert!(code.len() <= 64, "{code} is too long to be a stable code");
        assert!(
            code.chars().all(|character| character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'),
            "{code} is not lower_snake_case"
        );
    }
}
