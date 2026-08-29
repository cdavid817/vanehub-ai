//! Which searches are running, and whose registration is whose.
//!
//! A registry rather than a token passed down, because the thing that cancels a search is a
//! different command from the one running it: by the time a reader presses Escape, the search is
//! already inside a walk on the blocking pool, and the only way to reach it is a flag it polls.
//!
//! What makes this more than a map is the generation. A search id comes from the caller and is
//! reused every keystroke, so two registrations under one id is the normal case rather than the
//! exception — B replaces A while A is still winding down. Keyed by id alone, A's cleanup removes
//! whatever it finds, which by then is B: B keeps running and no cancel can reach it. Every
//! registration therefore carries a generation, and a registration can only ever remove *itself*.
//!
//! Cleanup is a guard rather than a call. An explicit `finish` after an `await` only runs when the
//! await returned; a future that is aborted, or dropped because its window closed, never reaches
//! it. Dropping the guard is the one cleanup path that cannot be skipped, so that is where both the
//! signal and the removal live.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Which registration under a search id this is.
///
/// Never zero. Zero is what an uninitialized counter reads as, and a generation that could collide
/// with "no generation" would make the compare-remove below silently permissive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SearchGeneration(u64);

impl SearchGeneration {
    pub(crate) fn value(self) -> u64 {
        self.0
    }
}

/// Why a search was asked to stop.
///
/// Three causes rather than a boolean because a reader is told different things: they cancelled it,
/// they typed another character, or the view that wanted the answer is gone. Only the first is
/// something they did to this search on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchCancellationCause {
    /// An explicit cancel arrived for this search.
    Cancelled,
    /// A newer generation of the same search id replaced this one.
    Superseded,
    /// The future that owned the registration was dropped or aborted.
    OwnerDropped,
}

impl SearchCancellationCause {
    fn code(self) -> u8 {
        match self {
            Self::Cancelled => 1,
            Self::Superseded => 2,
            Self::OwnerDropped => 3,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Cancelled),
            2 => Some(Self::Superseded),
            3 => Some(Self::OwnerDropped),
            _ => None,
        }
    }
}

/// Not cancelled. A distinct constant rather than `0` inline so the three causes and this one are
/// read from the same place.
const CAUSE_RUNNING: u8 = 0;

/// The flag a worker polls, and the reason it stopped.
///
/// One atomic rather than a boolean plus a side channel: a worker that observed "stopped" and then
/// read the cause separately could see the two disagree, and the cause is what decides whether the
/// answer says cancelled or superseded.
#[derive(Debug, Clone, Default)]
pub(crate) struct SearchCancellationToken {
    state: Arc<AtomicU8>,
}

impl SearchCancellationToken {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Asks the worker to stop. The first cause to arrive is the one reported.
    ///
    /// First-writer-wins rather than last, because the causes arrive in the order they happened and
    /// the first one is the one that actually stopped the work. A supersede landing after an
    /// explicit cancel would otherwise relabel something the reader did on purpose.
    pub(crate) fn signal(&self, cause: SearchCancellationCause) {
        let _ = self.state.compare_exchange(
            CAUSE_RUNNING,
            cause.code(),
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.state.load(Ordering::Acquire) != CAUSE_RUNNING
    }

    pub(crate) fn cause(&self) -> Option<SearchCancellationCause> {
        SearchCancellationCause::from_code(self.state.load(Ordering::Acquire))
    }

    /// Which allocation this token is, for the compare-remove below.
    ///
    /// Identity rather than equality: two tokens in the same state are indistinguishable by value,
    /// and the question a removal asks is "is the slot still mine", not "does it look like mine".
    fn identity(&self) -> usize {
        Arc::as_ptr(&self.state) as usize
    }
}

/// One registration under one search id.
struct SearchSlot {
    generation: SearchGeneration,
    token: SearchCancellationToken,
}

/// Which searches are in flight, one slot per id.
///
/// Entries are removed by the registration that owns them and by nothing else. A cancel for a
/// search that already finished is silently accepted — the caller cannot know which happened
/// first, and refusing would make an ordinary race look like an error.
pub(crate) struct WorkspaceSearchCancellation {
    slots: Mutex<HashMap<String, SearchSlot>>,
    /// Monotone across every id in this registry. Per-id counters would need their own lifetime
    /// rules the moment an id's last slot is removed, and a wrap is handled the same way either
    /// way.
    next_generation: AtomicU64,
}

impl Default for WorkspaceSearchCancellation {
    fn default() -> Self {
        Self::starting_at(1)
    }
}

impl WorkspaceSearchCancellation {
    /// A registry whose first generation is `first`.
    ///
    /// Exists so wrap can be exercised: seeding near `u64::MAX` reaches the wrap in two calls
    /// instead of never.
    pub(crate) fn starting_at(first: u64) -> Self {
        Self {
            slots: Mutex::new(HashMap::new()),
            next_generation: AtomicU64::new(first.max(1)),
        }
    }

