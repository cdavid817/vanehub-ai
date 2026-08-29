//! Keeping the best few of many without holding the many.
//!
//! Every listing and ranked search in this context has the same shape: without an index it must
//! *visit* every eligible entry to know which ones belong on the page, but it does not have to
//! *keep* them. A directory with two hundred thousand files still returns fifty, and the difference
//! between building a vector of two hundred thousand entries and holding fifty-one is the
//! difference between a panel that opens and one that does not.
//!
//! A max-heap under an ascending order, so the element on top is the worst one kept — exactly the
//! one a better arrival should displace. Everything worse than that is dropped where it is found.
//!
//! Retention is charged to the inspection budget, and eviction credits it back. That is what makes
//! the bound something a test reads off the answer rather than something it takes this file's word
//! for.

use crate::contexts::workspaces::application::WorkspaceInspectionBudget;
use std::collections::BinaryHeap;

pub(super) struct BoundedSelection<T: Ord> {
    capacity: usize,
    heap: BinaryHeap<T>,
}

impl<T: Ord> BoundedSelection<T> {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            heap: BinaryHeap::new(),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.heap.len()
    }

    /// Offers an item. Returns whether the walk that produced it may continue.
    ///
    /// `false` means the candidate budget refused the retention, which is a stop rather than a
    /// rejection: a selection that cannot hold what it needs cannot answer the question it was
    /// asked.
    pub(super) fn offer(&mut self, item: T, budget: &mut WorkspaceInspectionBudget) -> bool {
        if self.heap.len() < self.capacity {
            if !budget.try_retain_candidate() {
                return false;
            }
            self.heap.push(item);
            return true;
        }
        if self.heap.peek().is_none_or(|worst| &item >= worst) {
            return true;
        }
        // Credited before it is charged, so a selection that is merely *replacing* an element never
        // reads as one that grew. Charging first would stop a full heap at its next improvement.
        self.heap.pop();
        budget.release_candidate();
        if !budget.try_retain_candidate() {
            return false;
        }
        self.heap.push(item);
        true
    }

    pub(super) fn into_sorted(self) -> Vec<T> {
        let mut items = self.heap.into_vec();
        items.sort();
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::workspaces::application::{
        SearchCancellationToken, SystemMonotonicClock, WorkspaceInspectionBudgetLimits,
    };
    use std::sync::Arc;
    use std::time::Duration;

    fn budget(max_retained: u64) -> WorkspaceInspectionBudget {
        WorkspaceInspectionBudget::new(
            WorkspaceInspectionBudgetLimits {
                max_directories_visited: 1_000,
                max_entries_visited: 1_000_000,
                max_files_opened: 0,
                max_bytes_read: 0,
                max_metadata_operations: 1_000_000,
                max_retained_candidates: max_retained,
                max_results: 1_000,
                max_depth: 10,
                deadline: Duration::from_secs(600),
            },
            Arc::new(SystemMonotonicClock::default()),
            SearchCancellationToken::new(),
        )
    }

    #[test]
    fn a_stream_far_larger_than_the_capacity_still_yields_the_best_few() {
        let mut budget = budget(4);
        let mut selection = BoundedSelection::new(3);

        // Offered worst-first, so a selection that simply kept its first arrivals would fail.
        for value in (0..1_000).rev() {
            assert!(selection.offer(value, &mut budget));
        }

        assert_eq!(selection.len(), 3);
        assert_eq!(selection.into_sorted(), vec![0, 1, 2]);
        assert_eq!(budget.snapshot().candidates_retained, 3);
    }

    #[test]
    fn an_item_worse_than_everything_kept_is_dropped_where_it_is_found() {
        let mut budget = budget(4);
        let mut selection = BoundedSelection::new(2);

        assert!(selection.offer(1, &mut budget));
        assert!(selection.offer(2, &mut budget));
        assert!(selection.offer(99, &mut budget));

        // Three offers, two retentions. The third never occupied memory and never occupied budget.
        assert_eq!(budget.snapshot().candidates_retained, 2);
        assert_eq!(selection.into_sorted(), vec![1, 2]);
    }

    #[test]
    fn replacing_an_element_is_not_growth() {
        // Capacity two under a budget of exactly two: every improvement after the second arrival is
        // an eviction and a retention. Charging without crediting would stop the selection at its
        // first improvement.
        let mut budget = budget(2);
        let mut selection = BoundedSelection::new(2);

        for value in [5, 6, 4, 3, 2, 1] {
            assert!(selection.offer(value, &mut budget), "{value}");
        }

        assert_eq!(selection.into_sorted(), vec![1, 2]);
        assert_eq!(budget.snapshot().candidates_retained, 2);
    }

    #[test]
    fn a_candidate_budget_below_the_capacity_stops_the_walk() {
        let mut budget = budget(1);
        let mut selection = BoundedSelection::new(4);

        assert!(selection.offer(1, &mut budget));
        assert!(!selection.offer(2, &mut budget));
        assert_eq!(
            budget.stop_reason().map(|reason| reason.code()),
            Some("candidate_budget_exhausted")
        );
    }
}
