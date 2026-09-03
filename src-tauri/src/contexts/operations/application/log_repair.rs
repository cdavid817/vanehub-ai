//! The bounded repair pass, and everything it is allowed to claim afterwards.
//!
//! Repair exists because the live bridge is lossy on purpose: a log append is durable before the
//! index hears about it, and the index is never allowed to push back on the append. So the index
//! runs behind by design, and this is what catches it up — reading the same durable files a
//! reader's export would read, in bounded batches, resuming from where it last committed.
//!
//! Everything here is shaped by one asymmetry. Committing rows without their checkpoint costs a
//! re-read; committing a checkpoint without its rows loses records permanently, because the offset
//! says they were read and "read" is the only claim a checkpoint makes. That is why a batch is one
//! transaction, why the pass reads before it writes, and why the reconcile phase — the only phase
//! that deletes anything — runs last and only over what it can prove it covered.

use super::log_index::{
    SessionLogBackfillState, SessionLogBackfillStatus, SessionLogCoverageState,
};
use super::log_index_ports::{LogSourceIdentity, LogSourceSnapshot, RedactedLogBatch};
use super::log_query_service::{
    ActiveRepair, SessionLogQueryService, REPAIR_BATCHES_PER_FILE, REPAIR_BATCH_BYTES,
    REPAIR_BATCH_RECORDS, REPAIR_FILES_PER_PASS, REPAIR_PRUNE_ROWS,
};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

/// What a pass measured about the corpus before it touched anything.
///
/// Taken once, up front, and never refreshed mid-pass. A pass that re-measured would chase a file
/// still being appended to and could never declare itself caught up; worse, it could clear a gap
/// recorded after it started — a hole in records it never read.
pub(crate) struct RepairSnapshot {
    pub(crate) sources: Vec<LogSourceSnapshot>,
    pub(crate) directory_generation: String,
    /// The newest gap id when the pass began. Only gaps at or below this may ever be cleared.
    pub(crate) gap_watermark: i64,
}

impl RepairSnapshot {
    fn identities(&self) -> Vec<LogSourceIdentity> {
        self.sources
            .iter()
            .map(|source| source.identity.clone())
            .collect()
    }
}

/// One file's progress within a pass.
struct FileOutcome {
    records_indexed: u64,
    /// Whether the file was read all the way to the offset the snapshot captured.
    reached_target: bool,
}

impl SessionLogQueryService {
    /// The repair as a caller sees it, running or not.
    pub(crate) fn backfill_status(&self) -> SessionLogBackfillStatus {
        if let Some(active) = self.active_repair.lock().ok().and_then(|slot| {
            slot.as_ref()
                .filter(|active| active.status.state.is_active())
                .map(|active| active.status.clone())
        }) {
            return active;
        }
        // Nothing is running here, but a previous process may have been interrupted mid-pass. The
        // persisted row is what tells a restarted application that its index is behind rather than
        // that no repair has ever run.
        self.index
            .load_repair_state()
            .ok()
            .flatten()
            .unwrap_or_else(|| self.idle_status())
    }

    fn idle_status(&self) -> SessionLogBackfillStatus {
        SessionLogBackfillStatus {
            operation_id: String::new(),
            state: SessionLogBackfillState::Idle,
            files_completed: 0,
            files_total: 0,
            records_indexed: 0,
            started_at: None,
            updated_at: Some(self.clock.now()),
            reason_code: None,
        }
    }

