//! Opaque identifiers for everything the context owns.
//!
//! These are newtypes rather than `String` aliases so that a recording id cannot be passed where a
//! staged-input id is expected. That is not pedantry: both are user-supplied over IPC, and the
//! ownership checks that keep one composer from cancelling another's work are keyed on them.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! opaque_id {
    ($name:ident, $prefix:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub(crate) struct $name(String);

        impl $name {
            #[cfg_attr(not(test), allow(dead_code))]
            pub(crate) const PREFIX: &'static str = $prefix;

            pub(crate) fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[cfg_attr(not(test), allow(dead_code))]
            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }

            /// Reject anything that is not the shape this context mints. The value reaches the
            /// filesystem as a directory name, so a caller-supplied `..` or separator must never
            /// get that far -- containment is checked again downstream, but not being able to
            /// express an escape is cheaper than detecting one.
            #[cfg_attr(not(test), allow(dead_code))]
            pub(crate) fn parse(value: &str) -> Option<Self> {
                let suffix = value.strip_prefix($prefix)?;
                if suffix.len() != 32
                    || !suffix
                        .chars()
                        .all(|character| character.is_ascii_hexdigit())
                {
                    return None;
                }
                Some(Self(value.to_string()))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

opaque_id!(
    StagedInputId,
    "lmi-",
    "A file the user selected, copied into staging and not yet claimed by an operation."
);
opaque_id!(
    RecordingId,
    "lmr-",
    "One microphone capture, from press to release."
);
opaque_id!(
    LocalMediaOperationId,
    "lmo-",
    "A probe or inference operation. Distinct from the generic operations-context id, which this \
     wraps."
);
opaque_id!(
    PlaybackId,
    "lmp-",
    "One active local playback of generated speech."
);

/// A composer instance. Unlike the ids above this is minted by the frontend, so it is validated for
/// shape but not for provenance: it is a scope tag used to discard stale results, never a
/// capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ComposerScopeId(String);

impl ComposerScopeId {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Bounded and printable. A scope id ends up in operation metadata, so an unbounded string
    /// would be a way to push arbitrary content into diagnostics.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        if value.is_empty() || value.len() > 128 {
            return None;
        }
        if !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        {
            return None;
        }
        Some(Self(value.to_string()))
    }
}

impl fmt::Display for ComposerScopeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
#[path = "ids_tests.rs"]
mod tests;
