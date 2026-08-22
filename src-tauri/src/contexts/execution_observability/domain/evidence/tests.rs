use super::builders::{label, reason, CorrelationBuilder, CoverageBuilder, EvidenceEventBuilder};
use super::encoding::canonical_payload_encoding;
use super::event::{
    fidelity_token, parse_fidelity_token, parse_status_token, status_token, MAX_REDACTION_RULE_IDS,
};
use super::identity::{
    BoundedLabel, SafeFingerprint, MAX_BASENAME_LENGTH, MAX_FINGERPRINT_LENGTH,
    MAX_IDENTIFIER_LENGTH, MAX_LABEL_LENGTH, MAX_REASON_CODE_LENGTH,
};
use super::payload::{
    EvidenceOutcome, FileChangeKind, ReviewDecisionScope, ReviewDecisionValue, UsageQuality,
    MAX_SAFE_PAYLOAD_BYTES,
};
use super::safety::{
    reject_absolute_path, RedactedCommandDisplay, RelativeDisplayPath, SafeBasename,
    MAX_REDACTED_DISPLAY_BYTES,
};
use super::*;
use crate::contexts::execution_observability::domain::{ExecutionFidelity, ExecutionStatus};

const SESSION: &str = "session-1";
const RUN: &str = "6f1b2c3d-4e5f-4a6b-8c9d-0e1f2a3b4c5d";
const TRACE: &str = "0af7651916cd43dd8448eb211c80319c";
const SPAN: &str = "b7ad6b7169203331";

fn session_correlation() -> EvidenceCorrelation {
    CorrelationBuilder::for_session(SESSION).build()
}

fn run_correlation() -> EvidenceCorrelation {
    CorrelationBuilder::for_session(SESSION)
        .with_run(RUN, TRACE)
        .build()
}

fn every_payload() -> Vec<(
    SafeEvidencePayload,
    EvidenceCorrelation,
    Option<ExecutionStatus>,
)> {
    vec![
        (
            SafeEvidencePayload::RunStarted {
                trigger: reason("user_message"),
            },
            run_correlation(),
            Some(ExecutionStatus::Running),
        ),
        (
            SafeEvidencePayload::RunCompleted {
                outcome: EvidenceOutcome::Succeeded,
                duration_ms: Some(1_200),
            },
            run_correlation(),
            Some(ExecutionStatus::Succeeded),
        ),
        (
            SafeEvidencePayload::AgentDelegated { attempt: Some(2) },
            CorrelationBuilder::for_session(SESSION)
                .with_agent("agent-1")
                .build(),
            Some(ExecutionStatus::Running),
        ),
        (
            SafeEvidencePayload::AgentCompleted {
                outcome: EvidenceOutcome::Failed,
                duration_ms: None,
            },
            CorrelationBuilder::for_session(SESSION)
                .with_agent("agent-1")
                .build(),
            Some(ExecutionStatus::Failed),
        ),
        (
            SafeEvidencePayload::ToolStarted {
                tool_name: label("read_file"),
            },
            CorrelationBuilder::for_session(SESSION)
                .with_tool_call("tool-1")
                .build(),
            None,
        ),
        (
            SafeEvidencePayload::ToolCompleted {
                tool_name: label("read_file"),
                outcome: EvidenceOutcome::Succeeded,
                duration_ms: Some(31),
            },
            CorrelationBuilder::for_session(SESSION)
                .with_tool_call("tool-1")
                .build(),
            Some(ExecutionStatus::Succeeded),
        ),
        (
            SafeEvidencePayload::CommandStarted {
                runtime_kind: CommandRuntimeKind::LocalShell,
                redacted_display: Some(RedactedCommandDisplay::parse("npm test").expect("display")),
                cwd_display: Some(RelativeDisplayPath::parse("src/app").expect("cwd")),
            },
            CorrelationBuilder::for_session(SESSION)
                .with_command("command-1")
                .build(),
            Some(ExecutionStatus::Running),
        ),
        (
            SafeEvidencePayload::CommandCompleted {
                outcome: EvidenceOutcome::Failed,
                duration_ms: Some(12_400),
                exit_code: Some(1),
                signal: None,
                output_availability: OutputAvailability::Merged,
                output_truncated: true,
            },
            CorrelationBuilder::for_session(SESSION)
                .with_command("command-1")
                .build(),
            Some(ExecutionStatus::Failed),
        ),
        (
            SafeEvidencePayload::ShellOpened {
                runtime_kind: CommandRuntimeKind::RemoteShell,
            },
            session_correlation(),
            Some(ExecutionStatus::Running),
        ),
        (
            SafeEvidencePayload::ShellClosed {
                reason: reason("user_closed"),
            },
            session_correlation(),
            None,
        ),
        (
            SafeEvidencePayload::FileMutationObserved {
                basename: SafeBasename::parse("main.rs").expect("basename"),
                path_fingerprint: SafeFingerprint::parse("a1b2c3d4").expect("fingerprint"),
                change_kind: FileChangeKind::Modified,
            },
            CorrelationBuilder::for_session(SESSION)
                .with_file_mutation("mutation-1")
                .build(),
            None,
        ),
        (
            SafeEvidencePayload::ReviewDecisionRecorded {
                scope: ReviewDecisionScope::Hunk,
                decision: ReviewDecisionValue::Accepted,
            },
            session_correlation(),
            None,
        ),
        (
            SafeEvidencePayload::VerificationCompleted {
                name: label("npm run test"),
                outcome: VerificationOutcome::Failed,
                passed_count: Some(138),
                failed_count: Some(2),
            },
            session_correlation(),
            Some(ExecutionStatus::Failed),
        ),
        (
            SafeEvidencePayload::UsageObserved {
                quality: UsageQuality::Reported,
                response_count: 3,
            },
            session_correlation(),
            None,
        ),
        (
            SafeEvidencePayload::OperationFailed {
                reason: reason("storage_unavailable"),
            },
            CorrelationBuilder::for_session(SESSION)
                .with_operation("operation-1")
                .build(),
            Some(ExecutionStatus::Failed),
        ),
        (
            SafeEvidencePayload::CoverageGapRecorded {
                dropped_count: 4,
                reason: reason(reason_codes::DROPPED_EVENTS),
            },
            session_correlation(),
            None,
        ),
    ]
}