    /// Asks the running repair to stop.
    ///
    /// Committed checkpoints stay. A cancel that rolled back would make cancelling more expensive
    /// than finishing, which is the opposite of what a cancel is for.
    pub(crate) fn cancel_repair(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Indexes retained source records the index does not hold yet.
    ///
    /// Discover, index, reconcile — in that order, and the order is load-bearing. Reconcile is the
    /// only phase that removes anything, and it runs last so it can only act on a corpus that was
    /// actually read.
    pub(crate) fn repair(&self) -> SessionLogBackfillStatus {
        let operation_id = self.ids.next_operation_id();
        let started_at = self.clock.now();
        let mut progress = RepairProgress::new(operation_id, started_at);

        // Announced before any work, so a caller is never told "nothing is happening" about a
        // request that was accepted.
        let queued = progress.status(self, SessionLogBackfillState::Queued);
        self.publish(&queued);

        // Discovery runs before the claim, and that ordering is deliberate. Single-flight is per
        // *generation*, and the generation is a property of the listing — so a pass cannot know
        // which corpus it would be competing for until it has looked. Listing is read-only, so two
        // passes racing here cost one extra directory read and nothing else.
        let snapshot = match self.discover(&mut progress) {
            Ok(snapshot) => snapshot,
            Err(status) => return status,
        };
        match self.claim(&queued, Some(&snapshot.directory_generation)) {
            Claim::Taken => {}
            // Someone is already reading these files. Joining beats racing: two passes over one
            // corpus fight over the same checkpoints, and each undoes the other's progress.
            Claim::AlreadyRunning(running) => return running,
        }
        self.cancelled.store(false, Ordering::SeqCst);
        progress.files_total = u32::try_from(snapshot.sources.len()).unwrap_or(u32::MAX);

        let indexed = self.index_sources(&snapshot, &mut progress);
        if self.cancelled.load(Ordering::SeqCst) {
            return self.finish(&mut progress, SessionLogBackfillState::Cancelled, None);
        }

        self.reconcile(&snapshot, indexed, &mut progress);
        if self.cancelled.load(Ordering::SeqCst) {
            return self.finish(&mut progress, SessionLogBackfillState::Cancelled, None);
        }
        self.finish(&mut progress, SessionLogBackfillState::Completed, None)
    }

    /// Lists the corpus and captures what "caught up" will mean for this pass.
    fn discover(
        &self,
        progress: &mut RepairProgress,
    ) -> Result<RepairSnapshot, SessionLogBackfillStatus> {
        let discovering = progress.status(self, SessionLogBackfillState::Discovering);
        self.publish(&discovering);

        // Taken before the listing, so a gap recorded while this pass runs is outside the snapshot
        // and survives it. Taking it afterwards would let the pass clear a hole it never read.
        let gap_watermark = self.index.gap_watermark().unwrap_or(0);
        let sources = match self.sources.sources() {
            Ok(sources) => sources,
            // A listing that failed is not an empty corpus. Everything downstream that deletes is
            // gated on this succeeding, which is what stops a disk hiccup from erasing the index.
            Err(error) => {
                let code = error.code();
                return Err(self.finish(
                    progress,
                    SessionLogBackfillState::Failed,
                    Some(code.to_string()),
                ));
            }
        };
        let directory_generation = sources
            .first()
            .map(|source| source.identity.directory_generation.clone())
            .unwrap_or_default();
        Ok(RepairSnapshot {
            sources: sources.into_iter().take(REPAIR_FILES_PER_PASS).collect(),
            directory_generation,
            gap_watermark,
        })
    }

    /// Reads and writes, file by file, batch by batch.
    ///
    /// Returns the sources this pass carried all the way to their captured target. Only those may
    /// contribute to a claim that the corpus is whole.
    fn index_sources(
        &self,
        snapshot: &RepairSnapshot,
        progress: &mut RepairProgress,
    ) -> Vec<LogSourceIdentity> {
        let indexing = progress.status(self, SessionLogBackfillState::Indexing);
        self.reclaim(&indexing);
        self.publish(&indexing);

        let mut complete = Vec::new();
        for source in &snapshot.sources {
            if self.cancelled.load(Ordering::SeqCst) {
                break;
            }
            let outcome = self.index_one_source(source);
            progress.records_indexed += outcome.records_indexed;
            progress.files_completed += 1;
            if outcome.reached_target {
                complete.push(source.identity.clone());
            }
            // One progress notice per file, never one per record. Backfilled history is not news:
            // a subscriber that received a notice per indexed line would be told about the whole
            // corpus as though it had just happened.
            let running = progress.status(self, SessionLogBackfillState::Indexing);
            self.reclaim(&running);
            self.publish(&running);
        }
        complete
    }

    fn index_one_source(&self, source: &LogSourceSnapshot) -> FileOutcome {
        let mut offset = self
            .index
            .checkpoint(&source.identity)
            .ok()
            .flatten()
            .unwrap_or(0);
        // A file now shorter than what we already read past is not the file we read. Its offsets
        // point into bytes that are no longer there, so the only safe resume point is the start.
        if offset > source.end_offset {
            self.diagnostics.report(
                "log_source_truncated",
                BTreeMap::from([("source".into(), source.identity.as_key())]),
            );
            let _ = self
                .index
                .record_gap(&source.identity, "log_source_truncated", 1);
            self.prune_generation(&source.identity);
            offset = 0;
        }
        let mut records_indexed = 0u64;
        let mut reached_target = false;
        for _ in 0..REPAIR_BATCHES_PER_FILE {
            if self.cancelled.load(Ordering::SeqCst) {
                break;
            }
            // Read and parse first, with no transaction open. A transaction held across file IO
            // holds the write lock for as long as the disk takes.
            let batch = match self.sources.read_batch(
                &source.identity,
                offset,
                REPAIR_BATCH_RECORDS,
                REPAIR_BATCH_BYTES,
            ) {
                Ok(batch) => batch,
                Err(error) => {
                    self.diagnostics.report(
                        error.code(),
                        BTreeMap::from([
                            ("source".into(), source.identity.as_key()),
                            ("offset".into(), offset.to_string()),
                        ]),
                    );
                    break;
                }
            };
            let advanced = batch.next_offset > offset;
            let reached_end = batch.reached_end;
            match self.commit(source, &batch, offset) {
                Some(inserted) => records_indexed += u64::from(inserted),
                // Nothing moved, including the checkpoint. The next pass reads the same bytes
                // again, which is the cheap half of the asymmetry this whole design rests on.
                None => break,
            }
            offset = batch.next_offset;
            if reached_end || !advanced {
                reached_target = offset >= source.end_offset;
                break;
            }
        }
        FileOutcome {
            records_indexed,
            reached_target,
        }
    }

    /// Rows, gaps and the checkpoint, in one transaction. `None` means none of them moved.
    fn commit(
        &self,
        source: &LogSourceSnapshot,
        batch: &RedactedLogBatch,
        from_offset: u64,
    ) -> Option<u32> {
        // An empty batch that did not advance is not worth a transaction, and writing the same
        // checkpoint back would be a write that changes nothing.
        if batch.records.is_empty()
            && batch.rejections.is_empty()
            && batch.next_offset == from_offset
        {
            return Some(0);
        }
        match self.index.commit_batch(
            &source.identity,
            &batch.records,
            &batch.rejections,
            batch.next_offset,
        ) {
            Ok(commit) => Some(commit.inserted),
            Err(error) => {
                self.diagnostics.report(
                    error.code(),
                    BTreeMap::from([("stage".into(), "commit".into())]),
                );
                None
            }
        }
    }

    /// Removes what the pass can prove is obsolete, and nothing else.
    fn reconcile(
        &self,
        snapshot: &RepairSnapshot,
        indexed: Vec<LogSourceIdentity>,
        progress: &mut RepairProgress,
    ) {
        let reconciling = progress.status(self, SessionLogBackfillState::Reconciling);
        self.reclaim(&reconciling);
        self.publish(&reconciling);

        // Rows whose source is gone stop counting toward the corpus, and the oldest queryable
        // boundary moves with them. Reached only through a listing that succeeded, which is the
        // one thing standing between a disk hiccup and an erased index.
        let retained = snapshot.identities();
        let expired = self.expire_retired_sources(&retained);
        if expired > 0 {
            // Named so a reader can tell "the log does not go back that far" from "the index lost
            // something". Both make a page shorter, and only the code says which happened.
            if let Some(first) = snapshot.sources.first() {
                let _ = self
                    .index
                    .record_gap(&first.identity, "log_retention_expired", expired);
            }
        }

        // Every condition below has to hold. Each one is a different way the pass could have
        // covered less than it appears to, and clearing a gap the pass did not actually fill is
        // the one mistake that makes coverage lie in the confident direction.
        let covered_every_source = indexed.len() == snapshot.sources.len();
        let no_conflicts = self.index.conflict_count(&retained).unwrap_or(u32::MAX) == 0;
        let current = self.sources.sources();
        // A directory change replaces the corpus. The files this pass read are not the files a
        // reader would now be shown, so nothing it proved about them says anything about these.
        let same_directory = current.as_ref().is_ok_and(|current| {
            current
                .first()
                .map(|source| source.identity.directory_generation == snapshot.directory_generation)
                .unwrap_or(snapshot.sources.is_empty())
        });
        let sources_unchanged = current.is_ok_and(|current| {
            current.len() == snapshot.sources.len()
                && current.iter().zip(&snapshot.sources).all(|(now, then)| {
                    // Grown is fine — the pass covered up to its captured target, and the rest is
                    // the next pass's work. Shrunk is not: those bytes are gone.
                    now.identity == then.identity && now.end_offset >= then.end_offset
                })
        });

        if covered_every_source && no_conflicts && same_directory && sources_unchanged {
            if let Err(error) = self
                .index
                .clear_gaps_through(&retained, snapshot.gap_watermark)
            {
                self.diagnostics.report(
                    error.code(),
                    BTreeMap::from([("stage".into(), "gap_clear".into())]),
                );
            }
        }
    }

    /// Removes rows for sources that are no longer retained, in bounded batches.
    ///
    /// Loops rather than deleting in one statement, because retention can expire a great deal at
    /// once — a configured directory change expires the entire previous corpus — and one
    /// transaction spanning all of it holds the write lock for as long as the delete takes.
    fn expire_retired_sources(&self, retained: &[LogSourceIdentity]) -> u32 {
        let mut expired = 0u32;
        loop {
            if self.cancelled.load(Ordering::SeqCst) {
                return expired;
            }
            match self.index.expire_sources(retained, REPAIR_PRUNE_ROWS) {
                Ok(0) => return expired,
                Ok(removed) => expired = expired.saturating_add(removed),
                Err(error) => {
                    self.diagnostics.report(
                        error.code(),
                        BTreeMap::from([("stage".into(), "retention".into())]),
                    );
                    return expired;
                }
            }
        }
    }

    /// Deletes one superseded generation in bounded batches.
    fn prune_generation(&self, source: &LogSourceIdentity) {
        loop {
            if self.cancelled.load(Ordering::SeqCst) {
                return;
            }
            match self
                .index
                .prune_source_generation(source, REPAIR_PRUNE_ROWS)
            {
                Ok(0) => return,
                Ok(_) => {}
                Err(error) => {
                    self.diagnostics.report(
                        error.code(),
                        BTreeMap::from([("stage".into(), "prune".into())]),
                    );
                    return;
                }
            }
        }
    }

    /// Takes the single-flight claim, or reports who already holds it.
    ///
    /// Single-flight is per *generation*, not per service. Two passes over one corpus race to the
    /// same checkpoints and each undoes the other's progress, so the second joins. But a claim held
    /// against a corpus that no longer exists — the configured directory moved while a pass was
    /// running — blocks forever if it is treated the same way, and the pass holding it is reading
    /// files nobody will query. That one is taken over.
    fn claim(&self, status: &SessionLogBackfillStatus, generation: Option<&str>) -> Claim {
        let Ok(mut slot) = self.active_repair.lock() else {
            return Claim::Taken;
        };
        if let Some(active) = slot
            .as_ref()
            .filter(|active| active.status.state.is_active())
        {
            let stale = generation.is_some_and(|wanted| {
                // An empty claim is one that has not discovered its generation yet, which is not
                // evidence of a different corpus.
                !active.directory_generation.is_empty() && active.directory_generation != wanted
            });
            if !stale {
                return Claim::AlreadyRunning(active.status.clone());
            }
        }
        *slot = Some(ActiveRepair {
            directory_generation: generation.unwrap_or_default().to_string(),
            status: status.clone(),
        });
        Claim::Taken
    }

    /// Installs an active claim without running a pass.
    ///
    /// Single-flight is a race, and a test that tried to stage one by spawning two threads would be
    /// asserting on a scheduler. This puts the claim in the state a concurrent pass would have left
    /// it in, so the behaviour under test — join, or take over a stranded claim — is decided by the
    /// code rather than by timing.
    #[cfg(test)]
    pub(super) fn stage_active_repair(
        &self,
        directory_generation: &str,
        status: &SessionLogBackfillStatus,
    ) {
        if let Ok(mut slot) = self.active_repair.lock() {
            *slot = Some(ActiveRepair {
                directory_generation: directory_generation.to_string(),
                status: status.clone(),
            });
        }
    }

    /// Updates the claim in place as the pass moves between phases.
    fn reclaim(&self, status: &SessionLogBackfillStatus) {
        if let Ok(mut slot) = self.active_repair.lock() {
            if let Some(active) = slot.as_mut() {
                active.status = status.clone();
            }
        }
    }

    fn publish(&self, status: &SessionLogBackfillStatus) {
        let _ = self.index.save_repair_state(status);
        self.backfill.publish(status.clone());
    }

    fn finish(
        &self,
        progress: &mut RepairProgress,
        state: SessionLogBackfillState,
        reason_code: Option<String>,
    ) -> SessionLogBackfillStatus {
        let mut status = progress.status(self, state);
        status.reason_code = reason_code;
        self.reclaim(&status);
        self.publish(&status);
        status
    }

    /// What coverage a repair-aware caller should see while a pass is running.
    ///
    /// Reported as `indexing` rather than `partial`: the rows already returned are real, and the
    /// set is not final. Saying `partial` would claim something is known to be missing, which is a
    /// different fact and one this state does not establish.
    pub(crate) fn repair_coverage_state(&self) -> Option<SessionLogCoverageState> {
        self.backfill_status()
            .state
            .is_active()
            .then_some(SessionLogCoverageState::Indexing)
    }
}

enum Claim {
    Taken,
    AlreadyRunning(SessionLogBackfillStatus),
}

/// The counters one pass accumulates, and the status it renders them into.
struct RepairProgress {
    operation_id: String,
    started_at: String,
    files_completed: u32,
    files_total: u32,
    records_indexed: u64,
}

impl RepairProgress {
    fn new(operation_id: String, started_at: String) -> Self {
        Self {
            operation_id,
            started_at,
            files_completed: 0,
            files_total: 0,
            records_indexed: 0,
        }
    }

    fn status(
        &self,
        service: &SessionLogQueryService,
        state: SessionLogBackfillState,
    ) -> SessionLogBackfillStatus {
        SessionLogBackfillStatus {
            operation_id: self.operation_id.clone(),
            state,
            files_completed: self.files_completed,
            files_total: self.files_total,
            records_indexed: self.records_indexed,
            started_at: Some(self.started_at.clone()),
            updated_at: Some(service.clock.now()),
            reason_code: None,
        }
    }
}
