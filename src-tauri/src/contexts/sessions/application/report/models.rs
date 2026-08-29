//! What a session-run report is, as the sessions context defines it.
//!
//! Owned here rather than by observability or workspaces because the report is *about a session*:
//! it spans every context that did work under one, and the thing it reports on is the session's
//! own life. Any of the contributing contexts could have hosted it, and each would have made its
//! own signal the centre.
//!
//! Two rules run through every type below, and both are about not turning an absence into a
//! number. A figure nobody measured is `None`, never zero — a zero is a measurement, and a report
//! is precisely the artifact somebody quotes. And a section that could not be assembled says so on
//! its own, because a report can be useful while one of its sources is still indexing, and a single
//! report-level state would either hide that or discard the sections that are fine.

use std::collections::BTreeMap;

/// How rows are grouped in the per-dimension sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ReportGroupBy {
    #[default]
    Run,
    Agent,
    Seat,
    Model,
    Tool,
}

impl ReportGroupBy {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Agent => "agent",
            Self::Seat => "seat",
            Self::Model => "model",
            Self::Tool => "tool",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "run" => Some(Self::Run),
            "agent" => Some(Self::Agent),
            "seat" => Some(Self::Seat),
            "model" => Some(Self::Model),
            "tool" => Some(Self::Tool),
            _ => None,
        }
    }
}

/// What a section is willing to claim.
///
/// The same four states the rest of this change uses, and for the same reason: `Complete` is the
/// only one that licenses a conclusion from an absence, so it is the one a section has to earn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ReportCoverageState {
    Complete,
    Indexing,
    Partial,
    /// The default. A section nobody filled in must not read as one that found nothing.
    #[default]
    Unavailable,
}

impl ReportCoverageState {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Indexing => "indexing",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }

    /// The weakest claim among several.
    ///
    /// Used for the report-level state, which can only be as strong as its weakest section — a
    /// report whose overall coverage read `complete` while one section was unavailable would be
    /// asserting exactly what that section declined to.
    pub(crate) fn weakest(states: impl IntoIterator<Item = Self>) -> Self {
        states
            .into_iter()
            .fold(Self::Complete, |worst, state| match (worst, state) {
                (Self::Unavailable, _) | (_, Self::Unavailable) => Self::Unavailable,
                (Self::Partial, _) | (_, Self::Partial) => Self::Partial,
                (Self::Indexing, _) | (_, Self::Indexing) => Self::Indexing,
                _ => Self::Complete,
            })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReportSectionCoverage {
    pub(crate) state: ReportCoverageState,
    /// Stable codes, never prose: a reader groups by them and free text does not group.
    pub(crate) reason_codes: Vec<String>,
}

impl ReportSectionCoverage {
    pub(crate) fn complete() -> Self {
        Self {
            state: ReportCoverageState::Complete,
            reason_codes: Vec::new(),
        }
    }

    pub(crate) fn unavailable(reason: &str) -> Self {
        Self {
            state: ReportCoverageState::Unavailable,
            reason_codes: vec![reason.to_string()],
        }
    }

    pub(crate) fn partial(reason: &str) -> Self {
        Self {
            state: ReportCoverageState::Partial,
            reason_codes: vec![reason.to_string()],
        }
    }
}

/// Which sections a report has, and what each of them can claim.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReportCoverage {
    pub(crate) sections: BTreeMap<&'static str, ReportSectionCoverage>,
}

/// The section names, so a caller and a renderer cannot disagree about the set.
pub(crate) const REPORT_SECTIONS: &[&str] = &[
    "overview",
    "usage",
    "latency",
    "agents",
    "tools",
    "commands",
    "changes",
    "verification",
    "failures",
];

impl ReportCoverage {
    pub(crate) fn overall(&self) -> ReportCoverageState {
        // A section this build knows about but nobody filled in counts as unavailable, which is
        // why the fold runs over the known names rather than over whatever happens to be present.
        ReportCoverageState::weakest(REPORT_SECTIONS.iter().map(|name| {
            self.sections
                .get(name)
                .map(|section| section.state)
                .unwrap_or_default()
        }))
    }

