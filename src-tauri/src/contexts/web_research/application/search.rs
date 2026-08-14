use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchSafeMode {
    Strict,
    Moderate,
    Off,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchRequest {
    pub(crate) query: String,
    pub(crate) locale: String,
    pub(crate) safe_mode: SearchSafeMode,
    pub(crate) count: u8,
    pub(crate) deadline: Instant,
    pub(crate) cancelled: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchResult {
    pub(crate) rank: u8,
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) snippet: String,
    pub(crate) provider: String,
    pub(crate) evidence_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchResponse {
    pub(crate) contract_version: u16,
    pub(crate) provider: String,
    pub(crate) query: String,
    pub(crate) captured_at: String,
    pub(crate) results: Vec<SearchResult>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchHttpResponse {
    pub(crate) status: u16,
    pub(crate) body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchTransportError {
    Timeout,
    Cancelled,
    ResponseTooLarge,
    Network,
}

pub(crate) trait SearchHttpPort: Send + Sync {
    fn post_form(
        &self,
        endpoint: &str,
        fields: &[(&str, String)],
        timeout: Duration,
        cancelled: Arc<AtomicBool>,
    ) -> Result<SearchHttpResponse, SearchTransportError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchProviderError {
    InvalidRequest,
    Cancelled,
    DeadlineExceeded,
    Timeout,
    RateLimited,
    ProviderRejected,
    ProviderProtocolChanged,
    ResponseTooLarge,
    Network,
}
