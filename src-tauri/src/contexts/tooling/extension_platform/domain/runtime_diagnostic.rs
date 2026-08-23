// The runtime that produces these lands with Task Groups 4 and 5; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What an extension runtime is allowed to say about itself.
//!
//! ## The diagnostic carries no text from the extension. At all.
//!
//! A runtime diagnostic is a **host-owned code**, the subject it is about, and **numeric
//! measurements**. There is no message field, no detail field, no key-value map, no captured
//! stdout or stderr, no environment, and no path.
//!
//! Redaction was the obvious alternative and it is the wrong one. A redactor is a filter over
//! text an extension chose, and every filter is a list of patterns someone thought of; a value
//! that does not match a pattern goes through. Removing the ability to carry text removes the
//! question. An extension that wants to say something says it through a code the host defined,
//! which means someone reviewed the sentence before it could ever be emitted.
//!
//! The cost is real: a diagnostic cannot say *which* file it failed to open. That is deliberate.
//! The path is exactly the thing that must not reach a durable log, and a runtime that needs to
//! tell a user which file it wanted has a UI surface for that, live, in the session that asked.
//!
//! ## This goes to the log, not to a table
//!
//! There is no `runtime_diagnostics` table and this change does not add one. Diagnostics are
//! bounded, high-volume, and interesting for minutes rather than forever; the unified logging
//! service already has rotation, levels, and redaction. A second store would be a second retention
//! policy to get wrong.

use super::{ExtensionId, SnapshotId};

/// Everything an extension runtime may report.
///
/// A closed, host-owned set. Adding one is a deliberate edit here, next to the reason it exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RuntimeDiagnosticCode {
    /// The runtime instantiated the module and it accepted its start call.
    Started,
    /// It shut down when asked, within its budget.
    StoppedCleanly,
    /// It exceeded its wall-clock budget and was stopped.
    TimedOut,
    /// It exceeded its memory budget.
    MemoryExhausted,
    /// It exceeded its fuel budget -- a loop that does not terminate looks like this.
    FuelExhausted,
    /// It trapped. *Why* is the extension's business and is not recorded.
    Trapped,
    /// It asked the host for something its capabilities do not cover.
    CapabilityRefused,
    /// The host refused to start it because a gate is closed.
    GateClosed,
    /// The module could not be instantiated at all.
    InstantiationFailed,
}

impl RuntimeDiagnosticCode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "runtime_started",
            Self::StoppedCleanly => "runtime_stopped_cleanly",
            Self::TimedOut => "runtime_timed_out",
            Self::MemoryExhausted => "runtime_memory_exhausted",
            Self::FuelExhausted => "runtime_fuel_exhausted",
            Self::Trapped => "runtime_trapped",
            Self::CapabilityRefused => "runtime_capability_refused",
            Self::GateClosed => "runtime_gate_closed",
            Self::InstantiationFailed => "runtime_instantiation_failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        ALL_RUNTIME_DIAGNOSTIC_CODES
            .iter()
            .copied()
            .find(|code| code.as_str() == value)
    }

    /// Whether this is something an operator should see by default.
    pub(crate) const fn is_failure(self) -> bool {
        match self {
            Self::Started | Self::StoppedCleanly => false,
            Self::TimedOut
            | Self::MemoryExhausted
            | Self::FuelExhausted
            | Self::Trapped
            | Self::CapabilityRefused
            | Self::GateClosed
            | Self::InstantiationFailed => true,
        }
    }
}

pub(crate) const ALL_RUNTIME_DIAGNOSTIC_CODES: &[RuntimeDiagnosticCode] = &[
    RuntimeDiagnosticCode::Started,
    RuntimeDiagnosticCode::StoppedCleanly,
    RuntimeDiagnosticCode::TimedOut,
    RuntimeDiagnosticCode::MemoryExhausted,
    RuntimeDiagnosticCode::FuelExhausted,
    RuntimeDiagnosticCode::Trapped,
    RuntimeDiagnosticCode::CapabilityRefused,
    RuntimeDiagnosticCode::GateClosed,
    RuntimeDiagnosticCode::InstantiationFailed,
];

/// What a measurement measures.
///
/// Also closed, and also host-owned. A free-form measure name would be a string field by another
/// route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum DiagnosticMeasure {
    DurationMs,
    PeakMemoryBytes,
    FuelUsed,
    HostCallCount,
    /// The budget the run was given, so a reader can tell "close to the limit" from "nowhere near
    /// it" without the host having to decide which one it was.
    BudgetMs,
}

