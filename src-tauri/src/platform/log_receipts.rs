//! What the logging store says after a record is durably on disk.
//!
//! A receipt, not a copy. It carries the already-redacted fields plus the witness that identifies
//! the line — which file generation, which byte offset — so a consumer can index it, retry that
//! index, or find it again after a restart, without the logging store learning what a consumer is.
//!
//! The direction matters: `platform::logging` defines this port and publishes into it. It never
//! imports an index, a repository, or a context. A logging store that depended on its consumer
//! would make appending a log fail whenever the consumer did.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

/// Which file generation a record landed in.
///
/// A path is deliberately absent. Rotation renames a file whose records are the same records,
/// truncation reuses a path for unrelated bytes, and a directory change replaces the corpus; a
/// consumer that keyed on the path would resume a byte offset against content it was never
/// written for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogSourceWitness {
    pub(crate) directory_generation: String,
    pub(crate) file_id: String,
    pub(crate) offset: u64,
}

/// One durably appended, already-redacted record.
///
/// Published only after the append succeeded. A receipt for a line that is not on disk would let a
/// consumer index a record the log does not have, and a later repair would find no source for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RedactedLogAppendReceipt {
    /// Assigned before the append and written into the line, so a retry, a restart, and a backfill
    /// of the same line all present the same id.
    pub(crate) record_id: String,
    pub(crate) source: LogSourceWitness,
    pub(crate) timestamp: String,
    pub(crate) level: &'static str,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) context: BTreeMap<String, String>,
}

/// Where receipts go.
///
/// Infallible from the caller's side, and expected to return immediately. The record is already
/// durable by the time this is called; letting a consumer's back-pressure reach back into the
/// append would make observation a precondition of the thing observed.
pub(crate) trait RedactedLogAppendSink: Send + Sync {
    fn record_appended(&self, receipt: RedactedLogAppendReceipt);
}

static APPEND_SINK: OnceLock<Mutex<Option<Box<dyn RedactedLogAppendSink>>>> = OnceLock::new();

fn sink_slot() -> &'static Mutex<Option<Box<dyn RedactedLogAppendSink>>> {
    APPEND_SINK.get_or_init(|| Mutex::new(None))
}

/// Installs the process-wide sink. Called once from bootstrap.
///
/// A global rather than a parameter because `write_entry` is reached from every layer of the
/// application, including places that predate dependency injection and places that log while
/// bootstrap is still assembling. Threading a sink through all of them would mean the earliest
/// records — the ones describing a failed startup — are the ones that could not carry it.
pub(crate) fn set_append_sink(sink: Box<dyn RedactedLogAppendSink>) {
    if let Ok(mut slot) = sink_slot().lock() {
        *slot = Some(sink);
    }
}

/// Uninstalls the sink and returns it, so its own resources can be released.
///
/// The consumer behind it owns a thread that ends when every sender is dropped, and the sink is
/// where the last sender lives. Without this, a shutdown that waited for that thread waited for
/// something that could not happen: the sink is a `'static` and outlives the wait.
///
/// Returned rather than dropped in place, so the caller decides when the drop happens — dropping it
/// while holding the slot's lock would run a destructor under a lock every logging call takes.
pub(crate) fn take_append_sink() -> Option<Box<dyn RedactedLogAppendSink>> {
    sink_slot().lock().ok()?.take()
}

/// Publishes a receipt, if anything is listening.
///
/// A poisoned lock is treated as "no sink": the log is already written, and panicking here would
/// turn a consumer's earlier panic into a failure of the logging path itself.
pub(crate) fn publish_append_receipt(receipt: RedactedLogAppendReceipt) {
    let Ok(slot) = sink_slot().lock() else {
        return;
    };
    if let Some(sink) = slot.as_ref() {
        sink.record_appended(receipt);
    }
}
