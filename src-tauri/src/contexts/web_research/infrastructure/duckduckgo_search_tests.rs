use super::*;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

const FIXTURE: &str = r#"
<html><body>
<div class="result results_links results_links_deep web-result">
  <div class="links_main links_deep result__body">
    <h2 class="result__title"><a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fdocs%3Fa%3D1&amp;rut=x">Example &amp; Docs</a></h2>
    <a class="result__snippet">A bounded <b>documentation</b> snippet.</a>
  </div>
</div>
<div class="result results_links results_links_deep web-result">
  <div class="links_main links_deep result__body">
    <h2><a class="result__a" href="https://example.org/second">Second</a></h2>
    <a class="result__snippet">Second snippet.</a>
  </div>
</div>
</body></html>"#;

struct Http {
    response: Result<SearchHttpResponse, SearchTransportError>,
    fields: Mutex<Vec<(String, String)>>,
}

impl SearchHttpPort for Http {
    fn post_form(
        &self,
        endpoint: &str,
        fields: &[(&str, String)],
        _timeout: Duration,
        _cancelled: Arc<AtomicBool>,
    ) -> Result<SearchHttpResponse, SearchTransportError> {
        assert_eq!(endpoint, ENDPOINT);
        self.fields.lock().expect("fields").extend(
            fields
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone())),
        );
        self.response.clone()
    }
}

fn request(count: u8) -> SearchRequest {
    SearchRequest {
        query: "rust safety".to_owned(),
        locale: "us-en".to_owned(),
        safe_mode: SearchSafeMode::Moderate,
        count,
        deadline: Instant::now() + Duration::from_secs(10),
        cancelled: Arc::new(AtomicBool::new(false)),
    }
}

#[test]
fn captured_fixture_is_normalized_bounded_and_provenance_aware() {
    let http = Arc::new(Http {
        response: Ok(SearchHttpResponse {
            status: 200,
            body: FIXTURE.to_owned(),
        }),
        fields: Mutex::new(Vec::new()),
    });
    let adapter = DuckDuckGoSearchAdapter::new(http.clone());
    let response = adapter
        .search(request(1), "100".to_owned())
        .expect("search");

    assert_eq!(response.results.len(), 1);
    assert!(response.truncated);
    assert_eq!(response.results[0].title, "Example & Docs");
    assert_eq!(response.results[0].url, "https://example.com/docs?a=1");
    assert_eq!(response.results[0].evidence_kind, "provider_snippet");
    assert!(http
        .fields
        .lock()
        .expect("fields")
        .contains(&("kp".to_owned(), "-1".to_owned())));
}

#[test]
fn invalid_protocol_rate_limit_timeout_and_cancellation_are_stable() {
    for (response, expected) in [
        (
            Ok(SearchHttpResponse {
                status: 429,
                body: String::new(),
            }),
            SearchProviderError::RateLimited,
        ),
        (
            Err(SearchTransportError::Timeout),
            SearchProviderError::Timeout,
        ),
    ] {
        let adapter = DuckDuckGoSearchAdapter::new(Arc::new(Http {
            response,
            fields: Mutex::new(Vec::new()),
        }));
        assert_eq!(adapter.search(request(2), "100".to_owned()), Err(expected));
    }
    assert_eq!(
        parse_results(r#"<div class="result__changed">invalid</div>"#),
        Err(SearchProviderError::ProviderProtocolChanged)
    );
    let cancelled = request(2);
    cancelled.cancelled.store(true, Ordering::Release);
    let adapter = DuckDuckGoSearchAdapter::new(Arc::new(Http {
        response: Ok(SearchHttpResponse {
            status: 200,
            body: FIXTURE.to_owned(),
        }),
        fields: Mutex::new(Vec::new()),
    }));
    assert_eq!(
        adapter.search(cancelled, "100".to_owned()),
        Err(SearchProviderError::Cancelled)
    );
}
