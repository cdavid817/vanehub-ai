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
    BoundedLabel, CommandRuntimeKind, EvidenceCorrelation, EvidenceFileMutationId, EvidenceOutcome,
    EvidenceSessionId, EvidenceSourceContext, EvidenceToolCallId, ExecutionEvidenceApi,
    ExecutionFidelity, ExecutionStatus, FileChangeKind, RecordEvidenceInput, RedactionReceipt,
    ReviewDecisionScope, ReviewDecisionValue, SafeBasename, SafeEvidencePayload, SafeFingerprint,
    SafeReasonCode, SourceEventId, SpanId, UsageQuality, VerificationOutcome,
    MAX_IDENTIFIER_LENGTH,
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
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// A drop this session really had, whose specific reason the bounded accumulator had no slot
    /// for. The session is still named, because losing the reason is survivable and losing the
    /// attribution is not.
    AttributionOverflow,
}

impl EvidenceDropReason {
    /// The stable code a gap marker carries. Localized by the frontend, so it is part of the
    /// contract and cannot be reworded where it is emitted.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::QueueFull => "evidence_queue_full",
            Self::WorkerGone => "evidence_worker_unavailable",
            Self::UnmappableSignal => "evidence_signal_unmappable",
            Self::PersistenceFailed => "evidence_persistence_failed",
            Self::AttributionOverflow => "evidence_gap_attribution_overflow",
        }
    }
}

/// How many distinct sessions the accumulator tracks at once.
///
/// Bounded because the key comes from a producer: a bug that minted a fresh session id per event
/// would grow this map without limit, and an unbounded structure whose whole job is to report that
/// something overflowed is the wrong shape.
pub(crate) const MAX_TRACKED_DROP_SESSIONS: usize = 64;

/// Reasons per session. Above this, a session's further reasons fold into `AttributionOverflow`
/// rather than being discarded: the session keeps its gap, and only the "why" is lost.
pub(crate) const MAX_TRACKED_DROP_REASONS: usize = 8;

/// A hard ceiling on entries, independent of the two dimensional caps.
///
/// A batch being retried and a batch opened while that retry was in flight coexist under the same
/// session and reason, so entries can exceed sessions times reasons. This is what actually bounds
/// the memory.
pub(crate) const MAX_TRACKED_GAP_BATCHES: usize = MAX_TRACKED_DROP_SESSIONS * 2;

/// Names one run of the bridge, so a counter that restarts does not reuse an identity.
///
/// The generation below is a process counter: after a restart the first batch is generation one
/// again, and it collides with the first batch of the previous run. That collision is not a
/// harmless replay — the content fingerprint includes the occurrence time, so the journal records
/// a conflict and keeps the older row. A session that lost evidence in two separate runs would
/// report losing it once.
///
/// Sixty-four random bits rendered as fixed-width hex, and nothing else. Not a hostname, not a
/// user, not a path, not a start time: this value is written into a durable journal, and its only
/// job is to differ from every other run's. Not the whole UUID either — the source event id has a
/// 128-character bound that the session, the reason code, and the generation already share.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct BridgeInstanceId(String);

