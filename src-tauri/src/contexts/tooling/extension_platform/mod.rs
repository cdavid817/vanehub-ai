//! Extension package, installation, dependency, lifecycle, runtime-generation, contribution
//! registry, and capability-gate ownership.
//!
//! Task Group 0 lands the capability gates only; the rest arrives with its own task groups.

pub(crate) mod api;
pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;
