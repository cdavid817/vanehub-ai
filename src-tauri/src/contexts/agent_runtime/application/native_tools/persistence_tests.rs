use super::*;

#[test]
fn every_persisted_enum_has_a_stable_round_trip() {
    for status in [
        StoredToolOperationStatus::Queued,
        StoredToolOperationStatus::AwaitingApproval,
        StoredToolOperationStatus::Running,
        StoredToolOperationStatus::AwaitingHuman,
        StoredToolOperationStatus::Succeeded,
        StoredToolOperationStatus::Failed,
        StoredToolOperationStatus::Cancelled,
    ] {
        assert_eq!(
            StoredToolOperationStatus::parse(status.as_str()),
            Some(status)
        );
    }
    for target in [DelegationTarget::ClaudeCode, DelegationTarget::CodexCli] {
        assert_eq!(DelegationTarget::parse(target.as_str()), Some(target));
    }
    for mode in [DelegationMode::Analyze, DelegationMode::Edit] {
        assert_eq!(DelegationMode::parse(mode.as_str()), Some(mode));
    }
    for status in [
        DelegationStatus::Queued,
        DelegationStatus::Running,
        DelegationStatus::Succeeded,
        DelegationStatus::Failed,
        DelegationStatus::Cancelled,
    ] {
        assert_eq!(DelegationStatus::parse(status.as_str()), Some(status));
    }
    for status in [
        ChangeSetStatus::AwaitingApproval,
        ChangeSetStatus::Preflighting,
        ChangeSetStatus::Applying,
        ChangeSetStatus::Verifying,
        ChangeSetStatus::Succeeded,
        ChangeSetStatus::RolledBack,
        ChangeSetStatus::ManualRecoveryRequired,
        ChangeSetStatus::Failed,
    ] {
        assert_eq!(ChangeSetStatus::parse(status.as_str()), Some(status));
    }
    for status in [
        RecoveryStatus::NotRequired,
        RecoveryStatus::RolledBack,
        RecoveryStatus::ManualRecoveryRequired,
    ] {
        assert_eq!(RecoveryStatus::parse(status.as_str()), Some(status));
    }
    for kind in [
        FileChangeKind::Add,
        FileChangeKind::Modify,
        FileChangeKind::Delete,
        FileChangeKind::Rename,
    ] {
        assert_eq!(FileChangeKind::parse(kind.as_str()), Some(kind));
    }
}

#[test]
fn persisted_records_use_versioned_camel_case_json() {
    let record = RecoveryRecord {
        contract_version: 1,
        apply_attempt_id: "apply-1".to_owned(),
        status: RecoveryStatus::ManualRecoveryRequired,
        recovery_reference: Some("recovery-1".to_owned()),
        safe_instructions: vec!["Review files".to_owned()],
        updated_at: "100".to_owned(),
    };
    let json = serde_json::to_value(&record).expect("serialize");
    assert_eq!(json["contractVersion"], 1);
    assert_eq!(json["applyAttemptId"], "apply-1");
    assert_eq!(json["status"], "manual_recovery_required");
    assert_eq!(
        serde_json::from_value::<RecoveryRecord>(json).expect("deserialize"),
        record
    );
}

#[test]
fn unknown_persisted_enum_values_are_rejected() {
    assert_eq!(StoredToolOperationStatus::parse("complete"), None);
    assert_eq!(DelegationTarget::parse("shell"), None);
    assert_eq!(DelegationMode::parse("execute"), None);
    assert_eq!(DelegationStatus::parse("awaiting_approval"), None);
    assert_eq!(ChangeSetStatus::parse("cancelled"), None);
    assert_eq!(RecoveryStatus::parse("pending"), None);
    assert_eq!(FileChangeKind::parse("copy"), None);
}
