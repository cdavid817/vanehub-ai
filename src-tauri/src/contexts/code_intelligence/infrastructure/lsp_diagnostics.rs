use crate::contexts::code_intelligence::domain::models::{Language, ProcessState};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEFAULT_REPEAT_BURST: usize = 3;
const DEFAULT_REPEAT_WINDOW: Duration = Duration::from_secs(60);
const MAX_IDENTITY_CHARS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LspMethodCategory {
    Initialize,
    SemanticQuery,
    Transport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LspCrashReason {
    UnexpectedExit,
    ProtocolFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct LspDiagnosticIdentity {
    pub(crate) language: Language,
    pub(crate) workspace_id: Option<String>,
    pub(crate) correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LspDiagnosticKind {
    Lifecycle {
        from: ProcessState,
        to: ProcessState,
    },
    ProtocolLimit {
        method: LspMethodCategory,
        duration_ms: u64,
        observed_bytes: u64,
    },
    Timeout {
        method: LspMethodCategory,
        duration_ms: u64,
        server_state: ProcessState,
    },
    Cancellation {
        method: LspMethodCategory,
        duration_ms: u64,
        server_state: ProcessState,
    },
    Crash {
        exit_code: Option<i32>,
        restart_attempt: u32,
        reason: LspCrashReason,
    },
    Restart {
        restart_attempt: u32,
    },
    DiagnosticsCount {
        count: usize,
    },
    Shutdown {
        forced: bool,
        process_count: usize,
        duration_ms: u64,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LspPrivateDiagnosticData {
    pub(crate) raw_protocol_payload: Option<String>,
    pub(crate) diagnostic_message: Option<String>,
    pub(crate) hover_content: Option<String>,
    pub(crate) source_content: Option<String>,
    pub(crate) stderr: Option<String>,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) arguments: Vec<String>,
    pub(crate) credential: Option<String>,
    pub(crate) absolute_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LspDiagnosticEvent {
    pub(crate) identity: LspDiagnosticIdentity,
    pub(crate) kind: LspDiagnosticKind,
    pub(crate) private: LspPrivateDiagnosticData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RepeatedKind {
    Timeout(LspMethodCategory),
    Crash,
    Restart,
    DiagnosticsCount,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RateLimitKey {
    identity: LspDiagnosticIdentity,
    kind: RepeatedKind,
}

struct RateLimitEntry {
    window_started_at: Duration,
    emitted: usize,
    suppressed: u64,
}

#[derive(Clone)]
pub(crate) struct LspDiagnosticLogger {
    logging: Arc<dyn DiagnosticLogPort>,
    started_at: Instant,
    repeat_burst: usize,
    repeat_window: Duration,
    rate_limits: Arc<Mutex<HashMap<RateLimitKey, RateLimitEntry>>>,
}

impl LspDiagnosticLogger {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self::with_rate_limit(logging, DEFAULT_REPEAT_BURST, DEFAULT_REPEAT_WINDOW)
    }

    pub(crate) fn with_rate_limit(
        logging: Arc<dyn DiagnosticLogPort>,
        repeat_burst: usize,
        repeat_window: Duration,
    ) -> Self {
        Self {
            logging,
            started_at: Instant::now(),
            repeat_burst: repeat_burst.max(1),
            repeat_window: repeat_window.max(Duration::from_millis(1)),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn record(&self, event: LspDiagnosticEvent) {
        self.record_at(event, self.started_at.elapsed());
    }

    pub(crate) fn record_at(&self, event: LspDiagnosticEvent, now: Duration) {
        let Some(suppressed_count) = self.admit(&event, now) else {
            return;
        };
        let _ = self
            .logging
            .write_diagnostic(build_log(event, suppressed_count));
    }

    fn admit(&self, event: &LspDiagnosticEvent, now: Duration) -> Option<u64> {
        let Ok(mut limits) = self.rate_limits.lock() else {
            return Some(0);
        };
        if matches!(
            event.kind,
            LspDiagnosticKind::Lifecycle {
                to: ProcessState::Failed,
                ..
            }
        ) {
            let suppressed = limits
                .iter()
                .filter(|(key, _)| key.identity == event.identity)
                .map(|(_, entry)| entry.suppressed)
                .sum();
            limits.retain(|key, _| key.identity != event.identity);
            return Some(suppressed);
        }
        let kind = match event.kind {
            LspDiagnosticKind::Timeout { method, .. } => RepeatedKind::Timeout(method),
            LspDiagnosticKind::Crash { .. } => RepeatedKind::Crash,
            LspDiagnosticKind::Restart { .. } => RepeatedKind::Restart,
            LspDiagnosticKind::DiagnosticsCount { .. } => RepeatedKind::DiagnosticsCount,
            _ => return Some(0),
        };
        let entry = limits.entry(RateLimitKey {
            identity: event.identity.clone(),
            kind,
        });
        let entry = entry.or_insert(RateLimitEntry {
            window_started_at: now,
            emitted: 0,
            suppressed: 0,
        });
        if now.saturating_sub(entry.window_started_at) >= self.repeat_window {
            let suppressed = entry.suppressed;
            entry.window_started_at = now;
            entry.emitted = 1;
            entry.suppressed = 0;
            return Some(suppressed);
        }
        if entry.emitted < self.repeat_burst {
            entry.emitted += 1;
            return Some(0);
        }
        entry.suppressed = entry.suppressed.saturating_add(1);
        None
    }
}

fn build_log(event: LspDiagnosticEvent, suppressed_count: u64) -> DiagnosticLog {
    let LspDiagnosticEvent {
        identity,
        kind,
        private,
    } = event;
    drop(private);
    let mut context = BTreeMap::from([
        ("language".to_string(), identity.language.id.to_string()),
        (
            "server".to_string(),
            identity.language.server_id.to_string(),
        ),
    ]);
    insert_bounded(&mut context, "workspaceId", identity.workspace_id);
    insert_bounded(&mut context, "correlationId", identity.correlation_id);
    if suppressed_count > 0 {
        context.insert("suppressedCount".to_string(), suppressed_count.to_string());
    }
    let severity = append_kind_context(&mut context, kind);
    DiagnosticLog {
        severity,
        category: "code_intelligence.lsp".to_string(),
        message: "LSP runtime diagnostic".to_string(),
        context,
    }
}

fn append_kind_context(
    context: &mut BTreeMap<String, String>,
    kind: LspDiagnosticKind,
) -> LogSeverity {
    let (event, severity) = match kind {
        LspDiagnosticKind::Lifecycle { from, to } => {
            context.insert("fromState".to_string(), process_state(from).to_string());
            context.insert("toState".to_string(), process_state(to).to_string());
            ("lifecycle", LogSeverity::Info)
        }
        LspDiagnosticKind::ProtocolLimit {
            method,
            duration_ms,
            observed_bytes,
        } => {
            insert_method_duration(context, method, duration_ms);
            context.insert("observedBytes".to_string(), observed_bytes.to_string());
            ("protocol_limit", LogSeverity::Error)
        }
        LspDiagnosticKind::Timeout {
            method,
            duration_ms,
            server_state,
        } => {
            insert_method_duration(context, method, duration_ms);
            context.insert(
                "serverState".to_string(),
                process_state(server_state).to_string(),
            );
            ("timeout", LogSeverity::Warn)
        }
        LspDiagnosticKind::Cancellation {
            method,
            duration_ms,
            server_state,
        } => {
            insert_method_duration(context, method, duration_ms);
            context.insert(
                "serverState".to_string(),
                process_state(server_state).to_string(),
            );
            ("cancelled", LogSeverity::Info)
        }
        LspDiagnosticKind::Crash {
            exit_code,
            restart_attempt,
            reason,
        } => {
            if let Some(exit_code) = exit_code {
                context.insert("exitCode".to_string(), exit_code.to_string());
            }
            context.insert("restartAttempt".to_string(), restart_attempt.to_string());
            context.insert(
                "reasonCategory".to_string(),
                crash_reason_id(reason).to_string(),
            );
            ("crash", LogSeverity::Error)
        }
        LspDiagnosticKind::Restart { restart_attempt } => {
            context.insert("restartAttempt".to_string(), restart_attempt.to_string());
            ("restart", LogSeverity::Warn)
        }
        LspDiagnosticKind::DiagnosticsCount { count } => {
            context.insert("diagnosticCount".to_string(), count.to_string());
            ("diagnostics_count", LogSeverity::Info)
        }
        LspDiagnosticKind::Shutdown {
            forced,
            process_count,
            duration_ms,
        } => {
            context.insert("forced".to_string(), forced.to_string());
            context.insert("processCount".to_string(), process_count.to_string());
            context.insert("durationMs".to_string(), duration_ms.to_string());
            ("shutdown", LogSeverity::Info)
        }
    };
    context.insert("event".to_string(), event.to_string());
    severity
}

fn insert_method_duration(
    context: &mut BTreeMap<String, String>,
    method: LspMethodCategory,
    duration_ms: u64,
) {
    context.insert("methodCategory".to_string(), method_id(method).to_string());
    context.insert("durationMs".to_string(), duration_ms.to_string());
}

fn insert_bounded(context: &mut BTreeMap<String, String>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        context.insert(
            key.to_string(),
            value.chars().take(MAX_IDENTITY_CHARS).collect(),
        );
    }
}

const fn method_id(method: LspMethodCategory) -> &'static str {
    match method {
        LspMethodCategory::Initialize => "initialize",
        LspMethodCategory::SemanticQuery => "semantic_query",
        LspMethodCategory::Transport => "transport",
    }
}

const fn crash_reason_id(reason: LspCrashReason) -> &'static str {
    match reason {
        LspCrashReason::UnexpectedExit => "unexpected_exit",
        LspCrashReason::ProtocolFailure => "protocol_failure",
    }
}

const fn process_state(state: ProcessState) -> &'static str {
    match state {
        ProcessState::Absent => "absent",
        ProcessState::Starting => "starting",
        ProcessState::Initializing => "initializing",
        ProcessState::Ready => "ready",
        ProcessState::Stopping => "stopping",
        ProcessState::Backoff => "backoff",
        ProcessState::Failed => "failed",
    }
}