impl DiagnosticMeasure {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::DurationMs => "duration_ms",
            Self::PeakMemoryBytes => "peak_memory_bytes",
            Self::FuelUsed => "fuel_used",
            Self::HostCallCount => "host_call_count",
            Self::BudgetMs => "budget_ms",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        ALL_DIAGNOSTIC_MEASURES
            .iter()
            .copied()
            .find(|measure| measure.as_str() == value)
    }
}

pub(crate) const ALL_DIAGNOSTIC_MEASURES: &[DiagnosticMeasure] = &[
    DiagnosticMeasure::DurationMs,
    DiagnosticMeasure::PeakMemoryBytes,
    DiagnosticMeasure::FuelUsed,
    DiagnosticMeasure::HostCallCount,
    DiagnosticMeasure::BudgetMs,
];

/// The most measurements one diagnostic may carry.
///
/// Bounded because the set is closed and small; a diagnostic that exceeded this would be repeating
/// itself, which is a producer bug rather than something to accommodate.
pub(crate) const MAX_DIAGNOSTIC_MEASUREMENTS: usize = ALL_DIAGNOSTIC_MEASURES.len();

/// Why a diagnostic could not be admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticRejection {
    /// The same measure twice. Two values for one measure has no meaning and picking either is a
    /// guess.
    DuplicateMeasure {
        measure: DiagnosticMeasure,
    },
    TooManyMeasurements {
        count: usize,
        limit: usize,
    },
    /// A negative measurement. Every measure here is a count, a duration, or a size.
    NegativeMeasurement {
        measure: DiagnosticMeasure,
    },
}

impl DiagnosticRejection {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::DuplicateMeasure { .. } => "diagnostic_duplicate_measure",
            Self::TooManyMeasurements { .. } => "diagnostic_too_many_measurements",
            Self::NegativeMeasurement { .. } => "diagnostic_negative_measurement",
        }
    }
}

pub(crate) fn all_diagnostic_rejections() -> Vec<DiagnosticRejection> {
    vec![
        DiagnosticRejection::DuplicateMeasure {
            measure: DiagnosticMeasure::DurationMs,
        },
        DiagnosticRejection::TooManyMeasurements { count: 0, limit: 0 },
        DiagnosticRejection::NegativeMeasurement {
            measure: DiagnosticMeasure::DurationMs,
        },
    ]
}

/// One thing an extension runtime reported, in a form that cannot carry anything it chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SafeExtensionRuntimeDiagnostic {
    code: RuntimeDiagnosticCode,
    extension: ExtensionId,
    /// Which snapshot was running. Absent when the failure is that nothing could be started.
    snapshot: Option<SnapshotId>,
    /// Ordered by measure, so two diagnostics of the same run render identically.
    measurements: Vec<(DiagnosticMeasure, i64)>,
}

impl SafeExtensionRuntimeDiagnostic {
    /// The only constructor.
    pub(crate) fn admit(
        code: RuntimeDiagnosticCode,
        extension: ExtensionId,
        snapshot: Option<SnapshotId>,
        measurements: &[(DiagnosticMeasure, i64)],
    ) -> Result<Self, DiagnosticRejection> {
        if measurements.len() > MAX_DIAGNOSTIC_MEASUREMENTS {
            return Err(DiagnosticRejection::TooManyMeasurements {
                count: measurements.len(),
                limit: MAX_DIAGNOSTIC_MEASUREMENTS,
            });
        }
        let mut ordered: Vec<(DiagnosticMeasure, i64)> = Vec::with_capacity(measurements.len());
        for (measure, value) in measurements {
            if *value < 0 {
                return Err(DiagnosticRejection::NegativeMeasurement { measure: *measure });
            }
            if ordered.iter().any(|(held, _)| held == measure) {
                return Err(DiagnosticRejection::DuplicateMeasure { measure: *measure });
            }
            ordered.push((*measure, *value));
        }
        ordered.sort_by_key(|(measure, _)| *measure);

        Ok(Self {
            code,
            extension,
            snapshot,
            measurements: ordered,
        })
    }

    pub(crate) const fn code(&self) -> RuntimeDiagnosticCode {
        self.code
    }

    pub(crate) const fn extension(&self) -> &ExtensionId {
        &self.extension
    }

    pub(crate) const fn snapshot(&self) -> Option<&SnapshotId> {
        self.snapshot.as_ref()
    }

    pub(crate) fn measurements(&self) -> &[(DiagnosticMeasure, i64)] {
        &self.measurements
    }
}
