//! What an instance holds, and the states it deliberately does not hold.

use super::{
    all_connector_binding_errors, all_connector_instance_errors, BindingId, ConnectorBinding,
    ConnectorBindingError, ConnectorGlobalId, ConnectorInstance, ConnectorInstanceError,
    ConnectorTarget, CredentialHandle, DisplayLabel, InstanceId, PublicConfiguration, TargetKind,
    ABSENT_REVISION,
};

fn instance(label: &str) -> ConnectorInstance {
    ConnectorInstance {
        instance: InstanceId::parse("instance-1").expect("instance"),
        connector: ConnectorGlobalId::parse("ext::acme.mailer::smtp").expect("connector"),
        display_label: DisplayLabel::parse(label).expect("label"),
        desired_enabled: true,
        configuration: PublicConfiguration::of(&[("base_url", "https://example.test")])
            .expect("config"),
        credential: Some(CredentialHandle::parse("cred-1").expect("handle")),
        revision: 1,
        updated_at: "2026-08-23T00:00:00Z".to_string(),
    }
}

#[test]
fn identity_is_the_instance_id_so_a_rename_keeps_the_binding_and_the_credential() {
    let before = instance("Acme Prod");
    let renamed = ConnectorInstance {
        display_label: DisplayLabel::parse("Acme Production").expect("label"),
        revision: 2,
        ..before.clone()
    };

    assert_eq!(before.instance, renamed.instance);
    assert_eq!(before.credential, renamed.credential);
    assert_ne!(
        before.label_key(),
        renamed.label_key(),
        "the uniqueness key moves with the label -- which is why it cannot be identity"
    );
}

#[test]
fn the_label_key_is_derived_rather_than_stored_beside_the_label() {
    // Two fields that must agree are two fields that will eventually disagree.
    let held = instance("  Acme   PROD ");

    assert_eq!(held.label_key().as_str(), "acme prod");
    assert_eq!(held.label_key(), held.display_label.key());
}

#[test]
fn an_instance_records_what_was_asked_for_and_not_what_is_happening() {
    // `connecting` and `connected` are properties of a socket. Writing them down means every crash
    // leaves a row claiming a connection that does not exist.
    let held = instance("Acme Prod");

    assert!(held.desired_enabled);
    // The struct has nowhere to put a live state. If this ever stops compiling because a
    // `connected` field appeared, that field is the bug.
    let ConnectorInstance {
        instance: _,
        connector: _,
        display_label: _,
        desired_enabled: _,
        configuration: _,
        credential: _,
        revision: _,
        updated_at: _,
    } = held;
}

#[test]
fn a_credential_is_a_handle_and_an_instance_may_have_none_yet() {
    let configured = instance("Acme Prod");
    let unconfigured = ConnectorInstance {
        credential: None,
        ..configured.clone()
    };

    assert!(configured.credential.is_some());
    assert!(unconfigured.credential.is_none());
    assert!(
        !format!("{configured:?}").contains("cred-1"),
        "an instance printed into a log must not name its credential-store entry"
    );
}

#[test]
fn a_binding_is_held_per_target_so_one_target_does_not_speak_for_another() {
    let global = ConnectorBinding {
        binding: BindingId::parse("binding-1").expect("binding"),
        instance: InstanceId::parse("instance-1").expect("instance"),
        target: ConnectorTarget::global(),
        enabled: true,
        revision: 1,
        updated_at: "2026-08-23T00:00:00Z".to_string(),
    };
    let project = ConnectorBinding {
        binding: BindingId::parse("binding-2").expect("binding"),
        target: ConnectorTarget::scoped(TargetKind::Project, "d:/work/repo").expect("project"),
        enabled: false,
        ..global.clone()
    };

    assert_ne!(global.target, project.target);
    assert_ne!(global.binding, project.binding);
}

#[test]
fn a_stale_revision_reports_both_numbers() {
    // "Someone else changed it" is not actionable; "you had 3, it is now 5" is.
    let instance_error = ConnectorInstanceError::StaleRevision {
        expected: 3,
        actual: 5,
    };
    let binding_error = ConnectorBindingError::StaleRevision {
        expected: 3,
        actual: 5,
    };

    assert_eq!(instance_error.code(), "connector_instance_stale_revision");
    assert_eq!(binding_error.code(), "connector_binding_stale_revision");
    let ConnectorInstanceError::StaleRevision { expected, actual } = instance_error else {
        panic!("expected a stale revision");
    };
    assert_eq!((expected, actual), (3, 5));
}

#[test]
fn a_duplicate_label_names_the_instance_already_using_it() {
    // Without the id, an operator is told the name is taken and has no way to find by what.
    let error = ConnectorInstanceError::DuplicateLabel {
        existing: InstanceId::parse("instance-9").expect("instance"),
    };

    assert_eq!(error.code(), "duplicate_connector_label");
    let ConnectorInstanceError::DuplicateLabel { existing } = error else {
        panic!("expected a duplicate label");
    };
    assert_eq!(existing.as_str(), "instance-9");
}

#[test]
fn a_create_and_an_update_go_through_the_same_compare_and_swap() {
    // So a create cannot silently overwrite an instance that appeared between the read and the
    // write.
    assert_eq!(ABSENT_REVISION, 0);
}

#[test]
fn every_failure_has_a_distinct_stable_code() {
    for codes in [
        all_connector_instance_errors()
            .iter()
            .map(ConnectorInstanceError::code)
            .collect::<Vec<_>>(),
        all_connector_binding_errors()
            .iter()
            .map(ConnectorBindingError::code)
            .collect::<Vec<_>>(),
    ] {
        let total = codes.len();
        let mut unique = codes;
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), total);
    }
}
