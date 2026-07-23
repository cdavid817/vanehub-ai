mod bootstrap;
mod commands;
mod contexts;
mod platform;

#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod migration_fixture_tests;
#[cfg(test)]
mod test_support;

pub fn run() {
    if contexts::tooling::mcp::infrastructure::try_run_from_process_args() {
        return;
    }
    bootstrap::run();
}
