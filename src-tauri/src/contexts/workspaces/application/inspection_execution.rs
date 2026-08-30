//! Everything one walk needs to know about the walk it is.
//!
//! Quick Open used to take its generation, its token, its limits, its clock and its ignore rules as
//! five separate arguments, and its operation not at all — the ignore mode was a literal written
//! inside the traversal. That shape has one specific failure: a caller can supply four of the five
//! and be compiled. Content search's clock and Quick Open's clock were passed in a different order
//! from limits, and a test that wanted a shorter deadline had to know which position it was.
//!
//! Bundled, the missing piece is a missing field rather than a wrong argument, and the ignore mode
//! becomes something a caller states rather than something the walk decides for itself. That last
//! one matters beyond tidiness: a traversal that picks its own rules cannot be asked to run under
//! different ones, which is exactly what a test proving "an ignored tree is never searched" needs to
//! do from the other side.
//!
//! Content search and document discovery still take a bare token. They have no cursor, so nothing in
//! them binds a policy identity, and nothing in them reads a generation — giving them a context
//! would be a change with no behaviour behind it.

use super::ignore_policy::WorkspaceIgnorePolicy;
use super::inspection_budget::{
    MonotonicClockPort, WorkspaceInspectionBudget, WorkspaceInspectionBudgetLimits,
};
use super::search_cancellation::{SearchCancellationToken, SearchGeneration};
use std::sync::Arc;

/// Which inspection is running.
///
/// Carried so a walk can be identified without inferring it from the shape of its limits. The
/// variants are the operations that share the budget pipeline; anything that does not consume a
/// budget does not appear here rather than being given a variant it never uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceInspectionOperation {
    PathSearch,
    ContentSearch,
    DocumentDiscovery,
    DirectoryPage,
}

/// One walk's identity, bounds, and rules.
///
/// Cloneable so the async owner can keep one and hand another to a blocking worker. What it does
/// *not* carry is the registration guard: that stays with the caller, so no worker can remove a
/// registration — its own or anybody else's.
#[derive(Clone)]
pub(crate) struct WorkspaceInspectionExecution {
    operation: WorkspaceInspectionOperation,
    generation: SearchGeneration,
    cancellation: SearchCancellationToken,
    limits: WorkspaceInspectionBudgetLimits,
    clock: Arc<dyn MonotonicClockPort>,
    ignore: WorkspaceIgnorePolicy,
}

impl WorkspaceInspectionExecution {
    /// The context a Quick Open runs under, with the defaults for that operation.
    pub(crate) fn path_search(
        generation: SearchGeneration,
        cancellation: SearchCancellationToken,
        clock: Arc<dyn MonotonicClockPort>,
    ) -> Self {
        Self {
            operation: WorkspaceInspectionOperation::PathSearch,
            generation,
            cancellation,
            limits: WorkspaceInspectionBudgetLimits::path_search(),
            clock,
            ignore: WorkspaceIgnorePolicy::recursive_discovery(),
        }
    }