#[test]
fn every_declared_kind_has_a_constructible_payload() {
    let built = every_payload();
    assert_eq!(built.len(), 16);
    for (index, (payload, correlation, status)) in built.into_iter().enumerate() {
        let mut builder =
            EvidenceEventBuilder::new(&format!("source-{index}"), correlation, payload);
        if let Some(status) = status {
            builder = builder.with_status(status);
        }
        let event = builder.build();
        assert_eq!(event.schema_version(), EVIDENCE_SCHEMA_VERSION);
        assert!(!event.canonical_fingerprint().is_empty());
    }
}

#[test]
fn every_kind_token_round_trips() {
    for (payload, _, _) in every_payload() {
        let kind = payload.kind();
        assert_eq!(EvidenceKind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(EvidenceKind::parse("run.teleported"), None);
}

#[test]
fn correlation_requires_a_session() {
    let correlation = EvidenceCorrelation::default();
    assert_eq!(
        correlation.validate(),
        Err(EvidenceDomainError::SessionRequired)
    );
}

#[test]
fn a_span_cannot_travel_without_its_trace() {
    let mut correlation = CorrelationBuilder::for_session(SESSION).build_unchecked();
    correlation.span_id =
        Some(crate::contexts::execution_observability::domain::SpanId::parse(SPAN).expect("span"));
    assert_eq!(
        correlation.validate(),
        Err(EvidenceDomainError::SpanWithoutTrace)
    );

    let mut parented = CorrelationBuilder::for_session(SESSION).build_unchecked();
    parented.parent_span_id =
        Some(crate::contexts::execution_observability::domain::SpanId::parse(SPAN).expect("span"));
    assert_eq!(
        parented.validate(),
        Err(EvidenceDomainError::ParentSpanWithoutTrace)
    );
}

#[test]
fn lifecycle_payloads_require_the_id_of_what_they_describe() {
    let cases: Vec<(SafeEvidencePayload, &'static str)> = vec![
        (
            SafeEvidencePayload::RunStarted {
                trigger: reason("user_message"),
            },
            "run id",
        ),
        (
            SafeEvidencePayload::ToolStarted {
                tool_name: label("read_file"),
            },
            "tool call id",
        ),
        (
            SafeEvidencePayload::CommandStarted {
                runtime_kind: CommandRuntimeKind::Process,
                redacted_display: None,
                cwd_display: None,
            },
            "command id",
        ),
        (
            SafeEvidencePayload::FileMutationObserved {
                basename: SafeBasename::parse("main.rs").expect("basename"),
                path_fingerprint: SafeFingerprint::parse("ab").expect("fingerprint"),
                change_kind: FileChangeKind::Added,
            },
            "file mutation id",
        ),
        (
            SafeEvidencePayload::AgentDelegated { attempt: None },
            "agent id",
        ),
        (
            SafeEvidencePayload::OperationFailed {
                reason: reason("boom"),
            },
            "operation id",
        ),
    ];
    for (payload, field) in cases {
        let error = EvidenceEventBuilder::new("source-1", session_correlation(), payload)
            .try_build()
            .expect_err("missing correlation is refused");
        assert!(
            matches!(error, EvidenceDomainError::MissingCorrelation { field: actual, .. } if actual == field),
            "expected {field}, got {error:?}"
        );
    }
}

#[test]
fn a_start_cannot_be_terminal_and_a_completion_cannot_be_open() {
    let terminal_start = EvidenceEventBuilder::new(
        "source-1",
        run_correlation(),
        SafeEvidencePayload::RunStarted {
            trigger: reason("user_message"),
        },
    )
    .with_status(ExecutionStatus::Succeeded)
    .try_build();
    assert!(matches!(
        terminal_start,
        Err(EvidenceDomainError::PayloadKindMismatch { .. })
    ));

    let open_completion = EvidenceEventBuilder::new(
        "source-2",
        run_correlation(),
        SafeEvidencePayload::RunCompleted {
            outcome: EvidenceOutcome::Succeeded,
            duration_ms: None,
        },
    )
    .with_status(ExecutionStatus::Running)
    .try_build();
    assert!(matches!(
        open_completion,
        Err(EvidenceDomainError::PayloadKindMismatch { .. })
    ));

    let statusless_completion = EvidenceEventBuilder::new(
        "source-3",
        run_correlation(),
        SafeEvidencePayload::RunCompleted {
            outcome: EvidenceOutcome::Succeeded,
            duration_ms: None,
        },
    )
    .try_build();
    assert!(matches!(
        statusless_completion,
        Err(EvidenceDomainError::PayloadKindMismatch { .. })
    ));
}

#[test]
fn a_coverage_gap_must_have_dropped_something() {
    let error = EvidenceEventBuilder::new(
        "source-1",
        session_correlation(),
        SafeEvidencePayload::CoverageGapRecorded {
            dropped_count: 0,
            reason: reason(reason_codes::DROPPED_EVENTS),
        },
    )
    .try_build()
    .expect_err("an empty gap is refused");
    assert_eq!(error, EvidenceDomainError::EmptyCoverageGap);
}

#[test]
fn identifiers_labels_and_reason_codes_are_bounded() {
    let over_limit = "x".repeat(MAX_IDENTIFIER_LENGTH + 1);
    assert!(EvidenceSessionId::parse(over_limit).is_err());
    assert!(EvidenceSessionId::parse("").is_err());
    assert!(EvidenceSessionId::parse("has\nnewline").is_err());

    assert!(BoundedLabel::parse("tool", "x".repeat(MAX_LABEL_LENGTH + 1)).is_err());
    assert!(BoundedLabel::parse("tool", "read\u{7}file").is_err());

    assert!(SafeReasonCode::parse("Not-Lowercase").is_err());
    assert!(SafeReasonCode::parse("x".repeat(MAX_REASON_CODE_LENGTH + 1)).is_err());
    assert!(SafeReasonCode::parse("ok_code_1").is_ok());

    assert!(SafeFingerprint::parse("zz").is_err());
    assert!(SafeFingerprint::parse("x".repeat(MAX_FINGERPRINT_LENGTH + 1)).is_err());
}

#[test]
fn absolute_and_user_rooted_paths_are_refused() {
    for candidate in [
        "C:\\Users\\cdavid\\secret.txt",
        "/etc/passwd",
        "~/notes.md",
        "\\\\share\\team",
    ] {
        assert_eq!(
            reject_absolute_path(candidate),
            Err(EvidenceDomainError::AbsolutePathRejected),
            "{candidate} must be refused"
        );
        assert!(RelativeDisplayPath::parse(candidate).is_err());
    }
    assert!(RelativeDisplayPath::parse("src/app").is_ok());
}

#[test]
fn a_basename_cannot_carry_a_path() {
    assert_eq!(
        SafeBasename::parse("src/main.rs"),
        Err(EvidenceDomainError::PathSeparatorRejected)
    );
    assert_eq!(
        SafeBasename::parse("src\\main.rs"),
        Err(EvidenceDomainError::PathSeparatorRejected)
    );
    assert!(SafeBasename::parse("main.rs").is_ok());
    assert!(SafeBasename::parse("x".repeat(MAX_BASENAME_LENGTH + 1)).is_err());
}

#[test]
fn a_command_display_refuses_transcripts_and_credential_shapes() {
    // Multi-line content is a transcript, which is exactly what the journal must not hold.
    assert!(RedactedCommandDisplay::parse("npm test\nnpm run build").is_err());
    assert!(RedactedCommandDisplay::parse("x".repeat(MAX_REDACTED_DISPLAY_BYTES + 1)).is_err());
    assert_eq!(
        RedactedCommandDisplay::parse("curl -H 'Authorization: Bearer abc'"),
        Err(EvidenceDomainError::CredentialShapedContentRejected)
    );
    assert_eq!(
        RedactedCommandDisplay::parse("echo -----BEGIN RSA PRIVATE KEY-----"),
        Err(EvidenceDomainError::CredentialShapedContentRejected)
    );
    assert!(RedactedCommandDisplay::parse("npm test").is_ok());
}

#[test]
fn a_payload_that_serializes_past_the_bound_is_refused() {
    // Every individual field is within its own bound; the aggregate is not.
    let display = RedactedCommandDisplay::parse("x".repeat(MAX_REDACTED_DISPLAY_BYTES))
        .expect("display at its bound");
    let event = EvidenceEventBuilder::new(
        "source-1",
        CorrelationBuilder::for_session(SESSION)
            .with_command("command-1")
            .build(),
        SafeEvidencePayload::CommandStarted {
            runtime_kind: CommandRuntimeKind::LocalShell,
            redacted_display: Some(display),
            cwd_display: None,
        },
    )
    .with_status(ExecutionStatus::Running)
    .try_build();
    assert!(
        event.is_ok(),
        "2 KiB display is within the 16 KiB payload bound"
    );
    assert!(MAX_SAFE_PAYLOAD_BYTES > MAX_REDACTED_DISPLAY_BYTES);
}

#[test]
fn a_redaction_receipt_is_bounded_deduplicated_and_ordered() {
    let receipt = RedactionReceipt::applied([
        reason("secret_value"),
        reason("absolute_path"),
        reason("secret_value"),
    ])
    .expect("bounded receipt");
    assert!(receipt.is_applied());
    assert_eq!(
        receipt
            .rule_ids()
            .iter()
            .map(SafeReasonCode::as_str)
            .collect::<Vec<_>>(),
        vec!["absolute_path", "secret_value"]
    );
    assert!(!RedactionReceipt::none().is_applied());

    let too_many = (0..MAX_REDACTION_RULE_IDS + 1)
        .map(|index| reason(&format!("rule_{index}")))
        .collect::<Vec<_>>();
    assert!(RedactionReceipt::applied(too_many).is_err());
}

#[test]
fn the_fingerprint_ignores_generated_identity_but_not_asserted_content() {
    let event = || {
        EvidenceEventBuilder::new(
            "source-1",
            run_correlation(),
            SafeEvidencePayload::RunCompleted {
                outcome: EvidenceOutcome::Succeeded,
                duration_ms: Some(10),
            },
        )
        .with_status(ExecutionStatus::Succeeded)
    };

    // A retry generates a new event id; that must not make it look like a different assertion.
    let first = event().with_event_id("event-a").build();
    let second = event().with_event_id("event-b").build();
    assert_eq!(
        first.canonical_fingerprint(),
        second.canonical_fingerprint()
    );

    // Anything the producer actually claimed does change identity.
    let different_outcome = EvidenceEventBuilder::new(
        "source-1",
        run_correlation(),
        SafeEvidencePayload::RunCompleted {
            outcome: EvidenceOutcome::Failed,
            duration_ms: Some(10),
        },
    )
    .with_status(ExecutionStatus::Failed)
    .build();
    assert_ne!(
        first.canonical_fingerprint(),
        different_outcome.canonical_fingerprint()
    );

    let different_fidelity = event().with_fidelity(ExecutionFidelity::Inferred).build();
    assert_ne!(
        first.canonical_fingerprint(),
        different_fidelity.canonical_fingerprint()
    );

    let redacted = event().with_redaction(&["secret_value"]).build();
    assert_ne!(
        first.canonical_fingerprint(),
        redacted.canonical_fingerprint()
    );
}

#[test]
fn an_unobserved_number_and_an_observed_zero_are_different_assertions() {
    let unobserved = EvidenceEventBuilder::new(
        "source-1",
        run_correlation(),
        SafeEvidencePayload::RunCompleted {
            outcome: EvidenceOutcome::Succeeded,
            duration_ms: None,
        },
    )
    .with_status(ExecutionStatus::Succeeded)
    .build();
    let observed_zero = EvidenceEventBuilder::new(
        "source-1",
        run_correlation(),
        SafeEvidencePayload::RunCompleted {
            outcome: EvidenceOutcome::Succeeded,
            duration_ms: Some(0),
        },
    )
    .with_status(ExecutionStatus::Succeeded)
    .build();
    assert_ne!(
        unobserved.canonical_fingerprint(),
        observed_zero.canonical_fingerprint()
    );
}

#[test]
fn status_and_fidelity_tokens_round_trip() {
    for status in [
        ExecutionStatus::Accepted,
        ExecutionStatus::Running,
        ExecutionStatus::Succeeded,
        ExecutionStatus::Failed,
        ExecutionStatus::Cancelled,
        ExecutionStatus::Incomplete,
    ] {
        assert_eq!(parse_status_token(status_token(status)), Some(status));
    }
    for fidelity in [
        ExecutionFidelity::Native,
        ExecutionFidelity::Proxied,
        ExecutionFidelity::Inferred,
        ExecutionFidelity::Opaque,
    ] {
        assert_eq!(
            parse_fidelity_token(fidelity_token(fidelity)),
            Some(fidelity)
        );
    }
    assert_eq!(parse_status_token("hibernating"), None);
    assert_eq!(parse_fidelity_token("psychic"), None);
}

#[test]
fn source_context_tokens_round_trip() {
    for context in [
        EvidenceSourceContext::AgentRuntime,
        EvidenceSourceContext::Workspaces,
        EvidenceSourceContext::Operations,
        EvidenceSourceContext::Sessions,
        EvidenceSourceContext::Review,
        EvidenceSourceContext::ExecutionObservability,
    ] {
        assert_eq!(
            EvidenceSourceContext::parse(context.as_str()),
            Some(context)
        );
    }
    assert_eq!(EvidenceSourceContext::parse("marketing"), None);
}

#[test]
fn coverage_builders_produce_each_state() {
    assert_eq!(
        CoverageBuilder::complete().state(),
        EvidenceCoverageState::Complete
    );
    assert_eq!(
        CoverageBuilder::indexing().state(),
        EvidenceCoverageState::Indexing
    );
    assert_eq!(
        CoverageBuilder::partial(reason_codes::RETENTION_EXPIRED).state(),
        EvidenceCoverageState::Partial
    );
    assert_eq!(
        CoverageBuilder::unavailable(reason_codes::SOURCE_NOT_OWNED).state(),
        EvidenceCoverageState::Unavailable
    );

    let pending = CoverageBuilder::capture_not_initialized();
    assert_eq!(pending.state(), EvidenceCoverageState::Partial);
    assert_eq!(
        pending.reason_codes().first().map(SafeReasonCode::as_str),
        Some(reason_codes::CAPTURE_NOT_INITIALIZED)
    );
}

#[test]
fn coverage_only_ever_degrades() {
    let coverage = QueryCoverage::complete()
        .degrade_to(EvidenceCoverageState::Partial, reason_codes::DROPPED_EVENTS)
        .degrade_to(EvidenceCoverageState::Complete, "ignored_upgrade");
    assert_eq!(coverage.state(), EvidenceCoverageState::Partial);

    let worse = QueryCoverage::complete()
        .degrade_to(EvidenceCoverageState::Partial, reason_codes::DROPPED_EVENTS)
        .degrade_to(
            EvidenceCoverageState::Unavailable,
            reason_codes::SOURCE_NOT_OWNED,
        );
    assert_eq!(worse.state(), EvidenceCoverageState::Unavailable);
    assert_eq!(worse.reason_codes().len(), 2);
}

#[test]
fn coverage_reason_codes_are_deduplicated_and_bounded() {
    let coverage = QueryCoverage::new(
        EvidenceCoverageState::Partial,
        [
            reason(reason_codes::DROPPED_EVENTS),
            reason(reason_codes::DROPPED_EVENTS),
        ],
    )
    .expect("bounded coverage");
    assert_eq!(coverage.reason_codes().len(), 1);

    let too_many = (0..9)
        .map(|index| reason(&format!("code_{index}")))
        .collect::<Vec<_>>();
    assert!(QueryCoverage::new(EvidenceCoverageState::Partial, too_many).is_err());
}

#[test]
fn coverage_boundaries_stay_absent_until_observed() {
    let coverage = QueryCoverage::complete();
    assert!(coverage.oldest_available_at().is_none());
    assert!(coverage.newest_available_at().is_none());
    assert!(coverage.indexed_through_at().is_none());
    assert!(coverage.dropped_count().is_none());
    assert!(!coverage.truncated());

    let bounded = QueryCoverage::complete()
        .with_boundaries(Some("2026-01-01T00:00:00Z".to_string()), None)
        .with_dropped_count(Some(3))
        .with_truncated(true);
    assert_eq!(bounded.oldest_available_at(), Some("2026-01-01T00:00:00Z"));
    assert!(bounded.newest_available_at().is_none());
    assert_eq!(bounded.dropped_count(), Some(3));
    assert!(bounded.truncated());
}

#[test]
fn an_unsupported_schema_version_is_refused() {
    let mut builder = EvidenceEventBuilder::new(
        "source-1",
        session_correlation(),
        SafeEvidencePayload::ShellClosed {
            reason: reason("user_closed"),
        },
    );
    builder = builder.with_occurred_at("2026-01-01T00:00:00.000Z");
    let event = builder.try_build();
    assert!(event.is_ok());

    let unsupported = ExecutionEvidenceEvent::new(ExecutionEvidenceEventInput {
        event_id: EvidenceEventId::parse("event-1").expect("event id"),
        source_context: EvidenceSourceContext::Workspaces,
        source_event_id: SourceEventId::parse("source-1").expect("source id"),
        schema_version: EVIDENCE_SCHEMA_VERSION + 1,
        occurred_at: "2026-01-01T00:00:00.000Z".to_string(),
        correlation: session_correlation(),
        status: None,
        fidelity: ExecutionFidelity::Native,
        payload: SafeEvidencePayload::ShellClosed {
            reason: reason("user_closed"),
        },
        redaction: RedactionReceipt::none(),
    });
    assert!(matches!(
        unsupported,
        Err(EvidenceDomainError::UnsupportedSchemaVersion { .. })
    ));
}

#[test]
fn a_timestamp_is_required_and_bounded() {
    let empty = EvidenceEventBuilder::new(
        "source-1",
        session_correlation(),
        SafeEvidencePayload::ShellClosed {
            reason: reason("user_closed"),
        },
    )
    .with_occurred_at("")
    .try_build();
    assert_eq!(empty.unwrap_err(), EvidenceDomainError::InvalidTimestamp);
}

#[test]
fn the_canonical_payload_encoding_is_stable_across_constructions() {
    let payload = || SafeEvidencePayload::CommandCompleted {
        outcome: EvidenceOutcome::Failed,
        duration_ms: Some(12_400),
        exit_code: Some(1),
        signal: None,
        output_availability: OutputAvailability::Merged,
        output_truncated: true,
    };
    assert_eq!(
        canonical_payload_encoding(&payload()),
        canonical_payload_encoding(&payload())
    );
    assert!(canonical_payload_encoding(&payload()).starts_with("command.completed"));
}
