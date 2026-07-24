use super::super::domain::runtime::{
    HostKeyChallenge, HostKeyEvidence, RemotePtyRequest, RemoteSshChannelEvent,
};
use super::super::domain::SshConnectionProfile;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HostKeyVerification {
    Accepted,
    Challenge(HostKeyChallenge),
}

pub(crate) trait RemoteSshHostKeyVerifierPort: Send + Sync {
    fn verify(
        &self,
        profile: &SshConnectionProfile,
        evidence: &HostKeyEvidence,
    ) -> Result<HostKeyVerification, RemoteSshError>;
}

#[async_trait]
pub(crate) trait RemoteSshConnectorPort: Send + Sync {
    async fn connect(
        &self,
        profile: &SshConnectionProfile,
    ) -> Result<Arc<dyn RemoteSshTransportPort>, RemoteSshError>;
}

#[async_trait]
pub(crate) trait RemoteSshTransportPort: Send + Sync {
    async fn open_pty(
        &self,
        request: RemotePtyRequest,
    ) -> Result<Arc<dyn RemoteSshChannelPort>, RemoteSshError>;

    async fn open_exec(
        &self,
        command: &[u8],
    ) -> Result<Arc<dyn RemoteSshChannelPort>, RemoteSshError>;

    async fn keepalive(&self) -> Result<(), RemoteSshError>;
    async fn close(&self) -> Result<(), RemoteSshError>;
    fn is_healthy(&self) -> bool;
}

#[async_trait]
pub(crate) trait RemoteSshChannelPort: Send + Sync {
    async fn write(&self, content: &[u8]) -> Result<(), RemoteSshError>;
    async fn resize(&self, request: RemotePtyRequest) -> Result<(), RemoteSshError>;
    async fn next_event(&self) -> Result<Option<RemoteSshChannelEvent>, RemoteSshError>;
    async fn close(&self) -> Result<(), RemoteSshError>;
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum RemoteSshError {
    #[error("SSH profile is missing required authentication material.")]
    MissingAuthenticationMaterial,
    #[error("SSH host identity must be confirmed before authentication.")]
    HostKeyRequired(HostKeyChallenge),
    #[error("SSH host identity verification failed.")]
    HostKeyVerificationFailed,
    #[error("SSH authentication failed.")]
    AuthenticationFailed,
    #[error("SSH connection could not be established.")]
    ConnectionFailed,
    #[error("SSH connection timed out.")]
    ConnectionTimedOut,
    #[error("SSH transport is no longer available.")]
    TransportClosed,
    #[error("SSH channel operation failed.")]
    ChannelFailed,
    #[error("SSH profile was not found.")]
    ProfileNotFound,
    #[error("SSH profile changed before host trust was confirmed.")]
    StaleProfile,
    #[error("SSH host-key challenge was not found.")]
    TrustChallengeNotFound,
    #[error("SSH host-key confirmation did not match the pending challenge.")]
    TrustConfirmationMismatch,
    #[error("SSH host trust could not be persisted.")]
    TrustPersistenceFailed,
    #[error("SSH connection pool has no idle capacity.")]
    PoolAtCapacity,
}
