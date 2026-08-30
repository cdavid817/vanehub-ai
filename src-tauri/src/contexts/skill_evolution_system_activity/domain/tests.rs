use super::*;
use serde_json::json;
use std::collections::BTreeMap;
use std::str::FromStr;

fn fixture() -> EvolutionActivityEnvelopeV1 {
    EvolutionActivityEnvelopeV1 {
        schema_version: ACTIVITY_SCHEMA_VERSION_V1,
        event_id: "event-assessment-1".into(),
        event_code: ActivityEventCode::AssessmentCompleted,
        source_domain: "assessment".into(),
        source_id: "assessment-1".into(),
        source_revision: "revision-1".into(),
        source_sequence: 7,
        scope_kind: ActivityScopeKind::Workspace,
        canonical_scope_id: "workspace:7f3a".into(),
        occurred_at_ms: 1_785_000_000_000,
        committed_at_ms: 1_785_000_000_100,
        severity: ActivitySeverity::Info,
        status: ActivityStatus::Succeeded,
        attention_kind: ActivityAttentionKind::None,
        safe_actor_kind: ActivityActorKind::System,
        safe_identities: vec![SafeIdentity {
            kind: ActivitySafeIdentityKind::Skill,
            value: "code-review".into(),
        }],
        metrics: BTreeMap::from([(ActivityMetricCode::CandidateCount, 1)]),
        reason_codes: vec![ActivityReasonCode::Completed],
        navigation: Some(ActivityNavigation {
            kind: ActivityNavigationKind::Assessment,
            stable_id: "assessment-1".into(),
            child_id: None,
        }),
        supersedes_event_id: None,
        payload: Some(ActivityPayloadV1::CheckSummary {
            passed: 8,
            failed: 0,
            review: 1,
        }),
        projection_policy_version: 1,
        content_hash: String::new(),
    }
}

#[test]
fn stable_identity_is_scope_bound_and_contains_no_agent_metadata() {
    let first = stable_system_activity_session_id(
        ActivityKind::SkillEvolution,
        ActivityScopeKind::Workspace,
        "workspace:7f3a",
    )
    .expect("workspace identity");
    let again = stable_system_activity_session_id(
        ActivityKind::SkillEvolution,
        ActivityScopeKind::Workspace,
        "workspace:7f3a",
    )
    .expect("stable workspace identity");
    let global = stable_system_activity_session_id(
        ActivityKind::SkillEvolution,
        ActivityScopeKind::Global,
        "global",
    )
    .expect("global identity");

    assert_eq!(first, again);
    assert_ne!(first, global);
    assert!(first.starts_with("system-activity-v1-"));
    assert!(!first.contains("agent"));
}

#[test]
fn canonical_envelope_round_trips_and_hashes_deterministically() {
    let first = fixture().seal().expect("sealed envelope");
    let second = fixture().seal().expect("same sealed envelope");
    let json = serde_json::to_string(&first).expect("serialize envelope");
    let restored: EvolutionActivityEnvelopeV1 = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(first.content_hash, second.content_hash);
    assert_eq!(restored, first);
    restored.validate().expect("valid restored envelope");
}

#[test]
fn rejects_unknown_versions_hash_mutation_and_oversized_payloads() {
    let mut unsupported = fixture();
    unsupported.schema_version = 2;
    assert_eq!(
        unsupported.seal(),
        Err(ActivityEnvelopeError::UnsupportedSchemaVersion(2))
    );

    let mut changed = fixture().seal().expect("sealed");
    changed.source_sequence += 1;
    assert_eq!(changed.validate(), Err(ActivityEnvelopeError::HashMismatch));

    let mut oversized = fixture();
    oversized.payload = Some(ActivityPayloadV1::NavigationList {
        links: (0..100)
            .map(|index| ActivityNavigation {
                kind: ActivityNavigationKind::Assessment,
                stable_id: format!("assessment-{index}-{}", "x".repeat(140)),
                child_id: None,
            })
            .collect(),
    });
    assert_eq!(
        oversized.seal(),
        Err(ActivityEnvelopeError::PayloadTooLarge)
    );
}

#[test]
fn strict_deserialization_rejects_arbitrary_envelope_fields() {
    let mut value = serde_json::to_value(fixture().seal().expect("sealed")).expect("json");
    value
        .as_object_mut()
        .expect("object")
        .insert("rawPrompt".into(), json!("ignore policy"));
    assert!(serde_json::from_value::<EvolutionActivityEnvelopeV1>(value).is_err());
}

