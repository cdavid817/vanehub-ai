//! Tests for the four registered evidence read commands.
//!
//! Each one drives the real handler body against a real `ExecutionEvidenceApi` backed by a real
//! `SqliteEvidenceRepository` on a real temporary database. Group 2's fixture-transport tests prove
//! the frontend client parses a payload of the specified shape; they say nothing about whether
//! anything produces one, so they are not reused as evidence that these commands work.

use super::evidence_dto::{
    EvidenceFiltersDto, EvidenceScopeDto, ExecutionRecordDetailDto, ExecutionRecordDto,
};
use super::evidence_mapper;
use crate::contexts::execution_observability::api::evidence::{
    EvidenceApplicationError, EvidenceSessionId, ExecutionEvidenceApi, RecordEvidenceInput,
};
// The ports are implemented inside the context, so a test double names them where they are
// declared rather than through the published surface, which carries only what crosses it.
use crate::contexts::execution_observability::application::evidence::models::EvidenceNotice;
use crate::contexts::execution_observability::application::evidence::ports::{
    EvidenceIdGeneratorPort, PostCommitEvidenceNoticePublisherPort,
};
use crate::contexts::execution_observability::domain::evidence::builders::CorrelationBuilder;
use crate::contexts::execution_observability::domain::evidence::payload::EvidenceOutcome;
use crate::contexts::execution_observability::domain::evidence::safety::RedactedCommandDisplay;
use crate::contexts::execution_observability::domain::{
    CommandRuntimeKind, EvidenceSourceContext, ExecutionFidelity, ExecutionStatus,
    OutputAvailability, RedactionReceipt, SafeEvidencePayload, SourceEventId,
};
use crate::contexts::execution_observability::infrastructure::{
    DomainEvidenceRedactionValidator, SqliteEvidenceRepository, SystemEvidenceClock,
    UuidEvidenceIdGenerator,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::sync::{Arc, Mutex};

const SESSION: &str = "session-1";
const RUN: &str = "6f1b2c3d-4e5f-4a6b-8c9d-0e1f2a3b4c5d";
const TRACE: &str = "0af7651916cd43dd8448eb211c80319c";

#[derive(Default)]
struct CollectingPublisher {
    notices: Mutex<Vec<EvidenceNotice>>,
}

impl PostCommitEvidenceNoticePublisherPort for CollectingPublisher {
    fn publish(&self, notice: &EvidenceNotice) {
        self.notices.lock().expect("notices").push(notice.clone());
    }
}

#[derive(Default)]
struct SilentDiagnostics;

impl crate::contexts::execution_observability::application::evidence::ports::EvidenceGapDiagnosticsPort
    for SilentDiagnostics
{
    fn record_conflict(
        &self,
        _source_context: EvidenceSourceContext,
        _source_event_id: &crate::contexts::execution_observability::domain::SourceEventId,
    ) {
    }

    fn record_dropped(&self, _session_id: &EvidenceSessionId, _dropped_count: u32) {}
}

/// Ids are generated per call in production; a test needs them stable to assert on a record id, so
/// this counts instead of randomising. Everything else on the path is the production adapter.
#[derive(Default)]
struct SequentialIds {
    next: Mutex<u32>,
}

impl EvidenceIdGeneratorPort for SequentialIds {
    fn next_event_id(&self) -> String {
        let mut next = self.next.lock().expect("next");
        *next += 1;
        format!("event-{next}")
    }
}

struct Harness {
    _directory: TempDirectory,
    api: ExecutionEvidenceApi,
    publisher: Arc<CollectingPublisher>,
}

fn harness(name: &str) -> Harness {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let publisher = Arc::new(CollectingPublisher::default());
    let api = ExecutionEvidenceApi::new(
        Arc::new(SqliteEvidenceRepository::new(database)),
        Arc::new(SystemEvidenceClock),
        Arc::new(SequentialIds::default()),
        Arc::new(DomainEvidenceRedactionValidator),
        publisher.clone(),
        Arc::new(SilentDiagnostics),
    );
    Harness {
        _directory: directory,
        api,
        publisher,
    }
}

/// Drives the real ingestion path: domain validation, fingerprint, transaction, projection, then
/// the post-commit notice. Nothing here short-circuits into the store.
fn record(api: &ExecutionEvidenceApi, input: RecordEvidenceInput) {
    api.record(input).expect("record");
}

fn command_started(command_id: &str, occurred_at: &str) -> RecordEvidenceInput {
    RecordEvidenceInput {
        source_context: EvidenceSourceContext::AgentRuntime,
        source_event_id: SourceEventId::parse(format!("start-{command_id}")).expect("source id"),
        occurred_at: occurred_at.to_string(),
        correlation: CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_command(command_id)
            .build(),
        status: Some(ExecutionStatus::Running),
        fidelity: ExecutionFidelity::Native,
        payload: SafeEvidencePayload::CommandStarted {
            runtime_kind: CommandRuntimeKind::LocalShell,
            redacted_display: Some(RedactedCommandDisplay::parse("npm test").expect("display")),
            cwd_display: None,
        },
        redaction: RedactionReceipt::none(),
    }
}

fn command_completed(command_id: &str, occurred_at: &str, exit_code: i32) -> RecordEvidenceInput {
    let succeeded = exit_code == 0;
    RecordEvidenceInput {
        source_context: EvidenceSourceContext::AgentRuntime,
        source_event_id: SourceEventId::parse(format!("done-{command_id}")).expect("source id"),
        occurred_at: occurred_at.to_string(),
        correlation: CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_command(command_id)
            .build(),
        status: Some(if succeeded {
            ExecutionStatus::Succeeded
        } else {
            ExecutionStatus::Failed
        }),
        fidelity: ExecutionFidelity::Native,
        payload: SafeEvidencePayload::CommandCompleted {
            outcome: if succeeded {
                EvidenceOutcome::Succeeded
            } else {
                EvidenceOutcome::Failed
            },
            // Absent on purpose in the completion-only case: a duration the producer did not
            // measure must not appear, and nothing downstream may compute one from a start it
            // never saw.
            duration_ms: None,
            exit_code: Some(exit_code),
            signal: None,
            output_availability: OutputAvailability::Merged,
            output_truncated: false,
        },
        redaction: RedactionReceipt::none(),
    }
}

fn scope() -> EvidenceScopeDto {
    EvidenceScopeDto {
        session_id: Some(SESSION.to_string()),
        ..Default::default()
    }
}

/// The four handlers, called through their real bodies.
///
/// Only the `State` wrapper is skipped — it cannot be built outside a running app — so everything
/// under test here is the code the registered command runs: parsing, the API call, the DTO map,
/// and the error map.
fn summary_command(
    api: &ExecutionEvidenceApi,
    session_id: &str,
) -> Result<
    super::evidence_dto::WorkspaceEvidenceSummaryDto,
    super::evidence_dto::EvidenceCommandErrorDto,
> {
    summary_command_with_live_shells(api, session_id, 0)
}

fn summary_command_with_live_shells(
    api: &ExecutionEvidenceApi,
    session_id: &str,
    live_shells: usize,
) -> Result<
    super::evidence_dto::WorkspaceEvidenceSummaryDto,
    super::evidence_dto::EvidenceCommandErrorDto,
> {
    super::get_workspace_evidence_summary::workspace_evidence_summary(
        api,
        session_id.to_string(),
        None,
        live_shells,
    )
}

fn list_command(
    api: &ExecutionEvidenceApi,
    scope: EvidenceScopeDto,
    filters: Option<EvidenceFiltersDto>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<super::evidence_dto::ExecutionRecordPageDto, super::evidence_dto::EvidenceCommandErrorDto>
{
    super::list_execution_records::execution_record_page(api, scope, filters, cursor, limit)
}

fn detail_command(
    api: &ExecutionEvidenceApi,
    session_id: &str,
    record_id: &str,
) -> Result<
    super::evidence_dto::ExecutionRecordDetailViewDto,
    super::evidence_dto::EvidenceCommandErrorDto,
> {
    super::get_execution_record::execution_record_detail(
        api,
        session_id.to_string(),
        record_id.to_string(),
    )
}

fn bootstrap_command(
    api: &ExecutionEvidenceApi,
    session_id: &str,
) -> Result<
    super::evidence_dto::EvidenceSubscriptionBootstrapDto,
    super::evidence_dto::EvidenceCommandErrorDto,
> {
    super::get_evidence_subscription_bootstrap::evidence_subscription_bootstrap(
        api,
        session_id.to_string(),
    )
}

fn json(value: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).expect("serialize")
}

#[test]
fn a_summary_over_an_empty_store_succeeds_and_reports_partial_coverage() {
    let harness = harness("evidence-cmd-empty-summary");

    let summary = summary_command(&harness.api, SESSION).expect("summary");

    // Not `unavailable` and not `complete`: the store answered, so the query worked, but no
    // producer is wired to it yet, so an empty answer says nothing about whether work happened.
    assert_eq!(summary.coverage.state, "partial");
    assert!(summary
        .coverage
        .reason_codes
        .contains(&"evidence_capture_not_initialized".to_string()));
    assert!(!summary
        .coverage
        .reason_codes
        .contains(&"evidence_unavailable".to_string()));
}

#[test]
fn an_empty_list_query_succeeds_rather_than_failing() {
    let harness = harness("evidence-cmd-empty-list");

    let page = list_command(&harness.api, scope(), None, None, None).expect("page");

    assert!(page.items.is_empty());
    assert_eq!(page.coverage.state, "partial");
    assert!(page
        .coverage
        .reason_codes
        .contains(&"evidence_capture_not_initialized".to_string()));
}

/// The sources this context does not own must stay explicitly unavailable rather than reporting a
/// confident zero. Group 8 connects them; until then a zero next to `complete` would read as
/// "nothing happened", which is the exact false claim this capability exists to remove.
#[test]
fn a_summary_marks_sources_this_context_does_not_own() {
    let harness = harness("evidence-cmd-unowned");

    let summary = summary_command(&harness.api, SESSION).expect("summary");

    assert_eq!(summary.usage.coverage, "unavailable");
    assert_eq!(summary.logs.new_errors, 0);
    assert_eq!(summary.changes.changed_files, 0);
    assert_ne!(summary.coverage.state, "complete");
}

/// The Shell figure is the one that left that list. It is joined from the workspaces registry, so
/// it is a real count rather than a placeholder — and a summary that kept reporting zero while
/// three Shells were open would tell the user their session is idle while it is not.
#[test]
fn a_summary_reports_the_live_shells_the_registry_is_holding() {
    let harness = harness("evidence-cmd-live-shells");

    let summary = summary_command_with_live_shells(&harness.api, SESSION, 3).expect("summary");

    assert_eq!(summary.shells.live, 3);
}

#[test]
fn a_summary_counts_the_records_this_context_does_own() {
    let harness = harness("evidence-cmd-owned-counts");
    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    record(
        &harness.api,
        command_completed("command-a", "2026-08-22T10:00:05Z", 1),
    );
    record(
        &harness.api,
        command_started("command-b", "2026-08-22T10:00:06Z"),
    );

    let summary = summary_command(&harness.api, SESSION).expect("summary");

    assert_eq!(summary.execution_records.failed, 1);
    assert_eq!(summary.execution_records.running, 1);
}

#[test]
fn a_list_query_returns_the_records_that_were_recorded() {
    let harness = harness("evidence-cmd-list-records");
    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    record(
        &harness.api,
        command_completed("command-a", "2026-08-22T10:00:05Z", 0),
    );

    let page = list_command(&harness.api, scope(), None, None, None).expect("page");

    assert_eq!(page.items.len(), 1, "two events, one lifecycle, one record");
    assert_eq!(page.items[0].status, "succeeded");
    assert_eq!(page.items[0].kind, "command");
}

#[test]
fn a_detail_query_returns_the_record_with_related_counts() {
    let harness = harness("evidence-cmd-detail");
    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    let page = list_command(&harness.api, scope(), None, None, None).expect("page");
    let record_id = page.items[0].id.clone();

    let detail = detail_command(&harness.api, SESSION, &record_id).expect("detail");

    assert_eq!(detail.record.id, record_id);
    assert_eq!(detail.related_counts.commands, 1);
    // Logs belong to another context this one has no port to, so the count is a stated zero.
    assert_eq!(detail.related_counts.logs, 0);
    // Only classifications; nothing carried from the payload.
    assert_eq!(
        detail
            .safe_attributes
            .get("runtimeKind")
            .map(String::as_str),
        Some("local-shell")
    );
    assert!(!detail.safe_attributes.values().any(|v| v.contains("npm")));
}

#[test]
fn a_missing_record_is_a_not_found_code_rather_than_an_error_string() {
    let harness = harness("evidence-cmd-missing");

    let error = detail_command(&harness.api, SESSION, "command:absent").expect_err("not found");

    assert_eq!(error.reason_code, "evidence_record_not_found");
}

#[test]
fn a_bootstrap_returns_the_committed_watermark() {
    let harness = harness("evidence-cmd-bootstrap");
    let empty = bootstrap_command(&harness.api, SESSION).expect("bootstrap");
    assert_eq!(empty.watermark_sequence, 0);

    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    let after = bootstrap_command(&harness.api, SESSION).expect("bootstrap");

    // The watermark tracks what is committed, and the notice's sequence is what a subscriber
    // compares against it. A watermark that lagged the notice would replay; one that led it would
    // silently drop the notice describing work the client has not seen.
    let published = harness.publisher.notices.lock().expect("notices");
    assert_eq!(after.watermark_sequence, published[0].sequence);
}

#[test]
fn a_malformed_session_is_refused_before_the_store_is_touched() {
    let harness = harness("evidence-cmd-bad-session");

    let error = summary_command(&harness.api, "").expect_err("invalid");

    assert_eq!(error.reason_code, "evidence_invalid_request");
}

#[test]
fn an_unrecognised_filter_token_is_refused_rather_than_ignored() {
    let harness = harness("evidence-cmd-bad-filter");

    let error = list_command(
        &harness.api,
        scope(),
        Some(EvidenceFiltersDto {
            statuses: Some(vec!["teleported".to_string()]),
            ..Default::default()
        }),
        None,
        None,
    )
    .expect_err("invalid");

    // Dropping it would silently widen the query, and the caller could not tell.
    assert_eq!(error.reason_code, "evidence_invalid_request");
}

#[test]
fn an_undecodable_cursor_is_refused_with_its_own_code() {
    let harness = harness("evidence-cmd-bad-cursor");

    let error = list_command(
        &harness.api,
        scope(),
        None,
        Some("not-a-cursor".to_string()),
        None,
    )
    .expect_err("invalid cursor");

    assert_eq!(error.reason_code, "evidence_invalid_cursor");
}

/// A cursor is bound to the filters that produced it. Reusing it under different filters would
/// resume from a key that means something else in the new ordering, skipping rows silently.
#[test]
fn a_cursor_reused_under_different_filters_is_refused() {
    let harness = harness("evidence-cmd-cursor-mismatch");
    for index in 0..3 {
        record(
            &harness.api,
            command_started(&format!("command-{index}"), "2026-08-22T10:00:00Z"),
        );
    }
    let page = list_command(&harness.api, scope(), None, None, Some(1)).expect("page");
    let cursor = page.next_cursor.expect("a continuation token");

    let error = list_command(
        &harness.api,
        scope(),
        Some(EvidenceFiltersDto {
            statuses: Some(vec!["failed".to_string()]),
            ..Default::default()
        }),
        Some(cursor),
        None,
    )
    .expect_err("mismatch");

    assert_eq!(error.reason_code, "cursor_filter_mismatch");
}

/// Every code a handler can return has to be one the frontend can translate, and none of them may
/// carry the value that failed.
#[test]
fn every_command_error_is_a_bare_reason_code() {
    let errors = [
        evidence_mapper::invalid_request(),
        evidence_mapper::command_error(EvidenceApplicationError::RecordNotFound),
        evidence_mapper::command_error(EvidenceApplicationError::CursorFilterMismatch),
        evidence_mapper::command_error(EvidenceApplicationError::InvalidCursor),
        evidence_mapper::command_error(EvidenceApplicationError::Storage(
            "no such table: evidence_events; SELECT * FROM /home/user/private".to_string(),
        )),
    ];

    for error in &errors {
        let value = json(error);
        let object = value.as_object().expect("object");
        assert_eq!(object.len(), 1, "a reason code and nothing else");
        assert!(object.contains_key("reasonCode"));
        let code = object["reasonCode"].as_str().expect("string");
        assert!(
            code.bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit()),
            "a reason code cannot smuggle a message: {code}"
        );
    }
    // The storage failure's own text named a table and a path; neither survives.
    let storage = json(&errors[4]);
    assert_eq!(storage["reasonCode"], "evidence_unavailable");
}

