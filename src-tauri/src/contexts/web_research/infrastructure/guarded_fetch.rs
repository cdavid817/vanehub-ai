use crate::contexts::web_research::application::{
    FetchHttpPort, FetchHttpRequest, FetchHttpResponse, FetchTransportError, GuardedUrlPolicyError,
    UrlResolverPort,
};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use std::collections::BTreeMap;
use std::io::Read;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::atomic::Ordering;

const USER_AGENT: &str = "VaneHub-AI/1.0 (+https://vanehub.ai)";

#[derive(Debug, Default)]
pub(crate) struct SystemUrlResolver;

impl UrlResolverPort for SystemUrlResolver {
    fn resolve(&self, host: &str, port: u16) -> Result<Vec<String>, GuardedUrlPolicyError> {
        (host, port)
            .to_socket_addrs()
            .map(|addresses| addresses.map(|address| address.ip().to_string()).collect())
            .map_err(|_| GuardedUrlPolicyError::ResolutionFailed)
    }
}

#[derive(Debug, Default)]
pub(crate) struct ReqwestFetchHttpAdapter;

impl FetchHttpPort for ReqwestFetchHttpAdapter {
    fn get(&self, request: &FetchHttpRequest) -> Result<FetchHttpResponse, FetchTransportError> {
        if request.cancelled.load(Ordering::Acquire) {
            return Err(FetchTransportError::Cancelled);
        }
        let addrs = request
            .resolution
            .addresses
            .iter()
            .map(|address| {
                address
                    .parse::<IpAddr>()
                    .map(|address| SocketAddr::new(address, request.resolution.port))
                    .map_err(|_| FetchTransportError::Network)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let client = isolated_client(request, &addrs)?;
        let mut response = client
            .get(&request.resolution.normalized_url)
            .header(
                reqwest::header::ACCEPT,
                "text/html, text/plain, application/xhtml+xml, application/json, application/xml",
            )
            .header(reqwest::header::ACCEPT_ENCODING, "gzip, deflate, identity")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .map_err(map_reqwest_error)?;
        if content_length_exceeds(&response, request.max_compressed_bytes) {
            return Err(FetchTransportError::ResponseTooLarge);
        }
        let status = response.status().as_u16();
        let headers = admitted_headers(&response);
        let mut compressed_body = Vec::new();
        let mut bounded = response
            .by_ref()
            .take(request.max_compressed_bytes.saturating_add(1));
        bounded
            .read_to_end(&mut compressed_body)
            .map_err(|_| FetchTransportError::Network)?;
        if request.cancelled.load(Ordering::Acquire) {
            return Err(FetchTransportError::Cancelled);
        }
        if compressed_body.len() as u64 > request.max_compressed_bytes {
            return Err(FetchTransportError::ResponseTooLarge);
        }
        Ok(FetchHttpResponse {
            status,
            headers,
            compressed_body,
        })
    }
}

fn isolated_client(
    request: &FetchHttpRequest,
    addresses: &[SocketAddr],
) -> Result<Client, FetchTransportError> {
    Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .referer(false)
        .timeout(request.timeout)
        .connect_timeout(request.timeout)
        .resolve_to_addrs(&request.resolution.host, addresses)
        .build()
        .map_err(|_| FetchTransportError::Network)
}

fn content_length_exceeds(response: &reqwest::blocking::Response, limit: u64) -> bool {
    response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > limit)
}

fn admitted_headers(response: &reqwest::blocking::Response) -> BTreeMap<String, String> {
    ["location", "content-type", "content-encoding"]
        .into_iter()
        .filter_map(|name| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(|value| (name.to_string(), value.to_string()))
        })
        .collect()
}

fn map_reqwest_error(error: reqwest::Error) -> FetchTransportError {
    if error.is_timeout() {
        FetchTransportError::Timeout
    } else {
        FetchTransportError::Network
    }
}
