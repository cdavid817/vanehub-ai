use crate::AppError;
use serde::Serialize;
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::process::Command;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

pub const DEFAULT_BYPASS: &str = "localhost,127.0.0.1,::1";

static PROXY_STATE: OnceLock<RwLock<NetworkProxyState>> = OnceLock::new();

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkProxyState {
    pub url: String,
    pub bypass: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkProxyTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedNetworkProxy {
    pub url: String,
    pub proxy_type: String,
    pub port: u16,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestNetworkProxyInput {
    pub url: String,
    pub bypass: String,
}

const PROXY_PORTS: &[(u16, &str, bool)] = &[
    (7890, "http", true),
    (7891, "socks5", false),
    (1080, "socks5", false),
    (8080, "http", false),
    (8888, "http", false),
    (3128, "http", false),
    (10808, "socks5", false),
    (10809, "http", false),
];

fn state_lock() -> &'static RwLock<NetworkProxyState> {
    PROXY_STATE.get_or_init(|| {
        RwLock::new(NetworkProxyState {
            url: String::new(),
            bypass: DEFAULT_BYPASS.to_string(),
        })
    })
}

pub fn current_state() -> NetworkProxyState {
    state_lock()
        .read()
        .map(|state| state.clone())
        .unwrap_or_else(|_| NetworkProxyState {
            url: String::new(),
            bypass: DEFAULT_BYPASS.to_string(),
        })
}

pub fn apply(url: &str, bypass: &str) -> Result<(), AppError> {
    let normalized_url = normalize_proxy_url(url)?;
    let normalized_bypass = normalize_bypass(bypass)?;
    let mut state = state_lock()
        .write()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    state.url = normalized_url;
    state.bypass = normalized_bypass;
    Ok(())
}

pub fn normalize_proxy_url(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if trimmed != value || contains_control(trimmed) {
        return Err(AppError::Validation(
            "Network proxy URL contains invalid characters.".to_string(),
        ));
    }
    validate_proxy_scheme(trimmed)?;
    reqwest::Proxy::all(trimmed).map_err(|error| {
        AppError::Validation(format!(
            "Invalid network proxy URL '{}': {error}",
            mask_proxy_url(trimmed)
        ))
    })?;
    Ok(trimmed.to_string())
}

fn validate_proxy_scheme(value: &str) -> Result<(), AppError> {
    let Some((scheme, rest)) = value.split_once("://") else {
        return Err(AppError::Validation(
            "Network proxy URL must include a supported scheme.".to_string(),
        ));
    };
    if !matches!(scheme, "http" | "https" | "socks5" | "socks5h") {
        return Err(AppError::Validation(format!(
            "Unsupported network proxy scheme '{scheme}'."
        )));
    }
    let host_part = rest.rsplit('@').next().unwrap_or(rest);
    let host = host_part
        .split('/')
        .next()
        .unwrap_or("")
        .trim_matches('[')
        .trim_matches(']');
    if host.is_empty() || host.starts_with(':') {
        return Err(AppError::Validation(
            "Network proxy URL must include a host.".to_string(),
        ));
    }
    Ok(())
}

pub fn normalize_bypass(value: &str) -> Result<String, AppError> {
    if contains_control(value) {
        return Err(AppError::Validation(
            "Network proxy bypass list contains invalid characters.".to_string(),
        ));
    }
    Ok(value
        .split(|character: char| character == ',' || character.is_whitespace())
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>()
        .join(","))
}

fn contains_control(value: &str) -> bool {
    value.chars().any(char::is_control)
}

pub fn mask_proxy_url(value: &str) -> String {
    let Some((scheme, rest)) = value.split_once("://") else {
        return value.to_string();
    };
    let host_part = rest.rsplit('@').next().unwrap_or(rest);
    let host_and_port = host_part.split('/').next().unwrap_or(host_part);
    format!("{scheme}://{host_and_port}")
}

pub fn apply_to_std_command(command: &mut Command) {
    let state = current_state();
    if state.url.is_empty() {
        return;
    }
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command.env(key, &state.url);
    }
    if !state.bypass.is_empty() {
        command.env("NO_PROXY", &state.bypass);
        command.env("no_proxy", &state.bypass);
    }
}

pub fn apply_to_tokio_command(command: &mut tokio::process::Command) {
    let state = current_state();
    if state.url.is_empty() {
        return;
    }
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command.env(key, &state.url);
    }
    if !state.bypass.is_empty() {
        command.env("NO_PROXY", &state.bypass);
        command.env("no_proxy", &state.bypass);
    }
}

pub async fn test_proxy(input: TestNetworkProxyInput) -> Result<NetworkProxyTestResult, AppError> {
    let url = normalize_proxy_url(&input.url)?;
    let _bypass = normalize_bypass(&input.bypass)?;
    if url.is_empty() {
        return Err(AppError::Validation(
            "Network proxy URL is required for testing.".to_string(),
        ));
    }

    let proxy = reqwest::Proxy::all(&url).map_err(|error| {
        AppError::Validation(format!(
            "Invalid network proxy URL '{}': {error}",
            mask_proxy_url(&url)
        ))
    })?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| AppError::LaunchFailed(error.to_string()))?;
    let start = Instant::now();
    let test_urls = [
        "https://httpbin.org/get",
        "https://api.openai.com",
        "https://api.anthropic.com",
    ];
    let mut last_error = None;
    for test_url in test_urls {
        match client.head(test_url).send().await {
            Ok(response) => {
                return Ok(NetworkProxyTestResult {
                    success: true,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: Some(format!("{} {}", test_url, response.status())),
                });
            }
            Err(error) => last_error = Some(error.to_string()),
        }
    }
    Ok(NetworkProxyTestResult {
        success: false,
        latency_ms: start.elapsed().as_millis() as u64,
        error: last_error,
    })
}

pub async fn scan_local() -> Vec<DetectedNetworkProxy> {
    tokio::task::spawn_blocking(|| {
        let mut found = Vec::new();
        for &(port, primary_type, is_mixed) in PROXY_PORTS {
            let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
            if TcpStream::connect_timeout(&addr.into(), Duration::from_millis(100)).is_ok() {
                found.push(DetectedNetworkProxy {
                    url: format!("{primary_type}://127.0.0.1:{port}"),
                    proxy_type: primary_type.to_string(),
                    port,
                });
                if is_mixed {
                    let alt_type = if primary_type == "http" {
                        "socks5"
                    } else {
                        "http"
                    };
                    found.push(DetectedNetworkProxy {
                        url: format!("{alt_type}://127.0.0.1:{port}"),
                        proxy_type: alt_type.to_string(),
                        port,
                    });
                }
            }
        }
        found
    })
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_bypass_list() {
        assert_eq!(
            normalize_bypass(" localhost, 127.0.0.1 ::1 ").expect("bypass"),
            "localhost,127.0.0.1,::1"
        );
    }

    #[test]
    fn rejects_invalid_proxy_scheme() {
        assert!(normalize_proxy_url("ftp://127.0.0.1:7890").is_err());
    }

    #[test]
    fn masks_proxy_credentials() {
        assert_eq!(
            mask_proxy_url("http://user:pass@127.0.0.1:7890"),
            "http://127.0.0.1:7890"
        );
    }
}
