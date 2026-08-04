use crate::contexts::tooling::mcp::application::McpRuntimeError;
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use thiserror::Error;

const NO_FAILURE: u8 = 0;
const TIMEOUT_FAILURE: u8 = 1;
const CANCELLED_FAILURE: u8 = 2;
const PROTOCOL_FAILURE: u8 = 3;
const HTTP_FAILURE: u8 = 4;
const LIMIT_FAILURE: u8 = 5;
const TRANSPORT_FAILURE: u8 = 6;

#[derive(Clone, Default)]
pub(super) struct LegacySseStatus {
    failure: Arc<AtomicU8>,
}

impl LegacySseStatus {
    pub(super) fn failure(&self) -> Option<McpFailureCode> {
        match self.failure.load(Ordering::Acquire) {
            TIMEOUT_FAILURE => Some(McpFailureCode::Timeout),
            CANCELLED_FAILURE => Some(McpFailureCode::Cancelled),
            PROTOCOL_FAILURE => Some(McpFailureCode::Protocol),
            HTTP_FAILURE => Some(McpFailureCode::UpstreamHttp),
            LIMIT_FAILURE => Some(McpFailureCode::LimitExceeded),
            TRANSPORT_FAILURE => Some(McpFailureCode::Transport),
            _ => None,
        }
    }

    pub(super) fn record(&self, error: &LegacySseError) {
        let value = match error {
            LegacySseError::Timeout => TIMEOUT_FAILURE,
            LegacySseError::Cancelled => CANCELLED_FAILURE,
            LegacySseError::Protocol => PROTOCOL_FAILURE,
            LegacySseError::UpstreamHttp => HTTP_FAILURE,
            LegacySseError::LimitExceeded => LIMIT_FAILURE,
            LegacySseError::Transport => TRANSPORT_FAILURE,
        };
        let _ =
            self.failure
                .compare_exchange(NO_FAILURE, value, Ordering::AcqRel, Ordering::Acquire);
    }
}

#[derive(Debug, Error)]
pub(super) enum LegacySseError {
    #[error("legacy SSE request timed out")]
    Timeout,
    #[error("legacy SSE request was cancelled")]
    Cancelled,
    #[error("legacy SSE protocol failure")]
    Protocol,
    #[error("legacy SSE upstream HTTP failure")]
    UpstreamHttp,
    #[error("legacy SSE response exceeded its limit")]
    LimitExceeded,
    #[error("legacy SSE transport failure")]
    Transport,
}

impl LegacySseError {
    pub(super) fn code(&self) -> McpFailureCode {
        match self {
            Self::Timeout => McpFailureCode::Timeout,
            Self::Cancelled => McpFailureCode::Cancelled,
            Self::Protocol => McpFailureCode::Protocol,
            Self::UpstreamHttp => McpFailureCode::UpstreamHttp,
            Self::LimitExceeded => McpFailureCode::LimitExceeded,
            Self::Transport => McpFailureCode::Transport,
        }
    }
}

pub(super) fn runtime_error(error: LegacySseError) -> McpRuntimeError {
    McpRuntimeError::new(error.code())
}

pub(super) fn reqwest_error(error: reqwest::Error) -> LegacySseError {
    if error.is_timeout() {
        LegacySseError::Timeout
    } else {
        LegacySseError::Transport
    }
}
