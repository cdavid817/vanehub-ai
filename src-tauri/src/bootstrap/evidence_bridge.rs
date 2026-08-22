//! The producer-to-journal bridge.
//!
//! Producers speak their own vocabulary; the journal accepts one shape. Translation happens here,
//! in bootstrap, because that is the only layer permitted to know both — a producer that could map
//! its own events would need the evidence aggregate as a dependency, and every shell operation
//! would then be one refactor away from carrying a payload type it has no business holding.
//!
//! Two rules shape everything below. Producers never block: `try_publish` maps, attempts one
//! non-blocking send, and returns. And producers never fail because of evidence: a full queue, an
//! unavailable recorder, and a rejected append are all invisible to the operation being observed.

use crate::contexts::agent_runtime::api::{
    AgentEvidencePort, AgentEvidenceSignal, AgentRunEvidenceOutcome,
};
use crate::contexts::execution_observability::api::evidence::{
    CommandRuntimeKind, EvidenceCorrelation, EvidenceOutcome, EvidenceSessionId,
    EvidenceSourceContext, ExecutionEvidenceApi, ExecutionFidelity, ExecutionStatus,
    RecordEvidenceInput, RedactionReceipt, SafeEvidencePayload, SafeReasonCode, SourceEventId,
    UsageQuality,
};
use crate::contexts::operations::api::{OperationsEvidencePort, OperationsEvidenceSignal};
use crate::contexts::sessions::api::{
    SessionEvidencePort, SessionEvidenceSignal, SessionUsageEvidenceQuality,
};
use crate::contexts::workspaces::api::{
    WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceShellRuntimeKind,
};
use std::collections::BTreeMap;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How many observations may wait for the journal.
///
/// Small on purpose. The queue exists to decouple a producer from one SQLite write, not to buffer
/// a backlog: a deep queue would hide a stalled writer for minutes and then lose everything at
/// shutdown, whereas a shallow one reports the loss immediately through the drop accumulator, and
/// the coverage a reader sees says so.
pub(crate) const EVIDENCE_QUEUE_CAPACITY: usize = 256;

/// How long shutdown waits for the worker to drain.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

/// Sessions whose observations the queue refused, and how many.
///
/// Counted rather than queued: a drop that had to be queued to be reported would be dropped by the
/// same full queue that caused it. The worker flushes these into the journal once it has room.
#[derive(Default)]
struct DropAccumulator {
    by_session: Mutex<BTreeMap<String, u32>>,
}

impl DropAccumulator {
    fn record(&self, session_id: &str) {
        let mut counts = match self.by_session.lock() {
            Ok(counts) => counts,
            Err(poisoned) => poisoned.into_inner(),
        };
        *counts.entry(session_id.to_string()).or_insert(0) += 1;
    }

    fn take(&self) -> BTreeMap<String, u32> {
        let mut counts = match self.by_session.lock() {
            Ok(counts) => counts,
            Err(poisoned) => poisoned.into_inner(),
        };
        std::mem::take(&mut counts)
    }
}

/// The sender half, implementing every producer's port.
///
/// One type implements all four because translation is a bootstrap concern and splitting it into
/// four adapters would put four copies of the same "never fail, never block" contract in four
/// places, where they could drift.
#[derive(Clone)]
pub(crate) struct EvidenceBridge {
    sender: SyncSender<RecordEvidenceInput>,
    drops: Arc<DropAccumulator>,
}

impl EvidenceBridge {
    /// Maps first, sends second.
    ///
    /// Mapping before the queue is what keeps the queue safe: only an already-validated
    /// `RecordEvidenceInput` is ever in flight, so a raw producer object never sits in memory
    /// waiting to be redacted by someone else later.
    fn offer(&self, session_id: &str, input: Option<RecordEvidenceInput>) {
        let Some(input) = input else {
            return;
        };
        match self.sender.try_send(input) {
            Ok(()) => {}
            // Both failures are silent to the producer by design. `Full` means the journal is
            // behind; `Disconnected` means the worker is gone. Neither says anything about whether
            // the observed work succeeded, so neither may change its result.
            Err(TrySendError::Full(_)) => self.drops.record(session_id),
            Err(TrySendError::Disconnected(_)) => self.drops.record(session_id),
        }
    }
}

