//! Descriptive stages a CLI operation passes through.
//!
//! A phase tells the user what is happening right now. `OperationStatus` remains authoritative for
//! whether the work finished -- nothing may conclude success from a phase, and `completed` here is
//! a label, not a terminal state.
//!
//! These strings are mirrored by `CLI_OPERATION_PHASES` in `src/types/cli-environment.ts`, and both
//! sides assert the full list, so a rename fails one suite or the other.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliOperationPhase {
    Preflight,
    ResolvingSource,
    QueryingCatalog,
    Planning,
    Downloading,
    Mutating,
    VerifyingExecutable,
    RefreshingEnvironment,
    RunningDoctor,
    Completed,
}

impl CliOperationPhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Preflight => "preflight",
            Self::ResolvingSource => "resolving-source",
            Self::QueryingCatalog => "querying-catalog",
            Self::Planning => "planning",
            Self::Downloading => "downloading",
            Self::Mutating => "mutating",
            Self::VerifyingExecutable => "verifying-executable",
            Self::RefreshingEnvironment => "refreshing-environment",
            Self::RunningDoctor => "running-doctor",
            Self::Completed => "completed",
        }
    }

    /// Whether cancellation can still be honoured at this stage.
    ///
    /// `Mutating` is deliberately absent: once a package manager is writing, interrupting it does
    /// not undo what it already did, and offering cancel there would imply a rollback that cannot
    /// happen. Downloading is cancellable because nothing has been applied yet.
    pub(crate) fn is_cancellable(self) -> bool {
        matches!(
            self,
            Self::Preflight
                | Self::ResolvingSource
                | Self::QueryingCatalog
                | Self::Planning
                | Self::Downloading
        )
    }

    /// Every phase, in the order an operation would normally reach them. Used by the drift test.
    pub(crate) const ALL: [Self; 10] = [
        Self::Preflight,
        Self::ResolvingSource,
        Self::QueryingCatalog,
        Self::Planning,
        Self::Downloading,
        Self::Mutating,
        Self::VerifyingExecutable,
        Self::RefreshingEnvironment,
        Self::RunningDoctor,
        Self::Completed,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phase_list_matches_the_typescript_contract_exactly() {
        // Mirrors `CLI_OPERATION_PHASES` in src/types/cli-environment.ts, which asserts the same
        // ten values in the same order.
        assert_eq!(
            CliOperationPhase::ALL.map(CliOperationPhase::as_str),
            [
                "preflight",
                "resolving-source",
                "querying-catalog",
                "planning",
                "downloading",
                "mutating",
                "verifying-executable",
                "refreshing-environment",
                "running-doctor",
                "completed",
            ]
        );
    }

    #[test]
    fn no_phase_collides_with_an_operation_status() {
        // Status is authoritative; a phase that reused one of its words would invite code to
        // switch on the wrong field.
        for phase in CliOperationPhase::ALL {
            assert!(
                !["queued", "running", "succeeded", "failed", "cancelled"]
                    .contains(&phase.as_str()),
                "{} collides with a lifecycle status",
                phase.as_str()
            );
        }
    }

    #[test]
    fn cancellation_stops_being_offered_once_the_machine_is_being_written() {
        assert!(CliOperationPhase::Downloading.is_cancellable());
        assert!(CliOperationPhase::Planning.is_cancellable());
        assert!(CliOperationPhase::Preflight.is_cancellable());
        assert!(CliOperationPhase::ResolvingSource.is_cancellable());
        assert!(CliOperationPhase::QueryingCatalog.is_cancellable());

        // Interrupting a package manager mid-write does not undo what it already wrote, so cancel
        // must not be offered as though it would.
        assert!(!CliOperationPhase::Mutating.is_cancellable());
        assert!(!CliOperationPhase::VerifyingExecutable.is_cancellable());
        assert!(!CliOperationPhase::RefreshingEnvironment.is_cancellable());
        assert!(!CliOperationPhase::RunningDoctor.is_cancellable());
        assert!(!CliOperationPhase::Completed.is_cancellable());
    }

    #[test]
    fn every_phase_has_a_distinct_wire_string() {
        let mut seen = CliOperationPhase::ALL
            .map(CliOperationPhase::as_str)
            .to_vec();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), total);
    }
}
