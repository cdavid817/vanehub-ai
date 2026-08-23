//! Ports the authorization-rule store is driven through.
//!
//! Task Group 3 lands the ports and their SQLite adapters. Wiring them into the PDP — the
//! `NoMatch` fallthrough to the existing template, and the trace — lands with the task group that
//! owns evaluation, so that until then the permissions behaviour is byte-for-byte what it is now.

mod ports;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use ports::{ActiveRuleSetRepository, RuleSetRepository};