#[test]
fn a_record_serializes_with_camel_case_field_names() {
    let harness = harness("evidence-cmd-camel");
    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    let page = list_command(&harness.api, scope(), None, None, None).expect("page");

    let value = json(&page);
    let item = &value["items"][0];

    for key in [
        "sessionId",
        "startedAt",
        "commandId",
        "runtimeKind",
        "outputAvailability",
        "outputTruncated",
    ] {
        assert!(item.get(key).is_some(), "missing camelCase field {key}");
    }
    for key in ["session_id", "started_at", "command_id", "runtime_kind"] {
        assert!(item.get(key).is_none(), "snake_case leaked: {key}");
    }
    assert!(value.get("nextCursor").is_none() || value["nextCursor"].is_string());
    assert!(value["coverage"].get("reasonCodes").is_some());
}

/// The contract's one asymmetry: a record whose completion was observed but whose start was not.
/// Serializing an empty string or an invented time would let a UI compute a duration nobody
/// measured, so the field is absent from the JSON entirely.
#[test]
fn a_completion_only_record_omits_started_at_from_its_json() {
    let harness = harness("evidence-cmd-completion-only");
    record(
        &harness.api,
        command_completed("command-a", "2026-08-22T10:39:02Z", 1),
    );

    let page = list_command(&harness.api, scope(), None, None, None).expect("page");
    let value = json(&page);
    let item = &value["items"][0];

    assert!(
        item.get("startedAt").is_none(),
        "startedAt must be absent, not null or empty: {item}"
    );
    // The terminal status survives: not observing the start says nothing about the outcome.
    assert_eq!(item["status"], "failed");
    assert_eq!(item["endedAt"], "2026-08-22T10:39:02Z");
    // Nothing back-derived a duration from a start that was never seen.
    assert!(item.get("durationMs").is_none());
    assert_eq!(page.items[0].started_at, None);
}

