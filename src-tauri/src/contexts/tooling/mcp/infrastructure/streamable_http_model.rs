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
const CLEANUP_FAILURE: u8 = 7;

#[derive(Clone, Default)]
pub(super) struct StreamableHttpStatus {
    failure: Arc<AtomicU8>,
}

impl StreamableHttpStatus {
    pub(super) fn failure(&self) -> Option<McpFailureCode> {
        match self.failure.load(Ordering::Acquire) {
            TIMEOUT_FAILURE => Some(McpFailureCode::Timeout),
            CANCELLED_FAILURE => Some(McpFailureCode::Cancelled),
            PROTOCOL_FAILURE => Some(McpFailureCode::Protocol),
            HTTP_FAILURE => Some(McpFailureCode::UpstreamHttp),
            LIMIT_FAILURE => Some(McpFailureCode::LimitExceeded),
            TRANSPORT_FAILURE => Some(McpFailureCode::Transport),
            CLEANUP_FAILURE => Some(McpFailureCode::Cleanup),
            _ => None,
        }
    }

    pub(super) fn record(&self, error: &StreamableHttpError) {
        let value = match error {
            StreamableHttpError::Timeout => TIMEOUT_FAILURE,
            StreamableHttpError::Cancelled => CANCELLED_FAILURE,
            StreamableHttpError::Protocol => PROTOCOL_FAILURE,
            StreamableHttpError::UpstreamHttp => HTTP_FAILURE,
            StreamableHttpError::LimitExceeded => LIMIT_FAILURE,
            StreamableHttpError::Transport => TRANSPORT_FAILURE,
            StreamableHttpError::Cleanup => CLEANUP_FAILURE,
        };
        let _ =
            self.failure
                .compare_exchange(NO_FAILURE, value, Ordering::AcqRel, Ordering::Acquire);
    }
}

#[derive(Debug, Error)]
pub(super) enum StreamableHttpError {
    #[error("Streamable HTTP request timed out")]
    Timeout,
    #[error("Streamable HTTP request was cancelled")]
    Cancelled,
    #[error("Streamable HTTP protocol failure")]
    Protocol,
    #[error("Streamable HTTP upstream failure")]
    UpstreamHttp,
    #[error("Streamable HTTP response exceeded its limit")]
    LimitExceeded,
    #[error("Streamable HTTP transport failure")]
    Transport,
    #[error("Streamable HTTP cleanup failure")]
    Cleanup,
}

impl StreamableHttpError {
    #[cfg(test)]
    pub(super) fn code(&self) -> McpFailureCode {
        match self {
            Self::Timeout => McpFailureCode::Timeout,
            Self::Cancelled => McpFailureCode::Cancelled,
            Self::Protocol => McpFailureCode::Protocol,
            Self::UpstreamHttp => McpFailureCode::UpstreamHttp,
            Self::LimitExceeded => McpFailureCode::LimitExceeded,
            Self::Transport => McpFailureCode::Transport,
            Self::Cleanup => McpFailureCode::Cleanup,
        }
    }
}

pub(super) fn reqwest_error(error: reqwest::Error) -> StreamableHttpError {
    if error.is_timeout() {
        StreamableHttpError::Timeout
    } else {
        StreamableHttpError::Transport
    }
}
