#[cfg(test)]
use super::EvidenceDomainError;
use super::SafeReasonCode;

const MAX_COVERAGE_REASON_CODES: usize = 8;

/// Whether an answer describes the whole retained corpus.
///
/// `Complete` is a claim about the store, not a description of the result set. An empty page with
/// `Complete` coverage says "this really did not happen"; the same page with `Partial` says "we
/// cannot see all of it". Collapsing the two is the failure this whole capability exists to stop,
/// so the state is never inferred from a row count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceCoverageState {
    Complete,
    Indexing,
    Partial,
    Unavailable,
}

impl EvidenceCoverageState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Indexing => "indexing",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Stable reason codes this context emits. They are localized by the frontend, so they are part of
/// the contract and cannot be reworded at the call site.
pub(crate) mod reason_codes {
    /// The store answered, but no producer is wired to it yet, so an empty result says nothing
    /// about whether work happened. Removed in Task Group 4 when producers begin publishing.
    pub(crate) const CAPTURE_NOT_INITIALIZED: &str = "evidence_capture_not_initialized";
    /// A conflicting reuse of a source id left one version of an event unrecorded.
    pub(crate) const CONFLICTING_SOURCE_EVENT: &str = "evidence_conflicting_source_event";
    /// A bounded queue dropped events before they reached the journal.
    pub(crate) const DROPPED_EVENTS: &str = "evidence_dropped_events";
    /// Retention removed events that would otherwise be in range.
    pub(crate) const RETENTION_EXPIRED: &str = "evidence_retention_expired";
    /// A projection rebuild is in progress.
    pub(crate) const PROJECTION_REBUILDING: &str = "evidence_projection_rebuilding";
    /// The owning context of a summary figure has not been connected yet.
    pub(crate) const SOURCE_NOT_OWNED: &str = "evidence_source_not_owned";
    /// Evidence was lost somewhere in this process and the bounded accumulator could not keep the
    /// session it belonged to. No session can be told it is whole while this stands, because the
    /// one that lost the evidence is among them and nothing here can say which.
    pub(crate) const GAP_ATTRIBUTION_OVERFLOW: &str = "evidence_gap_attribution_overflow";
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueryCoverage {
    state: EvidenceCoverageState,
    reason_codes: Vec<SafeReasonCode>,
    oldest_available_at: Option<String>,
    newest_available_at: Option<String>,
    indexed_through_at: Option<String>,
    dropped_count: Option<u32>,
    truncated: bool,
}

impl QueryCoverage {
    /// The general constructor, reached only by the test coverage builder. Production paths start
    /// from `complete()` and degrade, because that is the direction the rules allow: an answer can
    /// lose completeness as sources are merged, never gain it.
    #[cfg(test)]
    pub(crate) fn new(
        state: EvidenceCoverageState,
        reason_codes: impl IntoIterator<Item = SafeReasonCode>,
    ) -> Result<Self, EvidenceDomainError> {
        let mut reason_codes = reason_codes.into_iter().collect::<Vec<_>>();
        reason_codes.sort();
        reason_codes.dedup();
        if reason_codes.len() > MAX_COVERAGE_REASON_CODES {
            return Err(EvidenceDomainError::TooManyEntries {
                field: "coverage reason codes",
                max: MAX_COVERAGE_REASON_CODES,
            });
        }
        Ok(Self {
            state,
            reason_codes,
            oldest_available_at: None,
            newest_available_at: None,
            indexed_through_at: None,
            dropped_count: None,
            truncated: false,
        })
    }

    pub(crate) fn complete() -> Self {
        Self {
            state: EvidenceCoverageState::Complete,
            reason_codes: Vec::new(),
            oldest_available_at: None,
            newest_available_at: None,
            indexed_through_at: None,
            dropped_count: None,
            truncated: false,
        }
    }

    pub(crate) fn with_boundaries(
        mut self,
        oldest: Option<String>,
        newest: Option<String>,
    ) -> Self {
        self.oldest_available_at = oldest;
        self.newest_available_at = newest;
        self
    }

    pub(crate) fn with_indexed_through(mut self, indexed_through: Option<String>) -> Self {
        self.indexed_through_at = indexed_through;
        self
    }

    pub(crate) fn with_dropped_count(mut self, dropped: Option<u32>) -> Self {
        self.dropped_count = dropped;
        self
    }

    pub(crate) fn with_truncated(mut self, truncated: bool) -> Self {
        self.truncated = truncated;
        self
    }

    /// Degradation is one-way. Merging two sources can only make an answer less complete, never
    /// more, so a partial source cannot be washed out by a complete one.
    pub(crate) fn degrade_to(mut self, state: EvidenceCoverageState, reason: &str) -> Self {
        let rank = |state: EvidenceCoverageState| match state {
            EvidenceCoverageState::Complete => 0,
            EvidenceCoverageState::Indexing => 1,
            EvidenceCoverageState::Partial => 2,
            EvidenceCoverageState::Unavailable => 3,
        };
        if rank(state) > rank(self.state) {
            self.state = state;
        }
        if let Ok(code) = SafeReasonCode::parse(reason) {
            if !self.reason_codes.contains(&code)
                && self.reason_codes.len() < MAX_COVERAGE_REASON_CODES
            {
                self.reason_codes.push(code);
                self.reason_codes.sort();
            }
        }
        self
    }

    pub(crate) fn state(&self) -> EvidenceCoverageState {
        self.state
    }

    pub(crate) fn reason_codes(&self) -> &[SafeReasonCode] {
        &self.reason_codes
    }

    pub(crate) fn oldest_available_at(&self) -> Option<&str> {
        self.oldest_available_at.as_deref()
    }

    pub(crate) fn newest_available_at(&self) -> Option<&str> {
        self.newest_available_at.as_deref()
    }

    pub(crate) fn indexed_through_at(&self) -> Option<&str> {
        self.indexed_through_at.as_deref()
    }

    pub(crate) fn dropped_count(&self) -> Option<u32> {
        self.dropped_count
    }

    pub(crate) fn truncated(&self) -> bool {
        self.truncated
    }
}
