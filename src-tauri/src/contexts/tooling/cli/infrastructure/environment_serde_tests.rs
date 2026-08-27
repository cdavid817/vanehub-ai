// Included through `#[path]` from environment_serde.rs.
//
// Pure value tests: no database, no filesystem. They pin the persisted shape and prove that every
// decode path is fallible.
use super::*;
use crate::contexts::tooling::cli::domain::installation::{derive_conflicts, select_active};
use crate::contexts::tooling::cli::domain::status::CliDiscoveryStatus;

fn stamp(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).expect("timestamp")
}

fn tool() -> CliToolId {
    CliToolId::new("claude-code").expect("tool id")
}

fn npm() -> CliSourceId {
    CliSourceId::new("npm").expect("source id")
}

fn installation(id: &str, status: CliExecutableStatus) -> CliInstallation {
    CliInstallation {
        id: CliInstallationId::new(id).expect("installation id"),
        executable_path: format!("/path/{id}"),
        canonical_path: Some(format!("/real/{id}")),
        alias_paths: vec![format!("/path/{id}.cmd")],
        target_missing: false,
        reported_version: Some(NormalizedCliVersion::parse("1.2.0")),
        source_id: Some(npm()),
        source_kind: CliSourceKind::Npm,
        source_confidence: CliSourceConfidence::Inferred,
        path_priority: Some(0),
        environment_origin: CliEnvironmentOrigin::Path,
        executable_status: status,
    }
}

fn snapshot() -> CliEnvironmentSnapshot {
    let mut snapshot = CliEnvironmentSnapshot::never_scanned(tool(), "fingerprint-a".to_string());
    snapshot.installations = vec![
        installation("a", CliExecutableStatus::Healthy),
        installation("b", CliExecutableStatus::Broken),
    ];
    snapshot.discovery = CliDiscoveryStatus::FoundMultiple;
    snapshot.authentication = CliAuthenticationStatus::Authenticated;
    snapshot.compatibility = CliCompatibilityStatus::Supported;
    snapshot.update = CliUpdateStatus::Available;
    snapshot.checked_at = Some(stamp(1_000));
    snapshot.freshness = CliFreshness::Fresh;
    snapshot.last_operation_id = Some("op-1".to_string());
    snapshot.last_mutation = Some(CliMutationSummary {
        outcome: CliMutationOutcome::AppliedUnverified,
        source_id: npm(),
        action: "upgrade".to_string(),
        target_version: Some("1.3.0".to_string()),
        operation_id: "op-1".to_string(),
        completed_at: stamp(2_000),
    });
    let selection = select_active(&snapshot.installations);
    snapshot.conflicts = derive_conflicts(&snapshot.installations, selection);
    snapshot.recompute_derived(false, false)
}

fn plan() -> CliActionPlan {
    let created_at = stamp(1_000);
    CliActionPlan {
        id: CliActionPlanId::new("plan-1").expect("plan id"),
        revision: 3,
        agent_id: tool(),
        action: CliActionKind::Upgrade,
        source_id: npm(),
        installation_id: Some(CliInstallationId::new("a").expect("id")),
        current_version: Some("1.2.0".to_string()),
        target_version: Some("1.3.0".to_string()),
        channel: Some("stable".to_string()),
        command_preview: CliCommandPreview::new(
            "npm",
            vec![
                "install".to_string(),
                "--global".to_string(),
                "p@1.3.0".to_string(),
            ],
        ),
        preconditions: vec![
            CliPrecondition::SourceExecutableAvailable {
                source: "npm".to_string(),
            },
            CliPrecondition::NetworkReachable {
                host: "registry.npmjs.org".to_string(),
            },
            CliPrecondition::ElevatedPrivileges,
        ],
        warnings: vec![CliPlanWarning::TargetIsLatestOnly],
        requires_elevation: true,
        requires_network: true,
        fallback_policy: CliFallbackPolicy::None,
        environment_fingerprint: "fingerprint-a".to_string(),
        state: CliActionPlanState::Draft,
        created_at,
        expires_at: CliActionPlan::default_expiry(created_at),
    }
}

#[test]
fn a_snapshot_survives_a_round_trip_unchanged() {
    let original = snapshot();

    let decoded = decode_snapshot(encode_snapshot(&original)).expect("decodes");

    assert_eq!(decoded, original);
    // Both identities survive, including the divergence between them.
    assert_eq!(
        decoded.path_selected_installation_id,
        original.path_selected_installation_id
    );
    assert_eq!(
        decoded.recommended_installation_id,
        original.recommended_installation_id
    );
    assert_eq!(decoded.conflicts.len(), original.conflicts.len());
}