fn correlation(
    session_id: &str,
    run_id: Option<&str>,
    trace_id: Option<&str>,
) -> Option<EvidenceCorrelation> {
    let session = EvidenceSessionId::parse(session_id).ok()?;
    let mut correlation = EvidenceCorrelation::for_session(session);
    if let (Some(run_id), Some(trace_id)) = (run_id, trace_id) {
        correlation.run_id = crate::contexts::execution_observability::api::ExecutionRunId::parse(
            run_id.to_string(),
        )
        .ok();
        correlation.trace_id =
            crate::contexts::execution_observability::api::TraceId::parse(trace_id.to_string())
                .ok();
    }
    Some(correlation)
}

impl AgentEvidencePort for EvidenceBridge {
    fn try_publish(&self, signal: AgentEvidenceSignal) {
        self.offer(agent_session(&signal), map_agent_signal(&signal));
    }
}

/// Returns `None` when a value the journal requires cannot be built from what the producer sent —
/// an unparseable session, a run id that is not a run id. Dropping is the right failure: an event
/// filed against a correlation nobody can join to can be counted but never found, which reads as
/// coverage without being it.
fn map_agent_signal(signal: &AgentEvidenceSignal) -> Option<RecordEvidenceInput> {
    match signal {
        AgentEvidenceSignal::RunStarted {
            session_id,
            run_id,
            trace_id,
            agent_id,
            seat_id,
            occurred_at,
        } => {
            let mut correlation = correlation(session_id, Some(run_id), Some(trace_id))?;
            bind_agent(&mut correlation, agent_id.as_deref(), seat_id.as_deref());
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::AgentRuntime,
                source_event_id: SourceEventId::parse(format!("run-started:{run_id}")).ok()?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: Some(ExecutionStatus::Running),
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::RunStarted {
                    trigger: SafeReasonCode::parse("agent_generation").ok()?,
                },
                redaction: RedactionReceipt::none(),
            })
        }
        AgentEvidenceSignal::RunFinished {
            session_id,
            run_id,
            trace_id,
            agent_id,
            seat_id,
            occurred_at,
            outcome,
            duration_ms,
        } => {
            let mut correlation = correlation(session_id, Some(run_id), Some(trace_id))?;
            bind_agent(&mut correlation, agent_id.as_deref(), seat_id.as_deref());
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::AgentRuntime,
                source_event_id: SourceEventId::parse(format!("run-finished:{run_id}")).ok()?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: Some(match outcome {
                    AgentRunEvidenceOutcome::Succeeded => ExecutionStatus::Succeeded,
                    AgentRunEvidenceOutcome::Failed => ExecutionStatus::Failed,
                    AgentRunEvidenceOutcome::Cancelled => ExecutionStatus::Cancelled,
                }),
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::RunCompleted {
                    outcome: match outcome {
                        AgentRunEvidenceOutcome::Succeeded => EvidenceOutcome::Succeeded,
                        AgentRunEvidenceOutcome::Failed => EvidenceOutcome::Failed,
                        AgentRunEvidenceOutcome::Cancelled => EvidenceOutcome::Cancelled,
                    },
                    duration_ms: *duration_ms,
                },
                redaction: RedactionReceipt::none(),
            })
        }
    }
}

fn bind_agent(
    correlation: &mut EvidenceCorrelation,
    agent_id: Option<&str>,
    seat_id: Option<&str>,
) {
    correlation.agent_id = agent_id.and_then(|value| {
        crate::contexts::execution_observability::api::evidence::EvidenceAgentId::parse(
            value.to_string(),
        )
        .ok()
    });
    correlation.seat_id = seat_id.and_then(|value| {
        crate::contexts::execution_observability::api::evidence::EvidenceSeatId::parse(
            value.to_string(),
        )
        .ok()
    });
}

