//! Instance-free system clock primitives for injected infrastructure adapters.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemClock;

impl SystemClock {
    pub(crate) fn unix_seconds(&self) -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string())
    }

    pub(crate) fn rfc3339(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_exposes_existing_timestamp_formats() {
        let clock = SystemClock;

        assert!(clock.unix_seconds().parse::<u64>().is_ok());
        assert!(chrono::DateTime::parse_from_rfc3339(&clock.rfc3339()).is_ok());
    }
}
