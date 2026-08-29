//! What an inspection is allowed to spend, counted as work rather than as results.
//!
//! A result cap bounds the answer, not the effort. A walk can visit a million entries, canonicalize
//! every one of them, open ten thousand files and read a gigabyte while producing three matches —
//! and every one of those limits is the one that was missing when a monorepo made the panel
//! unusable. What follows counts the work: directories opened, entries looked at (including the
//! ignored, unreadable and non-matching ones), files opened, bytes read, metadata calls, candidates
//! retained, results emitted, depth descended, and elapsed monotone time.
//!
//! Everything is consumed *before* the operation it pays for. Charging afterwards means the limit
//! is always exceeded by one, and "one" is a directory enumeration on a network mount.
//!
//! The tracker is deliberately not atomic. One traversal runs on one blocking thread, and shared
//! atomics would buy contention to solve a problem this shape of work does not have. Where work is
//! genuinely parallel, a tracker per worker plus an aggregated snapshot is the answer, not a hotter
//! cache line.

use super::search_cancellation::{SearchCancellationCause, SearchCancellationToken};
use std::sync::Arc;
use std::time::Duration;

/// Why an inspection stopped, or why its coverage is not complete.
///
/// One vocabulary for every provider. Local and remote inspect different machines by different
/// means, and a reader who gets `scan_limit` from one and `too many files` from the other has been
/// told the same thing twice in two languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceInspectionReason {
    /// An explicit cancel arrived for this search.
    Cancelled,
    /// A newer generation of the same search id replaced this one.
    Superseded,
    /// The future that owned the request was dropped or aborted.
    OwnerDropped,
    /// Admission capacity was exhausted, so no work was started.
    InspectionBusy,
    DirectoryBudgetExhausted,
    EntryBudgetExhausted,
    FileBudgetExhausted,
    ByteBudgetExhausted,
    MetadataBudgetExhausted,
    CandidateBudgetExhausted,
    ResultBudgetExhausted,
    DepthBudgetExhausted,
    DeadlineExceeded,
    /// Something eligible could not be read, so it was skipped.
    UnreadableEntries,
    /// The workspace could not be reached at all.
    ProviderUnavailable,
    /// The provider was reached and failed.
    ProviderFailed,
    /// The continuation token is malformed or belongs somewhere else.
    InvalidCursor,
    /// The continuation token was issued for a state the directory has left.
    StaleCursor,
}

impl WorkspaceInspectionReason {
    /// Which reason a signalled cancellation token carries.
    ///
    /// The mapping lives here rather than on the cause, so every provider that has to explain a
    /// stopped search reaches the same three answers. A reader who pressed Escape, a reader who
    /// typed another character, and a view that closed are three different events.
    pub(crate) fn from_cancellation(cause: SearchCancellationCause) -> Self {
        match cause {
            SearchCancellationCause::Cancelled => Self::Cancelled,
            SearchCancellationCause::Superseded => Self::Superseded,
            SearchCancellationCause::OwnerDropped => Self::OwnerDropped,
        }
    }

    /// The stable token the frontend translates.
    ///
    /// A code rather than a sentence: the wording belongs to the frontend, and a message assembled
    /// here would arrive untranslated in whatever language this build's sources are written in.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Superseded => "superseded",
            Self::OwnerDropped => "owner_dropped",
            Self::InspectionBusy => "inspection_busy",
            Self::DirectoryBudgetExhausted => "directory_budget_exhausted",
            Self::EntryBudgetExhausted => "entry_budget_exhausted",
            Self::FileBudgetExhausted => "file_budget_exhausted",
            Self::ByteBudgetExhausted => "byte_budget_exhausted",
            Self::MetadataBudgetExhausted => "metadata_budget_exhausted",
            Self::CandidateBudgetExhausted => "candidate_budget_exhausted",
            Self::ResultBudgetExhausted => "result_budget_exhausted",
            Self::DepthBudgetExhausted => "depth_budget_exhausted",
            Self::DeadlineExceeded => "deadline_exceeded",
            Self::UnreadableEntries => "unreadable_entries",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::ProviderFailed => "provider_failed",
            Self::InvalidCursor => "invalid_cursor",
            Self::StaleCursor => "stale_cursor",
        }
    }

    /// Whether a result carrying this reason is worth showing at all.
    ///
    /// The difference between "we looked and stopped early" and "we could not look". An empty
    /// result of the first kind still had a workspace under it; one of the second kind did not, and
    /// presenting them the same way is how somebody concludes a string is not in their files.
    pub(crate) fn is_unavailable(self) -> bool {
        matches!(
            self,
            Self::InspectionBusy
                | Self::ProviderUnavailable
                | Self::ProviderFailed
                | Self::InvalidCursor
                | Self::StaleCursor
        )
    }
}

