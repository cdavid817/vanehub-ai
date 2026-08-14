use crate::contexts::web_research::application::{
    SearchHttpPort, SearchHttpResponse, SearchProviderError, SearchRequest, SearchResponse,
    SearchResult, SearchSafeMode, SearchTransportError,
};
use regex::Regex;
use reqwest::blocking::Client;
use std::io::Read;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use url::Url;

const ENDPOINT: &str = "https://html.duckduckgo.com/html/";
const PROVIDER: &str = "duckduckgo";
const MAX_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;
const USER_AGENT: &str = "VaneHub-AI/1.0 (+https://vanehub.ai)";

pub(crate) struct DuckDuckGoSearchAdapter {
    http: Arc<dyn SearchHttpPort>,
}

impl DuckDuckGoSearchAdapter {
    pub(crate) fn new(http: Arc<dyn SearchHttpPort>) -> Self {
        Self { http }
    }

    pub(crate) fn search(
        &self,
        request: SearchRequest,
        captured_at: String,
    ) -> Result<SearchResponse, SearchProviderError> {
        validate_request(&request)?;
        if request.cancelled.load(Ordering::Acquire) {
            return Err(SearchProviderError::Cancelled);
        }
        let timeout = request
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or(SearchProviderError::DeadlineExceeded)?;
        let fields = vec![
            ("q", request.query.clone()),
            ("kl", request.locale.clone()),
            ("kp", safe_parameter(request.safe_mode).to_owned()),
            ("t", "vanehub-ai".to_owned()),
        ];
        let response = self
            .http
            .post_form(ENDPOINT, &fields, timeout, request.cancelled.clone())
            .map_err(map_transport_error)?;
        if request.cancelled.load(Ordering::Acquire) {
            return Err(SearchProviderError::Cancelled);
        }
        match response.status {
            200 => {}
            429 => return Err(SearchProviderError::RateLimited),
            400..=499 => return Err(SearchProviderError::ProviderRejected),
            _ => return Err(SearchProviderError::Network),
        }
        let mut results = parse_results(&response.body)?;
        let truncated = results.len() > usize::from(request.count);
        results.truncate(usize::from(request.count));
        for (index, result) in results.iter_mut().enumerate() {
            result.rank = u8::try_from(index + 1).unwrap_or(u8::MAX);
        }
        Ok(SearchResponse {
            contract_version: 1,
            provider: PROVIDER.to_owned(),
            query: request.query,
            captured_at,
            results,
            truncated,
        })
    }
}

pub(crate) struct ReqwestSearchHttpAdapter;

impl SearchHttpPort for ReqwestSearchHttpAdapter {
    fn post_form(
        &self,
        endpoint: &str,
        fields: &[(&str, String)],
        timeout: Duration,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<SearchHttpResponse, SearchTransportError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(SearchTransportError::Cancelled);
        }
        let client = Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|_| SearchTransportError::Network)?;
        let encoded = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(fields.iter().map(|(key, value)| (*key, value.as_str())))
            .finish();
        let response = client
            .post(endpoint)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(encoded)
            .send()
            .map_err(|error| {
                if error.is_timeout() {
                    SearchTransportError::Timeout
                } else {
                    SearchTransportError::Network
                }
            })?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES)
        {
            return Err(SearchTransportError::ResponseTooLarge);
        }
        let status = response.status().as_u16();
        let mut body = Vec::new();
        response
            .take(MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut body)
            .map_err(|_| SearchTransportError::Network)?;
        if body.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(SearchTransportError::ResponseTooLarge);
        }
        let body = String::from_utf8(body).map_err(|_| SearchTransportError::Network)?;
        Ok(SearchHttpResponse { status, body })
    }
}

fn validate_request(request: &SearchRequest) -> Result<(), SearchProviderError> {
    let locale = Regex::new(r"^(?:[a-z]{2}-[a-z]{2}|wt-wt)$")
        .map_err(|_| SearchProviderError::ProviderProtocolChanged)?;
    if request.query.trim().is_empty()
        || request.query.chars().count() > 256
        || !locale.is_match(&request.locale)
        || !(1..=10).contains(&request.count)
    {
        return Err(SearchProviderError::InvalidRequest);
    }
    Ok(())
}

