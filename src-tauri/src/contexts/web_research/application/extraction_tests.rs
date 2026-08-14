use super::*;

fn fetched(media_type: &str, body: &[u8]) -> FetchBody {
    FetchBody {
        normalized_url: "https://example.com/start".to_string(),
        final_url: "https://example.com/final".to_string(),
        media_type: media_type.to_string(),
        bytes: body.to_vec(),
        redirect_count: 1,
    }
}

#[test]
fn html_extraction_omits_active_content_and_preserves_fetch_provenance() {
    let page = WebContentExtractor::extract(
        fetched(
            "text/html",
            br#"<!doctype html><html><head><title>  Safe &amp; Useful </title>
            <style>.secret { display: none }</style><script>stealCookies()</script></head>
            <body><main><h1>Heading</h1><p>Hello <strong>world</strong>.</p>
            <noscript>fallback script instructions</noscript></main></body></html>"#,
        ),
        ExtractionLimits::default(),
    )
    .expect("fixture HTML should extract");

    assert_eq!(page.contract_version, 1);
    assert_eq!(page.provider, "guarded_http");
    assert_eq!(page.evidence_kind, "fetched_content");
    assert_eq!(page.normalized_url, "https://example.com/start");
    assert_eq!(page.final_url, "https://example.com/final");
    assert_eq!(page.title.as_deref(), Some("Safe & Useful"));
    assert!(page.text.contains("Heading\nHello world."));
    assert!(!page.text.contains("stealCookies"));
    assert!(!page.text.contains("display: none"));
    assert!(!page.text.contains("fallback script"));
    assert!(!page.captured_at.is_empty());
}

#[test]
fn plain_text_is_normalized_and_truncated_by_unicode_characters() {
    let page = WebContentExtractor::extract(
        fetched("text/plain", " 一  二\n\n三四五六 ".as_bytes()),
        ExtractionLimits { max_text_chars: 5 },
    )
    .expect("fixture text should extract");

    assert_eq!(page.text, "一 二\n三");
    assert!(page.truncated);
    assert_eq!(page.title, None);
}

#[test]
fn invalid_encoding_media_and_caller_relaxed_limits_fail_closed() {
    assert_eq!(
        WebContentExtractor::extract(
            fetched("text/plain", &[0xff, 0xfe]),
            ExtractionLimits::default(),
        ),
        Err(ExtractionError::InvalidEncoding)
    );
    assert_eq!(
        WebContentExtractor::extract(
            fetched("application/javascript", b"alert(1)"),
            ExtractionLimits::default(),
        ),
        Err(ExtractionError::UnsupportedMediaType)
    );
    assert_eq!(
        WebContentExtractor::extract(
            fetched("text/plain", b"hello"),
            ExtractionLimits {
                max_text_chars: CONTROLLER_MAX_TEXT_CHARS + 1,
            },
        ),
        Err(ExtractionError::InvalidLimit)
    );
}

#[test]
fn search_snippet_evidence_remains_distinct_from_fetched_content() {
    let search = crate::contexts::web_research::application::SearchResult {
        rank: 1,
        title: "Provider title".to_string(),
        url: "https://example.com".to_string(),
        snippet: "Provider supplied words".to_string(),
        provider: "duckduckgo".to_string(),
        evidence_kind: "provider_snippet".to_string(),
    };
    let fetched = WebContentExtractor::extract(
        fetched("text/plain", b"Origin bytes"),
        ExtractionLimits::default(),
    )
    .expect("fixture text should extract");

    assert_eq!(search.evidence_kind, "provider_snippet");
    assert_eq!(fetched.evidence_kind, "fetched_content");
    assert_ne!(search.provider, fetched.provider);
}
