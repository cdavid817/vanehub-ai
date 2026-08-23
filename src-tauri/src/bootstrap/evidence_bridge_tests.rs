//! Tests for the producer-to-journal bridge.
//!
//! The properties worth checking are the ones a producer depends on without being able to see:
//! that publishing cannot block, cannot fail, and cannot change what the producer returns. Each
//! test drives a real bounded channel, and the end-to-end ones drive a real recorder over a real
//! store — a stub would let a mapping pass here that the domain rejects at run time, which is the
//! failure mode most worth catching.

use super::evidence_bridge::{
    start_evidence_bridge, DropAccumulator, EvidenceDropReason, MAX_TRACKED_DROP_REASONS,
    MAX_TRACKED_DROP_SESSIONS,
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
    SessionEvidencePort, SessionEvidenceSignal, SessionUsageEvidenceQuality,
};
use crate::contexts::workspaces::api::{
    WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceShellRuntimeKind,
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
        .counts
        .keys()
        .map(|(session, _)| session.as_str())
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
    assert!(snapshot.counts.len() <= MAX_TRACKED_DROP_REASONS);
    assert_eq!(snapshot.counts.len(), 4, "each reason keys separately");
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

    assert_eq!(first.counts.values().sum::<u32>(), 1);
    assert_eq!(second.counts.values().sum::<u32>(), 1);
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