fn agent_session(signal: &AgentEvidenceSignal) -> &str {
    match signal {
        AgentEvidenceSignal::RunStarted { session_id, .. }
        | AgentEvidenceSignal::RunFinished { session_id, .. } => session_id,
    }
}

impl WorkspaceEvidencePort for EvidenceBridge {
    fn try_publish(&self, signal: WorkspaceEvidenceSignal) {
        let WorkspaceEvidenceSignal::ShellOpened {
            session_id,
            shell_id,
            seat_id,
            runtime,
            occurred_at,
        } = &signal;
        let input = (|| {
            let mut correlation = correlation(session_id, None, None)?;
            bind_agent(&mut correlation, None, seat_id.as_deref());
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Workspaces,
                source_event_id: SourceEventId::parse(format!("shell-opened:{shell_id}")).ok()?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: Some(ExecutionStatus::Running),
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::ShellOpened {
                    runtime_kind: match runtime {
                        WorkspaceShellRuntimeKind::Local => CommandRuntimeKind::LocalShell,
                        WorkspaceShellRuntimeKind::Remote => CommandRuntimeKind::RemoteShell,
                    },
                },
                redaction: RedactionReceipt::none(),
            })
        })();
        self.offer(session_id, input);
    }
}

impl OperationsEvidencePort for EvidenceBridge {
    fn try_publish(&self, signal: OperationsEvidenceSignal) {
        let OperationsEvidenceSignal::OperationFailed {
            session_id,
            operation_id,
            run_id,
            reason_code,
            occurred_at,
        } = &signal;
        let input = (|| {
            let mut correlation = correlation(session_id, None, None)?;
            correlation.operation_id =
                crate::contexts::execution_observability::api::evidence::EvidenceOperationId::parse(
                    operation_id.clone(),
                )
                .ok();
            correlation.run_id = run_id.as_ref().and_then(|value| {
                crate::contexts::execution_observability::api::ExecutionRunId::parse(value.clone())
                    .ok()
            });
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Operations,
                source_event_id: SourceEventId::parse(format!("operation-failed:{operation_id}"))
                    .ok()?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: Some(ExecutionStatus::Failed),
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::OperationFailed {
                    // A code, never the error text. The log store already holds the message and
                    // already redacts it; a second copy here would be a second thing to get right.
                    reason: SafeReasonCode::parse(sanitize_reason(reason_code)).ok()?,
                },
                redaction: RedactionReceipt::none(),
            })
        })();
        self.offer(session_id, input);
    }
}

/// Producer reason codes are already codes, but they are not this context's codes. Anything that
/// does not fit the journal's shape collapses to a generic one rather than being reshaped into
/// something that looks specific but is not.
fn sanitize_reason(reason_code: &str) -> String {
    let normalized: String = reason_code
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if normalized.is_empty() || normalized.len() > 64 {
        return "operation_failed".to_string();
    }
    normalized
}

impl SessionEvidencePort for EvidenceBridge {
    fn try_publish(&self, signal: SessionEvidenceSignal) {
        let SessionEvidenceSignal::UsageObserved {
            session_id,
            invocation_id,
            run_id,
            quality,
            occurred_at,
        } = &signal;
        let input = (|| {
            let mut correlation = correlation(session_id, None, None)?;
            correlation.run_id = run_id.as_ref().and_then(|value| {
                crate::contexts::execution_observability::api::ExecutionRunId::parse(value.clone())
                    .ok()
            });
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Sessions,
                source_event_id: SourceEventId::parse(format!("usage-observed:{invocation_id}"))
                    .ok()?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: None,
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::UsageObserved {
                    // A count and a classification. The token dimensions stay in sessions;
                    // duplicating them would create a second total that can disagree with the
                    // first, with nothing to say which is right. The invocation this points at is
                    // the source event id above, which is also what makes a retry idempotent.
                    response_count: 1,
                    quality: match quality {
                        SessionUsageEvidenceQuality::Reported => UsageQuality::Reported,
                        SessionUsageEvidenceQuality::ReportedDerived => {
                            UsageQuality::ReportedDerived
                        }
                        SessionUsageEvidenceQuality::Estimated => UsageQuality::Estimated,
                    },
                },
                redaction: RedactionReceipt::none(),
            })
        })();
        self.offer(session_id, input);
    }
}

