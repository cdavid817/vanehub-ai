//! Native entry point for VaneHub AI.
//!
//! Bootstrap assembles bounded-context application APIs and their infrastructure adapters.
//! React reaches this crate only through registered Tauri commands; local persistence, process
//! execution, and desktop lifecycle behavior stay on the native side of that boundary.

mod bootstrap;
mod commands;
mod contexts;
mod platform;

#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod migration_fixture_tests;
#[cfg(test)]
mod native_lifecycle_tests;
#[cfg(test)]
mod remote_terminal_migration_tests;
#[cfg(test)]
mod test_support;

/// Starts the VaneHub AI native runtime after handling any process-scoped helper mode.
pub fn run() {
    if contexts::tooling::mcp::infrastructure::try_run_from_process_args() {
        return;
    }
    bootstrap::run();
}