    /// A poisoned registry is recovered rather than propagated.
    ///
    /// The data behind the lock is a map of live registrations; a panic in one search leaves it
    /// structurally intact, and refusing every later search because of it would turn one failed
    /// walk into a permanently broken panel.
    fn slots(&self) -> MutexGuard<'_, HashMap<String, SearchSlot>> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The next generation for this id, skipping zero and anything currently occupied.
    fn allocate(&self, slots: &HashMap<String, SearchSlot>, search_id: &str) -> SearchGeneration {
        let occupied = slots.get(search_id).map(|slot| slot.generation.0);
        for _ in 0..3 {
            let raw = self.next_generation.fetch_add(1, Ordering::Relaxed);
            if raw == 0 || Some(raw) == occupied {
                continue;
            }
            return SearchGeneration(raw);
        }
        // Three collisions in a row is not reachable from a counter that moves every call; the
        // fallback exists so the loop has an answer rather than an unwrap.
        SearchGeneration(u64::MAX)
    }

    /// Registers a search and hands back the guard that owns the registration.
    ///
    /// Registering *before* the work starts is what makes a cancel that arrives immediately still
    /// land: a token created when the walk begins would miss every cancel sent in the window
    /// between the request leaving the frontend and the first directory being read.
    ///
    /// Replacing an id already in flight cancels the old one, and does so under the same lock that
    /// installs the new slot. A supersede signalled after the lock was released would leave a
    /// window in which two generations are both running and neither has been told to stop.
    pub(crate) fn begin(self: &Arc<Self>, search_id: &str) -> SearchRegistration {
        let mut slots = self.slots();
        let generation = self.allocate(&slots, search_id);
        let token = SearchCancellationToken::new();
        let previous = slots.insert(
            search_id.to_string(),
            SearchSlot {
                generation,
                token: token.clone(),
            },
        );
        if let Some(previous) = previous {
            previous.token.signal(SearchCancellationCause::Superseded);
        }
        drop(slots);

        SearchRegistration {
            search_id: search_id.to_string(),
            generation,
            token,
            registry: Arc::clone(self),
            completed: false,
        }
    }

    /// Asks a search to stop. Returns whether one was running under that id.
    pub(crate) fn cancel(&self, search_id: &str) -> bool {
        let slots = self.slots();
        match slots.get(search_id) {
            Some(slot) => {
                slot.token.signal(SearchCancellationCause::Cancelled);
                true
            }
            None => false,
        }
    }

    /// Asks one exact generation to stop, and refuses to touch any other.
    ///
    /// For a caller that already holds the token it wants stopped — a frontend replacing its own
    /// request, or a supervisor cleaning up after a specific generation. Cancelling by id alone
    /// there would stop whichever registration happens to be current, which by then may be one the
    /// caller never asked about.
    pub(crate) fn cancel_generation(&self, search_id: &str, generation: SearchGeneration) -> bool {
        let slots = self.slots();
        match slots.get(search_id) {
            Some(slot) if slot.generation == generation => {
                slot.token.signal(SearchCancellationCause::Cancelled);
                true
            }
            _ => false,
        }
    }

    /// Which generation currently answers for an id, if any.
    pub(crate) fn active_generation(&self, search_id: &str) -> Option<SearchGeneration> {
        self.slots().get(search_id).map(|slot| slot.generation)
    }

    /// How many registrations are live. A leak detector for tests, not a product signal.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "registry occupancy is asserted by tests; production reads one id at a time"
        )
    )]
    pub(crate) fn active_count(&self) -> usize {
        self.slots().len()
    }

    /// Removes a slot only when it is still the one the caller registered.
    fn remove_own(&self, search_id: &str, generation: SearchGeneration, identity: usize) {
        let mut slots = self.slots();
        let is_own = slots
            .get(search_id)
            .is_some_and(|slot| slot.generation == generation && slot.token.identity() == identity);
        if is_own {
            slots.remove(search_id);
        }
    }
}

/// One search's claim on its id, released when this is dropped.
///
/// Held by the async future that owns the request, never by the worker. The worker gets a clone of
/// the token and nothing else: a blocking walk that could reach the registry would be a second
/// place deciding whether a search is still wanted, and the two would disagree exactly when a
/// reader cancelled at the wrong moment.
pub(crate) struct SearchRegistration {
    search_id: String,
    generation: SearchGeneration,
    token: SearchCancellationToken,
    registry: Arc<WorkspaceSearchCancellation>,
    completed: bool,
}

impl SearchRegistration {
    pub(crate) fn token(&self) -> SearchCancellationToken {
        self.token.clone()
    }

    pub(crate) fn generation(&self) -> SearchGeneration {
        self.generation
    }