/// The receiving half. Owns the thread and the only handle that calls the recorder.
pub(crate) struct EvidenceBridgeWorker {
    handle: Option<JoinHandle<()>>,
}

/// Managed state so the exit handler can reach the worker it did not create.
///
/// `Mutex<Option<_>>` because shutdown consumes the worker and `RunEvent::Exit` can fire more than
/// once; the second take finds `None` and returns.
pub(crate) struct EvidenceBridgeShutdown {
    worker: Mutex<Option<EvidenceBridgeWorker>>,
}

impl EvidenceBridgeShutdown {
    pub(crate) fn new(worker: EvidenceBridgeWorker) -> Self {
        Self {
            worker: Mutex::new(Some(worker)),
        }
    }

    pub(crate) fn shutdown(&self) {
        let taken = match self.worker.lock() {
            Ok(mut worker) => worker.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some(worker) = taken {
            worker.shutdown();
        }
    }
}

impl EvidenceBridgeWorker {
    /// Waits a bounded time for the queue to drain, then gives up.
    ///
    /// Unbounded would let a stuck SQLite write hold the process open at exit. Evidence describes
    /// work that has already happened; losing the tail of it is strictly better than refusing to
    /// close.
    pub(crate) fn shutdown(mut self) {
        let Some(handle) = self.handle.take() else {
            return;
        };
        let deadline = Instant::now() + SHUTDOWN_GRACE;
        while !handle.is_finished() {
            if Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = handle.join();
    }
}

/// Starts the bridge: a bounded channel, a worker thread, and the sender that implements every
/// producer port.
///
/// The worker runs on its own thread rather than the async runtime because every call it makes is
/// a blocking SQLite write; parking a runtime worker on one would stall unrelated tasks.
pub(crate) fn start_evidence_bridge(
    evidence: ExecutionEvidenceApi,
) -> (EvidenceBridge, EvidenceBridgeWorker) {
    let (sender, receiver) = sync_channel(EVIDENCE_QUEUE_CAPACITY);
    let drops = Arc::new(DropAccumulator::default());
    let bridge = EvidenceBridge {
        sender,
        drops: drops.clone(),
    };
    let handle = std::thread::Builder::new()
        .name("evidence-bridge".to_string())
        .spawn(move || run_worker(evidence, receiver, drops))
        .ok();
    (bridge, EvidenceBridgeWorker { handle })
}

/// One failed record must not end the worker.
///
/// A store that rejects one event usually accepts the next — a conflicting source id, a value the
/// domain refuses — and a worker that exited on the first would turn one bad observation into the
/// permanent loss of every later one.
fn run_worker(
    evidence: ExecutionEvidenceApi,
    receiver: Receiver<RecordEvidenceInput>,
    drops: Arc<DropAccumulator>,
) {
    while let Ok(input) = receiver.recv() {
        let _ = evidence.record(input);
        flush_drops(&evidence, &drops);
    }
    // The senders are gone; report whatever the queue refused on the way down.
    flush_drops(&evidence, &drops);
}

/// Reports refused observations through the recorder, never through the queue.
///
/// Re-queueing the report would put it behind the same full queue that produced it, so a burst
/// would silence exactly the signal that says a burst happened.
fn flush_drops(evidence: &ExecutionEvidenceApi, drops: &DropAccumulator) {
    for (session_id, dropped) in drops.take() {
        let Ok(session) = EvidenceSessionId::parse(session_id) else {
            continue;
        };
        evidence.record_dropped_events(&session, dropped);
    }
}
