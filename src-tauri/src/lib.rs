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
    bootstrap::run();
}
