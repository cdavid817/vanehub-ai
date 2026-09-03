//! Tests for the producer-to-journal bridge.
//!
//! The properties worth checking are the ones a producer depends on without being able to see:
//! that publishing cannot block, cannot fail, and cannot change what the producer returns. Each
//! test drives a real bounded channel, and the end-to-end ones drive a real recorder over a real
//! store — a stub would let a mapping pass here that the domain rejects at run time, which is the
//! failure mode most worth catching.

use super::evidence_bridge::{
    start_evidence_bridge, BridgeInstanceId, DropAccumulator, DropSnapshot, EvidenceDropReason,
    EVIDENCE_QUEUE_CAPACITY, MAX_TRACKED_DROP_REASONS, MAX_TRACKED_DROP_SESSIONS,
};
use crate::contexts::agent_runtime::api::{
    AgentEvidenceObservation, AgentEvidencePort, AgentEvidenceSignal, AgentRunEvidenceOutcome,
};
use crate::contexts::execution_observability::api::evidence::{
    EvidenceSessionId, ExecutionEvidenceApi,
};
use crate::contexts::execution_observability::application::evidence::models::EvidenceNotice;
use crate::contexts::execution_observability::application::evidence::ports::{
    EvidenceGapDiagnosticsPort, PostCommitEvidenceNoticePublisherPort,
};
use crate::contexts::execution_observability::domain::{EvidenceSourceContext, SourceEventId};
use crate::contexts::execution_observability::infrastructure::{
    DomainEvidenceRedactionValidator, SqliteEvidenceRepository, SystemEvidenceClock,
    UuidEvidenceIdGenerator,
};
use crate::contexts::operations::api::{OperationsEvidencePort, OperationsEvidenceSignal};
use crate::contexts::sessions::api::{
    SessionEvidencePort, SessionEvidenceSignal, SessionReviewDecision, SessionUsageEvidenceQuality,
    SessionVerificationOutcome,
};
use crate::contexts::workspaces::api::{
    WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceFileChangeKind,
    WorkspaceShellCloseReason, WorkspaceShellRuntimeKind,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SESSION: &str = "session-1";
const TRACE: &str = "0af7651916cd43dd8448eb211c80319c";

/// A run id the domain accepts. It is a UUID because `ExecutionRunId` requires one — a producer
/// that supplies anything else has its signal dropped rather than filed under a malformed id, so
/// a test using a readable placeholder would silently exercise the drop path instead.
fn run_id(index: usize) -> String {
    format!("6f1b2c3d-4e5f-4a6b-8c9d-{index:012x}")
}

#[derive(Default)]
struct SilentNotices;

impl PostCommitEvidenceNoticePublisherPort for SilentNotices {
    fn publish(&self, _notice: &EvidenceNotice) {}
}

#[derive(Default)]
struct CountingDiagnostics {
    dropped: Mutex<u32>,
}

impl EvidenceGapDiagnosticsPort for CountingDiagnostics {
    fn record_conflict(&self, _context: EvidenceSourceContext, _source: &SourceEventId) {}

    fn record_dropped(&self, _session_id: &EvidenceSessionId, dropped_count: u32) {
        *self.dropped.lock().expect("dropped") += dropped_count;
    }
}

struct Harness {
    _directory: TempDirectory,
    api: ExecutionEvidenceApi,
    diagnostics: Arc<CountingDiagnostics>,
    /// Kept so a test can read the stored row rather than the mapper's own opinion of it.
    database: NativeDatabase,
}

fn harness(name: &str) -> Harness {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let diagnostics = Arc::new(CountingDiagnostics::default());
    let api = ExecutionEvidenceApi::new(
        Arc::new(SqliteEvidenceRepository::new(database.clone())),
        Arc::new(SystemEvidenceClock),
        Arc::new(UuidEvidenceIdGenerator),
        Arc::new(DomainEvidenceRedactionValidator),
        Arc::new(SilentNotices),
        diagnostics.clone(),
    );
    Harness {
        _directory: directory,
        api,
        diagnostics,
        database,
    }
}

/// Reads the redaction rule ids the journal actually stored. Asserting on the mapper's return
/// value would only prove the mapper agrees with itself.
fn stored_redaction_rules(harness: &Harness) -> Vec<String> {
    let connection = harness.database.connection().expect("connection");
    let rules: String = connection
        .query_row(
            "SELECT redaction_rule_ids_json FROM execution_evidence_events \
             ORDER BY sequence DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("stored event");
    serde_json::from_str(&rules).expect("rule ids")
}

fn session() -> EvidenceSessionId {
    EvidenceSessionId::parse(SESSION).expect("session")
}

fn run_started(id: &str) -> AgentEvidenceSignal {
    AgentEvidenceSignal::RunStarted {
        session_id: SESSION.to_string(),
        run_id: id.to_string(),
        trace_id: TRACE.to_string(),
        agent_id: Some("agent-1".to_string()),
        seat_id: None,
        occurred_at: "2026-08-22T10:00:00Z".to_string(),
    }
}

/// The journal's committed sequence, which is what the worker advances as it records.
fn watermark(api: &ExecutionEvidenceApi) -> i64 {
    api.subscription_bootstrap(&session())
        .map(|bootstrap| bootstrap.watermark_sequence)
        .unwrap_or_default()
}

fn wait_until<F: Fn() -> bool>(condition: F) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

/// The whole point of the bridge: a producer publishes, and the observation reaches the journal
/// without the producer ever touching it.
#[test]
fn a_published_signal_reaches_the_journal_through_the_worker() {
    let harness = harness("bridge-end-to-end");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));

    assert!(
        wait_until(|| watermark(&harness.api) > 0),
        "the worker never recorded the published signal"
    );
    worker.shutdown();
}

/// Shutdown ends the worker rather than waiting out a deadline it can never beat.
///
/// The defect: the bridge is cloned into five long-lived assemblies, each holding a sender for the
/// process's whole life, so the worker's `recv` could never return `Err`. A shutdown that only
/// waited for the channel to close therefore always ran to its full two-second grace — on the
/// event-loop thread, on every exit, with the window already gone and the application apparently
/// hung.
///
/// The bound asserted here is deliberately loose. What is being distinguished is "ended on request"
/// from "ran out the grace", and those are two seconds apart; a tight bound would turn a busy CI
/// machine into a failure about something this test is not measuring.
#[test]
fn shutdown_ends_the_worker_while_every_producer_still_holds_a_sender() {
    let harness = harness("bridge-shutdown");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());
    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));
    assert!(wait_until(|| watermark(&harness.api) > 0));

    // Still held, exactly as production holds it: the senders are not going anywhere.
    let still_publishing = bridge.clone();
    let started = std::time::Instant::now();
    worker.shutdown();
    let elapsed = started.elapsed();

    assert!(
        elapsed < std::time::Duration::from_millis(1_500),
        "shutdown took {elapsed:?}, which is the grace rather than an ending"
    );
    // And a producer that publishes afterwards is still refused silently rather than panicking: a
    // signal arriving after shutdown says nothing about whether the observed work succeeded.
    AgentEvidencePort::try_publish(&still_publishing, run_started(&run_id(2)));
}

/// Every producer's port lands in the same journal. A context whose signal silently mapped to
/// nothing would look identical from its own side — it publishes and returns either way.
#[test]
fn every_producer_port_records_through_the_same_worker() {
    let harness = harness("bridge-every-producer");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));
    WorkspaceEvidencePort::try_publish(
        &bridge,
        WorkspaceEvidenceSignal::ShellOpened {
            session_id: SESSION.to_string(),
            shell_id: "shell-1".to_string(),
            seat_id: None,
            runtime: WorkspaceShellRuntimeKind::Remote,
            occurred_at: "2026-08-22T10:00:01Z".to_string(),
        },
    );
    OperationsEvidencePort::try_publish(
        &bridge,
        OperationsEvidenceSignal::OperationFailed {
            session_id: SESSION.to_string(),
            operation_id: "operation-1".to_string(),
            run_id: None,
            reason_code: "runner_unavailable".to_string(),
            occurred_at: "2026-08-22T10:00:02Z".to_string(),
        },
    );
    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::UsageObserved {
            session_id: SESSION.to_string(),
            invocation_id: "invocation-1".to_string(),
            run_id: None,
            quality: SessionUsageEvidenceQuality::Reported,
            occurred_at: "2026-08-22T10:00:03Z".to_string(),
        },
    );

    assert!(
        wait_until(|| watermark(&harness.api) >= 4),
        "expected four journal events, watermark reached {}",
        watermark(&harness.api)
    );
    worker.shutdown();
}

/// A run that starts and finishes closes its own lifecycle. Publishing only the start would leave
/// every run reading `incomplete` forever, which is worse than not recording it: a reader cannot
/// tell a run that is still going from one whose ending nobody filed.
#[test]
fn a_run_lifecycle_reaches_a_terminal_record() {
    let harness = harness("bridge-run-lifecycle");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));
    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::RunFinished {
            session_id: SESSION.to_string(),
            run_id: run_id(1),
            trace_id: TRACE.to_string(),
            agent_id: Some("agent-1".to_string()),
            seat_id: None,
            occurred_at: "2026-08-22T10:05:00Z".to_string(),
            outcome: AgentRunEvidenceOutcome::Failed,
            duration_ms: None,
        },
    );

    assert!(wait_until(|| watermark(&harness.api) >= 2));
    worker.shutdown();
}

/// A queue with no reader fills and then refuses. The producer must not notice.
///
/// The worker is never started here, so nothing drains: the first `CAPACITY` publishes are
/// accepted and every later one is refused. Each call still returns normally, which is the only
/// thing the producer can observe and the only thing it depends on.
#[test]
fn a_full_queue_refuses_without_failing_the_producer() {
    let harness = harness("bridge-queue-full");
    let (bridge, _worker) = start_evidence_bridge(harness.api.clone());
    drop(_worker);

    // Far more than the queue holds, published as fast as the producer can.
    for index in 0..2_000 {
        AgentEvidencePort::try_publish(&bridge, run_started(&run_id(index)));
    }

    // Reaching here at all is the assertion: `try_publish` returns `()`, so a producer has no
    // error to handle and no value to branch on. A blocking send would have deadlocked instead.
    let _ = harness.diagnostics;
}

/// Publishing stays fast when the queue is full.
///
/// A bounded channel with a blocking send would park the producer until the worker drains, turning
/// an evidence write into a latency spike in whatever operation was being observed. The threshold
/// is loose on purpose — it is there to catch a blocking call, not to measure throughput.
#[test]
fn publishing_into_a_full_queue_does_not_block() {
    let harness = harness("bridge-queue-nonblocking");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());
    drop(worker);

    for index in 0..1_000 {
        AgentEvidencePort::try_publish(&bridge, run_started(&run_id(index)));
    }

    let started = Instant::now();
    for index in 0..1_000 {
        AgentEvidencePort::try_publish(&bridge, run_started(&run_id(index + 1_000)));
    }
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "1000 publishes into a full queue took {elapsed:?}; the send is blocking"
    );
}