/// A start that was observed survives to the wire, and a duration appears only if the producer
/// measured one. Two timestamps five seconds apart do not license a 5000 ms duration: the gap
/// between two observations is not the same quantity as how long the work took, and the store has
/// no way to tell the difference.
#[test]
fn a_started_record_keeps_its_start_and_only_reports_a_measured_duration() {
    let harness = harness("evidence-cmd-both-ends");
    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    record(
        &harness.api,
        command_completed("command-a", "2026-08-22T10:00:05Z", 0),
    );

    let page = list_command(&harness.api, scope(), None, None, None).expect("page");
    let item = &json(&page)["items"][0];

    assert_eq!(item["startedAt"], "2026-08-22T10:00:00Z");
    assert_eq!(item["endedAt"], "2026-08-22T10:00:05Z");
    assert!(
        item.get("durationMs").is_none(),
        "the producer reported no duration, so none may be invented: {item}"
    );
}

#[test]
fn a_duration_the_producer_measured_reaches_the_wire() {
    let harness = harness("evidence-cmd-measured-duration");
    let mut completed = command_completed("command-a", "2026-08-22T10:00:05Z", 0);
    if let SafeEvidencePayload::CommandCompleted { duration_ms, .. } = &mut completed.payload {
        *duration_ms = Some(4_812);
    }
    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    record(&harness.api, completed);

    let page = list_command(&harness.api, scope(), None, None, None).expect("page");

    assert_eq!(json(&page)["items"][0]["durationMs"], 4_812);
}

