// Assembled in bootstrap with the connect path in the Connector Lifecycle task group.
#![cfg_attr(not(test), allow(dead_code))]

//! Satisfying `ActiveConnectorSnapshotPort` by asking `extension_platform`.
//!
//! The adapter for a consumer-owned port: the interface belongs to this subdomain, the answer
//! belongs to the platform, and the translation between the two vocabularies happens here and
//! nowhere else. There is no SQL in this file and no reference to another subdomain's tables.
//!
//! The platform's three reads share one snapshot on its side, so the answer is one whole
//! generation. Identifiers cross as text and are re-validated on arrival: a value the platform
//! reports but this subdomain cannot parse is treated as "nothing to connect with" rather than
//! passed along, because the alternative is a value that only looks valid because it came from a
//! trusted neighbour.

use crate::contexts::tooling::connectors::application::ActiveConnectorSnapshotPort;
use crate::contexts::tooling::connectors::domain::{
    ActiveConnectorSnapshot, ConnectorDefinitionDigest, ConnectorGlobalId, ConnectorSnapshotRef,
};
use crate::contexts::tooling::extension_platform::api::{ActiveContribution, ExtensionPlatformApi};

pub(crate) struct ExtensionPlatformActiveConnector {
    platform: ExtensionPlatformApi,
}

impl ExtensionPlatformActiveConnector {
    pub(crate) fn new(platform: ExtensionPlatformApi) -> Self {
        Self { platform }
    }
}

impl ActiveConnectorSnapshotPort for ExtensionPlatformActiveConnector {
    fn active_snapshot(
        &self,
        connector: &ConnectorGlobalId,
    ) -> Result<ActiveConnectorSnapshot, String> {
        let answer = self
            .platform
            .active_contribution(connector.as_str())
            .map_err(|error| error.code().to_string())?;

        Ok(match answer {
            ActiveContribution::NotInstalled => ActiveConnectorSnapshot::NotInstalled,
            ActiveContribution::NoActiveGeneration => ActiveConnectorSnapshot::NoActiveGeneration,
            ActiveContribution::Running {
                snapshot_id,
                declared_digest,
            } => match ConnectorSnapshotRef::parse(&snapshot_id) {
                Ok(snapshot) => ActiveConnectorSnapshot::Running {
                    snapshot,
                    // A digest that does not parse is discarded rather than compared. Comparing an
                    // unparsed string would make `drifted` depend on byte equality of something
                    // neither side validated.
                    declared: declared_digest
                        .as_deref()
                        .and_then(|digest| ConnectorDefinitionDigest::parse(digest).ok()),
                },
                // The platform named a snapshot this subdomain cannot represent. Not knowing what
                // is running is not the same as knowing nothing runs.
                Err(_) => ActiveConnectorSnapshot::Unknown,
            },
        })
    }
}

/// The answer to give when the platform cannot be asked at all.
///
/// Exists so "the port is not wired yet" is a decision someone made rather than an `Option` every
/// caller has to remember to handle. Every path from `Unknown` is `Unavailable`.
pub(crate) struct UnknownActiveConnector;

impl ActiveConnectorSnapshotPort for UnknownActiveConnector {
    fn active_snapshot(
        &self,
        _connector: &ConnectorGlobalId,
    ) -> Result<ActiveConnectorSnapshot, String> {
        Ok(ActiveConnectorSnapshot::Unknown)
    }
}
