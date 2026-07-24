#![allow(clippy::result_large_err)]

use super::runtime::{HostKeyVerification, RemoteSshError, RemoteSshHostKeyVerifierPort};
use super::{SshConnectionClock, SshConnectionRepository};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::ssh_connections::domain::runtime::{
    HostKeyChallenge, HostKeyChallengeKind, HostKeyEvidence, RemoteSshConnectionKey,
};
use crate::contexts::ssh_connections::domain::{SshConnectionProfile, SshHostTrustMetadata};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct SshHostTrustService {
    repository: Arc<dyn SshConnectionRepository>,
    clock: Arc<dyn SshConnectionClock>,
    logging: Arc<dyn DiagnosticLogPort>,
    pending: Arc<Mutex<HashMap<RemoteSshConnectionKey, HostKeyChallenge>>>,
}

impl SshHostTrustService {
    pub(crate) fn new(
        repository: Arc<dyn SshConnectionRepository>,
        clock: Arc<dyn SshConnectionClock>,
        logging: Arc<dyn DiagnosticLogPort>,
    ) -> Self {
        Self {
            repository,
            clock,
            logging,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn pending_for(&self, connection_id: &str) -> Option<HostKeyChallenge> {
        self.pending.lock().ok().and_then(|pending| {
            pending
                .values()
                .filter(|challenge| challenge.connection_key.connection_id == connection_id)
                .max_by_key(|challenge| challenge.connection_key.revision)
                .cloned()
        })
    }

    pub(crate) fn confirm(
        &self,
        connection_id: &str,
        revision: i64,
        fingerprint: &str,
    ) -> Result<HostKeyChallenge, RemoteSshError> {
        let key = RemoteSshConnectionKey {
            connection_id: connection_id.to_string(),
            revision,
        };
        let challenge = self
            .pending
            .lock()
            .map_err(|_| RemoteSshError::TrustPersistenceFailed)?
            .get(&key)
            .cloned()
            .ok_or(RemoteSshError::TrustChallengeNotFound)?;
        if challenge.evidence.fingerprint != fingerprint {
            return Err(RemoteSshError::TrustConfirmationMismatch);
        }
        let mut profile = self
            .repository
            .find(connection_id)
            .map_err(|_| RemoteSshError::TrustPersistenceFailed)?
            .ok_or(RemoteSshError::ProfileNotFound)?;
        ensure_current_profile(&profile, &challenge)?;
        profile.host_trust = Some(SshHostTrustMetadata {
            host: challenge.host.clone(),
            port: challenge.port,
            algorithm: challenge.evidence.algorithm.clone(),
            fingerprint: challenge.evidence.fingerprint.clone(),
            confirmed_at: self.clock.now(),
        });
        self.repository
            .update(&profile)
            .map_err(|_| RemoteSshError::TrustPersistenceFailed)?;
        self.pending
            .lock()
            .map_err(|_| RemoteSshError::TrustPersistenceFailed)?
            .remove(&key);
        self.write_log(
            LogSeverity::Info,
            "remote-terminal.host-key.confirmed",
            "SSH host key trust was confirmed.",
            &challenge,
        );
        Ok(challenge)
    }

    fn challenge(
        &self,
        profile: &SshConnectionProfile,
        evidence: &HostKeyEvidence,
    ) -> Result<HostKeyChallenge, RemoteSshError> {
        let (kind, previous_fingerprint) = match &profile.host_trust {
            Some(trust) => (
                HostKeyChallengeKind::Changed,
                Some(trust.fingerprint.clone()),
            ),
            None => (HostKeyChallengeKind::FirstSeen, None),
        };
        let challenge = HostKeyChallenge {
            connection_key: RemoteSshConnectionKey::from(profile),
            host: profile.host.clone(),
            port: profile.port,
            kind,
            evidence: evidence.clone(),
            previous_fingerprint,
        };
        challenge
            .validate()
            .map_err(|_| RemoteSshError::HostKeyVerificationFailed)?;
        let stored = {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| RemoteSshError::HostKeyVerificationFailed)?;
            let entry = pending
                .entry(challenge.connection_key.clone())
                .or_insert_with(|| challenge.clone());
            if entry.evidence != challenge.evidence {
                *entry = challenge;
            }
            entry.clone()
        };
        self.write_log(
            LogSeverity::Warn,
            "remote-terminal.host-key.challenge",
            "SSH host key confirmation is required.",
            &stored,
        );
        Ok(stored)
    }

    fn write_log(
        &self,
        severity: LogSeverity,
        category: &str,
        message: &str,
        challenge: &HostKeyChallenge,
    ) {
        let context = BTreeMap::from([
            (
                "connectionId".to_string(),
                challenge.connection_key.connection_id.clone(),
            ),
            (
                "revision".to_string(),
                challenge.connection_key.revision.to_string(),
            ),
            ("host".to_string(), challenge.host.clone()),
            ("port".to_string(), challenge.port.to_string()),
            (
                "algorithm".to_string(),
                challenge.evidence.algorithm.clone(),
            ),
            (
                "fingerprint".to_string(),
                challenge.evidence.fingerprint.clone(),
            ),
        ]);
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity,
            category: category.to_string(),
            message: message.to_string(),
            context,
        });
    }
}

impl RemoteSshHostKeyVerifierPort for SshHostTrustService {
    fn verify(
        &self,
        profile: &SshConnectionProfile,
        evidence: &HostKeyEvidence,
    ) -> Result<HostKeyVerification, RemoteSshError> {
        let trusted = profile.host_trust.as_ref().is_some_and(|trust| {
            trust.host == profile.host
                && trust.port == profile.port
                && trust.algorithm == evidence.algorithm
                && trust.fingerprint == evidence.fingerprint
        });
        if trusted {
            if let Ok(mut pending) = self.pending.lock() {
                pending.remove(&RemoteSshConnectionKey::from(profile));
            }
            Ok(HostKeyVerification::Accepted)
        } else {
            self.challenge(profile, evidence)
                .map(HostKeyVerification::Challenge)
        }
    }
}

fn ensure_current_profile(
    profile: &SshConnectionProfile,
    challenge: &HostKeyChallenge,
) -> Result<(), RemoteSshError> {
    if profile.revision != challenge.connection_key.revision
        || profile.host != challenge.host
        || profile.port != challenge.port
    {
        Err(RemoteSshError::StaleProfile)
    } else {
        Ok(())
    }
}