/// Time that only ever moves forward.
///
/// A wall clock is the wrong instrument for a deadline: it is adjusted by NTP, by a user changing
/// the timezone, and by a laptop waking up, and each of those can make a deadline that has not
/// arrived look like one that passed an hour ago. A port rather than a bare `Instant` so a test can
/// advance an hour without waiting one.
pub(crate) trait MonotonicClockPort: Send + Sync {
    /// How long since this clock's fixed origin.
    fn elapsed(&self) -> Duration;
}

/// The process clock.
pub(crate) struct SystemMonotonicClock {
    origin: std::time::Instant,
}

impl Default for SystemMonotonicClock {
    fn default() -> Self {
        Self {
            origin: std::time::Instant::now(),
        }
    }
}

impl MonotonicClockPort for SystemMonotonicClock {
    fn elapsed(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// What one inspection may spend.
///
/// Every field is finite. A limit expressed as "unbounded for now" is one nobody notices is missing
/// until the machine it runs on has a network mount attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkspaceInspectionBudgetLimits {
    pub(crate) max_directories_visited: u64,
    pub(crate) max_entries_visited: u64,
    pub(crate) max_files_opened: u64,
    pub(crate) max_bytes_read: u64,
    pub(crate) max_metadata_operations: u64,
    pub(crate) max_retained_candidates: u64,
    pub(crate) max_results: u64,
    pub(crate) max_depth: u32,
    pub(crate) deadline: Duration,
}

impl WorkspaceInspectionBudgetLimits {
    /// Reading file contents across a workspace.
    ///
    /// Every value either restates a limit that already existed or tightens one. 20,000 entries and
    /// depth 10 are the shared walk's existing bounds. 2,000 files is what the old implementation
    /// effectively opened, because it read from a candidate vector capped at 2,000. 200 results is
    /// its existing match cap.
    ///
    /// The byte and metadata ceilings are new, and both are tighter than what was there. 2,000 files
    /// at the existing 1 MiB per-file bound is 2 GiB of reads with nothing counting them; 512 MiB is
    /// a quarter of that and still more than any interactive search needs. The 20-second deadline
    /// replaces no deadline at all.
    pub(crate) fn content_search() -> Self {
        Self {
            max_directories_visited: 20_000,
            max_entries_visited: 20_000,
            max_files_opened: 2_000,
            max_bytes_read: 512 * 1024 * 1024,
            max_metadata_operations: 60_000,
            // The breadth-first frontier, not the results. A tree wide enough to queue more than
            // this many unvisited directories at once stops and says so, rather than growing a
            // queue nothing bounds.
            max_retained_candidates: 4_096,
            max_results: 200,
            max_depth: 10,
            deadline: Duration::from_secs(20),
        }
    }

    /// Quick Open, which reads names rather than contents.
    ///
    /// No file is opened, so the file and byte budgets are zero: a path search that opened a file
    /// has done something it was not asked to, and zero is the only statement of that which a test
    /// can catch.
    pub(crate) fn path_search() -> Self {
        Self {
            max_directories_visited: 20_000,
            max_entries_visited: 20_000,
            max_files_opened: 0,
            max_bytes_read: 0,
            max_metadata_operations: 60_000,
            // The bounded selection holds one ranking window, not the 2,000-entry vector the old
            // full sort retained.
            max_retained_candidates: 256,
            max_results: 50,
            max_depth: 10,
            deadline: Duration::from_secs(10),
        }
    }

