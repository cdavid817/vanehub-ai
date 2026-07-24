use super::runtime::{RemoteSshConnectorPort, RemoteSshError, RemoteSshTransportPort};
use crate::contexts::ssh_connections::domain::runtime::RemoteSshConnectionKey;
use crate::contexts::ssh_connections::domain::SshConnectionProfile;
use futures_util::future::{BoxFuture, FutureExt, Shared};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type ConnectResult = Result<Arc<dyn RemoteSshTransportPort>, RemoteSshError>;
type SharedConnect = Shared<BoxFuture<'static, ConnectResult>>;

pub(crate) trait RemoteSshPoolClockPort: Send + Sync {
    fn now(&self) -> Instant;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteSshPoolHealth {
    Healthy,
    Draining,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteSshPoolEntrySnapshot {
    pub(crate) key: RemoteSshConnectionKey,
    pub(crate) leases: usize,
    pub(crate) health: RemoteSshPoolHealth,
}

#[derive(Clone)]
pub(crate) struct RemoteSshConnectionPool {
    inner: Arc<PoolInner>,
}

struct PoolInner {
    connector: Arc<dyn RemoteSshConnectorPort>,
    clock: Arc<dyn RemoteSshPoolClockPort>,
    capacity: usize,
    idle_timeout: Duration,
    state: Mutex<PoolState>,
}

#[derive(Default)]
struct PoolState {
    entries: HashMap<RemoteSshConnectionKey, PoolEntry>,
    failures: HashMap<RemoteSshConnectionKey, RemoteSshError>,
    next_generation: u64,
}

enum PoolEntry {
    Connecting {
        generation: u64,
        future: SharedConnect,
    },
    Ready {
        transport: Arc<dyn RemoteSshTransportPort>,
        leases: usize,
        last_used: Instant,
        health: RemoteSshPoolHealth,
    },
}

pub(crate) struct RemoteSshLease {
    key: RemoteSshConnectionKey,
    transport: Arc<dyn RemoteSshTransportPort>,
    pool: Arc<PoolInner>,
}

impl RemoteSshLease {
    pub(crate) fn key(&self) -> &RemoteSshConnectionKey {
        &self.key
    }

    pub(crate) fn transport(&self) -> Arc<dyn RemoteSshTransportPort> {
        self.transport.clone()
    }
}

impl Drop for RemoteSshLease {
    fn drop(&mut self) {
        self.pool.release(&self.key);
    }
}

impl RemoteSshConnectionPool {
    pub(crate) fn new(
        connector: Arc<dyn RemoteSshConnectorPort>,
        clock: Arc<dyn RemoteSshPoolClockPort>,
        capacity: usize,
        idle_timeout: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(PoolInner {
                connector,
                clock,
                capacity: capacity.max(1),
                idle_timeout,
                state: Mutex::new(PoolState::default()),
            }),
        }
    }

    pub(crate) async fn acquire(
        &self,
        profile: &SshConnectionProfile,
    ) -> Result<RemoteSshLease, RemoteSshError> {
        let key = RemoteSshConnectionKey::from(profile);
        let now = self.inner.clock.now();
        let prepared = {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| RemoteSshError::ConnectionFailed)?;
            let mut closing = evict_idle_locked(&mut state, now, self.inner.idle_timeout);
            if let Some(PoolEntry::Ready {
                transport,
                leases,
                last_used,
                health: RemoteSshPoolHealth::Healthy,
            }) = state.entries.get_mut(&key)
            {
                if transport.is_healthy() {
                    *leases += 1;
                    *last_used = now;
                    return Ok(self.lease(key, transport.clone()));
                }
            }
            if let Some(PoolEntry::Connecting { generation, future }) = state.entries.get(&key) {
                let generation = *generation;
                let future = future.clone();
                drop(state);
                return self.finish_connect(key, generation, future).await;
            }
            if let Some(entry) = state.entries.remove(&key) {
                if let Some(transport) = ready_transport(entry) {
                    closing.push(transport);
                }
            }
            closing.extend(make_capacity_locked(&mut state, self.inner.capacity)?);
            state.next_generation = state.next_generation.wrapping_add(1);
            let generation = state.next_generation;
            let connector = self.inner.connector.clone();
            let connect_profile = profile.clone();
            let future = async move { connector.connect(&connect_profile).await }
                .boxed()
                .shared();
            state.failures.remove(&key);
            state.entries.insert(
                key.clone(),
                PoolEntry::Connecting {
                    generation,
                    future: future.clone(),
                },
            );
            (future, generation, closing)
        };
        close_transports(prepared.2).await;
        self.finish_connect(key, prepared.1, prepared.0).await
    }

