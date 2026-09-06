use super::{ExecutionDomainError, ExecutionRunId, SafeAttributes, SpanId, TraceId};

const MAX_SPAN_NAME_LENGTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionStatus {
    Accepted,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Incomplete,
}

impl ExecutionStatus {
    /// Every status, so code that must reason about all of them has one place to read them from.
    ///
    /// Hand-written, because Rust cannot iterate an enum's variants. Kept adjacent to the
    /// definition so the two are edited together, and its length is part of the type — but be
    /// clear about the limit: no test here can prove a new variant was added to this list, since
    /// anything iterating `ALL` never visits what `ALL` omits. What the tests below do pin is that
    /// the list has no duplicates and that both sides of the terminal split stay populated.
    ///
    /// **Adding a variant means adding it here too.** The compiler will not say so.
    pub(crate) const ALL: [Self; 6] = [
        Self::Accepted,
        Self::Running,
        Self::Succeeded,
        Self::Failed,
        Self::Cancelled,
        Self::Incomplete,
    ];

    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Incomplete
        )
    }

    /// The stable wire token, matching the DTO's serde rename.
    ///
    /// Written once here rather than derived at each boundary, because a status a client switches
    /// on has to mean the same thing on every channel it can arrive over.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Incomplete => "incomplete",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionFidelity {
    Native,
    Proxied,
    Inferred,
    Opaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapturePolicy {
    MetadataOnly,
    RedactedContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExecutionSource {
    Desktop,
    InstantMessage { connector_id: String },
    Scheduled { task_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionLink {
    pub(crate) run_id: ExecutionRunId,
    pub(crate) trace_id: TraceId,
    pub(crate) span_id: Option<SpanId>,
    pub(crate) relationship: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionContext {
    pub(crate) run_id: ExecutionRunId,
    pub(crate) trace_id: TraceId,
    pub(crate) span_id: SpanId,
    pub(crate) capture_policy: CapturePolicy,
    pub(crate) sampling_per_million: u32,
    pub(crate) mcp_relay_enabled: bool,
}

impl ExecutionContext {
    pub(crate) fn traceparent(&self) -> String {
        format!("00-{}-{}-01", self.trace_id.as_str(), self.span_id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExecutionRun {
    pub(crate) context: ExecutionContext,
    pub(crate) source: ExecutionSource,
    pub(crate) status: ExecutionStatus,
    pub(crate) started_at: String,
    pub(crate) ended_at: Option<String>,
    pub(crate) error_classification: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) user_message_id: Option<String>,
    pub(crate) assistant_message_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) provider_session_id: Option<String>,
    pub(crate) attributes: SafeAttributes,
    pub(crate) links: Vec<ExecutionLink>,
}

impl ExecutionRun {
    pub(crate) fn validate(&self) -> Result<(), ExecutionDomainError> {
        require_timestamp(&self.started_at)?;
        if let Some(ended_at) = &self.ended_at {
            require_timestamp(ended_at)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExecutionSpan {
    pub(crate) context: ExecutionContext,
    pub(crate) parent_span_id: Option<SpanId>,
    pub(crate) name: String,
    pub(crate) status: ExecutionStatus,
    pub(crate) fidelity: ExecutionFidelity,
    pub(crate) started_at: String,
    pub(crate) ended_at: Option<String>,
    pub(crate) error_classification: Option<String>,
    pub(crate) attributes: SafeAttributes,
    pub(crate) links: Vec<ExecutionLink>,
}

impl ExecutionSpan {
    pub(crate) fn validate(&self) -> Result<(), ExecutionDomainError> {
        if self.name.trim().is_empty() || self.name.chars().count() > MAX_SPAN_NAME_LENGTH {
            return Err(ExecutionDomainError::InvalidSpanName {
                max: MAX_SPAN_NAME_LENGTH,
            });
        }
        require_timestamp(&self.started_at)?;
        if let Some(ended_at) = &self.ended_at {
            require_timestamp(ended_at)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExecutionEvent {
    pub(crate) run_id: ExecutionRunId,
    pub(crate) span_id: SpanId,
    pub(crate) sequence: u64,
    pub(crate) name: String,
    pub(crate) timestamp: String,
    pub(crate) attributes: SafeAttributes,
}

/// What a timeline response says about the events it did not return.
///
/// Always present rather than inferred from an absent field. A consumer that read "no coverage
/// stated" as "nothing was omitted" would present a truncated event record as the complete one,
/// which is precisely the failure this type exists to make impossible — and the reader would have
/// no way to notice, because a truncated list looks exactly like a short one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct EventCoverage {
    /// Whether events were omitted because the run exceeded the response bound.
    pub(crate) truncated: bool,
    /// Where to resume reading. Present exactly when `truncated`.
    pub(crate) next_page_token: Option<String>,
}

impl EventCoverage {
    /// Every event this run has is in the response.
    pub(crate) fn complete() -> Self {
        Self {
            truncated: false,
            next_page_token: None,
        }
    }

    /// The response stopped at its bound; `token` resumes after the last event returned.
    ///
    /// Constructed rather than assembled field by field so the two can never disagree — a
    /// `truncated` with no continuation would tell a reader something is missing and give them no
    /// way to reach it.
    pub(crate) fn truncated_at(token: String) -> Self {
        Self {
            truncated: true,
            next_page_token: Some(token),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExecutionTimeline {
    pub(crate) run: ExecutionRun,
    pub(crate) spans: Vec<ExecutionSpan>,
    pub(crate) events: Vec<ExecutionEvent>,
    /// What this response says about events beyond the ones it carries.
    pub(crate) event_coverage: EventCoverage,
}

impl ExecutionEvent {
    pub(crate) fn validate(&self) -> Result<(), ExecutionDomainError> {
        if self.name.trim().is_empty() || self.name.chars().count() > MAX_SPAN_NAME_LENGTH {
            return Err(ExecutionDomainError::InvalidSpanName {
                max: MAX_SPAN_NAME_LENGTH,
            });
        }
        require_timestamp(&self.timestamp)
    }
}

fn require_timestamp(value: &str) -> Result<(), ExecutionDomainError> {
    if value.trim().is_empty() {
        Err(ExecutionDomainError::TimestampRequired)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ExecutionStatus;

    /// `ALL` lists each variant at most once.
    ///
    /// A duplicate would be harmless in the SQL list it feeds but is a sign the list was edited
    /// carelessly, and the next such edit is the one that omits a variant instead. This cannot
    /// detect an omission — iterating `ALL` never reaches what `ALL` left out — so the match below
    /// is a compile-time prompt rather than a proof: adding a variant stops this file compiling,
    /// at which point the reader is standing next to `ALL`.
    #[test]
    fn all_lists_each_variant_at_most_once() {
        let listed = ExecutionStatus::ALL.len();
        let mut tokens = ExecutionStatus::ALL
            .iter()
            .map(|status| status.as_str())
            .collect::<Vec<_>>();
        tokens.sort_unstable();
        tokens.dedup();
        assert_eq!(tokens.len(), listed, "ALL repeats a variant");

        for status in ExecutionStatus::ALL {
            match status {
                ExecutionStatus::Accepted
                | ExecutionStatus::Running
                | ExecutionStatus::Succeeded
                | ExecutionStatus::Failed
                | ExecutionStatus::Cancelled
                | ExecutionStatus::Incomplete => (),
            }
        }
    }

    /// Both sides of the terminal split have members.
    ///
    /// An empty non-terminal side would make the advanceable SQL list empty, which is valid SQL
    /// that silently refuses every transition.
    #[test]
    fn the_terminal_split_leaves_both_sides_populated() {
        assert!(ExecutionStatus::ALL.iter().any(|s| s.is_terminal()));
        assert!(ExecutionStatus::ALL.iter().any(|s| !s.is_terminal()));
    }
}
