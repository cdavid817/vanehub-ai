use super::{GuardedUrlPolicy, GuardedUrlPolicyError, PublicUrlResolution, UrlResolverPort};
use flate2::read::{DeflateDecoder, GzDecoder};
use std::collections::BTreeMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const CONTROLLER_MAX_COMPRESSED_BYTES: u64 = 8 * 1024 * 1024;
const CONTROLLER_MAX_EXPANDED_BYTES: u64 = 16 * 1024 * 1024;
const CONTROLLER_MAX_REDIRECTS: u8 = 5;
const CONTROLLER_MAX_DURATION: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FetchLimits {
    pub(crate) max_compressed_bytes: u64,
    pub(crate) max_expanded_bytes: u64,
    pub(crate) max_redirects: u8,
    pub(crate) max_duration: Duration,
}

impl Default for FetchLimits {
    fn default() -> Self {
        Self {
            max_compressed_bytes: 2 * 1024 * 1024,
            max_expanded_bytes: 4 * 1024 * 1024,
            max_redirects: 3,
            max_duration: Duration::from_secs(15),
        }
    }
}

impl FetchLimits {
    fn validate(self) -> Result<Self, FetchError> {
        if self.max_compressed_bytes == 0
            || self.max_compressed_bytes > CONTROLLER_MAX_COMPRESSED_BYTES
            || self.max_expanded_bytes == 0
            || self.max_expanded_bytes > CONTROLLER_MAX_EXPANDED_BYTES
            || self.max_redirects > CONTROLLER_MAX_REDIRECTS
            || self.max_duration.is_zero()
            || self.max_duration > CONTROLLER_MAX_DURATION
        {
            return Err(FetchError::InvalidLimits);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FetchRequest {
    pub(crate) url: String,
    pub(crate) limits: FetchLimits,
    pub(crate) cancelled: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FetchBody {
    pub(crate) normalized_url: String,
    pub(crate) final_url: String,
    pub(crate) media_type: String,
    pub(crate) bytes: Vec<u8>,
    pub(crate) redirect_count: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct FetchHttpRequest {
    pub(crate) resolution: PublicUrlResolution,
    pub(crate) timeout: Duration,
    pub(crate) max_compressed_bytes: u64,
    pub(crate) cancelled: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FetchHttpResponse {
    pub(crate) status: u16,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) compressed_body: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FetchTransportError {
    Timeout,
    Cancelled,
    ResponseTooLarge,
    Network,
}

pub(crate) trait FetchHttpPort: Send + Sync {
    fn get(&self, request: &FetchHttpRequest) -> Result<FetchHttpResponse, FetchTransportError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FetchError {
    InvalidLimits,
    UrlPolicy(GuardedUrlPolicyError),
    Timeout,
    Cancelled,
    Network,
    CompressedBodyTooLarge,
    ExpandedBodyTooLarge,
    RedirectLimit,
    RedirectLocationMissing,
    HttpStatus,
    MediaTypeMissing,
    MediaTypeUnsupported,
    ContentEncodingUnsupported,
    DecompressionFailed,
}

pub(crate) struct GuardedFetchService {
    resolver: Arc<dyn UrlResolverPort>,
    http: Arc<dyn FetchHttpPort>,
}

impl GuardedFetchService {
    pub(crate) fn new(resolver: Arc<dyn UrlResolverPort>, http: Arc<dyn FetchHttpPort>) -> Self {
        Self { resolver, http }
    }

    pub(crate) fn fetch(&self, request: FetchRequest) -> Result<FetchBody, FetchError> {
        let limits = request.limits.validate()?;
        let started = Instant::now();
        let original = GuardedUrlPolicy::resolve_public(&request.url, self.resolver.as_ref())
            .map_err(FetchError::UrlPolicy)?;
        let mut current = original.clone();
        let mut redirects = 0_u8;
        loop {
            ensure_active(&request.cancelled, started, limits.max_duration)?;
            GuardedUrlPolicy::revalidate_resolution(&current, self.resolver.as_ref())
                .map_err(FetchError::UrlPolicy)?;
            let remaining = limits
                .max_duration
                .checked_sub(started.elapsed())
                .ok_or(FetchError::Timeout)?;
            let response = self
                .http
                .get(&FetchHttpRequest {
                    resolution: current.clone(),
                    timeout: remaining,
                    max_compressed_bytes: limits.max_compressed_bytes,
                    cancelled: Arc::clone(&request.cancelled),
                })
                .map_err(map_transport_error)?;
            ensure_active(&request.cancelled, started, limits.max_duration)?;
            if is_redirect(response.status) {
                if redirects >= limits.max_redirects {
                    return Err(FetchError::RedirectLimit);
                }
                let location = response
                    .headers
                    .get("location")
                    .ok_or(FetchError::RedirectLocationMissing)?;
                current =
                    GuardedUrlPolicy::validate_redirect(&current, location, self.resolver.as_ref())
                        .map_err(FetchError::UrlPolicy)?;
                redirects += 1;
                continue;
            }
            if !(200..300).contains(&response.status) {
                return Err(FetchError::HttpStatus);
            }
            if response.compressed_body.len() as u64 > limits.max_compressed_bytes {
                return Err(FetchError::CompressedBodyTooLarge);
            }
            let media_type = admitted_media_type(&response.headers)?;
            let bytes = decode_body(
                &response.compressed_body,
                response.headers.get("content-encoding").map(String::as_str),
                limits.max_expanded_bytes,
            )?;
            return Ok(FetchBody {
                normalized_url: original.normalized_url,
                final_url: current.normalized_url,
                media_type,
                bytes,
                redirect_count: redirects,
            });
        }
    }
}

fn ensure_active(
    cancelled: &AtomicBool,
    started: Instant,
    duration: Duration,
) -> Result<(), FetchError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(FetchError::Cancelled);
    }
    if started.elapsed() >= duration {
        return Err(FetchError::Timeout);
    }
    Ok(())
}

fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn admitted_media_type(headers: &BTreeMap<String, String>) -> Result<String, FetchError> {
    let raw = headers
        .get("content-type")
        .ok_or(FetchError::MediaTypeMissing)?;
    let media_type = raw
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if matches!(
        media_type.as_str(),
        "text/html"
            | "application/xhtml+xml"
            | "text/plain"
            | "application/json"
            | "application/xml"
            | "text/xml"
            | "application/pdf"
            | "image/png"
            | "image/jpeg"
    ) {
        Ok(media_type)
    } else {
        Err(FetchError::MediaTypeUnsupported)
    }
}

fn decode_body(
    compressed: &[u8],
    encoding: Option<&str>,
    max_expanded_bytes: u64,
) -> Result<Vec<u8>, FetchError> {
    let encoding = encoding.unwrap_or("identity").trim().to_ascii_lowercase();
    let reader: Box<dyn Read> = match encoding.as_str() {
        "" | "identity" => Box::new(compressed),
        "gzip" | "x-gzip" => Box::new(GzDecoder::new(compressed)),
        "deflate" => Box::new(DeflateDecoder::new(compressed)),
        _ => return Err(FetchError::ContentEncodingUnsupported),
    };
    let mut bounded = reader.take(max_expanded_bytes.saturating_add(1));
    let mut expanded = Vec::new();
    bounded
        .read_to_end(&mut expanded)
        .map_err(|_| FetchError::DecompressionFailed)?;
    if expanded.len() as u64 > max_expanded_bytes {
        return Err(FetchError::ExpandedBodyTooLarge);
    }
    Ok(expanded)
}

fn map_transport_error(error: FetchTransportError) -> FetchError {
    match error {
        FetchTransportError::Timeout => FetchError::Timeout,
        FetchTransportError::Cancelled => FetchError::Cancelled,
        FetchTransportError::ResponseTooLarge => FetchError::CompressedBodyTooLarge,
        FetchTransportError::Network => FetchError::Network,
    }
}

#[cfg(test)]
#[path = "fetch_tests.rs"]
mod tests;