    async fn finish_connect(
        &self,
        key: RemoteSshConnectionKey,
        generation: u64,
        future: SharedConnect,
    ) -> Result<RemoteSshLease, RemoteSshError> {
        let result = future.await;
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| RemoteSshError::ConnectionFailed)?;
        match result {
            Ok(transport) => match state.entries.get_mut(&key) {
                Some(PoolEntry::Connecting {
                    generation: current,
                    ..
                }) if *current == generation => {
                    state.entries.insert(
                        key.clone(),
                        PoolEntry::Ready {
                            transport: transport.clone(),
                            leases: 1,
                            last_used: self.inner.clock.now(),
                            health: RemoteSshPoolHealth::Healthy,
                        },
                    );
                    Ok(self.lease(key, transport))
                }
                Some(PoolEntry::Ready {
                    transport,
                    leases,
                    last_used,
                    health: RemoteSshPoolHealth::Healthy,
                }) => {
                    *leases += 1;
                    *last_used = self.inner.clock.now();
                    Ok(self.lease(key, transport.clone()))
                }
                _ => Err(RemoteSshError::TransportClosed),
            },
            Err(error) => {
                if matches!(
                    state.entries.get(&key),
                    Some(PoolEntry::Connecting {
                        generation: current,
                        ..
                    }) if *current == generation
                ) {
                    state.entries.remove(&key);
                    state.failures.insert(key, error.clone());
                }
                Err(error)
            }
        }
    }

    pub(crate) async fn evict_idle(&self) -> Result<usize, RemoteSshError> {
        let closing = {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| RemoteSshError::ConnectionFailed)?;
            evict_idle_locked(&mut state, self.inner.clock.now(), self.inner.idle_timeout)
        };
        let count = closing.len();
        close_transports(closing).await;
        Ok(count)
    }

    pub(crate) fn drain(&self, connection_id: &str) {
        let mut closing = Vec::new();
        if let Ok(mut state) = self.inner.state.lock() {
            let keys = state
                .entries
                .iter()
                .filter_map(|(key, entry)| {
                    (key.connection_id == connection_id).then_some((key.clone(), entry))
                })
                .filter_map(|(key, entry)| match entry {
                    PoolEntry::Ready { leases: 0, .. } => Some(key),
                    PoolEntry::Ready { .. } => None,
                    PoolEntry::Connecting { .. } => Some(key),
                })
                .collect::<Vec<_>>();
            closing.extend(remove_entries(&mut state, keys));
            for (key, entry) in state.entries.iter_mut() {
                if key.connection_id == connection_id {
                    if let PoolEntry::Ready { health, .. } = entry {
                        *health = RemoteSshPoolHealth::Draining;
                    }
                }
            }
        }
        tokio::spawn(close_transports(closing));
    }

    pub(crate) async fn shutdown(&self) -> Result<(), RemoteSshError> {
        let closing = {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| RemoteSshError::ConnectionFailed)?;
            let keys = state.entries.keys().cloned().collect::<Vec<_>>();
            remove_entries(&mut state, keys)
        };
        close_transports(closing).await;
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> Vec<RemoteSshPoolEntrySnapshot> {
        let Ok(state) = self.inner.state.lock() else {
            return Vec::new();
        };
        let mut snapshots = state
            .entries
            .iter()
            .map(|(key, entry)| snapshot_entry(key, entry))
            .collect::<Vec<_>>();
        snapshots.extend(state.failures.keys().map(|key| RemoteSshPoolEntrySnapshot {
            key: key.clone(),
            leases: 0,
            health: RemoteSshPoolHealth::Failed,
        }));
        snapshots.sort_by(|left, right| {
            left.key
                .connection_id
                .cmp(&right.key.connection_id)
                .then(left.key.revision.cmp(&right.key.revision))
        });
        snapshots
    }

    fn lease(
        &self,
        key: RemoteSshConnectionKey,
        transport: Arc<dyn RemoteSshTransportPort>,
    ) -> RemoteSshLease {
        RemoteSshLease {
            key,
            transport,
            pool: self.inner.clone(),
        }
    }
}

