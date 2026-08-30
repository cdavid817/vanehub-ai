use super::*;

#[test]
fn enabled_mode_requires_current_local_consent_and_a_stable_allowlist() {
    let current = EvolutionOrchestrationPolicyV1::default_off("workspace-one".into(), 10);
    let enabled = EvolutionPolicyMutationV1 {
        expected_revision: 0,
        mode: EvolutionPolicyMode::Enabled,
        allowed_skill_ids: vec!["skill-one".into(), "skill-one".into()],
        acknowledge_current_disclosure: true,
        notify_routine_completion: false,
        updated_at_ms: 20,
    };
    let policy = apply_policy_mutation(&current, enabled).expect("enabled policy");
    assert_eq!(policy.allowed_skill_ids, ["skill-one"]);
    assert!(policy_allows_skill(&policy, "skill-one"));
    assert!(!policy_allows_skill(&policy, "skill-two"));
    let no_allowlist = EvolutionPolicyMutationV1 {
        expected_revision: 0,
        mode: EvolutionPolicyMode::Enabled,
        allowed_skill_ids: Vec::new(),
        acknowledge_current_disclosure: true,
        notify_routine_completion: false,
        updated_at_ms: 20,
    };
    assert_eq!(
        apply_policy_mutation(&current, no_allowlist),
        Err(EvolutionPolicyError::AllowlistRequired)
    );
}

#[test]
fn wildcard_and_stale_revision_are_rejected() {
    let current = EvolutionOrchestrationPolicyV1::default_off("workspace-one".into(), 10);
    let wildcard = EvolutionPolicyMutationV1 {
        expected_revision: 0,
        mode: EvolutionPolicyMode::Observe,
        allowed_skill_ids: vec!["*".into()],
        acknowledge_current_disclosure: false,
        notify_routine_completion: false,
        updated_at_ms: 20,
    };
    assert_eq!(
        apply_policy_mutation(&current, wildcard),
        Err(EvolutionPolicyError::InvalidSkillId)
    );
    let stale = EvolutionPolicyMutationV1 {
        expected_revision: 1,
        mode: EvolutionPolicyMode::Off,
        allowed_skill_ids: Vec::new(),
        acknowledge_current_disclosure: false,
        notify_routine_completion: false,
        updated_at_ms: 20,
    };
    assert_eq!(
        apply_policy_mutation(&current, stale),
        Err(EvolutionPolicyError::RevisionConflict)
    );
}

#[test]
fn revocation_is_immediate_and_import_never_carries_local_consent() {
    let current = EvolutionOrchestrationPolicyV1::default_off("workspace-one".into(), 10);
    let enabled = apply_policy_mutation(
        &current,
        EvolutionPolicyMutationV1 {
            expected_revision: 0,
            mode: EvolutionPolicyMode::Enabled,
            allowed_skill_ids: vec!["skill-one".into()],
            acknowledge_current_disclosure: true,
            notify_routine_completion: false,
            updated_at_ms: 20,
        },
    )
    .expect("enabled");
    let revoked = revoke_policy_consent(&enabled, 30).expect("revoked");
    assert_eq!(revoked.mode, EvolutionPolicyMode::Off);
    assert!(!policy_allows_skill(&revoked, "skill-one"));
    assert_eq!(
        revoked.consent.as_ref().and_then(|item| item.revoked_at_ms),
        Some(30)
    );
    let imported =
        import_policy_without_local_consent(&enabled, "workspace-two".into(), 40).expect("import");
    assert_eq!(imported.mode, EvolutionPolicyMode::Observe);
    assert_eq!(imported.consent, None);
    assert!(!policy_allows_skill(&imported, "skill-one"));
}

#[test]
fn prior_disclosure_witness_remains_auditable_but_cannot_enable() {
    let current = EvolutionOrchestrationPolicyV1::default_off("workspace-one".into(), 10);
    let mut prior = apply_policy_mutation(
        &current,
        EvolutionPolicyMutationV1 {
            expected_revision: 0,
            mode: EvolutionPolicyMode::Observe,
            allowed_skill_ids: vec!["skill-one".into()],
            acknowledge_current_disclosure: true,
            notify_routine_completion: false,
            updated_at_ms: 20,
        },
    )
    .expect("observed policy");
    let consent = prior.consent.as_mut().expect("consent");
    consent.disclosure_version = "prior-disclosure".into();
    consent.witness_hash = canonical_hash(&(
        "workspace-one",
        1_u64,
        "prior-disclosure",
        20_i64,
        "interactive_user",
    ))
    .expect("prior witness");
    validate_policy_integrity(&prior).expect("auditable prior disclosure");
    assert_eq!(
        apply_policy_mutation(
            &prior,
            EvolutionPolicyMutationV1 {
                expected_revision: 1,
                mode: EvolutionPolicyMode::Enabled,
                allowed_skill_ids: vec!["skill-one".into()],
                acknowledge_current_disclosure: false,
                notify_routine_completion: false,
                updated_at_ms: 30,
            },
        ),
        Err(EvolutionPolicyError::ConsentRequired)
    );
}