#[test]
fn an_installation_keeps_its_aliases_and_stale_target_flag() {
    let mut original = snapshot();
    original.installations[0].target_missing = true;

    let decoded = decode_snapshot(encode_snapshot(&original)).expect("decodes");

    assert!(decoded.installations[0].target_missing);
    assert_eq!(decoded.installations[0].alias_paths, vec!["/path/a.cmd"]);
    assert_eq!(
        decoded.installations[0].canonical_path.as_deref(),
        Some("/real/a")
    );
}

#[test]
fn a_document_from_a_newer_build_is_a_typed_error_not_a_panic() {
    let mut document = encode_snapshot(&snapshot());
    document["documentVersion"] = json!(DOCUMENT_VERSION + 1);

    let error = decode_snapshot(document).expect_err("refused");

    assert!(error.contains("documentVersion"));
}

#[test]
fn malformed_snapshot_json_reports_the_field_rather_than_panicking() {
    let cases: Vec<(&str, Value)> = vec![
        ("agentId", json!(42)),
        ("overallState", json!("teleporting")),
        ("installations", json!("not an array")),
        ("checkedAt", json!("not-a-timestamp")),
        ("freshness", json!("very-fresh")),
    ];
    for (key, bad) in cases {
        let mut document = encode_snapshot(&snapshot());
        document[key] = bad;
        let error = decode_snapshot(document).expect_err(key);
        assert!(!error.is_empty(), "{key}");
    }
}

#[test]
fn a_missing_field_is_reported_by_name() {
    let mut document = encode_snapshot(&snapshot());
    as_object(&document.clone());
    document
        .as_object_mut()
        .expect("object")
        .remove("environmentFingerprint");

    let error = decode_snapshot(document).expect_err("refused");

    assert!(error.contains("environmentFingerprint"));
}

#[test]
fn a_conflicts_reason_code_is_derived_from_its_kind_not_trusted_from_the_row() {
    let mut document = encode_snapshot(&snapshot());
    // A row written by something that disagreed with itself.
    document["conflicts"][0]["reasonCode"] = json!("something-else");

    let decoded = decode_snapshot(document).expect("decodes");

    let conflict = decoded.conflicts.first().expect("conflict");
    assert_eq!(conflict.reason_code, conflict.kind.as_str());
}

#[test]
fn a_plan_survives_a_round_trip_including_its_preview_and_preconditions() {
    let original = plan();

    let decoded = decode_plan(encode_plan(&original)).expect("decodes");

    assert_eq!(decoded, original);
    assert_eq!(decoded.command_preview.args.len(), 3);
    assert_eq!(decoded.preconditions.len(), 3);
    assert_eq!(decoded.revision, 3);
}

#[test]
fn a_plan_claiming_an_unknown_fallback_policy_is_refused() {
    // A row that allows source fallback was written by something this build does not agree with,
    // and running it would be exactly the behaviour the change removes.
    let mut document = encode_plan(&plan());
    document["fallbackPolicy"] = json!("npm-on-failure");

    let error = decode_plan(document).expect_err("refused");

    assert!(error.contains("fallback policy"));
}

#[test]
fn an_unknown_plan_state_or_warning_is_refused() {
    let mut with_state = encode_plan(&plan());
    with_state["state"] = json!("half-done");
    assert!(decode_plan(with_state).is_err());

    let mut with_warning = encode_plan(&plan());
    with_warning["warnings"] = json!(["a-warning-from-the-future"]);
    assert!(decode_plan(with_warning).is_err());
}

#[test]
fn a_catalog_survives_a_round_trip_with_its_source_stamp() {
    let original = CliVersionCatalog {
        agent_id: tool(),
        source_id: npm(),
        channel: Some("stable".to_string()),
        versions: vec![
            NormalizedCliVersion::parse("1.3.0"),
            NormalizedCliVersion::parse("1.2.0"),
        ],
        latest: Some(NormalizedCliVersion::parse("1.3.0")),
        fetched_at: stamp(1_000),
        expires_at: stamp(1_900),
        status: CliCatalogStatus::Available,
    };

    let decoded = decode_catalog(encode_catalog(&original)).expect("decodes");

    assert_eq!(decoded, original);
    // The stamp is what stops another source's catalog standing in for this one.
    assert_eq!(decoded.source_id.as_str(), "npm");
}

#[test]
fn an_unavailable_catalog_keeps_its_reason() {
    let original = CliVersionCatalog::unavailable(
        tool(),
        npm(),
        Some("stable".to_string()),
        CliCatalogUnavailableReason::UnparseableOutput,
        stamp(1_000),
        stamp(1_900),
    );

    let decoded = decode_catalog(encode_catalog(&original)).expect("decodes");

    assert_eq!(decoded.status, original.status);
    assert!(!decoded.is_available());
}

