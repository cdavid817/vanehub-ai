use crate::contexts::ssh_connections::application::runtime::{
    HostKeyVerification, RemoteSshConnectorPort, RemoteSshError, RemoteSshHostKeyVerifierPort,
    RemoteSshTransportPort,
};
use crate::contexts::ssh_connections::application::SshConnectionCredentialPort;
use crate::contexts::ssh_connections::domain::runtime::HostKeyEvidence;
use crate::contexts::ssh_connections::domain::{SshAuthMode, SshConnectionProfile};
use async_trait::async_trait;
use russh::client;
use russh::keys::{load_secret_key, HashAlg, PrivateKeyWithHashAlg};
use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;

use super::russh_channel::RusshTransport;
use crate::contexts::workspaces::domain::{
    REMOTE_TERMINAL_CONNECT_TIMEOUT_SECONDS, REMOTE_TERMINAL_KEEPALIVE_SECONDS,
};

#[derive(Clone)]
pub(crate) struct RusshSshConnector {
    credentials: Arc<dyn SshConnectionCredentialPort>,
    host_keys: Arc<dyn RemoteSshHostKeyVerifierPort>,
}

impl RusshSshConnector {
    pub(crate) fn new(
        credentials: Arc<dyn SshConnectionCredentialPort>,
        host_keys: Arc<dyn RemoteSshHostKeyVerifierPort>,
    ) -> Self {
        Self {
            credentials,
            host_keys,
        }
    }
}

#[derive(Clone)]
pub(super) struct HostCheckingHandler {
    profile: SshConnectionProfile,
    verifier: Arc<dyn RemoteSshHostKeyVerifierPort>,
    rejection: Arc<Mutex<Option<RemoteSshError>>>,
}

impl client::Handler for HostCheckingHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        let evidence = HostKeyEvidence::new(
            server_public_key.algorithm().as_str(),
            server_public_key.fingerprint(HashAlg::Sha256).to_string(),
        );
        let decision = evidence
            .map_err(|_| RemoteSshError::HostKeyVerificationFailed)
            .and_then(|evidence| self.verifier.verify(&self.profile, &evidence));
        match decision {
            Ok(HostKeyVerification::Accepted) => Ok(true),
            Ok(HostKeyVerification::Challenge(challenge)) => {
                record_rejection(&self.rejection, RemoteSshError::HostKeyRequired(challenge));
                Ok(false)
            }
            Err(error) => {
                record_rejection(&self.rejection, error);
                Ok(false)
            }
        }
    }
}

fn record_rejection(slot: &Mutex<Option<RemoteSshError>>, error: RemoteSshError) {
    if let Ok(mut rejection) = slot.lock() {
        *rejection = Some(error);
    }
}

#[async_trait]
impl RemoteSshConnectorPort for RusshSshConnector {
    async fn connect(
        &self,
        profile: &SshConnectionProfile,
    ) -> Result<Arc<dyn RemoteSshTransportPort>, RemoteSshError> {
        let rejection = Arc::new(Mutex::new(None));
        let handler = HostCheckingHandler {
            profile: profile.clone(),
            verifier: self.host_keys.clone(),
            rejection: rejection.clone(),
        };
        let connect = client::connect(
            Arc::new(client_config()),
            (profile.host.as_str(), profile.port),
            handler,
        );
        let mut handle = match timeout(
            Duration::from_secs(REMOTE_TERMINAL_CONNECT_TIMEOUT_SECONDS),
            connect,
        )
        .await
        {
            Ok(Ok(handle)) => handle,
            Ok(Err(_)) => {
                return Err(take_rejection(&rejection).unwrap_or(RemoteSshError::ConnectionFailed));
            }
            Err(_) => return Err(RemoteSshError::ConnectionTimedOut),
        };
        authenticate(&mut handle, profile, self.credentials.as_ref()).await?;
        Ok(Arc::new(RusshTransport {
            handle: Arc::new(handle),
        }))
    }
}

fn take_rejection(slot: &Mutex<Option<RemoteSshError>>) -> Option<RemoteSshError> {
    slot.lock().ok().and_then(|mut rejection| rejection.take())
}

async fn authenticate(
    handle: &mut client::Handle<HostCheckingHandler>,
    profile: &SshConnectionProfile,
    credentials: &dyn SshConnectionCredentialPort,
) -> Result<(), RemoteSshError> {
    let result = match profile.auth_mode {
        SshAuthMode::Password => {
            let password = credentials
                .load(&profile.id)
                .map_err(|_| RemoteSshError::MissingAuthenticationMaterial)?
                .ok_or(RemoteSshError::MissingAuthenticationMaterial)?;
            handle
                .authenticate_password(profile.user.clone(), password.as_str())
                .await
                .map_err(|_| RemoteSshError::AuthenticationFailed)?
        }
        SshAuthMode::Key => {
            let path = profile
                .key_path
                .clone()
                .filter(|path| !path.trim().is_empty())
                .ok_or(RemoteSshError::MissingAuthenticationMaterial)?;
            let private_key = tokio::task::spawn_blocking(move || load_secret_key(path, None))
                .await
                .map_err(|_| RemoteSshError::MissingAuthenticationMaterial)?
                .map_err(|_| RemoteSshError::MissingAuthenticationMaterial)?;
            let hash = handle
                .best_supported_rsa_hash()
                .await
                .map_err(|_| RemoteSshError::AuthenticationFailed)?
                .flatten();
            handle
                .authenticate_publickey(
                    profile.user.clone(),
                    PrivateKeyWithHashAlg::new(Arc::new(private_key), hash),
                )
                .await
                .map_err(|_| RemoteSshError::AuthenticationFailed)?
        }
    };
    result
        .success()
        .then_some(())
        .ok_or(RemoteSshError::AuthenticationFailed)
}

fn client_config() -> client::Config {
    use russh::keys::{Algorithm, EcdsaCurve};
    client::Config {
        preferred: russh::Preferred {
            kex: Cow::Owned(vec![
                russh::kex::MLKEM768X25519_SHA256,
                russh::kex::CURVE25519,
                russh::kex::CURVE25519_PRE_RFC_8731,
                russh::kex::EXTENSION_SUPPORT_AS_CLIENT,
                russh::kex::EXTENSION_OPENSSH_STRICT_KEX_AS_CLIENT,
            ]),
            key: Cow::Owned(vec![
                Algorithm::Ed25519,
                Algorithm::Ecdsa {
                    curve: EcdsaCurve::NistP256,
                },
                Algorithm::Ecdsa {
                    curve: EcdsaCurve::NistP384,
                },
                Algorithm::Ecdsa {
                    curve: EcdsaCurve::NistP521,
                },
                Algorithm::Rsa {
                    hash: Some(HashAlg::Sha512),
                },
                Algorithm::Rsa {
                    hash: Some(HashAlg::Sha256),
                },
            ]),
            cipher: Cow::Owned(vec![
                russh::cipher::CHACHA20_POLY1305,
                russh::cipher::AES_256_GCM,
                russh::cipher::AES_256_CTR,
                russh::cipher::AES_128_CTR,
            ]),
            mac: Cow::Owned(vec![
                russh::mac::HMAC_SHA512_ETM,
                russh::mac::HMAC_SHA256_ETM,
                russh::mac::HMAC_SHA512,
                russh::mac::HMAC_SHA256,
            ]),
            compression: Cow::Owned(vec![russh::compression::NONE]),
        },
        keepalive_interval: Some(Duration::from_secs(REMOTE_TERMINAL_KEEPALIVE_SECONDS)),
        keepalive_max: 3,
        nodelay: true,
        ..client::Config::default()
    }
}