impl BridgeInstanceId {
    pub(crate) fn new() -> Self {
        let (high, _) = uuid::Uuid::new_v4().as_u64_pair();
        Self(format!("{high:016x}"))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Identifies one accumulation of drops, across a retry and across a restart.
///
/// A marker keyed by its count would collide with any later gap of the same size, and the journal
/// would report that as a conflicting duplicate rather than storing a second gap. The generation
/// is what makes a retry idempotent and two same-sized gaps distinct within a run; the instance is
/// what keeps a run's generation one from colliding with the previous run's. It counts rather than
/// reads a clock, because a marker written after a clock adjustment still has to be the same batch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct GapBatchIdentity {
    pub(crate) bridge_instance_id: BridgeInstanceId,
    pub(crate) generation: u64,
}

impl GapBatchIdentity {
    /// The identity as it appears inside a source event id. Deliberately not `Display`: this is a
    /// journal key, and it must change only when the journal contract changes.
    pub(crate) fn as_source_fragment(&self) -> String {
        format!("{}:{}", self.bridge_instance_id.as_str(), self.generation)
    }
}

/// The full identity of one batch: whose gap, why, and which accumulation.
pub(crate) type GapBatchKey = (String, EvidenceDropReason, GapBatchIdentity);

/// What never reached the journal, keyed by session, reason, and batch.
///
/// Counted rather than queued: a drop that had to be queued to be reported would be dropped by the
/// same full queue that caused it. The worker flushes these once it has room.
pub(crate) struct DropAccumulator {
    batches: Mutex<BTreeMap<GapBatchKey, u32>>,
    /// This run's namespace, minted once at bootstrap and never persisted.
    instance: BridgeInstanceId,
    next_generation: Mutex<u64>,
    /// Drops from sessions the accumulator has no slot for at all. There is no attribution to key
    /// a marker on, so these are reported to the context instead, which stops every session from
    /// claiming complete for as long as the count stands.
    unattributed: Mutex<u32>,
}

/// A default accumulator is a new runtime: it gets its own namespace, which is what a fresh
/// process would get. Sharing one across two accumulators is the collision this exists to prevent.
impl Default for DropAccumulator {
    fn default() -> Self {
        Self::new(BridgeInstanceId::new())
    }
}

impl DropAccumulator {
    pub(crate) fn new(instance: BridgeInstanceId) -> Self {
        Self {
            batches: Mutex::new(BTreeMap::new()),
            instance,
            next_generation: Mutex::new(0),
            unattributed: Mutex::new(0),
        }
    }

