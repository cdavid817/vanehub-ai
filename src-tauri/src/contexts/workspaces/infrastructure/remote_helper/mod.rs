//! Talking to a remote workspace through a static helper.
//!
//! Split four ways because the parts fail differently and are worth testing apart: the protocol is
//! shapes and bounds, the transport is one round trip, the probe is the mapping from what a host
//! turned out to have to what may be offered, and the session is the only thing that touches SSH.

mod probe;
mod protocol;
mod remote_provider;
mod ssh_session;
mod transport;

#[cfg(test)]
mod operation_tests;
#[cfg(test)]
mod tests;

/// Named only where a test double has to write the type down. The provider's own error mapping
/// and profile lookup are internal; the contract suite implements both to drive it.
#[cfg(test)]
pub(crate) use protocol::RemoteHelperError;
#[cfg(test)]
pub(crate) use remote_provider::RemoteProfileSource;
pub(crate) use remote_provider::RemoteWorkspaceInspectionProvider;
pub(crate) use ssh_session::{SshRemoteHelperSession, SshRemoteProfileSource};
#[cfg(test)]
pub(crate) use transport::scripted_session;