fn safe_parameter(mode: SearchSafeMode) -> &'static str {
    match mode {
        SearchSafeMode::Strict => "1",
        SearchSafeMode::Moderate => "-1",
        SearchSafeMode::Off => "-2",
    }
}

fn parse_results(body: &str) -> Result<Vec<SearchResult>, SearchProviderError> {
    let block = Regex::new(r#"(?s)<div[^>]+class="[^"]*result[^"]*"[^>]*>(.*?)</div>\s*</div>"#)
        .map_err(|_| SearchProviderError::ProviderProtocolChanged)?;
    let link =
        Regex::new(r#"(?s)<a[^>]+class="[^"]*result__a[^"]*"[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#)
            .map_err(|_| SearchProviderError::ProviderProtocolChanged)?;
    let snippet = Regex::new(r#"(?s)<a[^>]+class="[^"]*result__snippet[^"]*"[^>]*>(.*?)</a>"#)
        .map_err(|_| SearchProviderError::ProviderProtocolChanged)?;
    let tags =
        Regex::new(r"(?s)<[^>]*>").map_err(|_| SearchProviderError::ProviderProtocolChanged)?;
    let mut results = Vec::new();
    for capture in block.captures_iter(body) {
        let Some(content) = capture.get(1).map(|value| value.as_str()) else {
            continue;
        };
        let Some(link_capture) = link.captures(content) else {
            continue;
        };
        let raw_url = link_capture
            .get(1)
            .map(|value| value.as_str())
            .unwrap_or_default();
        let raw_title = link_capture
            .get(2)
            .map(|value| value.as_str())
            .unwrap_or_default();
        let raw_snippet = snippet
            .captures(content)
            .and_then(|value| value.get(1))
            .map(|value| value.as_str())
            .unwrap_or_default();
        results.push(SearchResult {
            rank: 0,
            title: normalize_text(&tags.replace_all(raw_title, " ")),
            url: normalize_result_url(raw_url)?,
            snippet: normalize_text(&tags.replace_all(raw_snippet, " ")),
            provider: PROVIDER.to_owned(),
            evidence_kind: "provider_snippet".to_owned(),
        });
    }
    if results.is_empty() && body.contains("result__") {
        return Err(SearchProviderError::ProviderProtocolChanged);
    }
    Ok(results)
}

fn normalize_result_url(raw: &str) -> Result<String, SearchProviderError> {
    let decoded = if raw.starts_with("//duckduckgo.com/l/") || raw.starts_with("/l/") {
        let absolute = if raw.starts_with("//") {
            format!("https:{raw}")
        } else {
            format!("https://duckduckgo.com{raw}")
        };
        let redirect =
            Url::parse(&absolute).map_err(|_| SearchProviderError::ProviderProtocolChanged)?;
        redirect
            .query_pairs()
            .find(|(key, _)| key == "uddg")
            .map(|(_, value)| value.into_owned())
            .ok_or(SearchProviderError::ProviderProtocolChanged)?
    } else {
        decode_entities(raw)
    };
    let url = Url::parse(&decoded).map_err(|_| SearchProviderError::ProviderProtocolChanged)?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(SearchProviderError::ProviderProtocolChanged);
    }
    Ok(url.to_string())
}

fn normalize_text(value: &str) -> String {
    decode_entities(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn map_transport_error(error: SearchTransportError) -> SearchProviderError {
    match error {
        SearchTransportError::Timeout => SearchProviderError::Timeout,
        SearchTransportError::Cancelled => SearchProviderError::Cancelled,
        SearchTransportError::ResponseTooLarge => SearchProviderError::ResponseTooLarge,
        SearchTransportError::Network => SearchProviderError::Network,
    }
}

#[cfg(test)]
#[path = "duckduckgo_search_tests.rs"]
mod tests;