    pub(crate) fn record(&self, session_id: &str, reason: EvidenceDropReason) {
        let mut batches = lock(&self.batches);
        // The newest open batch for this session and reason. A batch being retried has a lower id,
        // and adding to it would change the content and the fingerprint of a marker already in
        // flight — the journal would then see the retry as a conflicting duplicate.
        if let Some(newest) = batches
            .keys()
            .filter(|(session, key_reason, _)| session == session_id && *key_reason == reason)
            .map(|(_, _, identity)| identity.clone())
            // By generation rather than by the whole identity: a restored batch keeps whatever
            // namespace it was written under, and ordering on the namespace first would make the
            // newest batch depend on a random string.
            .max_by_key(|identity| identity.generation)
        {
            let entry = batches
                .entry((session_id.to_string(), reason, newest))
                .or_insert(0);
            // Saturating: a count that wrapped would report a smaller gap than occurred, which is
            // worse than reporting a capped one.
            *entry = entry.saturating_add(1);
            return;
        }

        let sessions = batches
            .keys()
            .map(|(session, _, _)| session.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let known_session = sessions.contains(session_id);
        let reasons_here = batches
            .keys()
            .filter(|(session, _, _)| session == session_id)
            .map(|(_, key_reason, _)| *key_reason)
            .collect::<std::collections::BTreeSet<_>>()
            .len();

        if !known_session && sessions.len() >= MAX_TRACKED_DROP_SESSIONS {
            // A session with no slot at all. Keeping its id would mean unbounded growth, so the
            // count goes global rather than being attributed to a session that did not lose it.
            drop(batches);
            let mut unattributed = lock(&self.unattributed);
            *unattributed = unattributed.saturating_add(1);
            return;
        }
        if reason != EvidenceDropReason::AttributionOverflow
            && (reasons_here >= MAX_TRACKED_DROP_REASONS
                || batches.len() >= MAX_TRACKED_GAP_BATCHES)
        {
            // A session the accumulator already knows, whose specific reason it cannot hold. The
            // session keeps its gap under a reason that says the reason itself was lost. An
            // untraceable gap is still a gap; discarding it would let the session read complete.
            drop(batches);
            self.record(session_id, EvidenceDropReason::AttributionOverflow);
            return;
        }
        let identity = self.next_identity();
        batches.insert((session_id.to_string(), reason, identity), 1);
    }

    fn next_identity(&self) -> GapBatchIdentity {
        let mut next = lock(&self.next_generation);
        *next = next.saturating_add(1);
        GapBatchIdentity {
            bridge_instance_id: self.instance.clone(),
            generation: *next,
        }
    }

    /// Puts a snapshot back after a failed flush.
    ///
    /// Keyed by batch, so a restored batch and one opened while the flush was in flight stay
    /// separate: the retry re-sends byte-identical content under the same id, and the newer drops
    /// wait for their own marker.
    pub(crate) fn restore(&self, snapshot: DropSnapshot) {
        let mut batches = lock(&self.batches);
        for (key, count) in snapshot.batches {
            let entry = batches.entry(key).or_insert(0);
            *entry = entry.saturating_add(count);
        }
        drop(batches);
        let mut unattributed = lock(&self.unattributed);
        *unattributed = unattributed.saturating_add(snapshot.unattributed);
    }

    /// Takes a snapshot and leaves the accumulator empty.
    ///
    /// Taking rather than reading-then-clearing is what makes a concurrent drop safe: anything
    /// recorded between those two steps would be erased by the clear, whereas a take leaves an
    /// empty map and the next drop opens a fresh batch.
    pub(crate) fn take(&self) -> DropSnapshot {
        DropSnapshot {
            batches: std::mem::take(&mut lock(&self.batches)),
            unattributed: std::mem::take(&mut lock(&self.unattributed)),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DropSnapshot {
    pub(crate) batches: BTreeMap<GapBatchKey, u32>,
    pub(crate) unattributed: u32,
}

impl DropSnapshot {
    pub(crate) fn is_empty(&self) -> bool {
        self.batches.is_empty() && self.unattributed == 0
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
    sender: SyncSender<BridgeMessage>,
    drops: Arc<DropAccumulator>,
}

/// What travels down the queue.
///
/// `Stop` exists because the worker cannot be ended by dropping the senders: the bridge is cloned
/// into five long-lived assemblies, every one of which holds a sender for the process's whole life.
/// A shutdown that only waited for the channel to close therefore waited for something that could
/// never happen, and paid its full grace on every exit.
enum BridgeMessage {
    Record(Box<RecordEvidenceInput>),
    Stop,
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
        match self.sender.try_send(BridgeMessage::Record(Box::new(input))) {
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
                source_event_id: source_event_id("run-started", &[run_id])?,
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
                source_event_id: source_event_id("run-finished", &[run_id])?,
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
                source_event_id: attempt_scoped("tool-started", call_id, *attempt)?,
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
                source_event_id: attempt_scoped("tool-finished", call_id, *attempt)?,
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
                source_event_id: attempt_scoped("delegation-started", delegation_id, *attempt)?,
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
                source_event_id: attempt_scoped("delegation-finished", delegation_id, *attempt)?,
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
fn attempt_scoped(prefix: &str, id: &str, attempt: Option<u32>) -> Option<SourceEventId> {
    match attempt {
        Some(attempt) => source_event_id(prefix, &[id, &attempt.to_string()]),
        None => source_event_id(prefix, &[id]),
    }
}

/// Builds a source event id whose length does not depend on how long a producer's identifiers are.
///
/// The readable form is kept whenever it fits, because it is what is already stored: changing the
/// shape of an id that fits would make every retry of an event recorded before this change look
/// like a new event, and the journal would hold both.
///
/// It does not always fit. A tool call id, a delegation id, and a model invocation id all come from
/// a provider and are bounded only by the journal's own 128-character limit, so a prefix, a
/// separator, and an attempt are enough to push a legal id past it — and `SourceEventId::parse`
/// then refuses, the signal is dropped as unmappable, and the console records nothing while
/// claiming a coverage gap it cannot explain. That is the same failure the review decision id had,
/// arriving through a different door.
///
/// Over the limit, the parts fold into one digest. Nothing is truncated: a truncated id is a
/// shorter id that two different events can share, which trades a refused write for a silent
/// collision. The digest is a pure function of the parts, so a retry of one observation produces
/// one id, and the kind and phase ride in the namespace while the attempt and the authoritative id
/// ride in the parts — so two attempts, two phases, and two events stay distinct.
fn source_event_id(namespace: &str, parts: &[&str]) -> Option<SourceEventId> {
    let readable = std::iter::once(namespace)
        .chain(parts.iter().copied())
        .collect::<Vec<_>>()
        .join(":");
    if readable.chars().count() <= MAX_IDENTIFIER_LENGTH {
        return SourceEventId::parse(readable).ok();
    }

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    hasher.update([0u8]);
    for part in parts {
        // Length-prefixed, so `["a", "bc"]` and `["ab", "c"]` cannot fold into one digest. A
        // separator alone would not do it: a part is free to contain the separator.
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    let digest = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    // `v1` names the folding scheme. Changing how parts are canonicalised later has to change this,
    // or one event would own two identities across the versions.
    SourceEventId::parse(format!("{namespace}:v1:{digest}")).ok()
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
        self.offer(workspace_session(&signal), map_workspace_signal(&signal));
    }
}

fn workspace_session(signal: &WorkspaceEvidenceSignal) -> &str {
    match signal {
        WorkspaceEvidenceSignal::ShellOpened { session_id, .. }
        | WorkspaceEvidenceSignal::ShellClosed { session_id, .. }
        | WorkspaceEvidenceSignal::FileMutationObserved { session_id, .. } => session_id,
    }
}

fn map_workspace_signal(signal: &WorkspaceEvidenceSignal) -> Option<RecordEvidenceInput> {
    match signal {
        WorkspaceEvidenceSignal::ShellOpened {
            session_id,
            shell_id,
            seat_id,
            runtime,
            occurred_at,
        } => {
            let mut correlation = correlation(session_id, None, None)?;
            bind_agent(&mut correlation, None, seat_id.as_deref());
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Workspaces,
                source_event_id: source_event_id("shell-opened", &[shell_id])?,
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
        }
        WorkspaceEvidenceSignal::ShellClosed {
            session_id,
            shell_id,
            seat_id,
            reason,
            occurred_at,
        } => {
            let mut correlation = correlation(session_id, None, None)?;
            bind_agent(&mut correlation, None, seat_id.as_deref());
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Workspaces,
                // Keyed by the shell alone, with no attempt: a shell closes once, so a replayed
                // close converges on the event already stored rather than adding a second ending.
                source_event_id: source_event_id("shell-closed", &[shell_id])?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: Some(ExecutionStatus::Succeeded),
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::ShellClosed {
                    reason: SafeReasonCode::parse(shell_close_reason(*reason)).ok()?,
                },
                redaction: RedactionReceipt::none(),
            })
        }
        WorkspaceEvidenceSignal::FileMutationObserved {
            session_id,
            basename,
            path_fingerprint,
            change_kind,
            witness_fingerprint,
            observed_directly,
            occurred_at,
        } => {
            let mut correlation = correlation(session_id, None, None)?;
            correlation.file_mutation_id =
                EvidenceFileMutationId::parse(path_fingerprint.clone()).ok();
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Workspaces,
                // Session-scoped. The path digest alone is not an identity: two sessions editing
                // the same relative path in different workspaces produce the same digest, and the
                // journal keys on `(source_context, source_event_id)` without a session — so one
                // session's edit would be filed as a replay of the other's and silently dropped.
                //
                // The witness carries the moment and the observer's own ordinal, so two writes to
                // one file are two observations while an exact duplicate of one observation
                // converges. Folded into a fixed-width revision rather than pasted on: the witness
                // is chosen by the producer, and an identity whose validity depends on how long
                // that choice is fails silently when someone lengthens it.
                source_event_id: source_event_id(
                    "file-mutated",
                    &[
                        session_id,
                        path_fingerprint,
                        &transition_revision(&[
                            change_kind_token(*change_kind),
                            witness_fingerprint,
                        ]),
                    ],
                )?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: None,
                // A change found by comparing two snapshots was not watched happening, and the
                // runtime cannot say who made it. Reporting that as native would claim otherwise.
                fidelity: if *observed_directly {
                    ExecutionFidelity::Native
                } else {
                    ExecutionFidelity::Inferred
                },
                payload: SafeEvidencePayload::FileMutationObserved {
                    basename: SafeBasename::parse(basename.clone()).ok()?,
                    path_fingerprint: SafeFingerprint::parse(path_fingerprint.clone()).ok()?,
                    change_kind: match change_kind {
                        WorkspaceFileChangeKind::Created => FileChangeKind::Added,
                        WorkspaceFileChangeKind::Modified => FileChangeKind::Modified,
                        WorkspaceFileChangeKind::Deleted => FileChangeKind::Deleted,
                        WorkspaceFileChangeKind::Renamed => FileChangeKind::Renamed,
                    },
                },
                redaction: RedactionReceipt::none(),
            })
        }
    }
}

/// The change kind's own token, so the identity does not rely on the producer's witness happening
/// to encode it.
fn change_kind_token(change_kind: WorkspaceFileChangeKind) -> &'static str {
    match change_kind {
        WorkspaceFileChangeKind::Created => "created",
        WorkspaceFileChangeKind::Modified => "modified",
        WorkspaceFileChangeKind::Deleted => "deleted",
        WorkspaceFileChangeKind::Renamed => "renamed",
    }
}

/// Folds the parts of a state transition into a fixed-width revision.
///
/// A source event id is bounded at 128 characters, and pasting variable-length parts together made
/// that bound depend on a choice made in another context. A review's snapshot fingerprint is a
/// full SHA-256 hex, so `review-decision:{uuid}:{fingerprint}:changes_requested` came to 135
/// characters and the journal refused it — while an acceptance fitted at 126. The console recorded
/// reviewers approving work and silently never recorded them rejecting it.
///
/// Folding also keeps these identities about transitions rather than states. The moment is one of
/// the parts, so a reviewer who accepts, retracts, and accepts again produces three ids instead of
/// colliding the third with the first, and a retry of one transition still produces one id because
/// every part comes from state that was saved before the signal was published.
fn transition_revision(parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        // A NUL separator, so two different part lists cannot concatenate into one string.
        hasher.update([0u8]);
    }
    hasher
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The decision's own token, which is part of its identity and not only its payload.
fn review_decision_token(decision: SessionReviewDecision) -> &'static str {
    match decision {
        SessionReviewDecision::Accepted => "accepted",
        SessionReviewDecision::ChangesRequested => "changes_requested",
    }
}

fn shell_close_reason(reason: WorkspaceShellCloseReason) -> &'static str {
    match reason {
        WorkspaceShellCloseReason::ExplicitClose => "shell_closed_by_request",
        WorkspaceShellCloseReason::ProcessExit => "shell_process_exited",
        WorkspaceShellCloseReason::RemoteDisconnect => "shell_remote_disconnected",
        WorkspaceShellCloseReason::IdleCleanup => "shell_idle_reclaimed",
        WorkspaceShellCloseReason::Shutdown => "shell_shutdown",
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
                source_event_id: source_event_id("operation-failed", &[operation_id])?,
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
        self.offer(session_session(&signal), map_session_signal(&signal));
    }
}

fn session_session(signal: &SessionEvidenceSignal) -> &str {
    match signal {
        SessionEvidenceSignal::UsageObserved { session_id, .. }
        | SessionEvidenceSignal::ReviewDecisionRecorded { session_id, .. }
        | SessionEvidenceSignal::ReviewHunkDecisionRecorded { session_id, .. }
        | SessionEvidenceSignal::ReviewFileViewedRecorded { session_id, .. }
        | SessionEvidenceSignal::VerificationCompleted { session_id, .. } => session_id,
    }
}

fn map_session_signal(signal: &SessionEvidenceSignal) -> Option<RecordEvidenceInput> {
    match signal {
        SessionEvidenceSignal::UsageObserved {
            session_id,
            invocation_id,
            run_id,
            quality,
            occurred_at,
        } => {
            let mut correlation = correlation(session_id, None, None)?;
            correlation.run_id = run_id.as_ref().and_then(|value| {
                crate::contexts::execution_observability::api::ExecutionRunId::parse(value.clone())
                    .ok()
            });
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Sessions,
                source_event_id: source_event_id("usage-observed", &[invocation_id])?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: None,
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::UsageObserved {
                    // A count and a classification. The token dimensions, the cache totals, and
                    // any cost stay in sessions; a second copy here would be a second total that
                    // can disagree with the first, with nothing to say which is right. The
                    // invocation is the source event id, which is also what makes a retry
                    // idempotent.
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
        }
        SessionEvidenceSignal::ReviewDecisionRecorded {
            session_id,
            review_id,
            decision,
            witness_fingerprint,
            occurred_at,
        } => {
            let correlation = correlation(session_id, None, None)?;
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Sessions,
                // The review, readable, and then the transition folded into a fixed-width
                // revision: the snapshot it was made about, the decision value, and the moment the
                // review recorded it. The witness distinguishes a decision made after the diff
                // moved on; the value distinguishes a reviewer who changed their mind; the moment
                // distinguishes a reviewer who changed it back, which the value alone reported as
                // a conflicting duplicate of the first decision.
                //
                // `occurred_at` is the review's own `updated_at`, saved before this signal was
                // published, so a replay of the same transition carries the same revision. It is
                // not a clock read here — that would mint a new identity per attempt and turn one
                // decision into as many events as the bridge retried it.
                source_event_id: source_event_id(
                    "review-decision",
                    &[
                        review_id,
                        &transition_revision(&[
                            witness_fingerprint,
                            review_decision_token(*decision),
                            occurred_at,
                        ]),
                    ],
                )?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: None,
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::ReviewDecisionRecorded {
                    // Review scope. The hunk scope is published by its own signal below, from its
                    // own store; the file-viewed scope waits for 13.5. Deriving either from this
                    // one would be a guess wearing an observation's clothes.
                    scope: ReviewDecisionScope::Review,
                    decision: match decision {
                        SessionReviewDecision::Accepted => ReviewDecisionValue::Accepted,
                        SessionReviewDecision::ChangesRequested => {
                            ReviewDecisionValue::ChangesRequested
                        }
                    },
                },
                redaction: RedactionReceipt::none(),
            })
        }
        SessionEvidenceSignal::ReviewHunkDecisionRecorded {
            session_id,
            review_id,
            hunk_fingerprint,
            decision,
            witness_fingerprint,
            occurred_at,
        } => {
            let correlation = correlation(session_id, None, None)?;
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Sessions,
                // The review and the hunk, then the transition folded into a revision, exactly as
                // the review-level decision does. The hunk fingerprint rather than the path: it
                // identifies the hunk without putting workspace content in the journal, and it is
                // already what the decision is keyed by.
                source_event_id: source_event_id(
                    "review-hunk-decision",
                    &[
                        review_id,
                        hunk_fingerprint,
                        &transition_revision(&[
                            witness_fingerprint,
                            review_decision_token(*decision),
                            occurred_at,
                        ]),
                    ],
                )?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: None,
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::ReviewDecisionRecorded {
                    scope: ReviewDecisionScope::Hunk,
                    decision: match decision {
                        SessionReviewDecision::Accepted => ReviewDecisionValue::Accepted,
                        SessionReviewDecision::ChangesRequested => {
                            ReviewDecisionValue::ChangesRequested
                        }
                    },
                },
                redaction: RedactionReceipt::none(),
            })
        }
        SessionEvidenceSignal::ReviewFileViewedRecorded {
            session_id,
            review_id,
            file_witness,
            witness_fingerprint,
            occurred_at,
        } => {
            let correlation = correlation(session_id, None, None)?;
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Sessions,
                // The review and the file's witness, then the moment folded in. Re-reading the
                // same version of the same file is the same observation and records once; reading
                // it again after it changed is a different one, because the witness changed with
                // it.
                source_event_id: source_event_id(
                    "review-file-viewed",
                    &[
                        review_id,
                        file_witness,
                        &transition_revision(&[witness_fingerprint, occurred_at]),
                    ],
                )?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: None,
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::ReviewDecisionRecorded {
                    scope: ReviewDecisionScope::FileViewed,
                    // Reading a file is not a judgement about it. `Pending` is the value that says
                    // so, and the scope is what distinguishes this from a decision nobody made.
                    decision: ReviewDecisionValue::Pending,
                },
                redaction: RedactionReceipt::none(),
            })
        }
        SessionEvidenceSignal::VerificationCompleted {
            session_id,
            run_id,
            verification_run_id,
            name,
            outcome,
            passed_count,
            failed_count,
            occurred_at,
        } => {
            let mut correlation = correlation(session_id, None, None)?;
            correlation.run_id = run_id.as_ref().and_then(|value| {
                crate::contexts::execution_observability::api::ExecutionRunId::parse(value.clone())
                    .ok()
            });
            Some(RecordEvidenceInput {
                source_context: EvidenceSourceContext::Sessions,
                source_event_id: source_event_id("verification", &[verification_run_id])?,
                occurred_at: occurred_at.clone(),
                correlation,
                status: Some(match outcome {
                    SessionVerificationOutcome::Passed => ExecutionStatus::Succeeded,
                    SessionVerificationOutcome::Failed => ExecutionStatus::Failed,
                }),
                fidelity: ExecutionFidelity::Native,
                payload: SafeEvidencePayload::VerificationCompleted {
                    // The check's name and its counts. What it said about any particular line is
                    // a finding, and findings live in the store that can render them safely.
                    name: BoundedLabel::parse("verification name", name.clone()).ok()?,
                    outcome: match outcome {
                        SessionVerificationOutcome::Passed => VerificationOutcome::Passed,
                        SessionVerificationOutcome::Failed => VerificationOutcome::Failed,
                    },
                    passed_count: *passed_count,
                    failed_count: *failed_count,
                },
                redaction: RedactionReceipt::none(),
            })
        }
    }
}