#[test]
fn a_page_request_above_the_maximum_is_clamped() {
    assert_eq!(evidence_mapper::clamp_limit(Some(9000)), 500);
    assert_eq!(evidence_mapper::clamp_limit(None), 100);
    assert_eq!(evidence_mapper::clamp_limit(Some(0)), 100);
    assert_eq!(evidence_mapper::clamp_limit(Some(7)), 7);
}

/// A record that reaches the wire may only carry classifications and identifiers. This walks the
/// serialized page looking for the two things the journal is designed never to hold.
#[test]
fn a_serialized_page_carries_no_output_or_transcript() {
    let harness = harness("evidence-cmd-no-content");
    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    record(
        &harness.api,
        command_completed("command-a", "2026-08-22T10:00:05Z", 1),
    );

    let rendered =
        json(&list_command(&harness.api, scope(), None, None, None).expect("page")).to_string();

    for forbidden in [
        "stdout",
        "stderr",
        "transcript",
        "diff",
        "prompt",
        "authorization",
        "secret",
    ] {
        assert!(
            !rendered.contains(forbidden),
            "the page carried a {forbidden} field"
        );
    }
}

/// The registry holds four evidence commands and no fifth. A write command would let a client
/// assert what happened, which is the one thing an evidence journal cannot allow.
#[test]
fn the_registry_exposes_exactly_the_four_read_commands() {
    let registry = include_str!("../core_registry.rs");
    let registered: Vec<&str> = registry
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("crate::commands::execution_observability::"))
        .filter_map(|line| line.trim_end_matches(',').rsplit("::").next())
        .collect();

    for expected in [
        "get_workspace_evidence_summary",
        "list_execution_records",
        "get_execution_record",
        "get_evidence_subscription_bootstrap",
    ] {
        assert!(
            registered.contains(&expected),
            "{expected} is not registered, so the frontend call returns \"unknown command\""
        );
    }

    let evidence_commands: Vec<&&str> = registered
        .iter()
        .filter(|name| name.contains("evidence") || name.contains("execution_record"))
        .collect();
    assert_eq!(
        evidence_commands.len(),
        4,
        "exactly four evidence commands, found {evidence_commands:?}"
    );
}