/// A worker whose recorder rejects one event keeps taking the next.
///
/// The two publishes share a source id and carry different content, so the second is a conflict the
/// store refuses. A worker that exited on the first refusal would turn one bad observation into the
/// permanent loss of every later one.
#[test]
fn the_worker_survives_a_rejected_record() {
    let harness = harness("bridge-worker-survives");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));
    assert!(wait_until(|| watermark(&harness.api) >= 1));

    // Same run id, so the same source event id, but a different payload: refused by the journal.
    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::RunStarted {
            session_id: SESSION.to_string(),
            run_id: run_id(1),
            trace_id: TRACE.to_string(),
            agent_id: Some("agent-2".to_string()),
            seat_id: None,
            occurred_at: "2026-08-22T11:00:00Z".to_string(),
        },
    );
    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(2)));

    assert!(
        wait_until(|| watermark(&harness.api) >= 2),
        "the worker stopped after the rejected record"
    );
    worker.shutdown();
}

/// A signal the journal cannot correlate is dropped rather than filed against a placeholder.
///
/// An event with an unusable session can be counted but never found, which reads as coverage
/// without being it. The producer still sees nothing.
#[test]
fn an_uncorrelatable_signal_is_dropped_rather_than_misfiled() {
    let harness = harness("bridge-uncorrelatable");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::RunStarted {
            session_id: String::new(),
            run_id: run_id(1),
            trace_id: TRACE.to_string(),
            agent_id: None,
            seat_id: None,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(9)));

    assert!(wait_until(|| watermark(&harness.api) >= 1));
    worker.shutdown();
    // Exactly one event: the unusable one never became a row under some substitute session.
    assert_eq!(watermark(&harness.api), 1);
}

/// Shutdown finishes on a bounded deadline whether or not the queue is empty.
///
/// Waiting forever would let one stuck SQLite write hold the process open at exit. Evidence
/// describes work that already happened, so losing its tail beats refusing to close.
#[test]
fn shutdown_returns_within_its_deadline() {
    let harness = harness("bridge-shutdown");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());
    for index in 0..200 {
        AgentEvidencePort::try_publish(&bridge, run_started(&run_id(index)));
    }

    let started = Instant::now();
    worker.shutdown();

    assert!(
        started.elapsed() < Duration::from_secs(5),
        "shutdown took {:?}, past its bounded deadline",
        started.elapsed()
    );
}

/// Startup must not mint evidence. An event created to prove the pipeline works is, once stored,
/// indistinguishable from an observation of real work.
#[test]
fn starting_the_bridge_records_nothing() {
    let harness = harness("bridge-no-startup-evidence");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(watermark(&harness.api), 0);

    drop(bridge);
    worker.shutdown();
    assert_eq!(watermark(&harness.api), 0);
}

/// Nothing but identifiers and classifications may reach the queue.
///
/// The mapping happens before the send, so this is a check on the mapper: a serialized round of
/// what the bridge produced must not contain the shell's path, the operation's error text, or the
/// token counts sessions owns.
#[test]
fn no_producer_content_reaches_the_journal() {
    let harness = harness("bridge-no-content");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    OperationsEvidencePort::try_publish(
        &bridge,
        OperationsEvidenceSignal::OperationFailed {
            session_id: SESSION.to_string(),
            operation_id: "operation-1".to_string(),
            run_id: None,
            // A producer that passed a message rather than a code: the bridge must not carry it
            // through, and must not smuggle it into a reason code either.
            reason_code: "Connection refused to 10.1.2.3:22 for /home/user/secret".to_string(),
            occurred_at: "2026-08-22T10:00:02Z".to_string(),
        },
    );

    assert!(wait_until(|| watermark(&harness.api) >= 1));
    worker.shutdown();

    let detail = harness
        .api
        .list_records(
            crate::contexts::execution_observability::api::evidence::ExecutionRecordQuery {
                scope:
                    crate::contexts::execution_observability::api::evidence::EvidenceQueryScope {
                        session_id: Some(session()),
                        ..Default::default()
                    },
                filters: Default::default(),
                cursor: None,
                limit: 100,
            },
        )
        .expect("page");
    let rendered = format!("{detail:?}");
    for forbidden in ["10.1.2.3", "/home/user", "secret", "Connection refused"] {
        assert!(
            !rendered.contains(forbidden),
            "producer content reached the journal: {forbidden}"
        );
    }
}

/// The accumulator is keyed by producer-supplied session ids, so its size has to be capped.
///
/// A producer bug minting a fresh session id per event would otherwise grow it without limit, and
/// a structure whose whole job is to report that something overflowed must not be the thing that
/// overflows.
#[test]
fn the_drop_accumulator_caps_the_sessions_it_tracks() {
    let accumulator = DropAccumulator::default();

    for index in 0..(MAX_TRACKED_DROP_SESSIONS * 4) {
        accumulator.record(&format!("session-{index}"), EvidenceDropReason::QueueFull);
    }

    let snapshot = accumulator.take();
    let sessions: std::collections::BTreeSet<&str> = snapshot
        .batches
        .keys()
        .map(|(session, _, _)| session.as_str())
        .collect();
    assert_eq!(sessions.len(), MAX_TRACKED_DROP_SESSIONS);
    // The overflow is not discarded: it is reported without a session, so the total stays honest
    // where the attribution cannot be.
    assert_eq!(
        snapshot.unattributed,
        (MAX_TRACKED_DROP_SESSIONS * 3) as u32
    );
}

/// One session cannot fill the table with reasons either. Every reason is an enum variant today,
/// so the cap has slack; it is asserted so a future variant cannot slip past unnoticed.
#[test]
fn the_drop_accumulator_caps_the_reasons_per_session() {
    let accumulator = DropAccumulator::default();
    for reason in [
        EvidenceDropReason::QueueFull,
        EvidenceDropReason::WorkerGone,
        EvidenceDropReason::UnmappableSignal,
        EvidenceDropReason::PersistenceFailed,
    ] {
        accumulator.record(SESSION, reason);
    }

    let snapshot = accumulator.take();
    assert!(snapshot.batches.len() <= MAX_TRACKED_DROP_REASONS);
    assert_eq!(snapshot.batches.len(), 4, "each reason opens its own batch");
    assert_eq!(snapshot.unattributed, 0);
}

/// Taking a snapshot clears the accumulator, so a drop recorded during a flush lands in a fresh
/// entry rather than being erased by it. A read-then-clear would lose exactly the drops that
/// happened while the report was in flight, which is when drops are most likely.
#[test]
fn a_drop_recorded_during_a_flush_is_not_lost() {
    let accumulator = DropAccumulator::default();
    accumulator.record(SESSION, EvidenceDropReason::QueueFull);

    let first = accumulator.take();
    accumulator.record(SESSION, EvidenceDropReason::QueueFull);
    let second = accumulator.take();

    assert_eq!(first.batches.values().sum::<u32>(), 1);
    assert_eq!(second.batches.values().sum::<u32>(), 1);
}

/// A signal the journal cannot correlate is counted, not silently forgotten. Dropping it without a
/// trace would leave a hole no coverage state accounts for.
#[test]
fn an_unmappable_signal_is_counted_as_a_gap() {
    let harness = harness("bridge-unmappable-counted");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::RunStarted {
            session_id: SESSION.to_string(),
            run_id: "not-a-uuid".to_string(),
            trace_id: TRACE.to_string(),
            agent_id: None,
            seat_id: None,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    // Something the worker will accept, so the flush runs.
    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));

    assert!(wait_until(|| *harness
        .diagnostics
        .dropped
        .lock()
        .expect("dropped")
        > 0));
    worker.shutdown();
}

/// A reason code the bridge had to rewrite carries a receipt naming the rule.
///
/// Rewriting silently would leave a stored value that does not match what the producer sent, next
/// to a receipt claiming nothing was redacted — and the receipt is the only way to tell a policy
/// rewrite from a producer bug.
#[test]
fn a_rewritten_reason_code_carries_its_redaction_rule() {
    let harness = harness("bridge-redaction-receipt");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    OperationsEvidencePort::try_publish(
        &bridge,
        OperationsEvidenceSignal::OperationFailed {
            session_id: SESSION.to_string(),
            operation_id: "operation-1".to_string(),
            run_id: None,
            // Prose, not a code: the mapper must replace it and say so.
            reason_code: "Connection refused after 3 attempts".to_string(),
            occurred_at: "2026-08-22T10:00:02Z".to_string(),
        },
    );

    assert!(wait_until(|| watermark(&harness.api) >= 1));
    worker.shutdown();
    assert_eq!(
        stored_redaction_rules(&harness),
        vec!["reason_code_normalized".to_string()],
        "a rewritten value must name the rule that rewrote it"
    );
}

/// A code that already fits passes through untouched and its receipt says so. A receipt claiming a
/// redaction on every event would make the real ones impossible to find.
#[test]
fn an_acceptable_reason_code_records_no_redaction() {
    let harness = harness("bridge-redaction-none");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    OperationsEvidencePort::try_publish(
        &bridge,
        OperationsEvidenceSignal::OperationFailed {
            session_id: SESSION.to_string(),
            operation_id: "operation-1".to_string(),
            run_id: None,
            reason_code: "runner_unavailable".to_string(),
            occurred_at: "2026-08-22T10:00:02Z".to_string(),
        },
    );

    assert!(wait_until(|| watermark(&harness.api) >= 1));
    worker.shutdown();
    assert!(stored_redaction_rules(&harness).is_empty());
}

fn tool_started(call_id: &str, attempt: Option<u32>) -> AgentEvidenceSignal {
    AgentEvidenceSignal::ToolStarted {
        session_id: SESSION.to_string(),
        run_id: run_id(1),
        trace_id: TRACE.to_string(),
        span_id: Some("00f067aa0ba902b7".to_string()),
        agent_id: Some("agent-1".to_string()),
        seat_id: Some("seat-builder".to_string()),
        call_id: call_id.to_string(),
        tool_name: "read_file".to_string(),
        observation: AgentEvidenceObservation::Direct,
        attempt,
        occurred_at: "2026-08-22T10:00:00Z".to_string(),
    }
}

fn tool_finished(
    call_id: &str,
    attempt: Option<u32>,
    observation: AgentEvidenceObservation,
    outcome: AgentRunEvidenceOutcome,
) -> AgentEvidenceSignal {
    AgentEvidenceSignal::ToolFinished {
        session_id: SESSION.to_string(),
        run_id: run_id(1),
        trace_id: TRACE.to_string(),
        span_id: Some("00f067aa0ba902b7".to_string()),
        agent_id: Some("agent-1".to_string()),
        seat_id: Some("seat-builder".to_string()),
        call_id: call_id.to_string(),
        tool_name: "read_file".to_string(),
        observation,
        attempt,
        outcome,
        occurred_at: "2026-08-22T10:00:05Z".to_string(),
    }
}

/// The one projected record of a kind, so a test can assert on what a reader would see rather than
/// on what the bridge believes it sent.
fn single_record(
    harness: &Harness,
    kind: &str,
) -> crate::contexts::execution_observability::api::evidence::ExecutionRecordProjection {
    let page = harness
        .api
        .list_records(
            crate::contexts::execution_observability::api::evidence::ExecutionRecordQuery {
                scope:
                    crate::contexts::execution_observability::api::evidence::EvidenceQueryScope {
                        session_id: Some(session()),
                        ..Default::default()
                    },
                filters: Default::default(),
                cursor: None,
                limit: 100,
            },
        )
        .expect("page");
    page.items
        .into_iter()
        .find(|record| record.kind.as_str() == kind)
        .unwrap_or_else(|| panic!("no {kind} record was projected"))
}

fn journal_event_count(harness: &Harness) -> i64 {
    harness
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_events",
            [],
            |row| row.get(0),
        )
        .expect("count")
}

