use super::transports::http::ReqwestHttpTransport;
use super::transports::wechat::{WeChatAdapter, WeChatCredentials, WeChatQrCode, WeChatQrStatus};
use super::transports::ConnectorRuntimeError;
use crate::contexts::communications::api::{CommunicationsApi, WeChatAuthorizationResult};
use crate::contexts::communications::application::{
    CommunicationsApplicationError, SaveConnectorRequest,
};
use crate::contexts::communications::domain::{
    AuthorizationAttempt, AuthorizationObservation, AuthorizationStatus, ConnectorKind,
};
use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use qrcode::render::svg;
use qrcode::QrCode;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;
use zeroize::Zeroizing;

const AUTHORIZATION_TIMEOUT: StdDuration = StdDuration::from_secs(45);
const AUTHORIZATION_LIFETIME: Duration = Duration::minutes(5);

#[async_trait]
trait WeChatAuthorizationTransport: Send + Sync {
    async fn create_qr_code(&self) -> Result<WeChatQrCode, ConnectorRuntimeError>;
    async fn poll_qr_status(&self, payload: &str) -> Result<WeChatQrStatus, ConnectorRuntimeError>;
}

struct LiveWeChatAuthorizationTransport;

#[async_trait]
impl WeChatAuthorizationTransport for LiveWeChatAuthorizationTransport {
    async fn create_qr_code(&self) -> Result<WeChatQrCode, ConnectorRuntimeError> {
        let transport = ReqwestHttpTransport::new(AUTHORIZATION_TIMEOUT)?;
        WeChatAdapter::create_qr_code(&transport).await
    }

    async fn poll_qr_status(&self, payload: &str) -> Result<WeChatQrStatus, ConnectorRuntimeError> {
        let transport = ReqwestHttpTransport::new(AUTHORIZATION_TIMEOUT)?;
        WeChatAdapter::poll_qr_status(&transport, payload).await
    }
}

struct PendingAuthorization {
    payload: Zeroizing<String>,
    image_data_url: String,
    expires_at: DateTime<Utc>,
    attempt: AuthorizationAttempt,
}

pub(crate) struct WeChatAuthorizationService {
    transport: Arc<dyn WeChatAuthorizationTransport>,
    persistence: Arc<dyn WeChatCredentialPersistence>,
    pending: Mutex<Option<PendingAuthorization>>,
}

#[async_trait]
trait WeChatCredentialPersistence: Send + Sync {
    async fn persist(
        &self,
        credentials: WeChatCredentials,
    ) -> Result<(), CommunicationsApplicationError>;
}

struct ApiWeChatCredentialPersistence {
    communications: CommunicationsApi,
}

#[async_trait]
impl WeChatCredentialPersistence for ApiWeChatCredentialPersistence {
    async fn persist(
        &self,
        credentials: WeChatCredentials,
    ) -> Result<(), CommunicationsApplicationError> {
        let summary = self
            .communications
            .list_connectors()
            .await?
            .into_iter()
            .find(|summary| summary.configuration.kind == ConnectorKind::WeChat)
            .ok_or_else(|| {
                CommunicationsApplicationError::failure("wechat-configuration-missing")
            })?;
        let secret = serde_json::to_string(&json!({
            "botToken": credentials.bot_token,
            "baseUrl": credentials.base_url,
            "botId": credentials.bot_id,
        }))
        .map_err(|_| CommunicationsApplicationError::failure("credential-payload-invalid"))?;
        self.communications
            .save_connector(SaveConnectorRequest {
                kind: ConnectorKind::WeChat,
                enabled: summary.configuration.enabled,
                display_name: summary.configuration.display_name,
                public_config: summary.configuration.public_config,
                replacement_secret: Some(secret),
            })
            .await?;
        Ok(())
    }
}

impl WeChatAuthorizationService {
    pub(crate) fn new(communications: CommunicationsApi) -> Self {
        Self {
            transport: Arc::new(LiveWeChatAuthorizationTransport),
            persistence: Arc::new(ApiWeChatCredentialPersistence { communications }),
            pending: Mutex::new(None),
        }
    }

    #[cfg(test)]
    fn with_parts(
        transport: Arc<dyn WeChatAuthorizationTransport>,
        persistence: Arc<dyn WeChatCredentialPersistence>,
    ) -> Self {
        Self {
            transport,
            persistence,
            pending: Mutex::new(None),
        }
    }

    pub(crate) async fn begin(
        &self,
    ) -> Result<WeChatAuthorizationResult, CommunicationsApplicationError> {
        let qr = self
            .transport
            .create_qr_code()
            .await
            .map_err(runtime_error)?;
        let code = QrCode::new(qr.qr_url.as_bytes())
            .map_err(|_| CommunicationsApplicationError::failure("wechat-qr-invalid"))?;
        let image = code.render::<svg::Color>().min_dimensions(256, 256).build();
        let image_data_url = format!(
            "data:image/svg+xml;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(image.as_bytes())
        );
        let started_at = Utc::now();
        let expires_at = started_at + AUTHORIZATION_LIFETIME;
        let attempt = AuthorizationAttempt::begin(
            started_at.timestamp_millis(),
            expires_at.timestamp_millis(),
        )?;
        *self.pending.lock().map_err(lock_error)? = Some(PendingAuthorization {
            payload: Zeroizing::new(qr.qr_payload),
            image_data_url: image_data_url.clone(),
            expires_at,
            attempt,
        });
        Ok(WeChatAuthorizationResult {
            status: AuthorizationStatus::Waiting.as_str().to_string(),
            image_data_url: Some(image_data_url),
            expires_at: Some(expires_at.to_rfc3339()),
            safe_error_code: None,
        })
    }

