#![allow(dead_code, clippy::result_large_err)]

use super::application::connection_pool::{
    RemoteSshConnectionPool, RemoteSshLease, RemoteSshPoolEntrySnapshot,
};
use super::application::host_trust::SshHostTrustService;
use super::application::runtime::RemoteSshError;
use super::application::{
    SshConnectionApplicationService, SshConnectionError, SshConnectionMutation,
    SshConnectionTestResult,
};
use super::domain::runtime::HostKeyChallenge;
use super::domain::SshConnectionProfile;

pub(crate) use super::application::runtime::RemoteSshError as SshRuntimeError;
pub(crate) use super::application::SshConnectionError as SshConnectionsError;
pub(crate) use super::application::SshConnectionMutation as SaveSshConnectionRequest;
pub(crate) use super::domain::runtime::HostKeyChallengeKind;
pub(crate) use super::domain::{SshAuthMode, SshConnectionTestStatus};

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

    pub(crate) fn pool_snapshot(&self) -> Vec<RemoteSshPoolEntrySnapshot> {
        self.pool.snapshot()
    }

    pub(crate) fn drain(&self, id: &str) {
        self.pool.drain(id);
    }

    pub(crate) async fn shutdown(&self) -> Result<(), RemoteSshError> {
        self.pool.shutdown().await
    }
}
