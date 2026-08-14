use super::{BrowserSidecarError, BrowserSidecarResponse};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BrowserOwnership {
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BrowserContextPolicy {
    pub(crate) max_lifetime: Duration,
    pub(crate) max_pages: u8,
    pub(crate) max_download_bytes: u64,
    pub(crate) max_event_count: u32,
}

impl Default for BrowserContextPolicy {
    fn default() -> Self {
        Self {
            max_lifetime: Duration::from_secs(20 * 60),
            max_pages: 2,
            max_download_bytes: 16 * 1024 * 1024,
            max_event_count: 500,
        }
    }
}

impl BrowserContextPolicy {
    fn validate(self) -> Result<Self, BrowserSessionError> {
        if self.max_lifetime.is_zero()
            || self.max_lifetime > Duration::from_secs(60 * 60)
            || !(1..=4).contains(&self.max_pages)
            || self.max_download_bytes == 0
            || self.max_download_bytes > 64 * 1024 * 1024
            || self.max_event_count == 0
            || self.max_event_count > 2_000
        {
            return Err(BrowserSessionError::InvalidPolicy);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BrowserSessionLease {
    pub(crate) expires_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BrowserSessionError {
    InvalidOwnership,
    InvalidPolicy,
    OwnershipMismatch,
    Expired,
    FactoryFailure,
    ProtocolFailure(BrowserSidecarError),
    UnsafeContext,
}

pub(crate) trait BrowserSession: Send {
    fn request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError>;

    fn close(&mut self) -> Result<(), BrowserSidecarError>;
}

pub(crate) trait BrowserSessionFactory: Send + Sync {
    fn create_isolated(
        &self,
        ownership: &BrowserOwnership,
        policy: BrowserContextPolicy,
    ) -> Result<Box<dyn BrowserSession>, BrowserSessionError>;
}

struct OwnedBrowserSession {
    ownership: BrowserOwnership,
    policy: BrowserContextPolicy,
    lease: BrowserSessionLease,
    session: Box<dyn BrowserSession>,
}

pub(crate) struct BrowserSessionManager {
    factory: Arc<dyn BrowserSessionFactory>,
    sessions: Mutex<BTreeMap<BrowserOwnership, OwnedBrowserSession>>,
}

impl BrowserSessionManager {
    pub(crate) fn new(factory: Arc<dyn BrowserSessionFactory>) -> Self {
        Self {
            factory,
            sessions: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn with_session<T>(
        &self,
        ownership: BrowserOwnership,
        policy: BrowserContextPolicy,
        operation: impl FnOnce(&mut dyn BrowserSession) -> Result<T, BrowserSessionError>,
    ) -> Result<T, BrowserSessionError> {
        validate_ownership(&ownership)?;
        let policy = policy.validate()?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| BrowserSessionError::FactoryFailure)?;
        remove_expired(&mut sessions);
        if !sessions.contains_key(&ownership) {
            let session = self.factory.create_isolated(&ownership, policy)?;
            sessions.insert(
                ownership.clone(),
                OwnedBrowserSession {
                    ownership: ownership.clone(),
                    policy,
                    lease: BrowserSessionLease {
                        expires_at: Instant::now() + policy.max_lifetime,
                    },
                    session,
                },
            );
        }
        let owned = sessions
            .get_mut(&ownership)
            .ok_or(BrowserSessionError::FactoryFailure)?;
        if owned.ownership != ownership || owned.policy != policy {
            return Err(BrowserSessionError::OwnershipMismatch);
        }
        if Instant::now() >= owned.lease.expires_at {
            let _ = owned.session.close();
            sessions.remove(&ownership);
            return Err(BrowserSessionError::Expired);
        }
        operation(owned.session.as_mut())
    }

    pub(crate) fn ensure_session(
        &self,
        ownership: BrowserOwnership,
        policy: BrowserContextPolicy,
    ) -> Result<(), BrowserSessionError> {
        self.with_session(ownership, policy, |_| Ok(()))
    }

    pub(crate) fn close_generation(
        &self,
        ownership: &BrowserOwnership,
    ) -> Result<(), BrowserSessionError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| BrowserSessionError::FactoryFailure)?;
        if let Some(mut session) = sessions.remove(ownership) {
            session
                .session
                .close()
                .map_err(BrowserSessionError::ProtocolFailure)?;
        }
        Ok(())
    }
}

impl Drop for BrowserSessionManager {
    fn drop(&mut self) {
        if let Ok(sessions) = self.sessions.get_mut() {
            for session in sessions.values_mut() {
                let _ = session.session.close();
            }
            sessions.clear();
        }
    }
}

fn validate_ownership(ownership: &BrowserOwnership) -> Result<(), BrowserSessionError> {
    if ownership.session_id.is_empty()
        || ownership.session_id.len() > 128
        || ownership.generation_id.is_empty()
        || ownership.generation_id.len() > 128
    {
        return Err(BrowserSessionError::InvalidOwnership);
    }
    Ok(())
}

fn remove_expired(sessions: &mut BTreeMap<BrowserOwnership, OwnedBrowserSession>) {
    let expired = sessions
        .iter()
        .filter_map(|(ownership, session)| {
            (Instant::now() >= session.lease.expires_at).then_some(ownership.clone())
        })
        .collect::<Vec<_>>();
    for ownership in expired {
        if let Some(mut session) = sessions.remove(&ownership) {
            let _ = session.session.close();
        }
    }
}

#[cfg(test)]
#[path = "session_policy_tests.rs"]
mod tests;