    pub(crate) async fn poll(
        &self,
    ) -> Result<WeChatAuthorizationResult, CommunicationsApplicationError> {
        let observed_at = Utc::now();
        let locally_expired = {
            let mut pending = self.pending.lock().map_err(lock_error)?;
            pending
                .as_mut()
                .ok_or_else(not_started)?
                .attempt
                .expire_if_due(observed_at.timestamp_millis())
        };
        if locally_expired {
            self.cancel()?;
            return Ok(WeChatAuthorizationResult {
                status: AuthorizationStatus::Expired.as_str().to_string(),
                image_data_url: None,
                expires_at: None,
                safe_error_code: None,
            });
        }

        let (payload, image_data_url, expires_at, current_status) = {
            let pending = self.pending.lock().map_err(lock_error)?;
            let pending = pending.as_ref().ok_or_else(not_started)?;
            (
                Zeroizing::new(pending.payload.to_string()),
                pending.image_data_url.clone(),
                pending.expires_at,
                pending.attempt.current_status(),
            )
        };
        let observed = self
            .transport
            .poll_qr_status(payload.as_str())
            .await
            .map_err(runtime_error)?;
        let mut result = WeChatAuthorizationResult {
            status: current_status.as_str().to_string(),
            image_data_url: Some(image_data_url),
            expires_at: Some(expires_at.to_rfc3339()),
            safe_error_code: None,
        };
        let (observation, hide_image) = match observed {
            WeChatQrStatus::Waiting => (AuthorizationObservation::Waiting, false),
            WeChatQrStatus::Scanned => (AuthorizationObservation::Scanned, false),
            WeChatQrStatus::Expired => (AuthorizationObservation::Expired, true),
            WeChatQrStatus::Error(code) => (AuthorizationObservation::Failed(code), false),
            WeChatQrStatus::Confirmed(credentials) => {
                self.persistence.persist(credentials).await?;
                (AuthorizationObservation::Confirmed, true)
            }
        };
        let (next_status, safe_error_code) = {
            let mut pending = self.pending.lock().map_err(lock_error)?;
            let pending = pending.as_mut().ok_or_else(not_started)?;
            let next = pending
                .attempt
                .observe(observed_at.timestamp_millis(), observation)?;
            (next, pending.attempt.safe_error_code().map(str::to_string))
        };
        result.status = next_status.as_str().to_string();
        result.safe_error_code = safe_error_code;
        if hide_image {
            result.image_data_url = None;
        }
        if next_status.is_terminal() {
            self.cancel()?;
        }
        Ok(result)
    }

    pub(crate) fn cancel(&self) -> Result<(), CommunicationsApplicationError> {
        let mut pending = self.pending.lock().map_err(lock_error)?;
        if let Some(pending) = pending.as_mut() {
            let _ = pending.attempt.cancel();
        }
        *pending = None;
        Ok(())
    }
}

fn runtime_error(error: ConnectorRuntimeError) -> CommunicationsApplicationError {
    match error.user_message {
        Some(message) => CommunicationsApplicationError::user_visible(error.safe_code, message),
        None => CommunicationsApplicationError::failure(error.safe_code),
    }
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> CommunicationsApplicationError {
    CommunicationsApplicationError::failure("wechat-authorization-lock-failed")
}

fn not_started() -> CommunicationsApplicationError {
    CommunicationsApplicationError::failure("wechat-authorization-not-started")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeTransport;

    #[async_trait]
    impl WeChatAuthorizationTransport for FakeTransport {
        async fn create_qr_code(&self) -> Result<WeChatQrCode, ConnectorRuntimeError> {
            Ok(WeChatQrCode {
                qr_url: "https://example.test/authorize/fixture".to_string(),
                qr_payload: "private-qr-payload".to_string(),
            })
        }

        async fn poll_qr_status(
            &self,
            payload: &str,
        ) -> Result<WeChatQrStatus, ConnectorRuntimeError> {
            assert_eq!(payload, "private-qr-payload");
            Ok(WeChatQrStatus::Confirmed(WeChatCredentials {
                bot_token: "private-bot-token".to_string(),
                base_url: "https://example.test".to_string(),
                bot_id: "bot-1".to_string(),
            }))
        }
    }

    #[derive(Default)]
    struct FakePersistence {
        saved: Mutex<Vec<WeChatCredentials>>,
    }

    #[async_trait]
    impl WeChatCredentialPersistence for FakePersistence {
        async fn persist(
            &self,
            credentials: WeChatCredentials,
        ) -> Result<(), CommunicationsApplicationError> {
            self.saved
                .lock()
                .expect("saved credentials")
                .push(credentials);
            Ok(())
        }
    }

    #[tokio::test]
    async fn authorization_uses_fake_network_and_credential_ports() {
        let persistence = Arc::new(FakePersistence::default());
        let service =
            WeChatAuthorizationService::with_parts(Arc::new(FakeTransport), persistence.clone());

        let started = service.begin().await.expect("begin");
        assert_eq!(started.status, "waiting");
        assert!(started
            .image_data_url
            .as_deref()
            .is_some_and(|value| value.starts_with("data:image/svg+xml;base64,")));

        let confirmed = service.poll().await.expect("confirm");
        assert_eq!(confirmed.status, "confirmed");
        assert!(confirmed.image_data_url.is_none());
        {
            let saved = persistence.saved.lock().expect("saved credentials");
            assert_eq!(saved.len(), 1);
            assert_eq!(saved[0].bot_id, "bot-1");
        }

        let error = service.poll().await.expect_err("terminal state cleared");
        assert_eq!(error.safe_code(), "wechat-authorization-not-started");
    }
}
