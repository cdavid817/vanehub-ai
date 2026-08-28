//! Reading a CLI's own configuration for the model a new session should default to.
//!
//! Separate from the environment use cases on purpose: this reads a vendor's config file to answer
//! "what would this tool pick by default", which is not part of discovering, planning, or changing
//! an installation. It lived in the flat lifecycle service's port list until that service was
//! deleted, which is the only reason the two were ever in the same file.

use std::fmt;

/// Why a CLI's own configuration could not be read.
///
/// The system adapter treats an unreadable or absent file as "no answer" and logs it, so this
/// exists for adapters that cannot: a caller must be able to tell "the tool has no configured
/// model" from "the configuration could not be read", because only the second is worth reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeConfigError(pub(crate) String);

impl fmt::Display for NativeConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeConfigError {}

pub(crate) trait NativeConfigPort: Send + Sync {
    /// The active model from a CLI's native configuration file.
    ///
    /// `workspace_path`, when available, lets a CLI check per-project state -- Claude Code's
    /// project-scoped usage cache, for instance -- in addition to its global configuration.
    ///
    /// `Ok(None)` means no source was available, readable, or carried a model value. Callers fall
    /// back to their own defaults rather than treating it as a failure.
    fn discover_model(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Option<String>, NativeConfigError>;
}
