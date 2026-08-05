use crate::contexts::permissions::application::PermissionsClockPort;
use crate::platform::clock::SystemClock;

/// Unix-seconds, not RFC3339 — `ApprovalBroker::sweep_timed_out` parses this value as an
/// integer to compare against its timeout window.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PermissionsSystemClock;

impl PermissionsClockPort for PermissionsSystemClock {
    fn now(&self) -> String {
        SystemClock.unix_seconds()
    }
}
