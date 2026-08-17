use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LocalEndpointVerificationRequest, LocalModelDiscoveryPort,
    LocalModelEndpointCandidate, OnePieceDiscoveredModel,
};
use crate::platform::network::blocking_no_redirect_http_client;
use serde_json::Value;
use std::io::Read;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const MAX_RESPONSE_BYTES: u64 = 256 * 1024;
const DISCOVERY_TIMEOUT_MS: u64 = 1_500;
const _: () = assert!(MAX_RESPONSE_BYTES <= 256 * 1024);
const _: () = assert!(DISCOVERY_TIMEOUT_MS <= 1_500);
const ALLOWLIST: [(&str, &str); 5] = [
    ("ollama", "http://127.0.0.1:11434"),
    ("lm-studio", "http://127.0.0.1:1234"),
    ("vllm", "http://127.0.0.1:8000"),
    ("sglang", "http://127.0.0.1:30000"),
    ("openai-compatible", "http://127.0.0.1:8080"),
];

pub(crate) struct HttpLocalModelDiscoveryAdapter;

impl LocalModelDiscoveryPort for HttpLocalModelDiscoveryAdapter {
    fn discover_loopback(
        &self,
    ) -> Result<Vec<LocalModelEndpointCandidate>, AgentRuntimeApplicationError> {
        let (sender, receiver) = mpsc::channel();
        std::thread::scope(|scope| {
            for (kind, base_url) in ALLOWLIST {
                let sender = sender.clone();
                scope.spawn(move || {
                    let _ = sender.send(probe(kind, base_url, DISCOVERY_TIMEOUT_MS));
                });
            }
        });
        drop(sender);
        let mut endpoints: Vec<_> = receiver.into_iter().filter_map(Result::ok).collect();
        endpoints.sort_by(|left, right| left.base_url.cmp(&right.base_url));
        Ok(endpoints)
    }

    fn verify_endpoint(
        &self,
        request: LocalEndpointVerificationRequest,
    ) -> Result<LocalModelEndpointCandidate, AgentRuntimeApplicationError> {
        validate_manual_base_url(&request.base_url)?;
        probe("openai-compatible", &request.base_url, request.timeout_ms)
    }
}

fn probe(
    kind: &str,
    base_url: &str,
    timeout_ms: u64,
) -> Result<LocalModelEndpointCandidate, AgentRuntimeApplicationError> {
    if !(100..=120_000).contains(&timeout_ms) {
        return Err(error("invalid timeout"));
    }
    let base_url = base_url.trim_end_matches('/');
    let url = if kind == "ollama" {
        format!("{base_url}/api/tags")
    } else if base_url.ends_with("/v1") {
        format!("{base_url}/models")
    } else {
        format!("{base_url}/v1/models")
    };
    let client = blocking_no_redirect_http_client(Duration::from_millis(timeout_ms))
        .map_err(|_| error("client unavailable"))?;
    let started = Instant::now();
    let response = client.get(url).send().map_err(|_| error("unavailable"))?;
    if response.status().is_redirection() {
        return Err(error("redirect rejected"));
    }
    if !response.status().is_success() {
        return Err(error("metadata request rejected"));
    }
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES)
    {
        return Err(error("response too large"));
    }
    let mut body = Vec::new();
    response
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|_| error("response unreadable"))?;
    if body.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(error("response too large"));
    }
    Ok(LocalModelEndpointCandidate {
        service_kind: kind.to_string(),
        base_url: base_url.to_string(),
        models: parse_model_list(&body)?,
        latency_bucket: latency_bucket(started.elapsed()),
    })
}

fn validate_manual_base_url(value: &str) -> Result<(), AgentRuntimeApplicationError> {
    let value = value.trim();
    let rest = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .ok_or_else(|| error("unsupported URL scheme"))?;
    let authority = rest.split('/').next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') || authority.chars().any(char::is_whitespace)
    {
        return Err(error("unsafe endpoint URL"));
    }
    Ok(())
}