/// The receiving half. Owns the thread and the only handle that calls the recorder.
pub(crate) struct EvidenceBridgeWorker {
    handle: Option<JoinHandle<()>>,
    /// The sender the shutdown uses to end the worker, and nothing else uses.
    ///
    /// Kept here rather than reached through the bridge because the two have opposite lifetimes: the
    /// bridge is cloned everywhere and lives forever, and this is dropped the moment shutdown has
    /// used it.
    stop: SyncSender<BridgeMessage>,
    /// Set as well as sent, for the case where the queue is full.
    ///
    /// A bounded queue can refuse the stop message, and a blocking send would then hold the exit
    /// open on whatever write the worker is in the middle of. The flag is what the worker sees after
    /// finishing that write.
    stopping: Arc<AtomicBool>,
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
        // Told to stop, then waited for. Waiting alone was the defect: the worker blocks on a
        // channel whose senders outlive it by design, so the wait could only ever end at the
        // deadline — two seconds on the event-loop thread, on every exit, with the window already
        // gone and the process apparently hung.
        self.stopping.store(true, Ordering::SeqCst);
        let _ = self.stop.try_send(BridgeMessage::Stop);
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
    // The namespace is minted here and nowhere else: one bridge is one run, and every gap batch it
    // reports carries that run's name so a restart cannot reuse a previous run's batch identity.
    let drops = Arc::new(DropAccumulator::new(BridgeInstanceId::new()));
    let bridge = EvidenceBridge {
        sender,
        drops: drops.clone(),
    };
    let stopping = Arc::new(AtomicBool::new(false));
    let stop = bridge.sender.clone();
    let handle = std::thread::Builder::new()
        .name("evidence-bridge".to_string())
        .spawn({
            let stopping = stopping.clone();
            move || run_worker(evidence, receiver, drops, stopping)
        })
        .ok();
    (
        bridge,
        EvidenceBridgeWorker {
            handle,
            stop,
            stopping,
        },
    )
}

