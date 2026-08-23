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
    AgentEvidenceObservation, AgentEvidencePort, AgentEvidenceSignal, AgentRunEvidenceOutcome,
};
use crate::contexts::execution_observability::api::evidence::{
    BoundedLabel, CommandRuntimeKind, EvidenceCorrelation, EvidenceOutcome, EvidenceSessionId,
    EvidenceSourceContext, EvidenceToolCallId, ExecutionEvidenceApi, ExecutionFidelity,
    ExecutionStatus, RecordEvidenceInput, RedactionReceipt, SafeEvidencePayload, SafeReasonCode,
    SourceEventId, SpanId, UsageQuality,
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

/// Why an observation never reached the journal. A closed set, because it is a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EvidenceDropReason {
    /// The bounded queue was full when the producer published.
    QueueFull,
    /// The worker was gone: the process is shutting down, or the bridge outlived it.
    WorkerGone,
    /// The producer's values could not be mapped into anything the journal accepts.
    UnmappableSignal,
    /// The journal took the call and refused the row.
    PersistenceFailed,
}

/// How many distinct sessions the accumulator tracks at once.
///
/// Bounded because the key comes from a producer: a bug that minted a fresh session id per event
/// would grow this map without limit, and an unbounded structure whose whole job is to report that
/// something overflowed is the wrong shape.
pub(crate) const MAX_TRACKED_DROP_SESSIONS: usize = 64;

/// Reasons per session. Every reason is a variant of a closed enum, so this caps at the enum's
/// size; it is stated anyway, because adding a variant past it would silently discard the new one.
pub(crate) const MAX_TRACKED_DROP_REASONS: usize = 8;

/// What never reached the journal, keyed by session and reason.
///
/// Counted rather than queued: a drop that had to be queued to be reported would be dropped by the
/// same full queue that caused it. The worker flushes these once it has room.
#[derive(Default)]
pub(crate) struct DropAccumulator {
    counts: Mutex<BTreeMap<(String, EvidenceDropReason), u32>>,
    /// Drops that arrived with the session cap already reached and no existing key to attribute
    /// them to. Reported without a session, so the total stays honest when the attribution cannot.
    unattributed: Mutex<u32>,
}