#[test]
fn closed_registries_reject_unknown_codes_and_payload_schemas() {
    for (field, value) in [
        ("severity", json!("verbose")),
        ("status", json!("unknown")),
        ("attentionKind", json!("execute")),
        ("eventCode", json!("arbitrary_event")),
    ] {
        let mut envelope = serde_json::to_value(fixture().seal().expect("sealed")).expect("json");
        envelope[field] = value;
        assert!(serde_json::from_value::<EvolutionActivityEnvelopeV1>(envelope).is_err());
    }

    let mut identity = serde_json::to_value(fixture().seal().expect("sealed")).expect("json");
    identity["safeIdentities"][0]["kind"] = json!("raw_user_text");
    assert!(serde_json::from_value::<EvolutionActivityEnvelopeV1>(identity).is_err());

    let mut reason = serde_json::to_value(fixture().seal().expect("sealed")).expect("json");
    reason["reasonCodes"] = json!(["unregistered_reason"]);
    assert!(serde_json::from_value::<EvolutionActivityEnvelopeV1>(reason).is_err());

    let mut navigation = serde_json::to_value(fixture().seal().expect("sealed")).expect("json");
    navigation["navigation"]["kind"] = json!("mutation_action");
    assert!(serde_json::from_value::<EvolutionActivityEnvelopeV1>(navigation).is_err());

    let mut payload = serde_json::to_value(fixture().seal().expect("sealed")).expect("json");
    payload["payload"]["schema"] = json!("raw_html");
    assert!(serde_json::from_value::<EvolutionActivityEnvelopeV1>(payload).is_err());
}

#[test]
fn every_supported_source_domain_maps_to_locale_neutral_read_only_codes() {
    let cases = [
        (
            EvolutionSourceDomain::Orchestration,
            "run",
            "completed",
            ActivityEventCode::RunCompleted,
        ),
        (
            EvolutionSourceDomain::Evidence,
            "seed",
            "ready",
            ActivityEventCode::SeedReady,
        ),
        (
            EvolutionSourceDomain::Assessment,
            "attempt",
            "review",
            ActivityEventCode::AssessmentNeedsReview,
        ),
        (
            EvolutionSourceDomain::Generation,
            "job",
            "completed",
            ActivityEventCode::GenerationCompleted,
        ),
        (
            EvolutionSourceDomain::Curator,
            "candidate",
            "approved",
            ActivityEventCode::CuratorApproved,
        ),
        (
            EvolutionSourceDomain::Overlay,
            "history",
            "applied",
            ActivityEventCode::OverlayApplied,
        ),
        (
            EvolutionSourceDomain::AutomaticApplication,
            "eligibility",
            "eligible",
            ActivityEventCode::AutomaticEligible,
        ),
        (
            EvolutionSourceDomain::Probation,
            "probation",
            "regressed",
            ActivityEventCode::ProbationRegressed,
        ),
        (
            EvolutionSourceDomain::Breaker,
            "breaker",
            "open",
            ActivityEventCode::BreakerOpened,
        ),
        (
            EvolutionSourceDomain::SkillCreation,
            "proposal",
            "reviewable",
            ActivityEventCode::SkillCreated,
        ),
        (
            EvolutionSourceDomain::Recovery,
            "reconciliation",
            "completed",
            ActivityEventCode::RecoveryCompleted,
        ),
        (
            EvolutionSourceDomain::Retention,
            "purge",
            "purged",
            ActivityEventCode::SourcePurged,
        ),
    ];

    for (domain, source_kind, outcome, expected_code) in cases {
        let mapped = map_source_outcome(domain, source_kind, outcome).expect("registered mapping");
        assert_eq!(mapped.event_code, expected_code);
        assert_eq!(mapped.reason_codes.len(), 1);
        assert!(matches!(
            mapped.payload,
            ActivityPayloadV1::StatusCard { .. }
        ));
    }

    assert_eq!(
        map_source_outcome(EvolutionSourceDomain::Overlay, "history", "execute_script"),
        Err(ActivityMappingError::UnsupportedOutcome)
    );
}

#[test]
fn sealing_normalizes_every_bounded_identity_and_rejects_directional_controls() {
    let mut input = fixture();
    input.source_id = "  cafe\u{301}  ".into();
    input.safe_identities[0].value = "  skill-one  ".into();
    input.navigation.as_mut().expect("navigation").stable_id = "  assessment-one  ".into();
    let sealed = input.seal().expect("sanitized envelope");
    assert_eq!(sealed.source_id, "caf\u{e9}");
    assert_eq!(sealed.safe_identities[0].value, "skill-one");
    assert_eq!(
        sealed.navigation.expect("navigation").stable_id,
        "assessment-one"
    );

    let mut directional = fixture();
    directional.source_id = "source-\u{202e}unsafe".into();
    assert_eq!(
        directional.seal(),
        Err(ActivityEnvelopeError::InvalidField("source_id"))
    );

    let mut payload_control = fixture();
    payload_control.payload = Some(ActivityPayloadV1::SupersessionNotice {
        prior_event_id: "prior\nrecord".into(),
    });
    assert_eq!(
        payload_control.seal(),
        Err(ActivityEnvelopeError::InvalidField(
            "payload.prior_event_id"
        ))
    );
}