fn stored_payload_json(harness: &Harness) -> String {
    harness
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT safe_payload_json FROM execution_evidence_events ORDER BY sequence DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("stored payload")
}

/// A tool call that started and finished reaches the journal as one record with a terminal status.
#[test]
fn a_tool_lifecycle_reaches_a_terminal_record() {
    let harness = harness("bridge-tool-lifecycle");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(&bridge, tool_started("call-1", None));
    AgentEvidencePort::try_publish(
        &bridge,
        tool_finished(
            "call-1",
            None,
            AgentEvidenceObservation::Direct,
            AgentRunEvidenceOutcome::Failed,
        ),
    );

    assert!(wait_until(|| watermark(&harness.api) >= 2));
    worker.shutdown();
    let record = single_record(&harness, "tool");
    assert_eq!(
        crate::contexts::execution_observability::api::evidence::status_token(record.status),
        "failed"
    );
    assert_eq!(
        record.seat_id.map(|seat| seat.as_str().to_string()),
        Some("seat-builder".to_string())
    );
}

/// Two attempts of one call are two executions, so they key separately. Sharing a source id would
/// make the journal treat the retry as a duplicate and keep only the first, so a call that failed
/// and then succeeded would read as having only failed.
#[test]
fn two_attempts_of_one_call_are_two_observations() {
    let harness = harness("bridge-tool-attempts");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(
        &bridge,
        tool_finished(
            "call-1",
            Some(1),
            AgentEvidenceObservation::Direct,
            AgentRunEvidenceOutcome::Failed,
        ),
    );
    AgentEvidencePort::try_publish(
        &bridge,
        tool_finished(
            "call-1",
            Some(2),
            AgentEvidenceObservation::Direct,
            AgentRunEvidenceOutcome::Succeeded,
        ),
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
}

/// One observation delivered twice is one event. This is what makes a duplicated provider
/// callback, a resume, and a restart replay converge on the same journal.
#[test]
fn a_duplicate_completion_is_recorded_once() {
    let harness = harness("bridge-tool-duplicate");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    let signal = tool_finished(
        "call-1",
        Some(1),
        AgentEvidenceObservation::Direct,
        AgentRunEvidenceOutcome::Succeeded,
    );
    AgentEvidencePort::try_publish(&bridge, signal.clone());
    AgentEvidencePort::try_publish(&bridge, signal);

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    assert_eq!(
        journal_event_count(&harness),
        1,
        "a second delivery of one observation must not become a second event"
    );
}

/// A delegation is recorded as a delegation, not as a tool named after whichever handler ran it.
#[test]
fn a_delegation_lifecycle_reaches_a_delegation_record() {
    let harness = harness("bridge-delegation-lifecycle");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::DelegationStarted {
            session_id: SESSION.to_string(),
            run_id: run_id(1),
            trace_id: TRACE.to_string(),
            span_id: None,
            parent_agent_id: Some("agent-1".to_string()),
            seat_id: None,
            delegation_id: "delegation-1".to_string(),
            call_id: "call-1".to_string(),
            attempt: Some(1),
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::DelegationFinished {
            session_id: SESSION.to_string(),
            run_id: run_id(1),
            trace_id: TRACE.to_string(),
            span_id: None,
            parent_agent_id: Some("agent-1".to_string()),
            seat_id: None,
            delegation_id: "delegation-1".to_string(),
            call_id: "call-1".to_string(),
            attempt: Some(1),
            outcome: AgentRunEvidenceOutcome::Cancelled,
            occurred_at: "2026-08-22T10:00:09Z".to_string(),
        },
    );

    assert!(wait_until(|| watermark(&harness.api) >= 2));
    worker.shutdown();
    assert_eq!(
        crate::contexts::execution_observability::api::evidence::status_token(
            single_record(&harness, "delegation").status
        ),
        "cancelled"
    );
}

/// A reconstructed observation must not be filed as one the runtime watched. Fidelity is the only
/// field a reader has to weigh a record by, and upgrading it is a claim nobody can back.
#[test]
fn a_reconstructed_observation_is_never_recorded_as_native() {
    let harness = harness("bridge-tool-fidelity");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(
        &bridge,
        tool_finished(
            "call-1",
            None,
            AgentEvidenceObservation::Reconstructed,
            AgentRunEvidenceOutcome::Succeeded,
        ),
    );

    assert!(wait_until(|| watermark(&harness.api) >= 1));
    worker.shutdown();
    assert_eq!(
        crate::contexts::execution_observability::api::evidence::fidelity_token(
            single_record(&harness, "tool").fidelity
        ),
        "inferred"
    );
}

/// A tool whose completion was seen without its start keeps the completion and omits the start.
#[test]
fn a_completion_only_tool_call_omits_its_start() {
    let harness = harness("bridge-tool-completion-only");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(
        &bridge,
        tool_finished(
            "call-1",
            None,
            AgentEvidenceObservation::Direct,
            AgentRunEvidenceOutcome::Succeeded,
        ),
    );

    assert!(wait_until(|| watermark(&harness.api) >= 1));
    worker.shutdown();
    let record = single_record(&harness, "tool");
    assert!(
        record.started_at.is_none(),
        "no start was observed, so none is reported"
    );
    assert_eq!(record.ended_at.as_deref(), Some("2026-08-22T10:00:05Z"));
}

/// The tool's name and its ids reach the journal; nothing it was asked to do or returned does.
#[test]
fn a_tool_record_carries_no_arguments_or_results() {
    let harness = harness("bridge-tool-no-payload");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(&bridge, tool_started("call-1", None));

    assert!(wait_until(|| watermark(&harness.api) >= 1));
    worker.shutdown();
    let stored = stored_payload_json(&harness);
    assert!(
        stored.contains("read_file"),
        "the tool name is the point of the record"
    );
    for forbidden in ["arguments", "input", "result", "output", "content"] {
        assert!(
            !stored.contains(forbidden),
            "the stored payload carried a {forbidden} field: {stored}"
        );
    }
}

/// A shell that opened and closed reaches a terminal record carrying why it ended.
#[test]
fn a_shell_lifecycle_records_its_close_reason() {
    let harness = harness("bridge-shell-lifecycle");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    WorkspaceEvidencePort::try_publish(
        &bridge,
        WorkspaceEvidenceSignal::ShellOpened {
            session_id: SESSION.to_string(),
            shell_id: "shell-1".to_string(),
            seat_id: None,
            runtime: WorkspaceShellRuntimeKind::Local,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    WorkspaceEvidencePort::try_publish(
        &bridge,
        WorkspaceEvidenceSignal::ShellClosed {
            session_id: SESSION.to_string(),
            shell_id: "shell-1".to_string(),
            seat_id: None,
            reason: WorkspaceShellCloseReason::RemoteDisconnect,
            occurred_at: "2026-08-22T10:30:00Z".to_string(),
        },
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
    assert!(stored_payload_json(&harness).contains("shell_remote_disconnected"));
}

/// One shell ends once. A close delivered twice — a stop racing a shutdown sweep — must not put a
/// second ending in the journal, because a reader counting closes would see two shells.
#[test]
fn a_shell_closes_once_however_many_times_it_is_reported() {
    let harness = harness("bridge-shell-close-once");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for _ in 0..3 {
        WorkspaceEvidencePort::try_publish(
            &bridge,
            WorkspaceEvidenceSignal::ShellClosed {
                session_id: SESSION.to_string(),
                shell_id: "shell-1".to_string(),
                seat_id: None,
                reason: WorkspaceShellCloseReason::ExplicitClose,
                occurred_at: "2026-08-22T10:30:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    assert_eq!(journal_event_count(&harness), 1);
}

/// Every close reason maps to its own code. Collapsing them would make an idle sweep and a remote
/// connection drop indistinguishable, and only one of those is worth investigating.
#[test]
fn every_shell_close_reason_has_its_own_code() {
    let harness = harness("bridge-shell-close-reasons");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());
    let reasons = [
        WorkspaceShellCloseReason::ExplicitClose,
        WorkspaceShellCloseReason::ProcessExit,
        WorkspaceShellCloseReason::RemoteDisconnect,
        WorkspaceShellCloseReason::IdleCleanup,
        WorkspaceShellCloseReason::Shutdown,
    ];

    for (index, reason) in reasons.into_iter().enumerate() {
        WorkspaceEvidencePort::try_publish(
            &bridge,
            WorkspaceEvidenceSignal::ShellClosed {
                session_id: SESSION.to_string(),
                shell_id: format!("shell-{index}"),
                seat_id: None,
                reason,
                occurred_at: "2026-08-22T10:30:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(
        || journal_event_count(&harness) >= reasons.len() as i64
    ));
    worker.shutdown();
    let codes = stored_reason_codes(&harness);
    assert_eq!(
        codes.len(),
        reasons.len(),
        "two reasons collapsed onto one code: {codes:?}"
    );
}

fn file_mutation(basename: &str, kind: WorkspaceFileChangeKind) -> WorkspaceEvidenceSignal {
    WorkspaceEvidenceSignal::FileMutationObserved {
        session_id: SESSION.to_string(),
        basename: basename.to_string(),
        path_fingerprint: "0123456789abcdef0123456789abcdef".to_string(),
        change_kind: kind,
        witness_fingerprint: "witness-1".to_string(),
        observed_directly: true,
        occurred_at: "2026-08-22T10:00:00Z".to_string(),
    }
}

/// A file change records the file's name and a digest of its path, never the path.
#[test]
fn a_file_mutation_records_a_basename_and_a_fingerprint_only() {
    let harness = harness("bridge-file-mutation");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    WorkspaceEvidencePort::try_publish(
        &bridge,
        file_mutation("main.rs", WorkspaceFileChangeKind::Created),
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    let stored = stored_payload_json(&harness);
    assert!(stored.contains("main.rs"));
    assert!(stored.contains("0123456789abcdef0123456789abcdef"));
    for forbidden in ["/", "\\", "C:", "content", "diff"] {
        assert!(
            !stored.contains(forbidden),
            "a file mutation carried {forbidden}: {stored}"
        );
    }
}

/// A retried write against the same witness is one observation. Without the witness in the key a
/// retry would double the count, and a file changed once would read as changed twice.
#[test]
fn a_retried_mutation_against_one_witness_is_recorded_once() {
    let harness = harness("bridge-file-retry");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for _ in 0..3 {
        WorkspaceEvidencePort::try_publish(
            &bridge,
            file_mutation("main.rs", WorkspaceFileChangeKind::Modified),
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    assert_eq!(journal_event_count(&harness), 1);
}

/// A change found by comparing snapshots was not watched happening, and the runtime cannot say who
/// made it. Recording that as native would claim otherwise.
#[test]
fn a_snapshot_detected_change_is_recorded_as_inferred() {
    let harness = harness("bridge-file-inferred");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    let WorkspaceEvidenceSignal::FileMutationObserved {
        session_id,
        basename,
        path_fingerprint,
        change_kind,
        witness_fingerprint,
        occurred_at,
        ..
    } = file_mutation("main.rs", WorkspaceFileChangeKind::Modified)
    else {
        unreachable!("constructed above")
    };
    WorkspaceEvidencePort::try_publish(
        &bridge,
        WorkspaceEvidenceSignal::FileMutationObserved {
            session_id,
            basename,
            path_fingerprint,
            change_kind,
            witness_fingerprint,
            observed_directly: false,
            occurred_at,
        },
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    assert_eq!(stored_fidelity(&harness), "inferred");
}

fn stored_reason_codes(harness: &Harness) -> std::collections::BTreeSet<String> {
    let connection = harness.database.connection().expect("connection");
    let mut statement = connection
        .prepare("SELECT safe_payload_json FROM execution_evidence_events")
        .expect("prepare");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query");
    rows.filter_map(Result::ok).collect()
}

/// A review decision reaches the journal as a decision about a review, with the snapshot it was
/// made against.
#[test]
fn a_review_decision_records_its_scope_and_witness() {
    let harness = harness("bridge-review-decision");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::ReviewDecisionRecorded {
            session_id: SESSION.to_string(),
            review_id: "review-1".to_string(),
            decision: SessionReviewDecision::ChangesRequested,
            witness_fingerprint: "snapshot-a".to_string(),
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    let stored = stored_payload_json(&harness);
    assert!(
        stored.contains("review"),
        "the scope is review-level: {stored}"
    );
    // A review-level decision says nothing about any hunk or any file. Both of those have their
    // own signals and their own stores; deriving either from this would be a guess wearing an
    // observation's clothes.
    assert!(!stored.contains("hunk"));
    assert!(!stored.contains("file_viewed"));
}

/// Reading a file reaches the journal under its own scope, carrying no judgement.
#[test]
fn a_file_viewed_records_the_file_viewed_scope_without_a_decision() {
    let harness = harness("bridge-file-viewed");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::ReviewFileViewedRecorded {
            session_id: SESSION.to_string(),
            review_id: "review-1".to_string(),
            file_witness: "file-witness-1".to_string(),
            witness_fingerprint: "snapshot-a".to_string(),
            occurred_at: "2026-08-27T10:00:00Z".to_string(),
        },
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    let stored = stored_payload_json(&harness);
    assert!(
        stored.contains("file_viewed"),
        "the scope says a file was read: {stored}"
    );
    // Having read a file is not a judgement about it, and `pending` is the value that says so.
    // Recording it as accepted would report an approval the reviewer never gave.
    assert!(!stored.contains("accepted"));
    assert!(!stored.contains("changes_requested"));
    // The path is not in the journal, for the same reason a hunk decision carries a fingerprint.
    assert!(!stored.contains("src/"));
}

/// Re-reading the same version of the same file is the same observation. Reading it again after it
/// changed is a different one, because the witness changed with it.
#[test]
fn re_reading_the_same_file_records_once_and_a_changed_file_records_again() {
    let harness = harness("bridge-file-viewed-again");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for witness in ["file-witness-1", "file-witness-1", "file-witness-2"] {
        SessionEvidencePort::try_publish(
            &bridge,
            SessionEvidenceSignal::ReviewFileViewedRecorded {
                session_id: SESSION.to_string(),
                review_id: "review-1".to_string(),
                file_witness: witness.to_string(),
                witness_fingerprint: "snapshot-a".to_string(),
                occurred_at: "2026-08-27T10:00:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
    assert_eq!(journal_event_count(&harness), 2);
}

/// A hunk decision reaches the journal under its own scope, from its own signal.
#[test]
fn a_hunk_decision_records_the_hunk_scope() {
    let harness = harness("bridge-hunk-decision");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::ReviewHunkDecisionRecorded {
            session_id: SESSION.to_string(),
            review_id: "review-1".to_string(),
            hunk_fingerprint: "hunk-1".to_string(),
            decision: SessionReviewDecision::Accepted,
            witness_fingerprint: "snapshot-a".to_string(),
            occurred_at: "2026-08-27T10:00:00Z".to_string(),
        },
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    let stored = stored_payload_json(&harness);
    assert!(stored.contains("hunk"), "the scope is the hunk: {stored}");
    // The fingerprint identifies the hunk in the event id; the path never enters the journal, so a
    // reader learns which hunk without the journal holding workspace content.
    assert!(!stored.contains("src/"));
}

/// Two hunks decided in the same review are two decisions. Keying on the review alone would fold
/// them into one and report a reviewer who accepted two hunks as having accepted one.
#[test]
fn two_hunks_decided_in_one_review_record_separately() {
    let harness = harness("bridge-hunk-decision-pair");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for hunk in ["hunk-1", "hunk-2"] {
        SessionEvidencePort::try_publish(
            &bridge,
            SessionEvidenceSignal::ReviewHunkDecisionRecorded {
                session_id: SESSION.to_string(),
                review_id: "review-1".to_string(),
                hunk_fingerprint: hunk.to_string(),
                decision: SessionReviewDecision::Accepted,
                witness_fingerprint: "snapshot-a".to_string(),
                occurred_at: "2026-08-27T10:00:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
}

/// The same decision published twice is one event. The bridge retries, and a retry that minted a
/// second identity would report a reviewer as having decided as many times as the machine stumbled.
#[test]
fn the_same_hunk_decision_published_twice_records_once() {
    let harness = harness("bridge-hunk-decision-retry");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for _ in 0..2 {
        SessionEvidencePort::try_publish(
            &bridge,
            SessionEvidenceSignal::ReviewHunkDecisionRecorded {
                session_id: SESSION.to_string(),
                review_id: "review-1".to_string(),
                hunk_fingerprint: "hunk-1".to_string(),
                decision: SessionReviewDecision::Accepted,
                witness_fingerprint: "snapshot-a".to_string(),
                occurred_at: "2026-08-27T10:00:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    assert_eq!(journal_event_count(&harness), 1);
}

/// Re-deciding a review after its diff moved on is a second decision. Keying on the review alone
/// would keep only the first, so a reviewer who accepted, saw new changes, and asked for more would
/// read as having only accepted.
#[test]
fn a_review_re_decided_against_a_new_snapshot_records_again() {
    let harness = harness("bridge-review-re-decided");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for witness in ["snapshot-a", "snapshot-b"] {
        SessionEvidencePort::try_publish(
            &bridge,
            SessionEvidenceSignal::ReviewDecisionRecorded {
                session_id: SESSION.to_string(),
                review_id: "review-1".to_string(),
                decision: SessionReviewDecision::Accepted,
                witness_fingerprint: witness.to_string(),
                occurred_at: "2026-08-22T10:00:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
}

/// A verification records its name and counts, and reaches a terminal record.
#[test]
fn a_verification_outcome_records_its_counts() {
    let harness = harness("bridge-verification");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::VerificationCompleted {
            session_id: SESSION.to_string(),
            run_id: None,
            verification_run_id: "verification-1".to_string(),
            name: "cargo test".to_string(),
            outcome: SessionVerificationOutcome::Failed,
            passed_count: Some(138),
            failed_count: Some(2),
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    let record = single_record(&harness, "verification");
    assert_eq!(
        crate::contexts::execution_observability::api::evidence::status_token(record.status),
        "failed"
    );
    let stored = stored_payload_json(&harness);
    assert!(stored.contains("138") && stored.contains('2'));
}

/// Usage evidence points at an observation; it never restates it. A second copy of a total is a
/// second number that can disagree with the first, with nothing to say which is right.
#[test]
fn usage_evidence_carries_no_totals_or_cost() {
    let harness = harness("bridge-usage-no-totals");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::UsageObserved {
            session_id: SESSION.to_string(),
            invocation_id: "invocation-1".to_string(),
            run_id: None,
            quality: SessionUsageEvidenceQuality::Estimated,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    let stored = stored_payload_json(&harness);
    assert!(
        stored.contains("estimated"),
        "the quality travels: {stored}"
    );
    for forbidden in [
        "inputTokens",
        "outputTokens",
        "cache",
        "cost",
        "price",
        "usd",
    ] {
        assert!(
            !stored.contains(forbidden),
            "usage evidence restated a {forbidden}: {stored}"
        );
    }
}

/// One invocation is one usage observation however many times it is reported.
#[test]
fn a_repeated_usage_reference_is_recorded_once() {
    let harness = harness("bridge-usage-idempotent");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for _ in 0..3 {
        SessionEvidencePort::try_publish(
            &bridge,
            SessionEvidenceSignal::UsageObserved {
                session_id: SESSION.to_string(),
                invocation_id: "invocation-1".to_string(),
                run_id: None,
                quality: SessionUsageEvidenceQuality::Reported,
                occurred_at: "2026-08-22T10:00:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    assert_eq!(journal_event_count(&harness), 1);
}

fn stored_fidelity(harness: &Harness) -> String {
    harness
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT fidelity FROM execution_evidence_events ORDER BY sequence DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("fidelity")
}

/// A failed flush puts its snapshot back by merging, so a drop recorded while the report was in
/// flight is added to the restored count rather than overwritten by it.
#[test]
fn restoring_a_failed_flush_merges_rather_than_overwrites() {
    let accumulator = DropAccumulator::default();
    accumulator.record(SESSION, EvidenceDropReason::QueueFull);
    let snapshot = accumulator.take();

    accumulator.record(SESSION, EvidenceDropReason::QueueFull);
    accumulator.restore(snapshot);

    assert_eq!(accumulator.take().batches.values().sum::<u32>(), 2);
}

#[test]
fn a_drop_count_saturates_rather_than_wrapping() {
    let accumulator = DropAccumulator::default();
    accumulator.record(SESSION, EvidenceDropReason::QueueFull);
    let snapshot = accumulator.take();
    let key = snapshot.batches.keys().next().expect("one batch").clone();
    accumulator.restore(snapshot);
    accumulator.restore(DropSnapshot {
        batches: std::iter::once((key.clone(), u32::MAX)).collect(),
        unattributed: 0,
    });

    // A wrapped count would report a smaller gap than occurred, which reads as less loss, not more.
    assert_eq!(
        *accumulator.take().batches.get(&key).expect("count"),
        u32::MAX
    );
}

/// The drop becomes a durable marker, not just a notice.
///
/// A notice is gone once the app restarts, so a session whose evidence was dropped during a burst
/// would read as complete afterwards. The marker is what keeps the coverage honest across a
/// restart, and it says how many were lost and why.
#[test]
fn a_queue_overflow_persists_a_coverage_gap_marker() {
    let harness = harness("bridge-gap-marker");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    // Far more than the queue holds, published faster than one SQLite write per item can drain.
    for index in 0..(EVIDENCE_QUEUE_CAPACITY * 8) {
        AgentEvidencePort::try_publish(&bridge, run_started(&run_id(index)));
    }

    assert!(
        wait_until(|| gap_marker_payload(&harness).is_some()),
        "an overflow left no durable record that anything was lost"
    );
    worker.shutdown();
    assert!(gap_marker_payload(&harness)
        .expect("a gap marker")
        .contains("evidence_queue_full"));
}

/// The marker carries a count and a reason, and nothing that could name what was lost.
///
/// This drives the persistence-failure path: a run id the producer could not supply as a UUID maps
/// to an input with no run correlation, which the domain refuses at record time. The refusal is a
/// gap, and the gap is what has to be durable.
#[test]
fn a_gap_marker_carries_only_a_count_and_a_reason() {
    let harness = harness("bridge-gap-marker-shape");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::RunStarted {
            session_id: SESSION.to_string(),
            run_id: "not-a-uuid".to_string(),
            trace_id: TRACE.to_string(),
            agent_id: None,
            seat_id: None,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    // A second signal so the worker loops again and flushes what the first one left behind.
    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));

    assert!(wait_until(|| gap_marker_payload(&harness).is_some()));
    worker.shutdown();
    let marker = gap_marker_payload(&harness).expect("a gap marker");
    assert!(
        marker.contains("evidence_persistence_failed"),
        "marker was: {marker}"
    );
    assert!(marker.contains("dropped_count"));
    for forbidden in ["not-a-uuid", "session-1", "agent", "tool", "trace"] {
        assert!(
            !marker.contains(forbidden),
            "the marker named what was lost: {marker}"
        );
    }
}

/// A drop the bridge cannot attribute to a session produces no marker.
///
/// The journal keys on a session. Filing a gap under a placeholder would attribute a loss to work
/// that lost nothing, which is worse than reporting it only as a count.
#[test]
fn a_sessionless_drop_never_becomes_a_marker() {
    let harness = harness("bridge-gap-no-session");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::RunStarted {
            // Unparseable: the mapper returns nothing and the drop has no session to key on.
            session_id: String::new(),
            run_id: run_id(1),
            trace_id: TRACE.to_string(),
            agent_id: None,
            seat_id: None,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(2)));

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    assert!(
        gap_marker_payload(&harness).is_none(),
        "a sessionless drop was filed against some session anyway"
    );
}

/// Reporting a gap must not produce a gap. A failure to record the marker is returned and the
/// count is kept; counting it again would describe failing to describe a failure, which never
/// settles.
#[test]
fn recording_a_gap_never_produces_another_gap() {
    let bridge = fs_read_bridge();
    let flush = bridge
        .split("fn flush_drops(")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("the flush is a free function");
    assert!(
        !flush.contains("drops.record("),
        "the flush counts its own failure, which is a gap about reporting a gap"
    );
    let marker = bridge
        .split("fn record_gap_marker(")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("the marker writer is a free function");
    assert!(
        !marker.contains("try_send") && !marker.contains("sender"),
        "the marker goes through the recorder, never the queue that produced the gap"
    );
}

fn fs_read_bridge() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bootstrap/evidence_bridge.rs"),
    )
    .expect("read the evidence bridge")
}

fn gap_marker_payload(harness: &Harness) -> Option<String> {
    harness
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT safe_payload_json FROM execution_evidence_events \
             WHERE kind = 'coverage.gap.recorded' ORDER BY sequence DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok()
}

/// One sentinel per category a producer could leak, each unique enough that a substring match
/// cannot be coincidence.
const PRIVACY_SENTINELS: &[(&str, &str)] = &[
    ("raw prompt", "SENTINELPROMPTaaa please summarise this"),
    ("model output", "SENTINELOUTPUTbbb here is the answer"),
    ("tool arguments", r#"{"path": "SENTINELTOOLARGccc"}"#),
    ("tool result", "SENTINELRESULTddd 42 rows"),
    ("terminal text", "$ npm test\nSENTINELTERMeee"),
    ("source code", "fn main() { SENTINELCODEfff(); }"),
    ("diff", "@@ -1,2 +1,3 @@\n+SENTINELDIFFggg"),
    ("windows path", r"C:\Users\SENTINELWINhhh\notes.txt"),
    ("unix path", "/home/SENTINELNIXiii/notes.txt"),
    ("unc path", r"\\server\share\SENTINELUNCjjj"),
    (
        "authorization header",
        "Authorization: Bearer SENTINELTOKENkkk",
    ),
    ("env secret", "AWS_SECRET_ACCESS_KEY=SENTINELENVlll"),
    ("private key", "-----BEGIN PRIVATE KEY-----SENTINELKEYmmm"),
];

/// Content passed where the contract expects a code is removed, not stored.
///
/// A reason code is the one free-text field a caller reliably gets wrong: an error message goes in
/// where a code belongs. Every sentinel is normalized to a generic code, and the check reads the
/// tables rather than the mapper, because what matters is not what the mapper meant to drop but
/// what the bytes on disk contain.
#[test]
fn no_privacy_sentinel_survives_a_reason_code() {
    let harness = harness("bridge-privacy-sentinels");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for (index, (_, sentinel)) in PRIVACY_SENTINELS.iter().enumerate() {
        OperationsEvidencePort::try_publish(
            &bridge,
            OperationsEvidenceSignal::OperationFailed {
                session_id: SESSION.to_string(),
                operation_id: format!("operation-{index}"),
                run_id: None,
                reason_code: (*sentinel).to_string(),
                occurred_at: "2026-08-22T10:00:00Z".to_string(),
            },
        );
    }
    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(9)));
    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();

    let stored = whole_store(&harness);
    for (label, sentinel) in PRIVACY_SENTINELS {
        assert!(
            !stored.contains(sentinel),
            "a {label} sentinel reached the store"
        );
        // The distinguishing token alone, in case a bound truncated the sentinel rather than
        // rejecting it: half a secret in a journal is still a secret in a journal.
        let core: String = sentinel
            .chars()
            .filter(char::is_ascii_uppercase)
            .collect::<String>();
        if core.len() >= 8 {
            assert!(
                !stored.contains(&core),
                "a truncated {label} sentinel reached the store"
            );
        }
    }
}

/// The producer signals have no field content could arrive in.
///
/// This is the guarantee the injection tests cannot make: a prompt passed as a verification name
/// is stored, because a name is what that field is for and the journal stores names. What keeps a
/// prompt out is that no signal has a field for one. A field added later with any of these names
/// would open that door silently, so the door is checked rather than watched.
#[test]
fn no_producer_signal_declares_a_field_that_could_hold_content() {
    let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/contexts");
    let ports = [
        "agent_runtime/application/evidence.rs",
        "workspaces/application/evidence.rs",
        "operations/application/evidence.rs",
        "sessions/application/evidence.rs",
    ];
    // Field names, not words: `path_fingerprint` is a digest and `tool_name` is a name, so the
    // check is anchored to a field declaration rather than to a substring anywhere in the file.
    let forbidden_fields = [
        "prompt:",
        "output:",
        "arguments:",
        "args:",
        "result:",
        "content:",
        "body:",
        "diff:",
        "patch:",
        "transcript:",
        "stdout:",
        "stderr:",
        "path:",
        "absolute_path:",
        "token:",
        "secret:",
        "header:",
        "environment:",
        "source_code:",
    ];

    let mut violations = Vec::new();
    for relative in ports {
        let source = std::fs::read_to_string(source_root.join(relative))
            .unwrap_or_else(|_| panic!("every producer declares an evidence port: {relative}"));
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            for field in forbidden_fields {
                if trimmed.starts_with(field) {
                    violations.push(format!("{relative}: declares `{trimmed}`"));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "a producer signal gained a field content could arrive in:
{}",
        violations.join(
            "
"
        )
    );
}

/// The domain refuses a label that is shaped like a path or carries a control character, so a
/// producer that passed one gets no record rather than a record naming a location.
#[test]
fn a_path_shaped_basename_is_refused_rather_than_stored() {
    let harness = harness("bridge-path-basename");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for path in [
        r"C:\Users\someone\notes.txt",
        "/home/someone/notes.txt",
        r"\\server\share\notes.txt",
    ] {
        WorkspaceEvidencePort::try_publish(
            &bridge,
            WorkspaceEvidenceSignal::FileMutationObserved {
                session_id: SESSION.to_string(),
                basename: path.to_string(),
                path_fingerprint: "0123456789abcdef0123456789abcdef".to_string(),
                change_kind: WorkspaceFileChangeKind::Modified,
                witness_fingerprint: "witness-1".to_string(),
                observed_directly: true,
                occurred_at: "2026-08-22T10:00:00Z".to_string(),
            },
        );
    }
    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    let stored = whole_store(&harness);
    assert!(!stored.contains("someone"));
    assert!(!stored.contains("server"));
}

/// Reads every column a producer's values could land in, across the journal, the projection, and
/// the coverage metadata. Scanning one table would miss a value the projection copied out.
fn whole_store(harness: &Harness) -> String {
    let connection = harness.database.connection().expect("connection");
    let mut dumped = String::new();
    for table in [
        "execution_evidence_events",
        "execution_evidence_records",
        "execution_evidence_coverage",
    ] {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM {table}"))
            .expect("prepare");
        let column_count = statement.column_count();
        let mut rows = statement.query([]).expect("query");
        while let Some(row) = rows.next().expect("row") {
            for index in 0..column_count {
                if let Ok(value) = row.get::<_, String>(index) {
                    dumped.push_str(&value);
                    dumped.push('\n');
                }
            }
        }
    }
    dumped
}

/// Every producer subject, published while the journal cannot take any of it.
///
/// The worker is gone, so each send is refused. What is being checked is that a producer can issue
/// any of these and carry on: none returns a value, none can fail, and none blocks. A regression
/// that made publishing fallible would not compile; one that made it blocking would hang here.
#[test]
fn no_producer_subject_is_affected_by_an_unavailable_recorder() {
    let harness = harness("bridge-recorder-unavailable");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());
    drop(worker);

    let started = Instant::now();
    // Agent, tool, delegation.
    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));
    AgentEvidencePort::try_publish(&bridge, tool_started("call-1", None));
    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::DelegationStarted {
            session_id: SESSION.to_string(),
            run_id: run_id(1),
            trace_id: TRACE.to_string(),
            span_id: None,
            parent_agent_id: None,
            seat_id: None,
            delegation_id: "delegation-1".to_string(),
            call_id: "call-1".to_string(),
            attempt: None,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    // Shell, command boundary, file mutation.
    WorkspaceEvidencePort::try_publish(
        &bridge,
        WorkspaceEvidenceSignal::ShellOpened {
            session_id: SESSION.to_string(),
            shell_id: "shell-1".to_string(),
            seat_id: None,
            runtime: WorkspaceShellRuntimeKind::Local,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    WorkspaceEvidencePort::try_publish(
        &bridge,
        WorkspaceEvidenceSignal::ShellClosed {
            session_id: SESSION.to_string(),
            shell_id: "shell-1".to_string(),
            seat_id: None,
            reason: WorkspaceShellCloseReason::ProcessExit,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    WorkspaceEvidencePort::try_publish(
        &bridge,
        file_mutation("main.rs", WorkspaceFileChangeKind::Modified),
    );
    // Operation failure, review, verification, usage.
    OperationsEvidencePort::try_publish(
        &bridge,
        OperationsEvidenceSignal::OperationFailed {
            session_id: SESSION.to_string(),
            operation_id: "operation-1".to_string(),
            run_id: None,
            reason_code: "runner_unavailable".to_string(),
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::ReviewDecisionRecorded {
            session_id: SESSION.to_string(),
            review_id: "review-1".to_string(),
            decision: SessionReviewDecision::Accepted,
            witness_fingerprint: "snapshot-a".to_string(),
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::VerificationCompleted {
            session_id: SESSION.to_string(),
            run_id: None,
            verification_run_id: "verification-1".to_string(),
            name: "cargo test".to_string(),
            outcome: SessionVerificationOutcome::Passed,
            passed_count: Some(1),
            failed_count: Some(0),
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::UsageObserved {
            session_id: SESSION.to_string(),
            invocation_id: "invocation-1".to_string(),
            run_id: None,
            quality: SessionUsageEvidenceQuality::Reported,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );

    // Reaching here at all is the assertion: every call returned `()`. The elapsed bound catches a
    // blocking send, which is the failure a return type cannot express.
    assert!(started.elapsed() < Duration::from_secs(2));
}

/// Two gaps of the same size are two gaps.
///
/// Before the batch id the marker was keyed by its count, so the second one collided with the
/// first. Because the content fingerprint includes the occurrence time, the journal did not treat
/// that as a harmless replay — it recorded a conflict and kept only the first, so a session that
/// lost evidence twice reported losing it once.
#[test]
fn two_equal_sized_gap_batches_have_distinct_source_ids() {
    let accumulator = DropAccumulator::default();

    accumulator.record(SESSION, EvidenceDropReason::QueueFull);
    let first = accumulator.take();
    accumulator.record(SESSION, EvidenceDropReason::QueueFull);
    let second = accumulator.take();

    let first_key = first.batches.keys().next().expect("first batch");
    let second_key = second.batches.keys().next().expect("second batch");
    assert_eq!(first.batches[first_key], second.batches[second_key]);
    assert_ne!(
        first_key.2, second_key.2,
        "same-sized gaps must not share a batch id"
    );
}

/// A retry re-sends the same batch under the same id, so the journal sees a replay rather than a
/// second gap. A fresh id per attempt would multiply one loss into as many markers as retries.
#[test]
fn a_failed_gap_flush_retries_with_the_same_batch_id() {
    let accumulator = DropAccumulator::default();
    accumulator.record(SESSION, EvidenceDropReason::QueueFull);

    let attempt = accumulator.take();
    let key = attempt.batches.keys().next().expect("batch").clone();
    accumulator.restore(attempt);
    let retry = accumulator.take();

    assert_eq!(retry.batches.keys().next(), Some(&key));
    assert_eq!(retry.batches[&key], 1);
}

/// A repository that refuses the first coverage-gap append and then behaves.
///
/// This is the ambiguous case that matters: the flush cannot tell "the write failed" from "the
/// write landed and the answer was lost", so it retries. If the retry carried a new identity the
/// one loss would become two markers, and a reader counting gaps would double them.
struct FlakyMarkerRepository {
    inner: SqliteEvidenceRepository,
    refused_once: Mutex<bool>,
}

impl crate::contexts::execution_observability::application::evidence::ports::EvidenceRepositoryPort
    for FlakyMarkerRepository
{
    fn append(
        &self,
        event: &crate::contexts::execution_observability::domain::ExecutionEvidenceEvent,
        fingerprint: &str,
        recorded_at: &str,
    ) -> Result<
        crate::contexts::execution_observability::application::evidence::ports::EvidenceAppendOutcome,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    >{
        let is_marker = event.kind()
            == crate::contexts::execution_observability::domain::EvidenceKind::CoverageGapRecorded;
        let mut refused = self.refused_once.lock().expect("refused");
        if is_marker && !*refused {
            *refused = true;
            return Err(
                crate::contexts::execution_observability::api::evidence::EvidenceApplicationError::Storage(
                    "refused once".to_string(),
                ),
            );
        }
        drop(refused);
        self.inner.append(event, fingerprint, recorded_at)
    }

    fn list_records(
        &self,
        query: &crate::contexts::execution_observability::api::evidence::ExecutionRecordQuery,
    ) -> Result<
        crate::contexts::execution_observability::api::evidence::EvidenceRecordPage,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    > {
        self.inner.list_records(query)
    }

    fn record_detail(
        &self,
        query: &crate::contexts::execution_observability::api::evidence::ExecutionRecordDetailQuery,
    ) -> Result<
        crate::contexts::execution_observability::api::evidence::ExecutionRecordDetailView,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    > {
        self.inner.record_detail(query)
    }

    fn summary(
        &self,
        query: &crate::contexts::execution_observability::api::evidence::WorkspaceEvidenceSummaryQuery,
    ) -> Result<
        crate::contexts::execution_observability::api::evidence::WorkspaceEvidenceSummary,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    > {
        self.inner.summary(query)
    }

    fn correlation_counts(
        &self,
        session_id: &EvidenceSessionId,
        run_id: Option<&str>,
    ) -> Result<
        crate::contexts::execution_observability::api::evidence::EvidenceCorrelationCounts,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    > {
        self.inner.correlation_counts(session_id, run_id)
    }

    fn subscription_bootstrap(
        &self,
        session_id: &EvidenceSessionId,
    ) -> Result<
        crate::contexts::execution_observability::api::evidence::EvidenceSubscriptionBootstrap,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    > {
        self.inner.subscription_bootstrap(session_id)
    }

    fn report_aggregate(
        &self,
        query: &crate::contexts::execution_observability::api::evidence::EvidenceReportQuery,
    ) -> Result<
        crate::contexts::execution_observability::api::evidence::EvidenceReportAggregate,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    > {
        self.inner.report_aggregate(query)
    }

    fn report_latency(
        &self,
        query: &crate::contexts::execution_observability::api::evidence::EvidenceReportQuery,
    ) -> Result<
        crate::contexts::execution_observability::api::evidence::EvidenceLatencyAggregate,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    > {
        self.inner.report_latency(query)
    }

    fn report_unattributed_gap(&self, count: u32) {
        self.inner.report_unattributed_gap(count);
    }

    fn projection_is_stale(
        &self,
    ) -> Result<
        bool,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    > {
        self.inner.projection_is_stale()
    }

    fn replay_projections(
        &self,
        session_id: Option<&EvidenceSessionId>,
    ) -> Result<
        usize,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    > {
        self.inner.replay_projections(session_id)
    }

    fn maintain_retention(
        &self,
        cutoff: &str,
        now: &str,
    ) -> Result<
        crate::contexts::execution_observability::application::evidence::ports::EvidenceRetentionSummary,
        crate::contexts::execution_observability::api::evidence::EvidenceApplicationError,
    >{
        // The trait method, not the inherent one: the repository has both, and the inherent
        // version returns the infrastructure's own outcome type.
        crate::contexts::execution_observability::application::evidence::ports::EvidenceRepositoryPort::maintain_retention(
            &self.inner, cutoff, now,
        )
    }
}

/// A retry after a refused marker converges on one event rather than adding a second.
#[test]
fn an_identical_gap_batch_retry_is_idempotent() {
    let directory = TempDirectory::new("bridge-gap-retry-idempotent");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let api = ExecutionEvidenceApi::new(
        Arc::new(FlakyMarkerRepository {
            inner: SqliteEvidenceRepository::new(database.clone()),
            refused_once: Mutex::new(false),
        }),
        Arc::new(SystemEvidenceClock),
        Arc::new(UuidEvidenceIdGenerator),
        Arc::new(DomainEvidenceRedactionValidator),
        Arc::new(SilentNotices),
        Arc::new(CountingDiagnostics::default()),
    );
    let harness = Harness {
        _directory: directory,
        api: api.clone(),
        diagnostics: Arc::new(CountingDiagnostics::default()),
        database,
    };
    let (bridge, worker) = start_evidence_bridge(api);

    // One unmappable run opens one batch; the first marker write is refused, so the batch is
    // restored and retried on the next flush.
    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::RunStarted {
            session_id: SESSION.to_string(),
            run_id: "not-a-uuid".to_string(),
            trace_id: TRACE.to_string(),
            agent_id: None,
            seat_id: None,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    for index in 1..4 {
        AgentEvidencePort::try_publish(&bridge, run_started(&run_id(index)));
    }

    assert!(wait_until(|| gap_marker_count(&harness) >= 1));
    worker.shutdown();
    assert_eq!(
        gap_marker_count(&harness),
        1,
        "the retried batch became a second marker"
    );
}

/// A drop arriving while a batch is being retried opens its own batch.
///
/// Adding it to the in-flight one would change the content behind a marker the journal may already
/// hold, and the retry would then arrive as a conflicting duplicate rather than a replay.
#[test]
fn new_drops_do_not_mutate_an_inflight_gap_batch() {
    let accumulator = DropAccumulator::default();
    accumulator.record(SESSION, EvidenceDropReason::QueueFull);

    // The flush takes the batch; the retry has not put it back yet.
    let inflight = accumulator.take();
    let inflight_key = inflight.batches.keys().next().expect("batch").clone();
    accumulator.record(SESSION, EvidenceDropReason::QueueFull);
    accumulator.restore(inflight);

    let both = accumulator.take();
    assert_eq!(
        both.batches.len(),
        2,
        "the new drop joined the retried batch"
    );
    assert_eq!(
        both.batches[&inflight_key], 1,
        "the retried batch's content changed"
    );
}

/// A batch that recovers reports once. The notice is what tells a live panel its coverage moved,
/// and one loss reported twice would read as two.
#[test]
fn a_recovered_batch_publishes_one_notice() {
    let harness = harness("bridge-gap-one-notice");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::RunStarted {
            session_id: SESSION.to_string(),
            run_id: "not-a-uuid".to_string(),
            trace_id: TRACE.to_string(),
            agent_id: None,
            seat_id: None,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));

    assert!(wait_until(|| *harness
        .diagnostics
        .dropped
        .lock()
        .expect("dropped")
        > 0));
    worker.shutdown();
    assert_eq!(
        *harness.diagnostics.dropped.lock().expect("dropped"),
        1,
        "the recovered batch was reported more than once"
    );
}

/// The 65th session cannot be tracked, and it must not be told it is whole.
///
/// Its loss is real; what the accumulator cannot do is keep its id without growing without bound.
/// So the count goes global, and while it stands no session claims `complete` — the one that lost
/// the evidence is among them and nothing here can say which.
#[test]
fn a_session_past_the_attribution_cap_never_reports_complete() {
    let accumulator = DropAccumulator::default();
    for index in 0..MAX_TRACKED_DROP_SESSIONS {
        accumulator.record(&format!("session-{index}"), EvidenceDropReason::QueueFull);
    }

    // The session past the cap.
    accumulator.record("session-overflow", EvidenceDropReason::QueueFull);
    let snapshot = accumulator.take();
    assert_eq!(snapshot.unattributed, 1);
    assert!(
        !snapshot
            .batches
            .keys()
            .any(|(session, _, _)| session == "session-overflow"),
        "an untrackable session was invented a slot"
    );

    let harness = harness("bridge-attribution-overflow");
    harness.api.report_unattributed_gap(snapshot.unattributed);
    let coverage = harness
        .api
        .subscription_bootstrap(&session())
        .expect("bootstrap")
        .coverage;

    assert_ne!(
        coverage.state(),
        crate::contexts::execution_observability::domain::EvidenceCoverageState::Complete,
        "a session claimed complete while this process had an unattributable loss"
    );
    assert!(coverage
        .reason_codes()
        .iter()
        .any(|code| code.as_str() == "evidence_gap_attribution_overflow"));
}

/// A known session whose reason cannot be held keeps its gap under a reason that says so. Losing
/// the reason is survivable; losing the attribution would let the session read complete.
#[test]
fn a_known_session_over_the_reason_cap_keeps_its_attribution() {
    let accumulator = DropAccumulator::default();
    // Fill this session's reason slots, then force one more distinct reason through.
    for index in 0..MAX_TRACKED_DROP_REASONS {
        accumulator.record(&format!("filler-{index}"), EvidenceDropReason::QueueFull);
    }
    for reason in [
        EvidenceDropReason::QueueFull,
        EvidenceDropReason::WorkerGone,
        EvidenceDropReason::UnmappableSignal,
        EvidenceDropReason::PersistenceFailed,
    ] {
        accumulator.record(SESSION, reason);
    }

    let snapshot = accumulator.take();
    let mine: Vec<_> = snapshot
        .batches
        .keys()
        .filter(|(session, _, _)| session == SESSION)
        .collect();
    assert!(!mine.is_empty(), "a known session lost its attribution");
}

/// Events of one kind. `journal_event_count` counts everything, and a signal the bridge dropped
/// leaves a coverage-gap marker behind — so a test asserting "two events" would pass on one
/// decision plus the marker recording that the other one was lost.
fn event_count_of_kind(harness: &Harness, kind: &str) -> i64 {
    harness
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_events WHERE kind = ?1",
            [kind],
            |row| row.get(0),
        )
        .unwrap_or_default()
}

/// The longest source event id the journal actually holds. `SourceEventId` refuses anything past
/// its bound, and the refusal is silent from the producer's side — the signal simply becomes a
/// coverage gap — so the invariant is worth asserting on stored rows rather than on arithmetic.
fn longest_source_event_id(harness: &Harness) -> i64 {
    harness
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COALESCE(MAX(LENGTH(source_event_id)), 0) FROM execution_evidence_events",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default()
}

fn gap_marker_count(harness: &Harness) -> i64 {
    harness
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_events WHERE kind = 'coverage.gap.recorded'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default()
}

/// A reviewer who accepts and then asks for changes on the same diff made two decisions.
///
/// Keyed on review and witness alone, the second would arrive as a conflicting duplicate and be
/// refused, so the journal would say the review was accepted and stop there.
#[test]
fn a_review_re_decided_on_one_snapshot_records_both_decisions() {
    let harness = harness("bridge-review-changed-mind");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for decision in [
        SessionReviewDecision::Accepted,
        SessionReviewDecision::ChangesRequested,
    ] {
        SessionEvidencePort::try_publish(
            &bridge,
            SessionEvidenceSignal::ReviewDecisionRecorded {
                session_id: SESSION.to_string(),
                review_id: "review-1".to_string(),
                decision,
                witness_fingerprint: "snapshot-a".to_string(),
                occurred_at: "2026-08-22T10:00:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
}

/// Two mutations that produce the same witness are still two mutations.
///
/// The witness carries a clock reading, and a clock has a resolution. Two writes to one file
/// inside one tick produced one witness, and the journal filed the second as a replay of the
/// first — so the file's second change was never recorded. The observer's ordinal is what makes
/// them distinct structurally rather than probabilistically.
#[test]
fn two_mutations_sharing_one_witness_moment_are_two_events() {
    let harness = harness("bridge-file-same-moment");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for observation in 0..2 {
        WorkspaceEvidencePort::try_publish(
            &bridge,
            WorkspaceEvidenceSignal::FileMutationObserved {
                session_id: SESSION.to_string(),
                basename: "main.rs".to_string(),
                path_fingerprint: "0123456789abcdef0123456789abcdef".to_string(),
                change_kind: WorkspaceFileChangeKind::Modified,
                // The same moment for both, which is what a coarse clock produces.
                witness_fingerprint: format!("modified:2026-08-22T10:00:00Z:{observation}"),
                observed_directly: true,
                occurred_at: "2026-08-22T10:00:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| event_count_of_kind(
        &harness,
        "file.mutation.observed"
    ) >= 2));
    worker.shutdown();
    assert_eq!(
        event_count_of_kind(&harness, "file.mutation.observed"),
        2,
        "a write was filed as a replay of the write before it"
    );
}

/// The same observation, delivered twice, is one event.
#[test]
fn a_repeated_file_mutation_observation_records_once() {
    let harness = harness("bridge-file-replay");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for _ in 0..3 {
        WorkspaceEvidencePort::try_publish(
            &bridge,
            WorkspaceEvidenceSignal::FileMutationObserved {
                session_id: SESSION.to_string(),
                basename: "main.rs".to_string(),
                path_fingerprint: "0123456789abcdef0123456789abcdef".to_string(),
                change_kind: WorkspaceFileChangeKind::Modified,
                witness_fingerprint: "modified:2026-08-22T10:00:00Z:7".to_string(),
                observed_directly: true,
                occurred_at: "2026-08-22T10:00:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| event_count_of_kind(
        &harness,
        "file.mutation.observed"
    ) >= 1));
    worker.shutdown();
    assert_eq!(event_count_of_kind(&harness, "file.mutation.observed"), 1);
}

/// A decision's identity must not depend on how long a fingerprint another context chose is.
///
/// The snapshot fingerprint is a full SHA-256 hex — sixty-four characters — and the review id is a
/// UUID. Pasted together with the longest decision token, that is 135 characters against the
/// journal's 128-character bound, so `SourceEventId` refused it and the signal was dropped as
/// unmappable. An acceptance squeaked under at 126 and a request for changes did not: the console
/// recorded reviewers approving work and never recorded them rejecting it.
#[test]
fn a_review_decision_identity_fits_a_real_snapshot_fingerprint() {
    let harness = harness("bridge-review-long-fingerprint");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for decision in [
        SessionReviewDecision::Accepted,
        SessionReviewDecision::ChangesRequested,
    ] {
        SessionEvidencePort::try_publish(
            &bridge,
            SessionEvidenceSignal::ReviewDecisionRecorded {
                session_id: SESSION.to_string(),
                review_id: "0192f0c4-8f3a-7c21-9f4e-6b2d1a5c8e70".to_string(),
                decision,
                // A real `fingerprint_snapshot` result: SHA-256, hex, sixty-four characters.
                witness_fingerprint:
                    "d7603be9087de5633bb80712e3c136bf7b67bd611d1c0e637aa0344e29e60bcb".to_string(),
                occurred_at: match decision {
                    SessionReviewDecision::Accepted => "2026-08-22T10:00:00Z".to_string(),
                    SessionReviewDecision::ChangesRequested => "2026-08-22T10:05:00Z".to_string(),
                },
            },
        );
    }

    assert!(wait_until(|| event_count_of_kind(
        &harness,
        "review.decision.recorded"
    ) >= 2));
    worker.shutdown();
    assert_eq!(
        event_count_of_kind(&harness, "review.decision.recorded"),
        2,
        "a decision was dropped because its identity did not fit"
    );
    assert_eq!(
        gap_marker_count(&harness),
        0,
        "a decision became a coverage gap"
    );
    // The bound the journal enforces, asserted against what was actually written rather than an
    // arithmetic the next person would have to redo by hand.
    assert!(
        longest_source_event_id(&harness) <= 128,
        "a source event id reached the journal's identifier bound"
    );
}

/// A reviewer who changes their mind back has made a third decision.
///
/// Keyed on review, witness, and decision value, the third arrives with the same id as the first.
/// The content fingerprint includes the occurrence time, so the journal records a conflict and
/// keeps the acceptance from ten minutes earlier: it would report the review as accepted once and
/// changed once, in that order, when the reviewer actually accepted, retracted, and accepted again.
#[test]
fn a_review_decided_back_and_forth_records_every_decision() {
    let harness = harness("bridge-review-oscillates");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for (decision, moment) in [
        (SessionReviewDecision::Accepted, "2026-08-22T10:00:00Z"),
        (
            SessionReviewDecision::ChangesRequested,
            "2026-08-22T10:05:00Z",
        ),
        (SessionReviewDecision::Accepted, "2026-08-22T10:10:00Z"),
    ] {
        SessionEvidencePort::try_publish(
            &bridge,
            SessionEvidenceSignal::ReviewDecisionRecorded {
                session_id: SESSION.to_string(),
                review_id: "review-1".to_string(),
                decision,
                witness_fingerprint: "snapshot-a".to_string(),
                occurred_at: moment.to_string(),
            },
        );
    }

    assert!(wait_until(|| event_count_of_kind(
        &harness,
        "review.decision.recorded"
    ) >= 3));
    worker.shutdown();
    assert_eq!(
        event_count_of_kind(&harness, "review.decision.recorded"),
        3,
        "a decision the reviewer made was refused as a duplicate of an earlier one"
    );
}

/// Re-asserting the same decision about the same snapshot is a replay, and converges.
#[test]
fn a_repeated_identical_review_decision_records_once() {
    let harness = harness("bridge-review-same-decision");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for _ in 0..3 {
        SessionEvidencePort::try_publish(
            &bridge,
            SessionEvidenceSignal::ReviewDecisionRecorded {
                session_id: SESSION.to_string(),
                review_id: "review-1".to_string(),
                decision: SessionReviewDecision::Accepted,
                witness_fingerprint: "snapshot-a".to_string(),
                occurred_at: "2026-08-22T10:00:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 1));
    worker.shutdown();
    assert_eq!(journal_event_count(&harness), 1);
}

/// Two sessions changing the same relative path are two observations.
///
/// The journal keys on `(source_context, source_event_id)` with no session of its own, so a path
/// digest that ignored the session would file the second session's edit as a replay of the first
/// and drop it — the second session would then report having changed nothing.
#[test]
fn two_sessions_changing_one_relative_path_record_separately() {
    let harness = harness("bridge-file-two-sessions");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for session in [SESSION, "session-2"] {
        WorkspaceEvidencePort::try_publish(
            &bridge,
            WorkspaceEvidenceSignal::FileMutationObserved {
                session_id: session.to_string(),
                basename: "main.rs".to_string(),
                path_fingerprint: "0123456789abcdef0123456789abcdef".to_string(),
                change_kind: WorkspaceFileChangeKind::Modified,
                witness_fingerprint: "modified:2026-08-22T10:00:00Z".to_string(),
                observed_directly: true,
                occurred_at: "2026-08-22T10:00:00Z".to_string(),
            },
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
}

/// Two writes to one file at two moments are two observations.
#[test]
fn two_writes_to_one_file_record_separately() {
    let harness = harness("bridge-file-two-writes");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    for moment in ["2026-08-22T10:00:00Z", "2026-08-22T10:05:00Z"] {
        WorkspaceEvidencePort::try_publish(
            &bridge,
            WorkspaceEvidenceSignal::FileMutationObserved {
                session_id: SESSION.to_string(),
                basename: "main.rs".to_string(),
                path_fingerprint: "0123456789abcdef0123456789abcdef".to_string(),
                change_kind: WorkspaceFileChangeKind::Modified,
                witness_fingerprint: format!("modified:{moment}"),
                observed_directly: true,
                occurred_at: moment.to_string(),
            },
        );
    }

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
}

/// Two runtimes each open their first batch, and the two are different events.
///
/// The generation alone is a process counter, so both runs call their first batch generation one.
/// Without a namespace the two source ids are byte-identical, and the journal — whose fingerprint
/// includes the occurrence time — files the second as a conflict and keeps the first.
#[test]
fn two_runtime_instances_can_each_persist_generation_one() {
    let first = DropAccumulator::new(BridgeInstanceId::new());
    let second = DropAccumulator::new(BridgeInstanceId::new());

    first.record(SESSION, EvidenceDropReason::QueueFull);
    second.record(SESSION, EvidenceDropReason::QueueFull);

    let first_key = first.take().batches.keys().next().expect("first").clone();
    let second_key = second.take().batches.keys().next().expect("second").clone();

    assert_eq!(first_key.2.generation, 1);
    assert_eq!(second_key.2.generation, 1);
    assert_ne!(
        first_key.2.bridge_instance_id, second_key.2.bridge_instance_id,
        "two runtimes shared one namespace"
    );
    assert_ne!(
        first_key.2.as_source_fragment(),
        second_key.2.as_source_fragment(),
        "two runtimes produced one source event id"
    );
}

/// The namespace and the generation both survive a retry, so the retry is a replay.
///
/// Re-minting either one would turn one loss into as many markers as attempts, and a reader
/// counting gaps would multiply them by however many times the write was ambiguous.
#[test]
fn a_retry_reuses_the_same_runtime_namespace_and_generation() {
    let accumulator = DropAccumulator::new(BridgeInstanceId::new());
    accumulator.record(SESSION, EvidenceDropReason::QueueFull);

    let attempt = accumulator.take();
    let identity = attempt.batches.keys().next().expect("batch").2.clone();
    accumulator.restore(attempt);
    let retry = accumulator.take();
    let retried = retry.batches.keys().next().expect("batch").2.clone();

    assert_eq!(retried, identity);
    assert_eq!(retried.as_source_fragment(), identity.as_source_fragment());
}

/// A namespace is never reused, so no batch a restarted runtime opens can collide with one the
/// journal already holds from the run before it.
#[test]
fn a_restart_never_conflicts_with_a_previous_gap_batch() {
    let before = DropAccumulator::new(BridgeInstanceId::new());
    for _ in 0..3 {
        before.record(SESSION, EvidenceDropReason::QueueFull);
        let _ = before.take();
    }
    let after = DropAccumulator::new(BridgeInstanceId::new());

    let mut seen = std::collections::BTreeSet::new();
    for accumulator in [&before, &after] {
        for _ in 0..3 {
            accumulator.record(SESSION, EvidenceDropReason::QueueFull);
            for key in accumulator.take().batches.keys() {
                assert!(
                    seen.insert(format!(
                        "coverage-gap:{}:{}:{}",
                        SESSION,
                        EvidenceDropReason::QueueFull.as_str(),
                        key.2.as_source_fragment()
                    )),
                    "a restarted runtime reused a source event id"
                );
            }
        }
    }
    assert_eq!(seen.len(), 6);
}

/// End to end across a bridge restart: two gaps of the same size, on one journal, are two markers.
///
/// The journal outlives the process, which is what makes this the case that matters — the count is
/// equal, the session is equal, the reason is equal, and only the runtime namespace differs.
#[test]
fn two_equal_batches_across_restarts_are_distinct_events() {
    let harness = harness("bridge-gap-across-restarts");

    for _ in 0..2 {
        let (bridge, worker) = start_evidence_bridge(harness.api.clone());
        // One unmappable run is one drop, so each runtime reports a gap of exactly one.
        AgentEvidencePort::try_publish(
            &bridge,
            AgentEvidenceSignal::RunStarted {
                session_id: SESSION.to_string(),
                run_id: "not-a-uuid".to_string(),
                trace_id: TRACE.to_string(),
                agent_id: None,
                seat_id: None,
                occurred_at: "2026-08-22T10:00:00Z".to_string(),
            },
        );
        // The shutdown flush is what discharges the batch, so the marker lands before the next
        // runtime starts — exactly the ordering a restart produces.
        worker.shutdown();
    }

    assert_eq!(
        gap_marker_count(&harness),
        2,
        "a session that lost evidence in two runs reported losing it once"
    );
}

/// The longest identifier the journal accepts, which is exactly what a provider is free to send.
fn maximal_provider_id(marker: &str) -> String {
    let mut id = marker.to_string();
    while id.chars().count() < 128 {
        id.push('a');
    }
    id
}

/// Every source event id the journal holds, so a test can assert identity rather than counts.
fn source_event_ids(harness: &Harness) -> Vec<String> {
    let connection = harness.database.connection().expect("connection");
    let mut statement = connection
        .prepare("SELECT source_event_id FROM execution_evidence_events ORDER BY sequence")
        .expect("prepare");
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("rows");
    ids
}

/// A tool call id at the journal's own limit still produces an event.
///
/// `tool-started:` plus a 128-character call id plus an attempt is 143 characters against a
/// 128-character bound, so the id was refused, the signal was counted as unmappable, and the
/// console showed a coverage gap where a tool call should have been. Provider call ids are the
/// producer's, not ours: the bound has to hold for every legal one.
#[test]
fn a_maximal_tool_call_id_still_records_both_phases() {
    let harness = harness("bridge-maximal-tool-id");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());
    let call_id = maximal_provider_id("call-");

    AgentEvidencePort::try_publish(&bridge, tool_started(&call_id, Some(2)));
    AgentEvidencePort::try_publish(
        &bridge,
        tool_finished(
            &call_id,
            Some(2),
            AgentEvidenceObservation::Direct,
            AgentRunEvidenceOutcome::Succeeded,
        ),
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
    assert_eq!(gap_marker_count(&harness), 0, "a tool call became a gap");
    let ids = source_event_ids(&harness);
    assert_eq!(ids.len(), 2);
    // Folded, not truncated: a truncated id is a shorter id two events can share.
    for id in &ids {
        assert!(id.chars().count() <= 128, "{id}");
        assert!(id.contains(":v1:"), "{id}");
    }
    assert_ne!(ids[0], ids[1], "two phases of one call shared one identity");
}

/// A folded id is a pure function of its parts, so a retry converges and an attempt does not.
#[test]
fn a_folded_identity_is_stable_per_attempt_and_distinct_across_attempts() {
    let harness = harness("bridge-folded-attempts");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());
    let call_id = maximal_provider_id("call-");

    // The same attempt twice is one observation redelivered.
    AgentEvidencePort::try_publish(&bridge, tool_started(&call_id, Some(1)));
    AgentEvidencePort::try_publish(&bridge, tool_started(&call_id, Some(1)));
    // A second attempt is a second execution.
    AgentEvidencePort::try_publish(&bridge, tool_started(&call_id, Some(2)));

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
    let ids = source_event_ids(&harness);
    assert_eq!(ids.len(), 2, "a redelivered attempt became a second event");
    assert_ne!(ids[0], ids[1], "two attempts shared one identity");
}

/// Two different part lists cannot fold into one digest.
///
/// Without a length prefix, `["ab", "c"]` and `["a", "bc"]` concatenate identically, and two calls
/// whose ids differ only in where the boundary falls would collide — silently, as a replay.
#[test]
fn folded_identities_separate_parts_that_concatenate_the_same_way() {
    let harness = harness("bridge-folded-boundaries");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());
    let stem = maximal_provider_id("call-");

    // Same total characters across the id and the attempt, different boundary.
    AgentEvidencePort::try_publish(&bridge, tool_started(&format!("{stem}1"), Some(23)));
    AgentEvidencePort::try_publish(&bridge, tool_started(&format!("{stem}12"), Some(3)));

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
    let ids = source_event_ids(&harness);
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

/// An id that fits keeps the exact shape already stored.
///
/// Changing it would make every retry of an event recorded before the fold look like a new event,
/// and the journal would then hold the same observation twice under two identities.
#[test]
fn an_identity_that_fits_keeps_its_readable_form() {
    let harness = harness("bridge-readable-identity");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(&bridge, run_started(&run_id(1)));
    AgentEvidencePort::try_publish(&bridge, tool_started("call-1", Some(1)));

    assert!(wait_until(|| journal_event_count(&harness) >= 2));
    worker.shutdown();
    let ids = source_event_ids(&harness);
    assert!(
        ids.contains(&format!("run-started:{}", run_id(1))),
        "{ids:?}"
    );
    assert!(
        ids.contains(&"tool-started:call-1:1".to_string()),
        "{ids:?}"
    );
}

/// Every producer's longest legal identifier reaches the journal.
#[test]
fn every_producer_bounds_its_longest_identifier() {
    let harness = harness("bridge-longest-per-producer");
    let (bridge, worker) = start_evidence_bridge(harness.api.clone());

    AgentEvidencePort::try_publish(
        &bridge,
        AgentEvidenceSignal::DelegationStarted {
            session_id: SESSION.to_string(),
            run_id: run_id(1),
            trace_id: TRACE.to_string(),
            span_id: None,
            parent_agent_id: Some("agent-1".to_string()),
            seat_id: None,
            delegation_id: maximal_provider_id("delegation-"),
            call_id: "call-1".to_string(),
            attempt: Some(7),
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::UsageObserved {
            session_id: SESSION.to_string(),
            invocation_id: maximal_provider_id("invocation-"),
            run_id: Some(run_id(1)),
            quality: SessionUsageEvidenceQuality::Reported,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    WorkspaceEvidencePort::try_publish(
        &bridge,
        WorkspaceEvidenceSignal::ShellOpened {
            session_id: SESSION.to_string(),
            shell_id: maximal_provider_id("shell-"),
            seat_id: None,
            runtime: WorkspaceShellRuntimeKind::Local,
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    OperationsEvidencePort::try_publish(
        &bridge,
        OperationsEvidenceSignal::OperationFailed {
            session_id: SESSION.to_string(),
            operation_id: maximal_provider_id("operation-"),
            run_id: None,
            reason_code: "operation_failed".to_string(),
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );
    SessionEvidencePort::try_publish(
        &bridge,
        SessionEvidenceSignal::VerificationCompleted {
            session_id: SESSION.to_string(),
            run_id: Some(run_id(1)),
            verification_run_id: maximal_provider_id("verification-"),
            name: "cargo test".to_string(),
            outcome: SessionVerificationOutcome::Passed,
            passed_count: Some(1),
            failed_count: Some(0),
            occurred_at: "2026-08-22T10:00:00Z".to_string(),
        },
    );

    assert!(wait_until(|| journal_event_count(&harness) >= 5));
    worker.shutdown();
    // Not one of them became a coverage gap, and not one of them reached the bound.
    assert_eq!(gap_marker_count(&harness), 0);
    assert_eq!(source_event_ids(&harness).len(), 5);
    assert!(longest_source_event_id(&harness) <= 128);
}

/// What the Files panel asks before it offers a link.
///
/// Driven through the same bridge that records a mutation, so the digest under test is the one the
/// producer actually wrote. A test that computed its own would prove the two agree only as long as
/// nobody changed either.
#[cfg(test)]
mod file_evidence_links {
    use super::*;
    use crate::contexts::execution_observability::api::evidence::{
        EvidenceFileMutationId, EvidenceSessionId, FileEvidenceLinkPort, FileEvidenceLinkQuery,
    };
    use crate::contexts::execution_observability::infrastructure::SqliteEvidenceRepository;

    fn links(
        subject: &Harness,
        fingerprint: &str,
    ) -> crate::contexts::execution_observability::api::evidence::FileEvidenceLinks {
        let repository = SqliteEvidenceRepository::new(subject.database.clone());
        repository
            .file_evidence_links(&FileEvidenceLinkQuery {
                session_id: EvidenceSessionId::parse(SESSION.to_string()).expect("session"),
                file_mutation_id: EvidenceFileMutationId::parse(fingerprint.to_string())
                    .expect("mutation"),
            })
            .expect("links")
    }

    #[test]
    fn a_file_nobody_touched_has_nothing_to_link_to() {
        let subject = harness("file-links-empty");

        // The common answer, and the one that decides whether an action is offered at all. Most
        // files in a workspace were never touched by an agent, and a link that led to an empty list
        // would be worse than no link.
        let answer = links(&subject, "0123456789abcdef0123456789abcdef");

        assert_eq!(answer.observations, 0);
        assert!(answer.run_ids.is_empty());
        assert!(!answer.truncated);
    }
}
