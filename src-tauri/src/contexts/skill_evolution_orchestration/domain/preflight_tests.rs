use super::*;

type PreflightFailure = (&'static str, fn(&mut AutomaticPreflightInputV1));

fn input() -> AutomaticPreflightInputV1 {
    AutomaticPreflightInputV1 {
        run_id: "run-one".into(),
        eligibility_id: "eligibility-one".into(),
        eligibility_proof_hash: format!("sha256:{}", "a".repeat(64)),
        reservation_id: "reservation-one".into(),
        overlay_preview_hash: format!("sha256:{}", "b".repeat(64)),
        automatic_mode_enabled: true,
        policy_current: true,
        consent_current: true,
        authorization_current: true,
        allowlist_current: true,
        assessment_current: true,
        draft_current: true,
        target_current: true,
        skill_mutable: true,
        overlay_revision_current: true,
        overlay_trusted: true,
        overlay_unpinned: true,
        quality_current: true,
        rate_reserved: true,
        idle_snapshot_fresh: true,
        probation_clear: true,
        circuit_breakers_closed: true,
        issued_at_ms: 100,
    }
}

#[test]
fn witness_is_bound_to_current_preview_and_expires_in_exactly_five_seconds() {
    let first = evaluate_automatic_preflight(&input()).expect("witness");
    let second = evaluate_automatic_preflight(&input()).expect("same witness");
    assert_eq!(first, second);
    assert_eq!(first.expires_at_ms - first.issued_at_ms, 5_000);

    let mut changed = input();
    changed.overlay_preview_hash = format!("sha256:{}", "c".repeat(64));
    assert_ne!(
        evaluate_automatic_preflight(&changed)
            .expect("changed witness")
            .proof_hash,
        first.proof_hash
    );
}

#[test]
fn every_mutable_preflight_condition_fails_closed_in_stable_order() {
    let failures: [PreflightFailure; 17] = [
        ("automatic-mode-disabled", |v| {
            v.automatic_mode_enabled = false
        }),
        ("policy-stale", |v| v.policy_current = false),
        ("consent-stale", |v| v.consent_current = false),
        ("authorization-stale", |v| v.authorization_current = false),
        ("allowlist-stale", |v| v.allowlist_current = false),
        ("assessment-stale", |v| v.assessment_current = false),
        ("draft-stale", |v| v.draft_current = false),
        ("target-stale", |v| v.target_current = false),
        ("skill-immutable", |v| v.skill_mutable = false),
        ("overlay-revision-stale", |v| {
            v.overlay_revision_current = false
        }),
        ("overlay-untrusted", |v| v.overlay_trusted = false),
        ("target-pinned", |v| v.overlay_unpinned = false),
        ("quality-stale", |v| v.quality_current = false),
        ("rate-reservation-stale", |v| v.rate_reserved = false),
        ("idle-snapshot-stale", |v| v.idle_snapshot_fresh = false),
        ("probation-blocked", |v| v.probation_clear = false),
        ("circuit-breaker-open", |v| {
            v.circuit_breakers_closed = false
        }),
    ];
    for (reason, fail) in failures {
        let mut candidate = input();
        fail(&mut candidate);
        assert_eq!(
            evaluate_automatic_preflight(&candidate),
            Err(AutomaticPreflightError::Failed(reason)),
            "{reason}"
        );
    }
}