    /// One page of one immediate directory. No descent, so the depth budget is that directory.
    ///
    /// The entry ceiling is deliberately large: this is the operation a reader triggers by clicking
    /// a folder, and a directory of two hundred thousand entries is one they should still be able
    /// to open a page of. What changed is that the page is *selected* rather than sorted, so a
    /// number that large costs a scan rather than a vector.
    pub(crate) fn directory_listing(page_size: usize) -> Self {
        Self {
            max_directories_visited: 1,
            max_entries_visited: 200_000,
            max_files_opened: 0,
            max_bytes_read: 0,
            max_metadata_operations: 400_000,
            // The page, plus the one entry that proves another page exists.
            max_retained_candidates: page_size as u64 + 1,
            max_results: page_size as u64,
            max_depth: 0,
            deadline: Duration::from_secs(10),
        }
    }
}

/// What one inspection actually spent.
///
/// Counts and limits, and nothing else. No paths, no names, no matched text: a budget summary
/// travels to the frontend and into logs, and a structural number is the only part of it that is
/// safe everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct WorkspaceInspectionBudgetSnapshot {
    pub(crate) directories_visited: u64,
    pub(crate) entries_visited: u64,
    pub(crate) files_opened: u64,
    pub(crate) bytes_read: u64,
    pub(crate) metadata_operations: u64,
    pub(crate) candidates_retained: u64,
    pub(crate) results_emitted: u64,
    pub(crate) max_depth_reached: u32,
    /// Entries that were eligible and could not be read.
    pub(crate) unreadable_entries: u64,
}

/// The counters, the limits, the clock and the token, in one place.
///
/// One object rather than a handful of loose numbers passed alongside each other, because the
/// off-by-one at each limit is the same off-by-one and writing it eight times is writing it wrong
/// at least once.
pub(crate) struct WorkspaceInspectionBudget {
    limits: WorkspaceInspectionBudgetLimits,
    clock: Arc<dyn MonotonicClockPort>,
    token: SearchCancellationToken,
    started_at: Duration,
    counts: WorkspaceInspectionBudgetSnapshot,
    stop: Option<WorkspaceInspectionReason>,
    /// Why part of the work was skipped while the rest carried on.
    ///
    /// Separate from `stop` because the two answer different questions. A stop ends the traversal;
    /// an omission is one subtree or one file the traversal declined and then continued past. Both
    /// make coverage partial, and collapsing them would either abandon a walk over one unreadable
    /// directory or claim a complete answer over a tree it never entered.
    omission: Option<WorkspaceInspectionReason>,
    /// How many checkpoints have been taken. The cancellation gate is asserted in checkpoints, not
    /// in milliseconds: a shared CI runner cannot promise the second and can always count the first.
    checkpoints: u64,
}

impl WorkspaceInspectionBudget {
    pub(crate) fn new(
        limits: WorkspaceInspectionBudgetLimits,
        clock: Arc<dyn MonotonicClockPort>,
        token: SearchCancellationToken,
    ) -> Self {
        let started_at = clock.elapsed();
        Self {
            limits,
            clock,
            token,
            started_at,
            counts: WorkspaceInspectionBudgetSnapshot::default(),
            stop: None,
            omission: None,
            checkpoints: 0,
        }
    }

    pub(crate) fn snapshot(&self) -> WorkspaceInspectionBudgetSnapshot {
        self.counts
    }

    pub(crate) fn stop_reason(&self) -> Option<WorkspaceInspectionReason> {
        self.stop
    }

    /// The reason coverage is not complete: what stopped the walk, or failing that, what it
    /// skipped along the way.
    ///
    /// A stop outranks an omission because it is the thing that ended the answer. A search that
    /// hit its deadline having also skipped a binary file is a search that ran out of time.
    pub(crate) fn incomplete_reason(&self) -> Option<WorkspaceInspectionReason> {
        self.stop.or(self.omission)
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.stop.is_some()
    }

