//! Connector subjects, versioned definitions, configured instances, and where they are bound.
//!
//! A sibling of `extension_platform` and `lifecycle_hooks` rather than a folder inside either.
//! Connectors hold the one thing neither of those does — a reference to a secret in the OS
//! credential store — and keeping that in its own subdomain is what lets "which credential does
//! this instance use" have exactly one owner.
//!
//! It knows a snapshot only as an opaque reference, and learns what the platform is running
//! through `extension_platform`'s published API.

pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;
