use crate::contexts::ssh_connections::application::connection_pool::RemoteSshPoolClockPort;
use std::time::Instant;

pub(crate) struct SystemRemoteSshPoolClock;

impl RemoteSshPoolClockPort for SystemRemoteSshPoolClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