/// Nothing the frontend can call may append, reset, rebuild, or mutate evidence. The whole command
/// tree is scanned rather than just the evidence files, because a write command added anywhere
/// would be equally reachable.
#[test]
fn no_registered_command_writes_evidence() {
    let registry = include_str!("../core_registry.rs");
    let supplemental = include_str!("../supplemental_registry.rs");
    let names: Vec<String> = format!("{registry}\n{supplemental}")
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("crate::commands::"))
        .filter_map(|line| line.trim_end_matches(',').rsplit("::").next())
        .map(str::to_string)
        .collect();

    let forbidden: Vec<&String> = names
        .iter()
        .filter(|name| {
            let touches_evidence = name.contains("evidence") || name.contains("execution_record");
            let mutates = name.starts_with("record_")
                || name.starts_with("append_")
                || name.starts_with("insert_")
                || name.starts_with("reset_")
                || name.starts_with("rebuild_")
                || name.contains("_correlation");
            touches_evidence && mutates
        })
        .collect();

    assert!(
        forbidden.is_empty(),
        "evidence is written by in-process producers, never by a client: {forbidden:?}"
    );
}

/// 10.8 registers the report. Until it does, no command by that name may exist — a half-registered
/// report would answer with an empty shape rather than the typed "not available yet" the panel
/// needs, and an empty report is indistinguishable from a session where nothing happened.
#[test]
fn the_session_run_report_command_is_still_absent() {
    let registry = include_str!("../core_registry.rs");
    let supplemental = include_str!("../supplemental_registry.rs");
    assert!(!registry.contains("get_session_run_report"));
    assert!(!supplemental.contains("get_session_run_report"));
}