/// One failed record must not end the worker.
///
/// A store that rejects one event usually accepts the next — a conflicting source id, a value the
/// domain refuses — and a worker that exited on the first would turn one bad observation into the
/// permanent loss of every later one.
fn run_worker(
    evidence: ExecutionEvidenceApi,
    receiver: Receiver<BridgeMessage>,
    drops: Arc<DropAccumulator>,
    stopping: Arc<AtomicBool>,
) {
    while let Ok(message) = receiver.recv() {
        let BridgeMessage::Record(input) = message else {
            break;
        };
        let input = *input;
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
        // Checked after the write rather than before the read, because the interesting case is a
        // stop that arrived while this thread was inside SQLite: the queue was full, the message
        // could not be sent, and the flag is what is left.
        if stopping.load(Ordering::SeqCst) {
            break;
        }
    }
    // Nothing more is coming; report whatever never made it, on the way down.
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
    let mut unflushed = DropSnapshot::default();
    for ((session_id, reason, identity), count) in snapshot.batches {
        let Ok(session) = EvidenceSessionId::parse(session_id.clone()) else {
            // An unusable session cannot key a marker, and inventing one would file the gap
            // against a session that never existed. It stays a global count.
            unflushed.unattributed = unflushed.unattributed.saturating_add(count);
            continue;
        };
        match record_gap_marker(evidence, &session, reason, &identity, count) {
            // The marker is durable, so the batch it represents is discharged. The notice and the
            // diagnostic ride along on the same call, which is why a recovered batch publishes
            // exactly one notice rather than one per retry.
            Ok(()) => evidence.record_dropped_events(&session, count),
            // The batch goes back under its own id, so the retry re-sends byte-identical content.
            // Anything recorded while this flush ran opened a newer batch and is untouched.
            Err(()) => {
                unflushed
                    .batches
                    .entry((session_id, reason, identity))
                    .and_modify(|existing| *existing = existing.saturating_add(count))
                    .or_insert(count);
            }
        }
    }
    // Sessionless drops never become a marker: the journal keys on a session, and one filed under
    // a placeholder would attribute a loss to work that lost nothing. They are reported to the
    // context instead, which stops every session from claiming complete while the count stands.
    if snapshot.unattributed > 0 {
        evidence.report_unattributed_gap(snapshot.unattributed);
    }
    if !unflushed.is_empty() {
        drops.restore(unflushed);
    }
}