    /// How many cancellation/deadline checkpoints have been taken.
    ///
    /// The cancellation gate is asserted in checkpoints rather than milliseconds: a shared runner
    /// cannot promise the second and can always count the first.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the checkpoint bound is a test assertion; production only calls checkpoints"
        )
    )]
    pub(crate) fn checkpoints(&self) -> u64 {
        self.checkpoints
    }

    /// Records the first reason the work stopped and keeps it.
    ///
    /// First rather than last: a cancellation observed at an entry boundary is followed by every
    /// later limit also being "reached", and reporting the last one would tell a reader their
    /// search hit a byte ceiling when in fact they cancelled it.
    pub(crate) fn stop(&mut self, reason: WorkspaceInspectionReason) {
        if self.stop.is_none() {
            self.stop = Some(reason);
        }
    }

    /// Something was skipped and the walk continued.
    ///
    /// The first omission is the one reported, matching how a stop reason is kept: later ones are
    /// consequences of the same tree rather than independent facts about it.
    pub(crate) fn note_omission(&mut self, reason: WorkspaceInspectionReason) {
        if reason == WorkspaceInspectionReason::UnreadableEntries {
            self.counts.unreadable_entries = self.counts.unreadable_entries.saturating_add(1);
        }
        if self.omission.is_none() {
            self.omission = Some(reason);
        }
    }

    /// Cancellation and deadline only, for the boundaries that consume nothing.
    ///
    /// Returns whether the work may continue. Called before a directory, before each entry batch,
    /// before each file open, before each read chunk, before appending a result and before
    /// serializing the answer.
    pub(crate) fn checkpoint(&mut self) -> bool {
        self.checkpoints = self.checkpoints.saturating_add(1);
        if self.stop.is_some() {
            return false;
        }
        if let Some(cause) = self.token.cause() {
            self.stop(WorkspaceInspectionReason::from_cancellation(cause));
            return false;
        }
        if self.clock.elapsed().saturating_sub(self.started_at) >= self.limits.deadline {
            self.stop(WorkspaceInspectionReason::DeadlineExceeded);
            return false;
        }
        true
    }

    /// One consume path for every dimension.
    ///
    /// Private so no provider writes its own comparison. The three lines below are the whole
    /// off-by-one contract: charge before the operation, refuse the one that would exceed, and
    /// record why.
    fn consume(
        &mut self,
        amount: u64,
        limit: u64,
        current: u64,
        reason: WorkspaceInspectionReason,
    ) -> Option<u64> {
        if !self.checkpoint() {
            return None;
        }
        let next = current.saturating_add(amount);
        if next > limit {
            self.stop(reason);
            return None;
        }
        Some(next)
    }

    pub(crate) fn try_visit_directory(&mut self) -> bool {
        match self.consume(
            1,
            self.limits.max_directories_visited,
            self.counts.directories_visited,
            WorkspaceInspectionReason::DirectoryBudgetExhausted,
        ) {
            Some(next) => {
                self.counts.directories_visited = next;
                true
            }
            None => false,
        }
    }

    /// One directory entry looked at, whatever happens to it next.
    ///
    /// Charged for hidden, ignored, unreadable and non-matching entries too. Those are exactly the
    /// entries a result cap does not see, and they are most of the work on the trees where this
    /// matters.
    pub(crate) fn try_visit_entry(&mut self) -> bool {
        match self.consume(
            1,
            self.limits.max_entries_visited,
            self.counts.entries_visited,
            WorkspaceInspectionReason::EntryBudgetExhausted,
        ) {
            Some(next) => {
                self.counts.entries_visited = next;
                true
            }
            None => false,
        }
    }

    /// A stat, a canonicalization, a symlink resolution — anything that asks the filesystem about
    /// an entry rather than reading it.
    pub(crate) fn try_metadata(&mut self) -> bool {
        match self.consume(
            1,
            self.limits.max_metadata_operations,
            self.counts.metadata_operations,
            WorkspaceInspectionReason::MetadataBudgetExhausted,
        ) {
            Some(next) => {
                self.counts.metadata_operations = next;
                true
            }
            None => false,
        }
    }

    pub(crate) fn try_open_file(&mut self) -> bool {
        match self.consume(
            1,
            self.limits.max_files_opened,
            self.counts.files_opened,
            WorkspaceInspectionReason::FileBudgetExhausted,
        ) {
            Some(next) => {
                self.counts.files_opened = next;
                true
            }
            None => false,
        }
    }

    /// Charged for the chunk about to be requested, not for the bytes that came back.
    ///
    /// A read that returns less than it asked for still cost the attempt, and charging the returned
    /// length would let a file that grows under the reader run past the aggregate ceiling one short
    /// read at a time.
    pub(crate) fn try_read_bytes(&mut self, bytes: u64) -> bool {
        match self.consume(
            bytes,
            self.limits.max_bytes_read,
            self.counts.bytes_read,
            WorkspaceInspectionReason::ByteBudgetExhausted,
        ) {
            Some(next) => {
                self.counts.bytes_read = next;
                true
            }
            None => false,
        }
    }

    /// Retaining one candidate in a bounded selection structure.
    ///
    /// Separate from a result because a candidate may still be discarded: the point of the budget
    /// is the memory held while deciding, which a result count never sees.
    pub(crate) fn try_retain_candidate(&mut self) -> bool {
        match self.consume(
            1,
            self.limits.max_retained_candidates,
            self.counts.candidates_retained,
            WorkspaceInspectionReason::CandidateBudgetExhausted,
        ) {
            Some(next) => {
                self.counts.candidates_retained = next;
                true
            }
            None => false,
        }
    }

    /// Releases one retained candidate, for a selection structure that evicts as it goes.
    pub(crate) fn release_candidate(&mut self) {
        self.counts.candidates_retained = self.counts.candidates_retained.saturating_sub(1);
    }

    pub(crate) fn try_emit_result(&mut self) -> bool {
        match self.consume(
            1,
            self.limits.max_results,
            self.counts.results_emitted,
            WorkspaceInspectionReason::ResultBudgetExhausted,
        ) {
            Some(next) => {
                self.counts.results_emitted = next;
                true
            }
            None => false,
        }
    }

    /// Whether the walk may descend to `depth`.
    ///
    /// Depth is a level rather than a running total, so it is compared instead of accumulated. The
    /// deepest level actually reached is recorded, which is what a coverage summary reports.
    pub(crate) fn try_descend(&mut self, depth: u32) -> bool {
        if !self.checkpoint() {
            return false;
        }
        if depth > self.limits.max_depth {
            self.stop(WorkspaceInspectionReason::DepthBudgetExhausted);
            return false;
        }
        self.counts.max_depth_reached = self.counts.max_depth_reached.max(depth);
        true
    }
}

