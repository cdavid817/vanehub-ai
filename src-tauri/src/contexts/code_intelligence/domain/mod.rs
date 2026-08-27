//! Provider-neutral language-server lifecycle and query concepts.

// Bootstrap assembly is introduced after the persistence foundation in the task sequence.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod configuration;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod language_id;
// These domain contracts land before their application consumers in the task sequence.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod models;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod registry;

#[cfg(test)]
mod registry_tests;
#[cfg(test)]
mod tests;
