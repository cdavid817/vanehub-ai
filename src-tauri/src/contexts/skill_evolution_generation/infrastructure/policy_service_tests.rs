use crate::{
    contexts::skill_evolution_generation::{
        application::{
            authorize_outbound_generation, evolve_generation_policy, update_generation_policy,
            GenerationOutboundAuthorization, GenerationPolicyChangeSource,
            GenerationPolicyChangeV1, GenerationPolicyError,
        },
        domain::{
            GenerationConsentState, GenerationPolicyV1, GenerationProviderReadinessV1,
            GENERATION_DISCLOSURE_VERSION_V1,
        },
        infrastructure::SqliteGenerationPolicyRepository,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

#[test]
fn policy_defaults_disabled_with_bounded_generation_budgets() {
    let policy = GenerationPolicyV1::default_disabled("workspace".into());
    assert_eq!(policy.consent_state, GenerationConsentState::Disabled);
    assert_eq!(policy.job_budget.wall_time_ms, 180_000);
    assert_eq!(policy.job_budget.model_calls, 3);
    assert_eq!(policy.job_budget.tool_calls, 8);
    assert_eq!(policy.daily_budget.concurrent_workspace_jobs, 1);
    assert_eq!(policy.daily_budget.concurrent_global_jobs, 2);
    assert_eq!(policy.retention.failed_job_days, 180);
    assert_eq!(policy.retention.completed_package_days, 365);
}

#[test]
fn enabling_requires_current_disclosure_and_ready_structured_provider() {
    let change = enabling_change(0, GenerationPolicyChangeSource::LocalInteractiveUser);
    assert_eq!(
        evolve_generation_policy(None, &change, None, 1),
        Err(GenerationPolicyError::ProviderUnavailable)
    );
    let mut stale_disclosure =
        enabling_change(0, GenerationPolicyChangeSource::LocalInteractiveUser);
    stale_disclosure.disclosure_acknowledgement = Some("old-disclosure");
    assert_eq!(
        evolve_generation_policy(None, &stale_disclosure, Some(&readiness()), 1),
        Err(GenerationPolicyError::DisclosureRequired)
    );
    let mut unavailable = readiness();
    unavailable.structured_json_supported = false;
    assert_eq!(
        evolve_generation_policy(None, &change, Some(&unavailable), 1),
        Err(GenerationPolicyError::ProviderUnavailable)
    );
    let policy = evolve_generation_policy(None, &change, Some(&readiness()), 1).expect("enabled");
    assert_eq!(policy.consent_state, GenerationConsentState::Enabled);
}

#[test]
fn imported_or_unrelated_consent_never_enables_generation() {
    let imported = enabling_change(0, GenerationPolicyChangeSource::ImportedSettings);
    assert_eq!(
        evolve_generation_policy(None, &imported, Some(&readiness()), 1),
        Err(GenerationPolicyError::ImportedConsentForbidden)
    );
    let disabled = GenerationPolicyV1::default_disabled("workspace".into());
    assert_eq!(
        authorize_outbound_generation(&disabled, Some(&readiness()), "", ""),
        GenerationOutboundAuthorization::Disabled
    );
}

#[test]
fn disclosure_upgrade_and_revocation_stop_outbound_stages() {
    let enabled = evolve_generation_policy(
        None,
        &enabling_change(0, GenerationPolicyChangeSource::LocalInteractiveUser),
        Some(&readiness()),
        1,
    )
    .expect("enabled");
    let mut disclosure_stale = enabled.clone();
    disclosure_stale.disclosure_version = "old-disclosure".into();
    assert_eq!(
        authorize_outbound_generation(
            &disclosure_stale,
            Some(&readiness()),
            &disclosure_stale.policy_hash,
            &disclosure_stale.consent_hash
        ),
        GenerationOutboundAuthorization::DisclosureStale
    );
    let revoked = evolve_generation_policy(
        Some(&enabled),
        &GenerationPolicyChangeV1 {
            workspace_id: "workspace",
            expected_revision: 1,
            requested_state: GenerationConsentState::Revoked,
            disclosure_acknowledgement: None,
            allowed_artifact_kinds: None,
            source: GenerationPolicyChangeSource::LocalInteractiveUser,
        },
        None,
        2,
    )
    .expect("revoked");
    assert_eq!(
        authorize_outbound_generation(
            &revoked,
            Some(&readiness()),
            &enabled.policy_hash,
            &enabled.consent_hash
        ),
        GenerationOutboundAuthorization::Revoked
    );
}

#[test]
fn sqlite_updates_are_conflict_safe_and_preserve_local_dossiers_on_revocation() {
    let directory = TempDirectory::new("generation-policy");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteGenerationPolicyRepository::new(database.clone());
    let enabled = update_generation_policy(
        &repository,
        &enabling_change(0, GenerationPolicyChangeSource::LocalInteractiveUser),
        Some(&readiness()),
        1,
    )
    .expect("enabled");
    assert_eq!(
        update_generation_policy(
            &repository,
            &enabling_change(0, GenerationPolicyChangeSource::LocalInteractiveUser),
            Some(&readiness()),
            2
        ),
        Err(GenerationPolicyError::Conflict)
    );
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO evolution_evidence_dossiers
        (dossier_id,schema_version,revision,input_witness_hash,builder_version,sanitizer_version,
         canonical_size_bytes,content_hash,created_at_ms) VALUES
        ('preserved',1,1,'input','builder','1',0,'dossier-hash',1)",
            [],
        )
        .expect("dossier");
    drop(connection);
    let revoked = update_generation_policy(
        &repository,
        &GenerationPolicyChangeV1 {
            workspace_id: "workspace",
            expected_revision: enabled.revision,
            requested_state: GenerationConsentState::Revoked,
            disclosure_acknowledgement: None,
            allowed_artifact_kinds: None,
            source: GenerationPolicyChangeSource::LocalInteractiveUser,
        },
        None,
        3,
    )
    .expect("revoked");
    assert_eq!(revoked.consent_state, GenerationConsentState::Revoked);
    let connection = database.connection().expect("connection");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_evidence_dossiers WHERE dossier_id='preserved'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 1);
}

fn enabling_change(
    revision: u64,
    source: GenerationPolicyChangeSource,
) -> GenerationPolicyChangeV1<'static> {
    GenerationPolicyChangeV1 {
        workspace_id: "workspace",
        expected_revision: revision,
        requested_state: GenerationConsentState::Enabled,
        disclosure_acknowledgement: Some(GENERATION_DISCLOSURE_VERSION_V1),
        allowed_artifact_kinds: None,
        source,
    }
}

fn readiness() -> GenerationProviderReadinessV1 {
    GenerationProviderReadinessV1 {
        profile_id: "profile-one".into(),
        model_id: "model-one".into(),
        provider_protocol: "openai_responses".into(),
        enabled: true,
        credentials_available: true,
        structured_json_supported: true,
    }
}