#[cfg(test)]
pub(crate) use test_clock::ManualClock;

#[cfg(test)]
mod test_clock {
    use super::{Duration, MonotonicClockPort};
    use std::sync::Mutex;

    /// A clock that only moves when a test moves it, or by a fixed step per reading.
    ///
    /// The alternative is sleeping, and a test that sleeps to prove a deadline proves it on the
    /// machine it was written on and flakes on every busier one.
    ///
    /// The per-reading step is what makes a deadline reachable *inside* a traversal. A test cannot
    /// interleave a clock advance with a walk that has already started, and advancing before it
    /// starts only moves the origin the budget is measured from. A clock that ticks once per
    /// checkpoint turns "how long did this take" into "how many checkpoints did it take", which is
    /// the same determinism the cancellation gate uses.
    #[derive(Default)]
    pub(crate) struct ManualClock {
        elapsed: Mutex<Duration>,
        step: Duration,
    }

    impl ManualClock {
        /// A clock that advances `step` every time it is read.
        pub(crate) fn ticking(step: Duration) -> Self {
            Self {
                elapsed: Mutex::new(Duration::ZERO),
                step,
            }
        }

        pub(crate) fn advance(&self, by: Duration) {
            let mut elapsed = self
                .elapsed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *elapsed = elapsed.saturating_add(by);
        }
    }

