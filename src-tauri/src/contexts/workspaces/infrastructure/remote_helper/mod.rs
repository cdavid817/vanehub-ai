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

pub(crate) use remote_provider::RemoteWorkspaceInspectionProvider;
pub(crate) use ssh_session::SshRemoteHelperSession;
