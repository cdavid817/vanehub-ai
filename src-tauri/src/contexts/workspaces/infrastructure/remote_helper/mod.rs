//! Talking to a remote workspace through a static helper.
//!
//! Split four ways because the parts fail differently and are worth testing apart: the protocol is
//! shapes and bounds, the transport is one round trip, the probe is the mapping from what a host
//! turned out to have to what may be offered, and the session is the only thing that touches SSH.

mod probe;
mod protocol;
mod ssh_session;
mod transport;

#[cfg(test)]
mod tests;

// Nothing is re-exported yet. The remote provider that consumes these arrives with the operations
// it needs (11.7-11.9), and publishing a surface before a caller exists would fix its shape around
// what happens to be written rather than around what that caller turns out to need.