    pub(crate) fn set(&mut self, section: &'static str, coverage: ReportSectionCoverage) {
        self.sections.insert(section, coverage);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReportOverview {
    pub(crate) run_count: u32,
    /// Absent when any run in scope is unfinished: a total that silently omitted them would read
    /// as the whole session's duration.
    pub(crate) duration_ms: Option<u64>,
    pub(crate) succeeded: u32,
    pub(crate) failed: u32,
    pub(crate) cancelled: u32,
    pub(crate) retries: u32,
}

/// Usage, with the three qualities kept apart all the way out.
///
/// Adding them would turn an estimate into a reported figure, which is the one thing a usage
/// report must not do — the number is quoted, and whoever quotes it will not carry the caveat.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SessionUsageReport {
    pub(crate) reported_input_tokens: Option<i64>,
    pub(crate) reported_output_tokens: Option<i64>,
    /// Derived from something the provider reported, but not itself reported.
    pub(crate) reported_derived_tokens: Option<i64>,
    /// Characters, not tokens. Naming it in tokens would put an estimate in the same unit as a
    /// measurement and invite the two to be summed.
    pub(crate) estimated_characters: Option<i64>,
    pub(crate) response_count: u32,
    /// Responses consumed by the product rather than shown to the user.
    ///
    /// Kept separate because "what did this session cost me" and "what did it show me" are
    /// different questions, and one number cannot answer both.
    pub(crate) internal_purpose_response_count: u32,
    pub(crate) coverage: ReportSectionCoverage,
    /// Always false in this change.
    ///
    /// Monetary cost requires an explicitly versioned provider-pricing observation, and this change
    /// introduces no pricing catalog. A cost computed from an unversioned rate would be a number
    /// nobody could check later against what was actually charged.
    pub(crate) cost_available: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LatencyReport {
    pub(crate) p50_ms: Option<u64>,
    pub(crate) p95_ms: Option<u64>,
    pub(crate) slowest_record_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AgentReportRow {
    pub(crate) agent_id: Option<String>,
    pub(crate) seat_id: Option<String>,
    pub(crate) run_count: u32,
    pub(crate) failed_count: u32,
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ToolReportRow {
    pub(crate) tool_name: String,
    pub(crate) invocations: u32,
    pub(crate) failures: u32,
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CommandReport {
    pub(crate) total: u32,
    pub(crate) failed: u32,
    pub(crate) running: u32,
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ChangeReport {
    pub(crate) changed_files: u32,
    /// Absent in this build.
    ///
    /// Nothing records per-file review progress: a review carries its files, its comments and its
    /// findings, and none of the three says whether a human looked at a file. Zero would claim
    /// every changed file had been reviewed, which is the opposite of what an unrecorded state
    /// means and the more dangerous direction to be wrong in.
    pub(crate) unviewed_files: Option<u32>,
    pub(crate) unresolved_findings: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct VerificationReport {
    pub(crate) passed: u32,
    pub(crate) failed: u32,
    pub(crate) skipped: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FailureReportRow {
    /// A stable code, never the message. A report is quoted, and a message quoted out of a report
    /// is producer text in a document nobody redacted.
    pub(crate) reason_code: String,
    pub(crate) count: u32,
    pub(crate) target: Option<ReportEvidenceLink>,
}

/// Where a report section points in the workspace.
///
/// Identifiers and a tab name. Following it is the reader's job; carrying what it points at would
/// make the report a second copy of the evidence it is summarising.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReportEvidenceLink {
    /// Which workspace tab answers this, as a stable token.
    pub(crate) tab: String,
    pub(crate) session_id: String,
    pub(crate) run_id: Option<String>,
    pub(crate) seat_id: Option<String>,
    pub(crate) trace_id: Option<String>,
    pub(crate) span_id: Option<String>,
    pub(crate) operation_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SessionRunReport {
    pub(crate) scope: super::scope::ReportScope,
    pub(crate) generated_at: String,
    pub(crate) coverage: ReportCoverage,
    pub(crate) overview: ReportOverview,
    pub(crate) usage: SessionUsageReport,
    pub(crate) latency: LatencyReport,
    pub(crate) agents: Vec<AgentReportRow>,
    pub(crate) tools: Vec<ToolReportRow>,
    pub(crate) commands: CommandReport,
    pub(crate) changes: ChangeReport,
    pub(crate) verification: VerificationReport,
    pub(crate) failures: Vec<FailureReportRow>,
    pub(crate) evidence_links: Vec<ReportEvidenceLink>,
}
