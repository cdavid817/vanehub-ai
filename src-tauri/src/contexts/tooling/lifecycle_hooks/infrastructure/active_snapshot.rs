// Assembled in bootstrap with the dispatch engine in Task Group 7; see `sqlite_definitions.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Satisfying `ActiveExtensionSnapshotPort` by asking `extension_platform`.
//!
//! The adapter for a consumer-owned port: the interface belongs to this subdomain, the answer
//! belongs to the platform, and the translation between the two vocabularies happens here and
//! nowhere else. There is no SQL in this file and no reference to another subdomain's tables — the
//! only thing it can do is call the published API.
//!
//! Identifiers cross as text and are re-validated on arrival. A snapshot id or digest the platform
//! reports but this subdomain cannot parse is treated as "nothing to dispatch" rather than passed
//! along unchecked, because the alternative is a value that only looks valid because it came from
//! a trusted neighbour.

use crate::contexts::tooling::extension_platform::api::{ActiveContribution, ExtensionPlatformApi};
use crate::contexts::tooling::lifecycle_hooks::application::ActiveExtensionSnapshotPort;
use crate::contexts::tooling::lifecycle_hooks::domain::{
    ActiveSnapshot, DefinitionDigest, HookGlobalId, SnapshotRef,
};

pub(crate) struct ExtensionPlatformActiveSnapshot {
    platform: ExtensionPlatformApi,
}

impl ExtensionPlatformActiveSnapshot {
    pub(crate) fn new(platform: ExtensionPlatformApi) -> Self {
        Self { platform }
    }
}

impl ActiveExtensionSnapshotPort for ExtensionPlatformActiveSnapshot {
    fn active_snapshot(&self, hook: &HookGlobalId) -> Result<ActiveSnapshot, String> {
        let answer = self
            .platform
            .active_contribution(hook.as_str())
            .map_err(|error| error.code().to_string())?;

        Ok(match answer {
            ActiveContribution::NotInstalled => ActiveSnapshot::NotInstalled,
            ActiveContribution::NoActiveGeneration => ActiveSnapshot::NoActiveGeneration,
            ActiveContribution::Running {
                snapshot_id,
                declared_digest,
            } => match SnapshotRef::parse(&snapshot_id) {
                Ok(snapshot) => ActiveSnapshot::Running {
                    snapshot,
                    // A digest that does not parse is discarded rather than compared. Comparing an
                    // unparsed string would make `drifted` depend on byte equality of something
                    // neither side validated.
                    declared: declared_digest
                        .as_deref()
                        .and_then(|digest| DefinitionDigest::parse(digest).ok()),
                },
                // The platform named a snapshot this subdomain cannot represent. Not knowing what
                // is running is not the same as knowing nothing runs, so it answers `Unknown`,
                // whose every path is `Unavailable`.
                Err(_) => ActiveSnapshot::Unknown,
            },
        })
    }
}

/// The answer to give when the platform cannot be asked at all.
///
/// Exists so that "the port is not wired yet" is a decision someone made rather than an `Option`
/// every caller has to remember to handle. Every path from `Unknown` is `Unavailable`: a subdomain
/// that cannot reach the authority does not get to conclude that a Hook is ready.
pub(crate) struct UnknownActiveSnapshot;

impl ActiveExtensionSnapshotPort for UnknownActiveSnapshot {
    fn active_snapshot(&self, _hook: &HookGlobalId) -> Result<ActiveSnapshot, String> {
        Ok(ActiveSnapshot::Unknown)
    }
}
