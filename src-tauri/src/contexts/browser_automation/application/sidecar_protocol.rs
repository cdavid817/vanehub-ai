use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

pub(crate) const BROWSER_SIDECAR_PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BrowserSidecarLimits {
    pub(crate) max_message_bytes: usize,
    pub(crate) request_timeout: Duration,
    pub(crate) max_restart_attempts: u8,
}

impl Default for BrowserSidecarLimits {
    fn default() -> Self {
        Self {
            max_message_bytes: 1024 * 1024,
            request_timeout: Duration::from_secs(10),
            max_restart_attempts: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct BrowserSidecarRequest {
    pub(crate) protocol_version: u16,
    pub(crate) request_id: String,
    pub(crate) method: String,
    pub(crate) params: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct BrowserSidecarResponse {
    pub(crate) protocol_version: u16,
    pub(crate) request_id: String,
    pub(crate) ok: bool,
    pub(crate) result: Option<Value>,
    pub(crate) error_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserSidecarError {
    InvalidLimits,
    SpawnFailed,
    MessageTooLarge,
    ProtocolMismatch,
    RequestMismatch,
    MalformedMessage,
    Timeout,
    ProcessExited,
    HealthCheckFailed,
    RestartLimitExceeded,
    ShutdownFailed,
}

pub(crate) trait BrowserSidecarTransport: Send {
    fn request(
        &mut self,
        request: &BrowserSidecarRequest,
        limits: BrowserSidecarLimits,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError>;

    fn shutdown(&mut self, timeout: Duration) -> Result<(), BrowserSidecarError>;
}

pub(crate) trait BrowserSidecarFactory: Send + Sync {
    fn spawn(
        &self,
        limits: BrowserSidecarLimits,
    ) -> Result<Box<dyn BrowserSidecarTransport>, BrowserSidecarError>;
}

pub(crate) struct BrowserSidecarSupervisor {
    limits: BrowserSidecarLimits,
    factory: Arc<dyn BrowserSidecarFactory>,
    transport: Option<Box<dyn BrowserSidecarTransport>>,
    restart_attempts: u8,
    next_request_id: u64,
}

impl BrowserSidecarSupervisor {
    pub(crate) fn new(
        limits: BrowserSidecarLimits,
        factory: Arc<dyn BrowserSidecarFactory>,
    ) -> Result<Self, BrowserSidecarError> {
        validate_limits(limits)?;
        Ok(Self {
            limits,
            factory,
            transport: None,
            restart_attempts: 0,
            next_request_id: 1,
        })
    }

    pub(crate) fn start(&mut self) -> Result<(), BrowserSidecarError> {
        if self.transport.is_some() {
            return self.health_check();
        }
        self.spawn_and_handshake()
    }

    pub(crate) fn health_check(&mut self) -> Result<(), BrowserSidecarError> {
        let response = self.request_once("health", Value::Null)?;
        if response.ok
            && response
                .result
                .as_ref()
                .and_then(|value| value.get("status"))
                .and_then(Value::as_str)
                == Some("ready")
        {
            Ok(())
        } else {
            Err(BrowserSidecarError::HealthCheckFailed)
        }
    }

    pub(crate) fn request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError> {
        if self.transport.is_none() {
            self.spawn_and_handshake()?;
        }
        match self.request_once(method, params.clone()) {
            Ok(response) => Ok(response),
            Err(error) if is_restartable(error) => {
                self.restart()?;
                self.request_once(method, params)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn shutdown(&mut self) -> Result<(), BrowserSidecarError> {
        match self.transport.as_mut() {
            Some(transport) => {
                let result = transport.shutdown(self.limits.request_timeout);
                self.transport = None;
                result
            }
            None => Ok(()),
        }
    }

    fn spawn_and_handshake(&mut self) -> Result<(), BrowserSidecarError> {
        let transport = self.factory.spawn(self.limits)?;
        self.transport = Some(transport);
        let response = self.request_once(
            "handshake",
            serde_json::json!({"protocol_version": BROWSER_SIDECAR_PROTOCOL_VERSION}),
        )?;
        let version = response
            .result
            .as_ref()
            .and_then(|value| value.get("protocol_version"))
            .and_then(Value::as_u64);
        if !response.ok || version != Some(u64::from(BROWSER_SIDECAR_PROTOCOL_VERSION)) {
            self.transport = None;
            return Err(BrowserSidecarError::ProtocolMismatch);
        }
        self.health_check()
    }

    fn restart(&mut self) -> Result<(), BrowserSidecarError> {
        if self.restart_attempts >= self.limits.max_restart_attempts {
            return Err(BrowserSidecarError::RestartLimitExceeded);
        }
        self.restart_attempts += 1;
        if let Some(transport) = self.transport.as_mut() {
            let _ = transport.shutdown(self.limits.request_timeout);
        }
        self.transport = None;
        self.spawn_and_handshake()
    }

    fn request_once(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError> {
        let request_id = format!("browser-request-{}", self.next_request_id);
        self.next_request_id = self.next_request_id.saturating_add(1);
        let request = BrowserSidecarRequest {
            protocol_version: BROWSER_SIDECAR_PROTOCOL_VERSION,
            request_id: request_id.clone(),
            method: method.to_string(),
            params,
        };
        let response = self
            .transport
            .as_mut()
            .ok_or(BrowserSidecarError::ProcessExited)?
            .request(&request, self.limits)?;
        if response.protocol_version != BROWSER_SIDECAR_PROTOCOL_VERSION {
            return Err(BrowserSidecarError::ProtocolMismatch);
        }
        if response.request_id != request_id {
            return Err(BrowserSidecarError::RequestMismatch);
        }
        Ok(response)
    }
}

impl Drop for BrowserSidecarSupervisor {
    fn drop(&mut self) {
        if let Some(transport) = self.transport.as_mut() {
            let _ = transport.shutdown(self.limits.request_timeout);
        }
    }
}

fn validate_limits(limits: BrowserSidecarLimits) -> Result<(), BrowserSidecarError> {
    if !(1024..=4 * 1024 * 1024).contains(&limits.max_message_bytes)
        || limits.request_timeout.is_zero()
        || limits.request_timeout > Duration::from_secs(60)
        || limits.max_restart_attempts > 3
    {
        return Err(BrowserSidecarError::InvalidLimits);
    }
    Ok(())
}

fn is_restartable(error: BrowserSidecarError) -> bool {
    matches!(
        error,
        BrowserSidecarError::Timeout
            | BrowserSidecarError::ProcessExited
            | BrowserSidecarError::MalformedMessage
    )
}

#[cfg(test)]
#[path = "sidecar_protocol_tests.rs"]
mod tests;
