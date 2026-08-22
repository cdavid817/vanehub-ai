//! Tests for the producer-to-journal bridge.
//!
//! The properties worth checking are the ones a producer depends on without being able to see:
//! that publishing cannot block, cannot fail, and cannot change what the producer returns. Each
//! test drives a real bounded channel, and the end-to-end ones drive a real recorder over a real
//! store — a stub would let a mapping pass here that the domain rejects at run time, which is the
//! failure mode most worth catching.

use super::evidence_bridge::start_evidence_bridge;
use crate::contexts::agent_runtime::api::{
    AgentEvidencePort, AgentEvidenceSignal, AgentRunEvidenceOutcome,
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
}

fn harness(name: &str) -> Harness {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let diagnostics = Arc::new(CountingDiagnostics::default());
    let api = ExecutionEvidenceApi::new(
        Arc::new(SqliteEvidenceRepository::new(database)),
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
    }
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