    /// Whether this registration is still the one answering for its id.
    ///
    /// What a caller checks before publishing a result. A superseded generation's answer is about a
    /// query the reader has already replaced, and applying it would overwrite the newer one with an
    /// older truth.
    pub(crate) fn is_current(&self) -> bool {
        self.registry.active_generation(&self.search_id) == Some(self.generation)
    }

    /// Normal completion: release the slot without claiming anybody stopped the work.
    ///
    /// Consuming rather than borrowing, so the guard cannot be used after the registration it names
    /// is gone.
    pub(crate) fn complete(mut self) {
        self.completed = true;
        self.registry
            .remove_own(&self.search_id, self.generation, self.token.identity());
    }
}

impl Drop for SearchRegistration {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        // The owner went away without completing: aborted, or dropped because the view that wanted
        // the answer closed. The worker is still on the blocking pool and this is the only thing
        // that will ever tell it to stop.
        self.token.signal(SearchCancellationCause::OwnerDropped);
        self.registry
            .remove_own(&self.search_id, self.generation, self.token.identity());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Arc<WorkspaceSearchCancellation> {
        Arc::new(WorkspaceSearchCancellation::default())
    }

    #[test]
    fn a_cancel_reaches_a_search_that_registered_first() {
        let registry = registry();
        let registration = registry.begin("search-1");

        assert!(registry.cancel("search-1"));
        assert!(registration.token().is_cancelled());
        assert_eq!(
            registration.token().cause(),
            Some(SearchCancellationCause::Cancelled)
        );
    }

    #[test]
    fn a_cancel_for_a_search_that_already_ended_is_accepted_quietly() {
        let registry = registry();
        registry.begin("search-1").complete();

        // False means "there was nothing to stop", not "you did something wrong". A caller cannot
        // know whether their cancel beat the search's own completion, and turning that ordinary
        // race into an error would put a failure on screen for a keystroke that worked.
        assert!(!registry.cancel("search-1"));
    }

    #[test]
    fn reusing_an_id_cancels_the_search_it_replaces() {
        let registry = registry();
        let first = registry.begin("search-1");
        let second = registry.begin("search-1");

        assert_eq!(
            first.token().cause(),
            Some(SearchCancellationCause::Superseded)
        );
        assert!(!second.token().is_cancelled());
        assert!(second.generation() > first.generation());
    }

    /// The defect this module exists for.
    ///
    /// A finishes after B replaced it. Under the id-only registry that removed B's slot and left B
    /// running with nothing able to reach it.
    #[test]
    fn an_older_search_completing_cannot_remove_its_replacement() {
        let registry = registry();
        let a = registry.begin("search-1");
        let b = registry.begin("search-1");

        a.complete();

        assert_eq!(
            registry.active_generation("search-1"),
            Some(b.generation()),
            "B still owns the id"
        );
        assert!(registry.cancel("search-1"), "B remains cancellable");
        assert_eq!(
            b.token().cause(),
            Some(SearchCancellationCause::Cancelled),
            "the cancel reached B"
        );
    }

    #[test]
    fn an_older_search_dropping_cannot_remove_its_replacement() {
        let registry = registry();
        let a = registry.begin("search-1");
        let b = registry.begin("search-1");

        // The abort path rather than the completion path: A's future is dropped where it stood.
        drop(a);

        assert_eq!(registry.active_generation("search-1"), Some(b.generation()));
        assert!(registry.cancel("search-1"));
        assert!(b.token().is_cancelled());
    }

    #[test]
    fn three_generations_leave_only_the_newest_registered() {
        let registry = registry();
        let first = registry.begin("search-1");
        let second = registry.begin("search-1");
        let third = registry.begin("search-1");

        assert!(first.token().is_cancelled());
        assert!(second.token().is_cancelled());
        assert!(!third.token().is_cancelled());

        first.complete();
        second.complete();

        assert_eq!(
            registry.active_generation("search-1"),
            Some(third.generation())
        );
        assert_eq!(registry.active_count(), 1);
    }