impl PoolInner {
    fn release(&self, key: &RemoteSshConnectionKey) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let mut closing = None;
        if let Some(PoolEntry::Ready {
            transport,
            leases,
            last_used,
            health,
        }) = state.entries.get_mut(key)
        {
            *leases = leases.saturating_sub(1);
            *last_used = self.clock.now();
            if *leases == 0 && *health == RemoteSshPoolHealth::Draining {
                closing = Some(transport.clone());
                state.entries.remove(key);
            }
        }
        drop(state);
        if let Some(transport) = closing {
            tokio::spawn(async move {
                let _ = transport.close().await;
            });
        }
    }
}

fn evict_idle_locked(
    state: &mut PoolState,
    now: Instant,
    idle_timeout: Duration,
) -> Vec<Arc<dyn RemoteSshTransportPort>> {
    let keys = state
        .entries
        .iter()
        .filter_map(|(key, entry)| match entry {
            PoolEntry::Ready {
                leases: 0,
                last_used,
                ..
            } if now.saturating_duration_since(*last_used) >= idle_timeout => Some(key.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    remove_entries(state, keys)
}

fn make_capacity_locked(
    state: &mut PoolState,
    capacity: usize,
) -> Result<Vec<Arc<dyn RemoteSshTransportPort>>, RemoteSshError> {
    if state.entries.len() < capacity {
        return Ok(Vec::new());
    }
    let candidate = state
        .entries
        .iter()
        .filter_map(|(key, entry)| match entry {
            PoolEntry::Ready {
                leases: 0,
                last_used,
                ..
            } => Some((key.clone(), *last_used)),
            _ => None,
        })
        .min_by_key(|(_, last_used)| *last_used)
        .map(|(key, _)| key)
        .ok_or(RemoteSshError::PoolAtCapacity)?;
    Ok(remove_entries(state, vec![candidate]))
}

fn remove_entries(
    state: &mut PoolState,
    keys: Vec<RemoteSshConnectionKey>,
) -> Vec<Arc<dyn RemoteSshTransportPort>> {
    keys.into_iter()
        .filter_map(|key| state.entries.remove(&key))
        .filter_map(ready_transport)
        .collect()
}

fn ready_transport(entry: PoolEntry) -> Option<Arc<dyn RemoteSshTransportPort>> {
    match entry {
        PoolEntry::Ready { transport, .. } => Some(transport),
        PoolEntry::Connecting { .. } => None,
    }
}

fn snapshot_entry(key: &RemoteSshConnectionKey, entry: &PoolEntry) -> RemoteSshPoolEntrySnapshot {
    match entry {
        PoolEntry::Connecting { .. } => RemoteSshPoolEntrySnapshot {
            key: key.clone(),
            leases: 0,
            health: RemoteSshPoolHealth::Healthy,
        },
        PoolEntry::Ready { leases, health, .. } => RemoteSshPoolEntrySnapshot {
            key: key.clone(),
            leases: *leases,
            health: *health,
        },
    }
}

async fn close_transports(transports: Vec<Arc<dyn RemoteSshTransportPort>>) {
    for transport in transports {
        let _ = transport.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::ssh_connections::domain::{SshAuthMode, SshConnectionTestStatus};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Clock;
    impl RemoteSshPoolClockPort for Clock {
        fn now(&self) -> Instant {
            Instant::now()
        }
    }

    struct Transport {
        closes: Arc<AtomicUsize>,
    }
    #[async_trait]
    impl RemoteSshTransportPort for Transport {
        async fn open_pty(
            &self,
            _: crate::contexts::ssh_connections::domain::runtime::RemotePtyRequest,
        ) -> Result<Arc<dyn super::super::runtime::RemoteSshChannelPort>, RemoteSshError> {
            Err(RemoteSshError::ChannelFailed)
        }
        async fn open_exec(
            &self,
            _: &[u8],
        ) -> Result<Arc<dyn super::super::runtime::RemoteSshChannelPort>, RemoteSshError> {
            Err(RemoteSshError::ChannelFailed)
        }
        async fn keepalive(&self) -> Result<(), RemoteSshError> {
            Ok(())
        }
        async fn close(&self) -> Result<(), RemoteSshError> {
            self.closes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn is_healthy(&self) -> bool {
            true
        }
    }

    struct Connector {
        connects: Arc<AtomicUsize>,
        closes: Arc<AtomicUsize>,
    }
    #[async_trait]
    impl RemoteSshConnectorPort for Connector {
        async fn connect(
            &self,
            _: &SshConnectionProfile,
        ) -> Result<Arc<dyn RemoteSshTransportPort>, RemoteSshError> {
            self.connects.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(Transport {
                closes: self.closes.clone(),
            }))
        }
    }

    fn profile(id: &str, revision: i64) -> SshConnectionProfile {
        SshConnectionProfile {
            id: id.into(),
            name: id.into(),
            host: "host.example".into(),
            port: 22,
            user: "user".into(),
            default_path: "/work".into(),
            auth_mode: SshAuthMode::Password,
            key_path: None,
            credential_ref: Some("ref".into()),
            revision,
            host_trust: None,
            test_status: SshConnectionTestStatus::NotTested,
            last_connected_at: None,
            last_error: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }

    #[tokio::test]
    async fn reuses_compatible_revision_and_isolates_revision_changes() {
        let connects = Arc::new(AtomicUsize::new(0));
        let closes = Arc::new(AtomicUsize::new(0));
        let pool = RemoteSshConnectionPool::new(
            Arc::new(Connector {
                connects: connects.clone(),
                closes: closes.clone(),
            }),
            Arc::new(Clock),
            2,
            Duration::from_secs(60),
        );
        let first = pool.acquire(&profile("one", 1)).await.expect("first");
        let second = pool.acquire(&profile("one", 1)).await.expect("second");
        assert_eq!(connects.load(Ordering::SeqCst), 1);
        drop(first);
        drop(second);
        let changed = pool.acquire(&profile("one", 2)).await.expect("changed");
        assert_eq!(connects.load(Ordering::SeqCst), 2);
        drop(changed);
    }

    #[tokio::test]
    async fn draining_closes_after_last_lease_and_failed_capacity_is_reported() {
        let closes = Arc::new(AtomicUsize::new(0));
        let pool = RemoteSshConnectionPool::new(
            Arc::new(Connector {
                connects: Arc::new(AtomicUsize::new(0)),
                closes: closes.clone(),
            }),
            Arc::new(Clock),
            1,
            Duration::from_secs(60),
        );
        let lease = pool.acquire(&profile("one", 1)).await.expect("lease");
        pool.drain("one");
        drop(lease);
        tokio::task::yield_now().await;
        assert!(closes.load(Ordering::SeqCst) >= 1);
    }
}