#[test]
fn an_unavailable_catalog_without_a_reason_is_refused() {
    let mut document = encode_catalog(&CliVersionCatalog::unavailable(
        tool(),
        npm(),
        None,
        CliCatalogUnavailableReason::QueryFailed,
        stamp(1_000),
        stamp(1_900),
    ));
    document["unavailableReason"] = Value::Null;

    assert!(decode_catalog(document).is_err());
}

#[test]
fn a_bulk_plan_survives_a_round_trip_with_items_and_skips() {
    let original = CliBulkActionPlan {
        id: CliBulkPlanId::new("bulk-1").expect("bulk id"),
        revision: 2,
        items: vec![CliBulkActionItem {
            agent_id: tool(),
            plan_id: CliActionPlanId::new("plan-1").expect("plan id"),
            source_id: npm(),
            current_version: Some("1.2.0".to_string()),
            target_version: Some("1.3.0".to_string()),
            requires_elevation: false,
            requires_network: true,
            state: CliActionPlanState::Draft,
            skipped_reason: None,
        }],
        skipped: vec![CliBulkSkip {
            agent_id: CliToolId::new("codex-cli").expect("tool id"),
            reason: CliBulkSkipReason::InstallationConflict,
        }],
        environment_fingerprint: "fingerprint-a".to_string(),
        created_at: stamp(1_000),
        expires_at: stamp(1_600),
    };

    let decoded = decode_bulk_plan(encode_bulk_plan(&original)).expect("decodes");

    assert_eq!(decoded, original);
    assert_eq!(
        decoded.skipped[0].reason,
        CliBulkSkipReason::InstallationConflict
    );
}

#[test]
fn every_skip_reason_round_trips() {
    // A reason that encodes but does not decode would make a whole bulk plan unreadable.
    for reason in ALL_SKIP_REASONS {
        assert_eq!(skip_reason(reason.as_str()).expect("decodes"), reason);
    }
    assert!(skip_reason("invented").is_err());
}

#[test]
fn a_legacy_row_becomes_a_stale_snapshot_that_claims_nothing_it_cannot_establish() {
    let migrated = legacy_row_to_stale_snapshot(
        tool(),
        "fingerprint-a",
        Some("/usr/local/bin/claude".to_string()),
        Some("1.0.0".to_string()),
        Some(stamp(500)),
    );

    // What the old row knew.
    assert_eq!(migrated.installations.len(), 1);
    assert_eq!(
        migrated.installations[0].executable_path,
        "/usr/local/bin/claude"
    );
    assert_eq!(migrated.checked_at, Some(stamp(500)));

    // What it did not. None of these is guessed.
    assert_eq!(migrated.freshness, CliFreshness::Stale);
    assert_eq!(
        migrated.installations[0].source_confidence,
        CliSourceConfidence::Unknown
    );
    assert_eq!(migrated.authentication, CliAuthenticationStatus::Unknown);
    assert_eq!(migrated.update, CliUpdateStatus::Unknown);
    assert_eq!(
        migrated.installations[0].executable_status,
        CliExecutableStatus::Unknown
    );
    assert!(migrated.violations().is_empty());
    // And it survives persistence.
    assert_eq!(
        decode_snapshot(encode_snapshot(&migrated)).expect("decodes"),
        migrated
    );
}

#[test]
fn a_legacy_row_with_no_path_migrates_to_a_never_scanned_snapshot() {
    let migrated = legacy_row_to_stale_snapshot(tool(), "fingerprint-a", None, None, None);

    assert!(migrated.installations.is_empty());
    assert_eq!(migrated.discovery, CliDiscoveryStatus::NotScanned);
    // Not `Missing`: the legacy row established nothing.
    assert_ne!(migrated.overall_state, CliOverallState::Missing);
}

#[test]
fn every_plan_state_round_trips_through_the_column_decoder() {
    for state in [
        CliActionPlanState::Draft,
        CliActionPlanState::Executing,
        CliActionPlanState::Completed,
        CliActionPlanState::Failed,
        CliActionPlanState::Cancelled,
        CliActionPlanState::Expired,
    ] {
        assert_eq!(
            decode_plan_state(state.as_str()).expect("known state"),
            state
        );
    }
}

#[test]
fn an_unknown_plan_state_column_is_refused_rather_than_defaulted() {
    // Defaulting to `draft` here would resurrect a consumed plan; defaulting to a terminal state
    // would silently strand a live one. Neither guess is safe, so the row is simply unreadable.
    assert!(decode_plan_state("teleported").is_err());
    assert!(decode_plan_state("").is_err());
    assert!(decode_plan_state("DRAFT").is_err());
}
