use crate::contexts::workspaces::application::WorkspaceClockPort;
use crate::platform::clock::SystemClock;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemWorkspaceClock;

impl WorkspaceClockPort for SystemWorkspaceClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}