fn parse_model_list(
    body: &[u8],
) -> Result<Vec<OnePieceDiscoveredModel>, AgentRuntimeApplicationError> {
    let value: Value = serde_json::from_slice(body).map_err(|_| error("malformed metadata"))?;
    let entries = value
        .get("data")
        .or_else(|| value.get("models"))
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .ok_or_else(|| error("unsupported model list"))?;
    let mut models = Vec::new();
    for entry in entries {
        let id = entry
            .get("id")
            .or_else(|| entry.get("name"))
            .or_else(|| entry.get("model"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        if !id.is_empty()
            && !models
                .iter()
                .any(|model: &OnePieceDiscoveredModel| model.id == id)
        {
            models.push(OnePieceDiscoveredModel {
                id: id.to_string(),
                display_name: id.to_string(),
            });
        }
    }
    models.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(models)
}

fn latency_bucket(duration: Duration) -> String {
    match duration.as_millis() {
        0..=99 => "under-100ms",
        100..=499 => "100-499ms",
        500..=1_499 => "500-1499ms",
        _ => "1500ms-or-more",
    }
    .to_string()
}

fn error(category: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(format!("Local model probe failed: {category}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc::Receiver;

    fn server(status: &str, body: Vec<u8>, delay: Duration) -> (String, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake local endpoint");
        let address = listener.local_addr().expect("local address");
        let (sender, receiver) = mpsc::channel();
        let status = status.to_string();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept probe");
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .expect("read timeout");
            let mut request = [0_u8; 4096];
            let size = stream.read(&mut request).unwrap_or(0);
            let request = String::from_utf8_lossy(&request[..size]).to_string();
            let _ = sender.send(request);
            std::thread::sleep(delay);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(&body);
        });
        (format!("http://{address}"), receiver)
    }

    #[test]
    fn parses_openai_ollama_and_array_variations_deterministically() {
        let openai = parse_model_list(br#"{"data":[{"id":"b"},{"id":"a"}]}"#).expect("openai list");
        let ollama = parse_model_list(br#"{"models":[{"name":"qwen"}]}"#).expect("ollama list");
        let array = parse_model_list(br#"[{"model":"vllm-model"}]"#).expect("array list");
        assert_eq!(
            openai
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!(ollama[0].id, "qwen");
        assert_eq!(array[0].id, "vllm-model");
    }

    #[test]
    fn manual_openai_base_with_v1_does_not_duplicate_the_version_segment() {
        let (base_url, request) = server(
            "200 OK",
            br#"{"data":[{"id":"local-model"}]}"#.to_vec(),
            Duration::ZERO,
        );
        let candidate = probe("openai-compatible", &format!("{base_url}/v1"), 1_000)
            .expect("probe versioned base URL");
        assert_eq!(candidate.models[0].id, "local-model");
        assert!(request
            .recv()
            .expect("captured request")
            .starts_with("GET /v1/models "));
    }

    #[test]
    fn manual_url_rejects_credentials_and_non_http_schemes() {
        assert!(validate_manual_base_url("file:///tmp/models").is_err());
        assert!(validate_manual_base_url("http://user:secret@127.0.0.1:8000").is_err());
        assert!(validate_manual_base_url("https://inference.corp.test/v1").is_ok());
    }

    #[test]
    fn automatic_allowlist_is_loopback_only_and_bounded() {
        assert_eq!(ALLOWLIST.len(), 5);
        assert!(ALLOWLIST
            .iter()
            .all(|(_, url)| url.starts_with("http://127.0.0.1:")));
    }

    #[test]
    fn fake_openai_and_ollama_servers_return_metadata_without_task_content() {
        for (kind, body, expected_path) in [
            (
                "openai-compatible",
                br#"{"data":[{"id":"local-model"}]}"#.to_vec(),
                "/v1/models",
            ),
            (
                "ollama",
                br#"{"models":[{"name":"ollama-model"}]}"#.to_vec(),
                "/api/tags",
            ),
        ] {
            let (base_url, request) = server("200 OK", body, Duration::ZERO);
            let result = probe(kind, &base_url, 1_000).expect("metadata probe");
            assert_eq!(result.models.len(), 1);
            let request = request.recv().expect("captured request");
            assert!(request.starts_with(&format!("GET {expected_path} ")));
            assert!(!request.to_ascii_lowercase().contains("authorization:"));
            assert!(!request.contains("SECRET_SOURCE_MARKER"));
        }
    }

    #[test]
    fn timeout_malformed_oversized_redirect_and_missing_models_are_bounded() {
        let (slow, _) = server(
            "200 OK",
            br#"{"data":[]}"#.to_vec(),
            Duration::from_millis(250),
        );
        assert!(probe("openai-compatible", &slow, 100).is_err());

        for (status, body) in [
            ("200 OK", b"not-json".to_vec()),
            ("200 OK", vec![b'x'; MAX_RESPONSE_BYTES as usize + 1]),
            ("302 Found", Vec::new()),
            ("200 OK", br#"{"ready":true}"#.to_vec()),
        ] {
            let (base_url, _) = server(status, body, Duration::ZERO);
            assert!(probe("openai-compatible", &base_url, 1_000).is_err());
        }
    }
}
