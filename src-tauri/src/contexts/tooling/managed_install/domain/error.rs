//! This capability's own error.
//!
//! Deliberately not the CLI context's: a consumer converts at its boundary. Sharing an error type
//! is how a shared capability starts accumulating one consumer's vocabulary, and the variants
//! below are the only outcomes retrieval actually produces.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum ManagedInstallError {
    /// The URL, a redirect target, or a declaration was refused before any bytes were fetched.
    #[error("{0}")]
    Refused(String),
    /// The transfer or the filesystem failed. Already redacted.
    #[error("{0}")]
    Transfer(String),
    /// The declared deadline elapsed.
    #[error("the download exceeded its time budget")]
    TimedOut,
    /// The caller signalled cancellation. Nothing was applied.
    #[error("the download was cancelled")]
    Cancelled,
    /// The bytes did not match the declared digest, and were discarded.
    #[error("the downloaded artifact did not match its published checksum")]
    ChecksumMismatch,
}