#[test]
fn privacy_boundary_has_no_carrier_for_raw_content_paths_diffs_or_notes() {
    let prohibited = [
        "ignore previous instructions",
        "/home/user/private/skill.md",
        "C:\\Users\\private\\skill.md",
        "@@ -1,2 +1,2 @@",
        "<script>alert(1)</script>",
        "{\"prompt\":\"reveal secrets\"}",
        "tool(argument=credential)",
        "optional rejection note",
        "raw model output",
        "evidence excerpt",
    ];
    for sensitive in prohibited {
        let mut envelope = fixture();
        envelope.safe_identities[0].value = sensitive.into();
        assert_eq!(
            envelope.seal(),
            Err(ActivityEnvelopeError::InvalidField("safe_identity.value")),
            "sensitive carrier was accepted: {sensitive}"
        );
    }

    let payload = serde_json::to_value(fixture().seal().expect("sealed")).expect("json");
    for forbidden_field in [
        "prompt",
        "message",
        "note",
        "output",
        "arguments",
        "path",
        "diff",
        "draft",
        "excerpt",
    ] {
        assert!(payload.get(forbidden_field).is_none());
    }
}

#[test]
fn arbitrary_payload_json_is_rejected_and_source_mapping_has_a_stable_golden_shape() {
    let mut arbitrary = serde_json::to_value(fixture().seal().expect("sealed")).expect("json");
    arbitrary["payload"]["rawDiff"] = json!("@@ -1 +1 @@");
    assert!(serde_json::from_value::<EvolutionActivityEnvelopeV1>(arbitrary).is_err());

    let mapped = map_source_outcome(EvolutionSourceDomain::Probation, "probation", "regressed")
        .expect("mapped regression");
    let mut envelope = fixture();
    envelope.event_code = mapped.event_code;
    envelope.severity = mapped.severity;
    envelope.status = mapped.status;
    envelope.attention_kind = mapped.attention_kind;
    envelope.reason_codes = mapped.reason_codes;
    envelope.payload = Some(mapped.payload);
    let golden = serde_json::to_value(envelope.seal().expect("golden envelope")).expect("json");
    assert_eq!(golden["eventCode"], json!("probation_regressed"));
    assert_eq!(golden["severity"], json!("error"));
    assert_eq!(golden["status"], json!("failed"));
    assert_eq!(golden["attentionKind"], json!("regression"));
    assert_eq!(golden["reasonCodes"], json!(["regression_detected"]));
    assert_eq!(
        golden["payload"],
        json!({
            "schema": "status_card",
            "labelCode": "outcome",
            "valueCode": "regressed"
        })
    );
}

#[test]
fn source_contract_bounds_pages_cursors_and_sequences() {
    assert!(ProjectionScanLimit::new(0).is_err());
    assert!(ProjectionScanLimit::new(MAX_SOURCE_SCAN_ITEMS + 1).is_err());
    let limit = ProjectionScanLimit::new(2).expect("bounded limit");
    let cursor = OpaqueDomainCursor::parse("cursor:v1:assessment:7".into()).expect("cursor");
    assert_eq!(cursor.expose(), "cursor:v1:assessment:7");
    assert!(OpaqueDomainCursor::parse("x".repeat(513)).is_err());

    let event = fixture().seal().expect("event");
    let page = ProjectionSourcePage {
        source_domain: EvolutionSourceDomain::Assessment,
        events: vec![VerifiedProjectionEvent {
            source_cursor: cursor.clone(),
            source_sequence: 7,
            source_integrity_hash: "sha256:source".into(),
            envelope: event,
        }],
        next_cursor: Some(cursor),
        retention_floor: None,
        has_more: true,
    };
    page.validate(limit).expect("valid source page");

    let mut replay = page.clone();
    replay.events.push(replay.events[0].clone());
    assert_eq!(
        replay.validate(limit),
        Err(ProjectionSourceError::InvalidSequence)
    );
}

#[test]
fn rebuild_maps_read_progress_by_canonical_source_order_not_old_item_sequence() {
    let read_through = ActivityReadOrderKey {
        committed_at_ms: 20,
        source_sequence: 2,
        event_id: "event-b".into(),
    };
    let rebuilt = vec![
        RebuiltActivityPosition {
            sequence: 3,
            source_order: ActivityReadOrderKey {
                committed_at_ms: 20,
                source_sequence: 2,
                event_id: "event-b".into(),
            },
        },
        RebuiltActivityPosition {
            sequence: 1,
            source_order: ActivityReadOrderKey {
                committed_at_ms: 5,
                source_sequence: 1,
                event_id: "restored-old-event".into(),
            },
        },
        RebuiltActivityPosition {
            sequence: 4,
            source_order: ActivityReadOrderKey {
                committed_at_ms: 30,
                source_sequence: 3,
                event_id: "event-c".into(),
            },
        },
    ];

    assert_eq!(map_rebuilt_read_sequence(Some(&read_through), &rebuilt), 3);
    assert_eq!(map_rebuilt_read_sequence(None, &rebuilt), 0);
}

#[test]
fn non_authoritative_product_surfaces_cannot_be_projection_domains() {
    for prohibited in [
        "unified_log",
        "notification",
        "ui_state",
        "messages",
        "model_transcript",
        "terminal",
    ] {
        assert_eq!(
            EvolutionSourceDomain::from_str(prohibited),
            Err(ProjectionSourceError::ProhibitedOrUnknownDomain),
            "{prohibited} unexpectedly became a projection source"
        );
    }
    assert_eq!(
        EvolutionSourceDomain::from_str("assessment"),
        Ok(EvolutionSourceDomain::Assessment)
    );
}