/// The command names are duplicated across two languages with nothing but agreement holding them
/// together. A rename on one side produces a call that reaches no handler.
#[test]
fn every_command_name_matches_the_typescript_transport() {
    let transport = include_str!("../../../../src/services/native-evidence-transport.ts");
    let registry = include_str!("../core_registry.rs");

    for name in [
        "get_workspace_evidence_summary",
        "list_execution_records",
        "get_execution_record",
        "get_evidence_subscription_bootstrap",
    ] {
        assert!(
            transport.contains(&format!("\"{name}\"")),
            "{name} is registered in Rust but not named in EvidenceCommandName"
        );
        assert!(registry.contains(name), "{name} is not in the registry");
    }
}

#[test]
fn a_detail_view_never_serializes_a_raw_payload_field() {
    let harness = harness("evidence-cmd-detail-shape");
    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    let page = list_command(&harness.api, scope(), None, None, None).expect("page");
    let detail = detail_command(&harness.api, SESSION, &page.items[0].id).expect("detail");

    let value = json(&detail);
    let object = value.as_object().expect("object");
    let allowed = [
        "record",
        "relatedCounts",
        "safeAttributes",
        "errorReasonCode",
    ];
    for key in object.keys() {
        assert!(allowed.contains(&key.as_str()), "unexpected field: {key}");
    }
}

/// The clock is the production one, so this is the only assertion available about it: an event
/// recorded now must carry a timestamp that parses.
#[test]
fn the_system_clock_produces_a_parseable_timestamp() {
    use crate::contexts::execution_observability::application::evidence::ports::EvidenceClockPort;
    let now = SystemEvidenceClock.now_rfc3339();
    assert!(chrono::DateTime::parse_from_rfc3339(&now).is_ok(), "{now}");
}