    #[test]
    fn dropping_the_owner_signals_the_worker_it_left_running() {
        let registry = registry();
        let registration = registry.begin("search-1");
        let worker_token = registration.token();

        drop(registration);

        // The only cleanup path an aborted future takes. Without it the walk on the blocking pool
        // runs to its natural end for an answer nobody will read.
        assert_eq!(
            worker_token.cause(),
            Some(SearchCancellationCause::OwnerDropped)
        );
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn a_completed_registration_does_not_report_itself_as_cancelled() {
        let registry = registry();
        let registration = registry.begin("search-1");
        let worker_token = registration.token();

        registration.complete();

        // Normal completion is not something a reader did, and reporting it as a cancellation would
        // put "search cancelled" on screen under a full result list.
        assert!(!worker_token.is_cancelled());
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn a_worker_that_returned_an_error_still_completes_rather_than_cancels() {
        let registry = registry();
        let registration = registry.begin("search-1");
        let worker_token = registration.token();

        // The worker failed and the owner is handling the failure. That is still a completion:
        // labelling it a cancellation would put "search cancelled" beside an error notice, and a
        // reader would be told two different things about one event.
        let outcome: Result<(), &str> = Err("directory unreadable");
        registration.complete();

        assert!(outcome.is_err());
        assert!(!worker_token.is_cancelled());
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn the_first_cause_to_arrive_is_the_one_reported() {
        let registry = registry();
        let registration = registry.begin("search-1");

        registry.cancel("search-1");
        // A supersede landing afterwards must not relabel what the reader did on purpose.
        let _replacement = registry.begin("search-1");

        assert_eq!(
            registration.token().cause(),
            Some(SearchCancellationCause::Cancelled)
        );
    }

    #[test]
    fn a_generation_qualified_cancel_refuses_a_generation_that_moved_on() {
        let registry = registry();
        let first = registry.begin("search-1");
        let stale = first.generation();
        let second = registry.begin("search-1");

        assert!(!registry.cancel_generation("search-1", stale));
        assert!(registry.cancel_generation("search-1", second.generation()));
        assert_eq!(
            second.token().cause(),
            Some(SearchCancellationCause::Cancelled)
        );
    }

    #[test]
    fn a_generation_never_wraps_onto_zero_or_onto_the_slot_it_replaces() {
        let registry = Arc::new(WorkspaceSearchCancellation::starting_at(u64::MAX));
        let first = registry.begin("search-1");
        let second = registry.begin("search-1");

        assert_eq!(first.generation().value(), u64::MAX);
        // The counter wrapped to zero and zero was skipped, so the replacement is a real generation
        // rather than the "no generation" sentinel the compare-remove tests against.
        assert_ne!(second.generation().value(), 0);
        assert_ne!(second.generation(), first.generation());
    }

    #[test]
    fn a_registration_knows_when_it_is_no_longer_the_current_one() {
        let registry = registry();
        let first = registry.begin("search-1");
        assert!(first.is_current());

        let second = registry.begin("search-1");

        // What a caller checks before publishing: A's answer is about a query the reader already
        // replaced, and applying it would overwrite the newer one with an older truth.
        assert!(!first.is_current());
        assert!(second.is_current());
    }

    #[test]
    fn two_ids_never_remove_each_others_registrations() {
        let registry = registry();
        let left = registry.begin("search-1");
        let right = registry.begin("search-2");

        left.complete();

        assert_eq!(registry.active_generation("search-1"), None);
        assert!(registry.cancel("search-2"));
        assert!(right.token().is_cancelled());
    }

    #[test]
    fn a_worker_panicking_still_releases_the_slot() {
        let registry = registry();
        let outcome = std::panic::catch_unwind({
            let registry = Arc::clone(&registry);
            move || {
                let _registration = registry.begin("search-1");
                panic!("worker failed");
            }
        });

        // The guard unwinds with the panic, so the id is free for the next search rather than
        // permanently occupied by a registration nothing will ever complete.
        assert!(outcome.is_err());
        assert_eq!(registry.active_count(), 0);
    }

    /// The abort path as it actually happens, rather than as a `drop` call.
    ///
    /// A cancelled Tauri command, a closed window, a `select!` that took the other branch: each of
    /// them drops the future where it stood, and none of them reaches a line of cleanup code. The
    /// guard is the only thing that runs.
    #[tokio::test]
    async fn aborting_the_owning_future_signals_its_worker_and_frees_the_id() {
        let registry = registry();
        let (send_token, receive_token) = tokio::sync::oneshot::channel();

        let owner = tokio::spawn({
            let registry = Arc::clone(&registry);
            async move {
                let registration = registry.begin("search-1");
                let _ = send_token.send(registration.token());
                // Stands in for awaiting the blocking worker. Nothing after this ever runs.
                std::future::pending::<()>().await;
                registration.complete();
            }
        });

        let worker_token = receive_token.await.expect("token");
        owner.abort();
        let _ = owner.await;

        assert_eq!(
            worker_token.cause(),
            Some(SearchCancellationCause::OwnerDropped)
        );
        assert_eq!(registry.active_count(), 0);
        assert!(!registry.cancel("search-1"));
    }

    #[test]
    fn a_late_result_is_recognisable_by_its_generation() {
        let registry = registry();
        let a = registry.begin("search-1");
        let late_generation = a.generation();
        let b = registry.begin("search-1");

        // A's worker returns after B registered. Comparing generations is what lets the caller drop
        // the answer instead of writing it over B's.
        assert_ne!(late_generation, b.generation());
        assert_eq!(registry.active_generation("search-1"), Some(b.generation()));
    }
}
