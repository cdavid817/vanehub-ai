use super::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Mutex;

#[derive(Debug)]
struct PublicResolver;

impl UrlResolverPort for PublicResolver {
    fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<String>, GuardedUrlPolicyError> {
        Ok(vec!["93.184.216.34".to_owned()])
    }
}

#[derive(Debug)]
struct FixtureHttp {
    responses: Mutex<VecDeque<Result<FetchHttpResponse, FetchTransportError>>>,
    seen: Mutex<Vec<FetchHttpRequest>>,
}

impl FixtureHttp {
    fn new(responses: Vec<Result<FetchHttpResponse, FetchTransportError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            seen: Mutex::new(Vec::new()),
        }
    }
}

impl FetchHttpPort for FixtureHttp {
    fn get(&self, request: &FetchHttpRequest) -> Result<FetchHttpResponse, FetchTransportError> {
        self.seen
            .lock()
            .map_err(|_| FetchTransportError::Network)?
            .push(request.clone());
        self.responses
            .lock()
            .map_err(|_| FetchTransportError::Network)?
            .pop_front()
            .unwrap_or(Err(FetchTransportError::Network))
    }
}

fn response(
    status: u16,
    headers: &[(&str, &str)],
    body: Vec<u8>,
) -> Result<FetchHttpResponse, FetchTransportError> {
    Ok(FetchHttpResponse {
        status,
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
        compressed_body: body,
    })
}

fn request(url: &str, limits: FetchLimits) -> FetchRequest {
    FetchRequest {
        url: url.to_string(),
        limits,
        cancelled: Arc::new(AtomicBool::new(false)),
    }
}

#[test]
fn redirects_are_explicitly_followed_and_gzip_is_bounded_and_decoded() {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    assert!(encoder.write_all(b"hello from fetched page").is_ok());
    let compressed = encoder.finish().unwrap_or_default();
    let http = Arc::new(FixtureHttp::new(vec![
        response(302, &[("location", "https://www.example.org/page")], vec![]),
        response(
            200,
            &[
                ("content-type", "text/html; charset=utf-8"),
                ("content-encoding", "gzip"),
            ],
            compressed,
        ),
    ]));
    let service = GuardedFetchService::new(Arc::new(PublicResolver), http.clone());

    let result = service
        .fetch(request(
            "https://example.com/start#fragment",
            FetchLimits::default(),
        ))
        .expect("fixture fetch should succeed");

    assert_eq!(result.normalized_url, "https://example.com/start");
    assert_eq!(result.final_url, "https://www.example.org/page");
    assert_eq!(result.redirect_count, 1);
    assert_eq!(result.media_type, "text/html");
    assert_eq!(result.bytes, b"hello from fetched page");
    let seen = http.seen.lock().expect("fixture lock should be available");
    assert_eq!(seen.len(), 2);
    assert!(seen
        .iter()
        .all(|item| item.timeout <= Duration::from_secs(15)));
    assert!(seen
        .iter()
        .all(|item| item.max_compressed_bytes == 2 * 1024 * 1024));
}

#[test]
fn compressed_and_expanded_limits_fail_closed() {
    let limits = FetchLimits {
        max_compressed_bytes: 8,
        max_expanded_bytes: 16,
        ..FetchLimits::default()
    };
    let oversized_transport = Arc::new(FixtureHttp::new(vec![Err(
        FetchTransportError::ResponseTooLarge,
    )]));
    let service = GuardedFetchService::new(Arc::new(PublicResolver), oversized_transport);
    assert_eq!(
        service.fetch(request("https://example.com", limits)),
        Err(FetchError::CompressedBodyTooLarge)
    );

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    assert!(encoder.write_all(&[b'a'; 128]).is_ok());
    let compressed = encoder.finish().unwrap_or_default();
    let limits = FetchLimits {
        max_compressed_bytes: compressed.len() as u64,
        max_expanded_bytes: 16,
        ..FetchLimits::default()
    };
    let http = Arc::new(FixtureHttp::new(vec![response(
        200,
        &[("content-type", "text/plain"), ("content-encoding", "gzip")],
        compressed,
    )]));
    let service = GuardedFetchService::new(Arc::new(PublicResolver), http);
    assert_eq!(
        service.fetch(request("https://example.com", limits)),
        Err(FetchError::ExpandedBodyTooLarge)
    );
}

#[test]
fn unsupported_media_encoding_redirect_overflow_and_cancellation_are_stable() {
    let unsupported = Arc::new(FixtureHttp::new(vec![response(
        200,
        &[("content-type", "application/javascript")],
        b"alert(1)".to_vec(),
    )]));
    let service = GuardedFetchService::new(Arc::new(PublicResolver), unsupported);
    assert_eq!(
        service.fetch(request("https://example.com", FetchLimits::default())),
        Err(FetchError::MediaTypeUnsupported)
    );

    let redirect = || response(302, &[("location", "/again")], vec![]);
    let http = Arc::new(FixtureHttp::new(vec![redirect(), redirect()]));
    let service = GuardedFetchService::new(Arc::new(PublicResolver), http);
    let no_redirects = FetchLimits {
        max_redirects: 0,
        ..FetchLimits::default()
    };
    assert_eq!(
        service.fetch(request("https://example.com", no_redirects)),
        Err(FetchError::RedirectLimit)
    );

    let cancelled = Arc::new(AtomicBool::new(true));
    let service =
        GuardedFetchService::new(Arc::new(PublicResolver), Arc::new(FixtureHttp::new(vec![])));
    assert_eq!(
        service.fetch(FetchRequest {
            url: "https://example.com".to_string(),
            limits: FetchLimits::default(),
            cancelled,
        }),
        Err(FetchError::Cancelled)
    );
}

#[test]
fn controller_owned_limit_ceiling_cannot_be_relaxed_by_a_caller() {
    let service =
        GuardedFetchService::new(Arc::new(PublicResolver), Arc::new(FixtureHttp::new(vec![])));
    let excessive = FetchLimits {
        max_compressed_bytes: CONTROLLER_MAX_COMPRESSED_BYTES + 1,
        ..FetchLimits::default()
    };
    assert_eq!(
        service.fetch(request("https://example.com", excessive)),
        Err(FetchError::InvalidLimits)
    );
}