    impl MonotonicClockPort for ManualClock {
        /// Returns the reading, then steps.
        ///
        /// Before rather than after, so a budget's first reading — the origin it measures from — is
        /// zero on a ticking clock as well as on a still one.
        fn elapsed(&self) -> Duration {
            let mut elapsed = self
                .elapsed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let reading = *elapsed;
            *elapsed = elapsed.saturating_add(self.step);
            reading
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> WorkspaceInspectionBudgetLimits {
        WorkspaceInspectionBudgetLimits {
            max_directories_visited: 2,
            max_entries_visited: 3,
            max_files_opened: 2,
            max_bytes_read: 10,
            max_metadata_operations: 2,
            max_retained_candidates: 2,
            max_results: 1,
            max_depth: 2,
            deadline: Duration::from_secs(5),
        }
    }

    fn budget() -> (
        WorkspaceInspectionBudget,
        SearchCancellationToken,
        Arc<ManualClock>,
    ) {
        let token = SearchCancellationToken::new();
        let clock = Arc::new(ManualClock::default());
        let budget = WorkspaceInspectionBudget::new(limits(), clock.clone(), token.clone());
        (budget, token, clock)
    }

    #[test]
    fn every_dimension_admits_exactly_its_limit_and_refuses_the_next() {
        let (mut budget, _token, _clock) = budget();

        assert!(budget.try_visit_directory());
        assert!(budget.try_visit_directory());
        // Charged before the operation, so the call that would exceed is the one refused — not the
        // one after it, which is where an off-by-one puts a directory enumeration.
        assert!(!budget.try_visit_directory());
        assert_eq!(
            budget.stop_reason(),
            Some(WorkspaceInspectionReason::DirectoryBudgetExhausted)
        );
        assert_eq!(budget.snapshot().directories_visited, 2);
    }

    #[test]
    fn each_budget_names_its_own_dimension() {
        for (name, exhaust) in [
            (
                WorkspaceInspectionReason::EntryBudgetExhausted,
                Box::new(|budget: &mut WorkspaceInspectionBudget| budget.try_visit_entry())
                    as Box<dyn Fn(&mut WorkspaceInspectionBudget) -> bool>,
            ),
            (
                WorkspaceInspectionReason::FileBudgetExhausted,
                Box::new(|budget: &mut WorkspaceInspectionBudget| budget.try_open_file()),
            ),
            (
                WorkspaceInspectionReason::MetadataBudgetExhausted,
                Box::new(|budget: &mut WorkspaceInspectionBudget| budget.try_metadata()),
            ),
            (
                WorkspaceInspectionReason::CandidateBudgetExhausted,
                Box::new(|budget: &mut WorkspaceInspectionBudget| budget.try_retain_candidate()),
            ),
            (
                WorkspaceInspectionReason::ResultBudgetExhausted,
                Box::new(|budget: &mut WorkspaceInspectionBudget| budget.try_emit_result()),
            ),
        ] {
            let (mut budget, _token, _clock) = budget();
            for _ in 0..10 {
                if !exhaust(&mut budget) {
                    break;
                }
            }
            assert_eq!(budget.stop_reason(), Some(name), "{}", name.code());
        }
    }

    #[test]
    fn bytes_are_charged_for_the_chunk_requested_rather_than_the_bytes_returned() {
        let (mut budget, _token, _clock) = budget();

        assert!(budget.try_read_bytes(8));
        // 8 + 4 exceeds 10, so the chunk is refused before it is read. Charging what came back
        // would let a file that grows under the reader run past the ceiling one short read at a
        // time.
        assert!(!budget.try_read_bytes(4));
        assert_eq!(budget.snapshot().bytes_read, 8);
        assert_eq!(
            budget.stop_reason(),
            Some(WorkspaceInspectionReason::ByteBudgetExhausted)
        );
    }

    #[test]
    fn depth_is_compared_rather_than_accumulated() {
        let (mut budget, _token, _clock) = budget();

        assert!(budget.try_descend(1));
        assert!(budget.try_descend(2));
        assert!(!budget.try_descend(3));
        assert_eq!(budget.snapshot().max_depth_reached, 2);
        assert_eq!(
            budget.stop_reason(),
            Some(WorkspaceInspectionReason::DepthBudgetExhausted)
        );
    }

    #[test]
    fn a_deadline_is_read_from_the_injected_clock_rather_than_the_wall() {
        let (mut budget, _token, clock) = budget();

        assert!(budget.checkpoint());
        clock.advance(Duration::from_secs(5));

        assert!(!budget.checkpoint());
        assert_eq!(
            budget.stop_reason(),
            Some(WorkspaceInspectionReason::DeadlineExceeded)
        );
    }

    #[test]
    fn cancellation_is_observed_at_the_next_checkpoint() {
        let (mut budget, token, _clock) = budget();

        assert!(budget.checkpoint());
        token.signal(SearchCancellationCause::Cancelled);

        // One checkpoint, not a millisecond threshold. A shared runner cannot promise the second
        // and can always count the first.
        let before = budget.checkpoints();
        assert!(!budget.try_visit_entry());
        assert_eq!(budget.checkpoints(), before + 1);
        assert_eq!(
            budget.stop_reason(),
            Some(WorkspaceInspectionReason::Cancelled)
        );
    }

    #[test]
    fn a_supersede_and_an_owner_drop_are_told_apart() {
        for (cause, reason) in [
            (
                SearchCancellationCause::Superseded,
                WorkspaceInspectionReason::Superseded,
            ),
            (
                SearchCancellationCause::OwnerDropped,
                WorkspaceInspectionReason::OwnerDropped,
            ),
        ] {
            let (mut budget, token, _clock) = budget();
            token.signal(cause);
            assert!(!budget.checkpoint());
            assert_eq!(budget.stop_reason(), Some(reason));
        }
    }

    #[test]
    fn the_first_stop_reason_is_the_one_reported() {
        let (mut budget, token, clock) = budget();

        token.signal(SearchCancellationCause::Cancelled);
        assert!(!budget.checkpoint());
        clock.advance(Duration::from_secs(60));
        assert!(!budget.checkpoint());

        // Reporting the last would tell a reader their search hit a deadline when in fact they
        // cancelled it.
        assert_eq!(
            budget.stop_reason(),
            Some(WorkspaceInspectionReason::Cancelled)
        );
    }

    #[test]
    fn a_released_candidate_frees_its_place_in_the_selection() {
        let (mut budget, _token, _clock) = budget();

        assert!(budget.try_retain_candidate());
        assert!(budget.try_retain_candidate());
        budget.release_candidate();

        // An evicting top-K holds at most K at any moment; charging the arrivals without crediting
        // the evictions would stop a bounded structure at its first refill.
        assert!(budget.try_retain_candidate());
        assert_eq!(budget.snapshot().candidates_retained, 2);
    }

    #[test]
    fn an_omission_makes_coverage_incomplete_without_ending_the_walk() {
        let (mut budget, _token, _clock) = budget();

        budget.note_omission(WorkspaceInspectionReason::DepthBudgetExhausted);

        // A subtree too deep to enter is not a reason to abandon the entries beside it, but it is
        // a reason the answer cannot claim to be complete.
        assert_eq!(budget.stop_reason(), None);
        assert!(budget.try_visit_entry());
        assert_eq!(
            budget.incomplete_reason(),
            Some(WorkspaceInspectionReason::DepthBudgetExhausted)
        );
    }

    #[test]
    fn a_stop_outranks_an_omission_that_came_before_it() {
        let (mut budget, _token, clock) = budget();

        budget.note_omission(WorkspaceInspectionReason::UnreadableEntries);
        clock.advance(Duration::from_secs(5));
        assert!(!budget.checkpoint());

        // A search that ran out of time having also skipped a binary file is a search that ran out
        // of time. The skip is in the counters; the reason is the thing that ended the answer.
        assert_eq!(
            budget.incomplete_reason(),
            Some(WorkspaceInspectionReason::DeadlineExceeded)
        );
        assert_eq!(budget.snapshot().unreadable_entries, 1);
    }

    #[test]
    fn a_snapshot_carries_counts_and_nothing_that_could_name_a_file() {
        let (mut budget, _token, _clock) = budget();
        budget.try_visit_entry();
        budget.note_omission(WorkspaceInspectionReason::UnreadableEntries);

        let snapshot = budget.snapshot();

        assert_eq!(snapshot.entries_visited, 1);
        assert_eq!(snapshot.unreadable_entries, 1);
        // The type has no string field at all, which is the enforcement: a summary that travels to
        // the frontend and into logs cannot leak a path it has nowhere to put.
        assert_eq!(std::mem::size_of_val(&snapshot.entries_visited), 8);
    }

    #[test]
    fn a_stopped_budget_refuses_everything_afterwards() {
        let (mut budget, token, _clock) = budget();
        token.signal(SearchCancellationCause::Cancelled);

        assert!(!budget.try_visit_directory());
        assert!(!budget.try_visit_entry());
        assert!(!budget.try_open_file());
        assert!(!budget.try_read_bytes(1));
        assert!(!budget.try_metadata());
        assert!(!budget.try_retain_candidate());
        assert!(!budget.try_emit_result());
        assert!(!budget.try_descend(0));
        assert_eq!(
            budget.snapshot(),
            WorkspaceInspectionBudgetSnapshot::default()
        );
    }

    #[test]
    fn the_content_profile_never_widens_a_bound_that_already_existed() {
        let content = WorkspaceInspectionBudgetLimits::content_search();

        // The shared walk stopped at 20,000 scanned entries and 10 levels, retained at most 2,000
        // candidates, and the search returned at most 200 matches. None of those moves outward.
        assert_eq!(content.max_entries_visited, 20_000);
        assert_eq!(content.max_depth, 10);
        assert_eq!(content.max_files_opened, 2_000);
        assert_eq!(content.max_results, 200);
        // 2,000 files at the existing 1 MiB per-file bound is 2 GiB with nothing counting it.
        assert!(content.max_bytes_read < 2_000 * 1024 * 1024);
    }

    #[test]
    fn a_path_search_may_not_open_a_file_at_all() {
        let paths = WorkspaceInspectionBudgetLimits::path_search();

        assert_eq!(paths.max_files_opened, 0);
        assert_eq!(paths.max_bytes_read, 0);
        // And it retains far less than the 2,000-entry vector the old full sort held.
        assert!(paths.max_retained_candidates < 2_000);
    }

    #[test]
    fn every_profile_is_finite() {
        for profile in [
            WorkspaceInspectionBudgetLimits::content_search(),
            WorkspaceInspectionBudgetLimits::path_search(),
            WorkspaceInspectionBudgetLimits::directory_listing(500),
        ] {
            assert!(profile.max_entries_visited > 0);
            assert!(profile.max_directories_visited > 0);
            assert!(profile.max_results > 0);
            assert!(profile.deadline > Duration::ZERO);
            // A path search that opened a file has done something it was not asked to, so zero is
            // the statement rather than an omission.
            assert!(profile.max_metadata_operations > 0);
        }
    }

    #[test]
    fn unavailable_reasons_are_the_ones_with_no_result_set_behind_them() {
        assert!(WorkspaceInspectionReason::InspectionBusy.is_unavailable());
        assert!(WorkspaceInspectionReason::ProviderUnavailable.is_unavailable());
        assert!(WorkspaceInspectionReason::StaleCursor.is_unavailable());
        // A budget stop looked at a real workspace and stopped early. That is partial, and calling
        // it unavailable would throw away results that exist.
        assert!(!WorkspaceInspectionReason::EntryBudgetExhausted.is_unavailable());
        assert!(!WorkspaceInspectionReason::Cancelled.is_unavailable());
    }
}