/// Writes the durable coverage gap.
///
/// Through the recorder, never the queue: re-queueing the report would put it behind the same full
/// queue that produced it, so a burst would silence exactly the signal that says a burst happened.
/// A failure here is returned rather than counted, because counting it would produce a gap about
/// failing to record a gap, and that recursion has no fixed point.
fn record_gap_marker(
    evidence: &ExecutionEvidenceApi,
    session: &EvidenceSessionId,
    reason: EvidenceDropReason,
    identity: &GapBatchIdentity,
    dropped_count: u32,
) -> Result<(), ()> {
    let Ok(reason_code) = SafeReasonCode::parse(reason.as_str()) else {
        return Err(());
    };
    // Session, reason, runtime namespace, and generation. The count is deliberately not part of
    // the identity: two gaps of the same size are two gaps, and keying on the size would make the
    // second collide with the first — which the journal reports as a conflicting duplicate rather
    // than storing. The namespace is what keeps this run's generation one from colliding with the
    // previous run's, which the journal outlives.
    let Some(source_event_id) = source_event_id(
        "coverage-gap",
        &[
            session.as_str(),
            reason.as_str(),
            &identity.as_source_fragment(),
        ],
    ) else {
        return Err(());
    };
    let input = RecordEvidenceInput {
        source_context: EvidenceSourceContext::ExecutionObservability,
        source_event_id,
        occurred_at: chrono::Utc::now().to_rfc3339(),
        correlation: EvidenceCorrelation::for_session(session.clone()),
        status: None,
        fidelity: ExecutionFidelity::Native,
        payload: SafeEvidencePayload::CoverageGapRecorded {
            dropped_count,
            reason: reason_code,
        },
        redaction: RedactionReceipt::none(),
    };
    evidence.record(input).map(|_| ()).map_err(|_| ())
}