impl DropAccumulator {
    pub(crate) fn record(&self, session_id: &str, reason: EvidenceDropReason) {
        let key = (session_id.to_string(), reason);
        let mut counts = lock(&self.counts);
        if let Some(existing) = counts.get_mut(&key) {
            // Saturating: a count that wrapped would report a smaller gap than occurred, which is
            // worse than reporting a capped one.
            *existing = existing.saturating_add(1);
            return;
        }
        let tracked_sessions = counts
            .keys()
            .map(|(session, _)| session.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let known_session = tracked_sessions.contains(key.0.as_str());
        let reasons_here = counts.keys().filter(|(s, _)| *s == key.0).count();
        if (!known_session && tracked_sessions.len() >= MAX_TRACKED_DROP_SESSIONS)
            || reasons_here >= MAX_TRACKED_DROP_REASONS
        {
            drop(counts);
            let mut unattributed = lock(&self.unattributed);
            *unattributed = unattributed.saturating_add(1);
            return;
        }
        counts.insert(key, 1);
    }

    /// Takes a snapshot and leaves the accumulator empty.
    ///
    /// Taking rather than reading-then-clearing is what makes a concurrent drop safe: anything
    /// recorded between those two steps would be erased by the clear, whereas a take leaves an
    /// empty map and the new count lands in a fresh entry.
    pub(crate) fn take(&self) -> DropSnapshot {
        DropSnapshot {
            counts: std::mem::take(&mut lock(&self.counts)),
            unattributed: std::mem::take(&mut lock(&self.unattributed)),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DropSnapshot {
    pub(crate) counts: BTreeMap<(String, EvidenceDropReason), u32>,
    pub(crate) unattributed: u32,
}

impl DropSnapshot {
    pub(crate) fn is_empty(&self) -> bool {
        self.counts.is_empty() && self.unattributed == 0
    }
}

/// A poisoned lock must not silence a diagnostic: this count is the only signal that evidence went
/// missing, so it is read through the poison rather than raised as a panic.
fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match value.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
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
            // A signal the journal cannot accept is a gap, not a non-event. Reporting it as
            // nothing would leave a hole no coverage state accounts for.
            self.drops
                .record(session_id, EvidenceDropReason::UnmappableSignal);
            return;
        };
        match self.sender.try_send(input) {
            Ok(()) => {}
            // Both failures are silent to the producer by design. `Full` means the journal is
            // behind; `Disconnected` means the worker is gone. Neither says anything about whether
            // the observed work succeeded, so neither may change its result.
            Err(TrySendError::Full(_)) => {
                self.drops.record(session_id, EvidenceDropReason::QueueFull)
            }
            Err(TrySendError::Disconnected(_)) => self
                .drops
                .record(session_id, EvidenceDropReason::WorkerGone),
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
        AgentEvidenceSignal::ToolStarted {
            session_id,
            run_id,
            trace_id,
            span_id,
            agent_id,
            seat_id,
            call_id,
            tool_name,
            observation,
            attempt,
            occurred_at,
        } => {
            let mut correlation = correlation(session_id, Some(run_id), Some(trace_id))?;
            bind_agent(&mut correlation, agent_id.as_deref(), seat_id.as_deref());
            bind_tool(&mut correlation, span_id.as_deref(), call_id);
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::AgentRuntime,
                // The call id, plus the attempt when there is one. A retry of the same call is a
                // second observation of a second execution: sharing one id would make the journal
                // treat the retry as a duplicate of the first and drop it.
                source_event_id: SourceEventId::parse(attempt_scoped(
                    "tool-started",
                    call_id,
                    *attempt,
                ))
                .ok()?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: Some(ExecutionStatus::Running),
                fidelity: observed_fidelity(*observation),
                payload: SafeEvidencePayload::ToolStarted {
                    // The name, never the arguments. What a tool was asked to do is the payload
                    // this journal exists not to hold.
                    tool_name: BoundedLabel::parse("tool name", tool_name.clone()).ok()?,
                },
                redaction: RedactionReceipt::none(),
            })
        }
        AgentEvidenceSignal::ToolFinished {
            session_id,
            run_id,
            trace_id,
            span_id,
            agent_id,
            seat_id,
            call_id,
            tool_name,
            observation,
            attempt,
            outcome,
            occurred_at,
        } => {
            let mut correlation = correlation(session_id, Some(run_id), Some(trace_id))?;
            bind_agent(&mut correlation, agent_id.as_deref(), seat_id.as_deref());
            bind_tool(&mut correlation, span_id.as_deref(), call_id);
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::AgentRuntime,
                source_event_id: SourceEventId::parse(attempt_scoped(
                    "tool-finished",
                    call_id,
                    *attempt,
                ))
                .ok()?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: Some(execution_status(*outcome)),
                fidelity: observed_fidelity(*observation),
                payload: SafeEvidencePayload::ToolCompleted {
                    tool_name: BoundedLabel::parse("tool name", tool_name.clone()).ok()?,
                    outcome: evidence_outcome(*outcome),
                    // The runtime does not measure a tool's wall clock here. Subtracting the two
                    // timestamps would report the gap between two observations as a duration.
                    duration_ms: None,
                },
                redaction: RedactionReceipt::none(),
            })
        }
        AgentEvidenceSignal::DelegationStarted {
            session_id,
            run_id,
            trace_id,
            span_id,
            parent_agent_id,
            seat_id,
            delegation_id,
            call_id,
            attempt,
            occurred_at,
        } => {
            let mut correlation = correlation(session_id, Some(run_id), Some(trace_id))?;
            bind_agent(
                &mut correlation,
                parent_agent_id.as_deref(),
                seat_id.as_deref(),
            );
            bind_tool(&mut correlation, span_id.as_deref(), call_id);
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::AgentRuntime,
                source_event_id: SourceEventId::parse(attempt_scoped(
                    "delegation-started",
                    delegation_id,
                    *attempt,
                ))
                .ok()?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: Some(ExecutionStatus::Running),
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::AgentDelegated {
                    // The delegation id is what a reader follows; the child agent is not known at
                    // hand-off, and naming one here would name an agent that may never start.
                    attempt: *attempt,
                },
                redaction: RedactionReceipt::none(),
            })
        }
        AgentEvidenceSignal::DelegationFinished {
            session_id,
            run_id,
            trace_id,
            span_id,
            parent_agent_id,
            seat_id,
            delegation_id,
            call_id,
            attempt,
            outcome,
            occurred_at,
        } => {
            let mut correlation = correlation(session_id, Some(run_id), Some(trace_id))?;
            bind_agent(
                &mut correlation,
                parent_agent_id.as_deref(),
                seat_id.as_deref(),
            );
            bind_tool(&mut correlation, span_id.as_deref(), call_id);
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::AgentRuntime,
                source_event_id: SourceEventId::parse(attempt_scoped(
                    "delegation-finished",
                    delegation_id,
                    *attempt,
                ))
                .ok()?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: Some(execution_status(*outcome)),
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::AgentCompleted {
                    outcome: evidence_outcome(*outcome),
                    duration_ms: None,
                },
                redaction: RedactionReceipt::none(),
            })
        }
    }
}

/// A source event id that survives a retry without colliding with it.
///
/// Two executions of the same call are two things that happened; giving them one id would make the
/// journal treat the second as a duplicate of the first and keep only the first. An id without the
/// attempt is used when the producer never reported one, which is the same value on every replay of
/// that same observation — that is what makes a duplicate callback idempotent.
fn attempt_scoped(prefix: &str, id: &str, attempt: Option<u32>) -> String {
    match attempt {
        Some(attempt) => format!("{prefix}:{id}:{attempt}"),
        None => format!("{prefix}:{id}"),
    }
}

/// Never upgrades. A reconstruction reported as a direct observation is a claim the runtime cannot
/// back, and the fidelity is the only field a reader has to weigh the record by.
fn observed_fidelity(observation: AgentEvidenceObservation) -> ExecutionFidelity {
    match observation {
        AgentEvidenceObservation::Direct => ExecutionFidelity::Native,
        AgentEvidenceObservation::Reported => ExecutionFidelity::Proxied,
        AgentEvidenceObservation::Reconstructed => ExecutionFidelity::Inferred,
    }
}

