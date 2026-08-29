use chrono::{DateTime, Utc};

use crate::contexts::personalization::application::ClockPort;

/// The wall clock, for production assembly.
///
/// A separate type rather than the platform clock directly, because this context's port returns a
/// `DateTime<Utc>` and the platform one returns formatted strings. Adapting once here keeps every
/// time-dependent rule in this context testable against a fixed instant.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemPersonalizationClock;

impl ClockPort for SystemPersonalizationClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