#[test]
fn generated_event_ids_are_unique() {
    let generator = UuidEvidenceIdGenerator;
    let first = generator.next_event_id();
    let second = generator.next_event_id();
    assert_ne!(first, second);
    assert!(uuid::Uuid::parse_str(&first).is_ok());
}

/// A record kind whose detail carries no command should not gain command attributes.
#[test]
fn a_tool_record_maps_to_the_tool_variant() {
    let detail = ExecutionRecordDetailDto::Tool {
        tool_call_id: Some("toolcall-1".to_string()),
        tool_name: "read_file".to_string(),
        source: "native".to_string(),
    };
    let value = json(&detail);
    assert_eq!(value["toolName"], "read_file");
    assert_eq!(value["toolCallId"], "toolcall-1");
    assert!(value.get("commandId").is_none());
}

#[test]
fn an_absent_optional_identifier_is_omitted_rather_than_null() {
    let record = ExecutionRecordDto {
        id: "command:command-1".to_string(),
        kind: "command".to_string(),
        session_id: SESSION.to_string(),
        run_id: None,
        trace_id: None,
        span_id: None,
        operation_id: None,
        agent_id: None,
        seat_id: None,
        started_at: None,
        ended_at: Some("2026-08-22T10:39:02Z".to_string()),
        duration_ms: None,
        status: "failed".to_string(),
        fidelity: "native".to_string(),
        coverage: evidence_mapper::coverage_dto(
            &crate::contexts::execution_observability::api::evidence::QueryCoverage::complete(),
        ),
        detail: ExecutionRecordDetailDto::Command {
            command_id: "command-1".to_string(),
            runtime_kind: "local-shell".to_string(),
            redacted_display: None,
            cwd_display: None,
            exit_code: Some(1),
            signal: None,
            output_availability: "unavailable".to_string(),
            output_truncated: false,
        },
    };

    let value = json(&record);
    let object = value.as_object().expect("object");
    // A null would be indistinguishable from a value the frontend schema treats as present.
    assert!(!object.values().any(serde_json::Value::is_null));
    assert!(!object.contains_key("startedAt"));
    assert!(!object.contains_key("runId"));
}

#[test]
fn the_session_is_required_before_a_record_id_is_used() {
    let harness = harness("evidence-cmd-detail-session");
    let error = detail_command(&harness.api, SESSION, "  ").expect_err("blank record id");
    assert_eq!(error.reason_code, "evidence_invalid_request");
}

/// The related counts a detail reports come from the store, not from a constant.
///
/// They are computed in the same read that produced the record, so this checks the number tracks
/// what was actually recorded: two commands in, two commands reported.
#[test]
fn a_detail_reports_counts_the_store_can_vouch_for() {
    let harness = harness("evidence-cmd-authoritative-counts");
    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    record(
        &harness.api,
        command_started("command-b", "2026-08-22T10:00:01Z"),
    );
    let page = list_command(&harness.api, scope(), None, None, None).expect("page");

    let detail = detail_command(&harness.api, SESSION, &page.items[0].id).expect("detail");

    assert_eq!(detail.related_counts.commands, 2);
    // Owned by other contexts, so a stated zero rather than a plausible substitute. A review
    // finding is an unresolved comment; filling the field with a verification count would put a
    // number there that answers a different question.
    assert_eq!(detail.related_counts.logs, 0);
    assert_eq!(detail.related_counts.findings, 0);
}

/// Counts follow the store as it grows, which is what distinguishes a live query from a snapshot
/// taken once and reused.
#[test]
fn detail_counts_track_later_records() {
    let harness = harness("evidence-cmd-counts-track");
    record(
        &harness.api,
        command_started("command-a", "2026-08-22T10:00:00Z"),
    );
    let page = list_command(&harness.api, scope(), None, None, None).expect("page");
    let record_id = page.items[0].id.clone();
    assert_eq!(
        detail_command(&harness.api, SESSION, &record_id)
            .expect("detail")
            .related_counts
            .commands,
        1
    );

    record(
        &harness.api,
        command_started("command-b", "2026-08-22T10:00:01Z"),
    );

    assert_eq!(
        detail_command(&harness.api, SESSION, &record_id)
            .expect("detail")
            .related_counts
            .commands,
        2
    );
}
