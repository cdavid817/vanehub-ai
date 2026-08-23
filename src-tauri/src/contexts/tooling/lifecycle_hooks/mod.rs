//! Lifecycle Hook subjects, versioned definitions, user bindings, and execution evidence.
//!
//! A sibling of `extension_platform` rather than a folder inside it. Hooks have their own
//! aggregates, their own retention rule, and their own reason to refuse a write; folding them into
//! the extension subdomain would make one repository responsible for both and would put the
//! authority for "may this Hook run" next to the authority for "is this package trustworthy".
//!
//! It knows a snapshot only as an opaque reference. Snapshots belong to `extension_platform`, and
//! everything this subdomain learns about one comes through that context's published API.

pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;
