#![allow(dead_code, clippy::result_large_err)]

use super::application::connection_pool::{
    RemoteSshConnectionPool, RemoteSshLease, RemoteSshPoolHealth,
};
use super::application::host_trust::SshHostTrustService;
use super::application::runtime::{RemoteSshChannelPort, RemoteSshError};
use super::application::{
    SshConnectionApplicationService, SshConnectionError, SshConnectionMutation,
    SshConnectionTestResult,
};
use super::domain::runtime::{HostKeyChallenge, RemotePtyRequest};
use super::domain::SshConnectionProfile;
use std::sync::Arc;

pub(crate) use super::application::runtime::RemoteSshError as SshRuntimeError;
pub(crate) use super::application::SshConnectionError as SshConnectionsError;
pub(crate) use super::application::SshConnectionMutation as SaveSshConnectionRequest;
pub(crate) use super::domain::runtime::HostKeyChallengeKind;
pub(crate) use super::domain::runtime::RemoteSshChannelEvent as SshExecutionChannelEvent;
pub(crate) use super::domain::{SshAuthMode, SshConnectionTestStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SshExecutionPoolHealth {
    Healthy,
    Draining,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SshExecutionPoolSnapshot {
    pub(crate) connection_id: String,
    pub(crate) revision: i64,
    pub(crate) leases: usize,
    pub(crate) health: SshExecutionPoolHealth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SshExecutionProfile {
    pub(crate) connection_id: String,
    pub(crate) revision: i64,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) host_trusted: bool,
    pub(crate) credential_configured: bool,
}

pub(crate) struct SshExecutionLease {
    lease: RemoteSshLease,
}

impl SshExecutionLease {
    pub(crate) fn connection_id(&self) -> &str {
        &self.lease.key().connection_id
    }

    pub(crate) fn revision(&self) -> i64 {
        self.lease.key().revision
    }

    pub(crate) fn is_healthy(&self) -> bool {
        self.lease.transport().is_healthy()
    }

    pub(crate) async fn keepalive(&self) -> Result<(), RemoteSshError> {
        self.lease.transport().keepalive().await
    }

    pub(crate) async fn open_exec(
        &self,
        command: &[u8],
    ) -> Result<SshExecutionChannel, RemoteSshError> {
        self.lease
            .transport()
            .open_exec(command)
            .await
            .map(SshExecutionChannel::new)
    }

    pub(crate) async fn open_pty(
        &self,
        columns: u16,
        rows: u16,
    ) -> Result<SshExecutionChannel, RemoteSshError> {
        self.lease
            .transport()
            .open_pty(RemotePtyRequest::bounded(columns, rows))
            .await
            .map(SshExecutionChannel::new)
    }
}

#[derive(Clone)]
pub(crate) struct SshExecutionChannel {
    channel: Arc<dyn RemoteSshChannelPort>,
}

impl SshExecutionChannel {
    fn new(channel: Arc<dyn RemoteSshChannelPort>) -> Self {
        Self { channel }
    }

    pub(crate) async fn write(&self, content: &[u8]) -> Result<(), RemoteSshError> {
        self.channel.write(content).await
    }

    pub(crate) async fn resize(&self, columns: u16, rows: u16) -> Result<(), RemoteSshError> {
        self.channel
            .resize(RemotePtyRequest::bounded(columns, rows))
            .await
    }

    pub(crate) async fn next_event(
        &self,
    ) -> Result<Option<SshExecutionChannelEvent>, RemoteSshError> {
        self.channel.next_event().await
    }

    pub(crate) async fn close(&self) -> Result<(), RemoteSshError> {
        self.channel.close().await
    }
}

#[derive(Clone)]
pub(crate) struct SshConnectionsApi {
    service: SshConnectionApplicationService,
    host_trust: SshHostTrustService,
    pool: RemoteSshConnectionPool,
}

impl SshConnectionsApi {
    pub(crate) fn new(
        service: SshConnectionApplicationService,
        host_trust: SshHostTrustService,
        pool: RemoteSshConnectionPool,
    ) -> Self {
        Self {
            service,
            host_trust,
            pool,
        }
    }

    pub(crate) fn list(&self) -> Result<Vec<SshConnectionProfile>, SshConnectionError> {
        self.service.list()
    }

    pub(crate) fn create(
        &self,
        mutation: SshConnectionMutation,
    ) -> Result<SshConnectionProfile, SshConnectionError> {
        self.service.create(mutation)
    }

    pub(crate) fn update(
        &self,
        id: &str,
        mutation: SshConnectionMutation,
    ) -> Result<SshConnectionProfile, SshConnectionError> {
        let result = self.service.update(id, mutation);
        if result.is_ok() {
            self.pool.drain(id);
        }
        result
    }

    pub(crate) fn delete(&self, id: &str) -> Result<(), SshConnectionError> {
        let result = self.service.delete(id);
        if result.is_ok() {
            self.pool.drain(id);
        }
        result
    }

    pub(crate) fn test(&self, id: &str) -> Result<SshConnectionTestResult, SshConnectionError> {
        self.service.test(id)
    }

    pub(crate) fn pending_host_key(&self, id: &str) -> Option<HostKeyChallenge> {
        self.host_trust.pending_for(id)
    }

    pub(crate) fn confirm_host_key(
        &self,
        id: &str,
        revision: i64,
        fingerprint: &str,
    ) -> Result<HostKeyChallenge, RemoteSshError> {
        self.host_trust.confirm(id, revision, fingerprint)
    }

    pub(crate) async fn acquire(
        &self,
        profile: &SshConnectionProfile,
    ) -> Result<RemoteSshLease, RemoteSshError> {
        self.pool.acquire(profile).await
    }

    pub(crate) async fn acquire_execution(
        &self,
        connection_id: &str,
        revision: i64,
    ) -> Result<SshExecutionLease, RemoteSshError> {
        let profile = self
            .service
            .find(connection_id)
            .map_err(|_| RemoteSshError::ConnectionFailed)?
            .ok_or(RemoteSshError::ProfileNotFound)?;
        if profile.revision != revision {
            return Err(RemoteSshError::StaleProfile);
        }
        self.pool
            .acquire(&profile)
            .await
            .map(|lease| SshExecutionLease { lease })
    }

    pub(crate) fn execution_profile(
        &self,
        connection_id: &str,
    ) -> Result<SshExecutionProfile, RemoteSshError> {
        let profile = self
            .service
            .find(connection_id)
            .map_err(|_| RemoteSshError::ConnectionFailed)?
            .ok_or(RemoteSshError::ProfileNotFound)?;
        Ok(SshExecutionProfile {
            connection_id: profile.id,
            revision: profile.revision,
            host: profile.host,
            port: profile.port,
            user: profile.user,
            host_trusted: profile.host_trust.is_some(),
            credential_configured: profile.credential_ref.is_some() || profile.key_path.is_some(),
        })
    }

    pub(crate) fn execution_pool_snapshot(&self) -> Vec<SshExecutionPoolSnapshot> {
        self.pool
            .snapshot()
            .into_iter()
            .map(|snapshot| SshExecutionPoolSnapshot {
                connection_id: snapshot.key.connection_id,
                revision: snapshot.key.revision,
                leases: snapshot.leases,
                health: match snapshot.health {
                    RemoteSshPoolHealth::Healthy => SshExecutionPoolHealth::Healthy,
                    RemoteSshPoolHealth::Draining => SshExecutionPoolHealth::Draining,
                    RemoteSshPoolHealth::Failed => SshExecutionPoolHealth::Failed,
                },
            })
            .collect()
    }

    pub(crate) fn drain(&self, id: &str) {
        self.pool.drain(id);
    }

    pub(crate) async fn shutdown(&self) -> Result<(), RemoteSshError> {
        self.pool.shutdown().await
    }
}