fn execution_status(outcome: AgentRunEvidenceOutcome) -> ExecutionStatus {
    match outcome {
        AgentRunEvidenceOutcome::Succeeded => ExecutionStatus::Succeeded,
        AgentRunEvidenceOutcome::Failed => ExecutionStatus::Failed,
        AgentRunEvidenceOutcome::Cancelled => ExecutionStatus::Cancelled,
    }
}

fn evidence_outcome(outcome: AgentRunEvidenceOutcome) -> EvidenceOutcome {
    match outcome {
        AgentRunEvidenceOutcome::Succeeded => EvidenceOutcome::Succeeded,
        AgentRunEvidenceOutcome::Failed => EvidenceOutcome::Failed,
        AgentRunEvidenceOutcome::Cancelled => EvidenceOutcome::Cancelled,
    }
}

fn bind_tool(correlation: &mut EvidenceCorrelation, span_id: Option<&str>, call_id: &str) {
    correlation.span_id = span_id.and_then(|value| SpanId::parse(value.to_string()).ok());
    correlation.tool_call_id = EvidenceToolCallId::parse(call_id.to_string()).ok();
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
        | AgentEvidenceSignal::RunFinished { session_id, .. }
        | AgentEvidenceSignal::ToolStarted { session_id, .. }
        | AgentEvidenceSignal::ToolFinished { session_id, .. }
        | AgentEvidenceSignal::DelegationStarted { session_id, .. }
        | AgentEvidenceSignal::DelegationFinished { session_id, .. } => session_id,
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
        let sanitized = sanitize_reason(reason_code);
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
                    reason: SafeReasonCode::parse(sanitized.code).ok()?,
                },
                redaction: receipt_for(sanitized.rewritten)?,
            })
        })();
        self.offer(session_id, input);
    }
}

/// The rule that fires when a producer's reason code had to be rewritten.
///
/// It travels on the receipt rather than being applied silently. A record whose payload does not
/// match what the producer sent, with a receipt claiming nothing was redacted, would be a value
/// nobody can trace back — and tracing it back is the only way to tell a policy rewrite from a
/// producer bug.
const REASON_NORMALIZED_RULE: &str = "reason_code_normalized";

/// What became of a producer's reason code on the way in.
struct SanitizedReason {
    code: String,
    /// `true` when the stored code is not the one the producer supplied.
    rewritten: bool,
}

/// Producer reason codes are already codes, but they are not this context's codes.
///
/// Anything that does not fit the journal's shape collapses to a generic one rather than being
/// reshaped into something that looks specific but is not — an error message squeezed through the
/// character filter would come out as a long underscore-separated string that reads like a code
/// and groups like nothing.
fn sanitize_reason(reason_code: &str) -> SanitizedReason {
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
    if normalized.is_empty() || normalized.len() > MAX_REASON_CODE_CHARS {
        return SanitizedReason {
            code: "operation_failed".to_string(),
            rewritten: true,
        };
    }
    SanitizedReason {
        rewritten: normalized != reason_code,
        code: normalized,
    }
}

/// Longer than this and the value is prose, not a code.
const MAX_REASON_CODE_CHARS: usize = 64;

/// A receipt that names the rules applied, or an empty one when the value passed through as sent.
fn receipt_for(rewritten: bool) -> Option<RedactionReceipt> {
    if !rewritten {
        return Some(RedactionReceipt::none());
    }
    RedactionReceipt::applied([SafeReasonCode::parse(REASON_NORMALIZED_RULE).ok()?]).ok()
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
        // Captured before the input moves: a failure has to be attributable, and the correlation
        // is the only place the session survives once `record` has taken ownership.
        let session = input
            .correlation
            .session()
            .map(|session| session.as_str().to_string());
        if evidence.record(input).is_err() {
            if let Some(session) = session {
                drops.record(&session, EvidenceDropReason::PersistenceFailed);
            }
        }
        flush_drops(&evidence, &drops);
    }
    // The senders are gone; report whatever never made it, on the way down.
    flush_drops(&evidence, &drops);
}

/// Reports refused observations through the recorder, never through the queue.
///
/// Re-queueing the report would put it behind the same full queue that produced it, so a burst
/// would silence exactly the signal that says a burst happened.
fn flush_drops(evidence: &ExecutionEvidenceApi, drops: &DropAccumulator) {
    let snapshot = drops.take();
    if snapshot.is_empty() {
        return;
    }
    let mut per_session: BTreeMap<String, u32> = BTreeMap::new();
    for ((session_id, _reason), count) in &snapshot.counts {
        let total = per_session.entry(session_id.clone()).or_insert(0);
        *total = total.saturating_add(*count);
    }
    for (session_id, dropped) in per_session {
        let Ok(session) = EvidenceSessionId::parse(session_id) else {
            continue;
        };
        evidence.record_dropped_events(&session, dropped);
    }
}