    /// Narrower limits, for a test that needs to reach one.
    ///
    /// A budget dimension is only worth having if a test can drive a traversal into it, and the
    /// alternative is a fixture large enough to exhaust a realistic ceiling.
    pub(crate) fn with_limits(mut self, limits: WorkspaceInspectionBudgetLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Different ignore rules, for a caller that has a reason to state them.
    ///
    /// Exists so "an ignored tree is never searched" can be tested from both sides: a walk that
    /// picks its own rules can only ever be observed obeying them.
    pub(crate) fn with_ignore(mut self, ignore: WorkspaceIgnorePolicy) -> Self {
        self.ignore = ignore;
        self
    }

    /// Relabels the context, for a test that needs a mismatch to exist.
    ///
    /// Test-only because production has no reason to build one: each operation has its own
    /// constructor, and a walk that could be relabelled after the fact would make the guard in
    /// `search_session_paths` something a caller can talk its way past.
    #[cfg(test)]
    pub(crate) fn with_operation(mut self, operation: WorkspaceInspectionOperation) -> Self {
        self.operation = operation;
        self
    }

    pub(crate) fn operation(&self) -> WorkspaceInspectionOperation {
        self.operation
    }

    pub(crate) fn generation(&self) -> SearchGeneration {
        self.generation
    }

    pub(crate) fn cancellation(&self) -> &SearchCancellationToken {
        &self.cancellation
    }

    pub(crate) fn ignore(&self) -> WorkspaceIgnorePolicy {
        self.ignore
    }

    pub(crate) fn limits(&self) -> WorkspaceInspectionBudgetLimits {
        self.limits
    }

    /// A fresh tracker for this walk.
    ///
    /// Built here rather than held, because a budget is spent and a context is not: two walks under
    /// one context — a retry, or a page after a page — must not inherit the first one's spend.
    pub(crate) fn budget(&self) -> WorkspaceInspectionBudget {
        WorkspaceInspectionBudget::new(
            self.limits,
            Arc::clone(&self.clock),
            self.cancellation.clone(),
        )
    }

    /// The same context with the page bounds applied.
    ///
    /// The page, plus the one entry that proves another page exists. Stated as a budget rather than
    /// only as a selection capacity, so the bound is something a test reads off the answer instead of
    /// something it takes the implementation's word for.
    pub(crate) fn bounded_to_page(mut self, limit: usize) -> Self {
        let capacity = limit.saturating_add(1) as u64;
        self.limits.max_retained_candidates = self.limits.max_retained_candidates.min(capacity);
        self.limits.max_results = self.limits.max_results.min(limit as u64);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::super::search_cancellation::{
        SearchCancellationCause, SearchRegistration, WorkspaceSearchCancellation,
    };
    use super::*;
    use crate::contexts::workspaces::application::ManualClock;

    /// Built from a real registration rather than a hand-made generation.
    ///
    /// A test-only constructor for `SearchGeneration` would be a second way to mint one, and the
    /// invariant that makes compare-remove safe — never zero, allocated under the registry's lock —
    /// is exactly the kind that a convenience constructor quietly stops holding.
    fn execution() -> (WorkspaceInspectionExecution, SearchRegistration) {
        let registry = Arc::new(WorkspaceSearchCancellation::default());
        let registration = registry.begin("quick-open");
        let execution = WorkspaceInspectionExecution::path_search(
            registration.generation(),
            registration.token(),
            Arc::new(ManualClock::default()),
        );
        (execution, registration)
    }

    #[test]
    fn a_path_search_context_carries_its_operation_and_generation() {
        let (execution, registration) = execution();

        assert_eq!(
            execution.operation(),
            WorkspaceInspectionOperation::PathSearch
        );
        assert_eq!(execution.generation(), registration.generation());
        assert!(execution.ignore().is_recursive_discovery());
    }

    /// Two walks under one context start with the full budget each.
    ///
    /// A context that held its tracker would let the second page of a listing inherit the first
    /// page's spend, and the listing would report itself incomplete for work it did not do.
    #[test]
    fn each_walk_gets_its_own_spend() {
        let (execution, _registration) = execution();

        let mut first = execution.budget();
        assert!(first.try_visit_directory());
        let second = execution.budget();

        assert_eq!(second.snapshot().directories_visited, 0);
        assert_eq!(first.snapshot().directories_visited, 1);
    }

    /// A smaller page asks for less work, not merely a shorter list.
    #[test]
    fn a_page_bound_narrows_the_result_and_candidate_budgets() {
        let (execution, _registration) = execution();
        let execution = execution.bounded_to_page(3);

        assert_eq!(execution.limits().max_results, 3);
        assert_eq!(execution.limits().max_retained_candidates, 4);
    }

    /// Never widens. A caller asking for a thousand results does not get a bigger budget than the
    /// operation's own ceiling — the page bound is a floor on restriction, not a setting.
    #[test]
    fn a_page_bound_never_raises_a_ceiling() {
        let ceiling = WorkspaceInspectionBudgetLimits::path_search();
        let (execution, _registration) = execution();

        let execution = execution.bounded_to_page(usize::MAX - 1);

        assert_eq!(execution.limits().max_results, ceiling.max_results);
        assert_eq!(
            execution.limits().max_retained_candidates,
            ceiling.max_retained_candidates
        );
    }

    /// The context observes a cancel issued after it was built.
    ///
    /// A clone that had copied the flag rather than shared it would compile, pass every other test
    /// here, and leave every walk uninterruptible.
    #[test]
    fn the_cancellation_token_is_the_one_it_was_given() {
        let (execution, registration) = execution();

        registration
            .token()
            .signal(SearchCancellationCause::Cancelled);

        assert!(execution.cancellation().is_cancelled());
        assert!(execution.clone().cancellation().is_cancelled());
    }
}
