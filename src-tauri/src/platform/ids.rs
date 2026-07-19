//! Per-instance monotonic identifiers without global mutable counters.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub(crate) struct MonotonicIdGenerator {
    prefix: String,
    counter: AtomicU64,
}

impl MonotonicIdGenerator {
    pub(crate) fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            counter: AtomicU64::new(0),
        }
    }

    pub(crate) fn next(&self, timestamp: &str) -> String {
        format!(
            "{}-{timestamp}-{}",
            self.prefix,
            self.counter.fetch_add(1, Ordering::Relaxed) + 1
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_are_monotonic_within_and_isolated_between_instances() {
        let first = MonotonicIdGenerator::new("op");
        let second = MonotonicIdGenerator::new("op");

        assert_eq!(first.next("100"), "op-100-1");
        assert_eq!(first.next("100"), "op-100-2");
        assert_eq!(second.next("100"), "op-100-1");
    }
}
