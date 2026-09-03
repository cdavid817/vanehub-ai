//! The repair half of the index port, for doubles that are not testing repair.
//!
//! Three test doubles exist to exercise the live bridge and the query bounds. None of them is about
//! backfill, so all three would otherwise carry the same seven stubs — and seven stubs copied three
//! times is where one copy quietly starts returning something else.
//!
//! Deliberately a macro rather than a default method on the trait. A default would let a *production*
//! repository silently inherit a stub, and the one thing worse than a missing implementation is one
//! that compiles and returns "nothing to do".

/// Expands to the repair methods a non-repair double does not exercise.
///
/// Every body is the honest no-op for a store with no repair state: nothing indexed, nothing
/// pending, nothing to clear. A double that needs real behaviour writes the method itself instead
/// of invoking this.
macro_rules! inert_repair_methods {
    () => {
        fn commit_batch(
            &self,
            _source: &crate::contexts::operations::application::LogSourceIdentity,
            _records: &[crate::contexts::operations::application::RedactedLogRecord],
            _rejections: &crate::contexts::operations::application::LineRejections,
            _next_offset: u64,
        ) -> Result<
            crate::contexts::operations::application::LogBatchCommit,
            crate::contexts::operations::application::OperationsLogError,
        > {
            Ok(crate::contexts::operations::application::LogBatchCommit::default())
        }

        fn load_repair_state(
            &self,
        ) -> Result<
            Option<crate::contexts::operations::application::SessionLogBackfillStatus>,
            crate::contexts::operations::application::OperationsLogError,
        > {
            Ok(None)
        }

        fn save_repair_state(
            &self,
            _status: &crate::contexts::operations::application::SessionLogBackfillStatus,
        ) -> Result<(), crate::contexts::operations::application::OperationsLogError> {
            Ok(())
        }

        fn gap_watermark(
            &self,
        ) -> Result<i64, crate::contexts::operations::application::OperationsLogError> {
            Ok(0)
        }

        fn clear_gaps_through(
            &self,
            _sources: &[crate::contexts::operations::application::LogSourceIdentity],
            _through_id: i64,
        ) -> Result<u32, crate::contexts::operations::application::OperationsLogError> {
            Ok(0)
        }

        fn conflict_count(
            &self,
            _sources: &[crate::contexts::operations::application::LogSourceIdentity],
        ) -> Result<u32, crate::contexts::operations::application::OperationsLogError> {
            Ok(0)
        }

        fn prune_source_generation(
            &self,
            _source: &crate::contexts::operations::application::LogSourceIdentity,
            _limit: u32,
        ) -> Result<u32, crate::contexts::operations::application::OperationsLogError> {
            Ok(0)
        }
    };
}

pub(crate) use inert_repair_methods;
