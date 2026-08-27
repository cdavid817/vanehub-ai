use base64::Engine;
use serde::Serialize;
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::process::Command;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream as TokioTcpStream;
use tokio_rustls::rustls::{pki_types::ServerName, ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;
use url::Url;

#[derive(Debug, Error)]
pub(crate) enum NetworkError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("launch failed: {0}")]
    LaunchFailed(String),
}

type AppError = NetworkError;

pub trait AsyncIo: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncIo for T {}
pub type BoxedAsyncIo = Box<dyn AsyncIo>;

pub const DEFAULT_BYPASS: &str = "localhost,127.0.0.1,::1";

static PROXY_STATE: OnceLock<RwLock<NetworkProxyState>> = OnceLock::new();
static RUSTLS_PROVIDER_INIT: OnceLock<()> = OnceLock::new();

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

pub fn http_client(timeout: Duration) -> Result<reqwest::Client, AppError> {
    build_routed_client(reqwest::Client::builder().timeout(timeout))
}

pub(crate) fn blocking_http_client(
    timeout: Duration,
) -> Result<reqwest::blocking::Client, AppError> {
    build_routed_client(reqwest::blocking::Client::builder().timeout(timeout))
}

pub(crate) fn blocking_no_redirect_http_client(
    timeout: Duration,
) -> Result<reqwest::blocking::Client, AppError> {
    build_routed_client(
        reqwest::blocking::Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(0)
            .redirect(reqwest::redirect::Policy::none()),
    )
}

pub(crate) fn no_redirect_http_client(timeout: Duration) -> Result<reqwest::Client, AppError> {
    build_routed_client(
        reqwest::Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(0)
            .redirect(reqwest::redirect::Policy::none()),
    )
}

/// `reqwest`'s two builders share no trait, so the routing decision would otherwise be written
/// four times. It was, and all four copies carried the same defect.
pub(crate) trait RoutableClientBuilder: Sized {
    type Client;

    fn route_through(self, proxy: reqwest::Proxy) -> Self;
    fn route_directly(self) -> Self;
    fn finish(self) -> Result<Self::Client, reqwest::Error>;
}

impl RoutableClientBuilder for reqwest::ClientBuilder {
    type Client = reqwest::Client;

    fn route_through(self, proxy: reqwest::Proxy) -> Self {
        self.proxy(proxy)
    }

    fn route_directly(self) -> Self {
        self.no_proxy()
    }

    fn finish(self) -> Result<Self::Client, reqwest::Error> {
        self.build()
    }
}

impl RoutableClientBuilder for reqwest::blocking::ClientBuilder {
    type Client = reqwest::blocking::Client;

    fn route_through(self, proxy: reqwest::Proxy) -> Self {
        self.proxy(proxy)
    }

    fn route_directly(self) -> Self {
        self.no_proxy()
    }

    fn finish(self) -> Result<Self::Client, reqwest::Error> {
        self.build()
    }
}

/// Exposed so callers that need their own timeout, pool, or redirect policy still get the routing
/// decision instead of hand-rolling a builder that silently inherits an OS proxy.
pub(crate) fn apply_proxy_routing<B: RoutableClientBuilder>(builder: B) -> Result<B, AppError> {
    let state = current_state();
    if state.url.is_empty() {
        // Direct connection has to be stated, not merely left unconfigured: a builder that is
        // never told anything lets reqwest discover a proxy from the environment or, on Windows,
        // the Internet Settings registry keys. That routing is invisible in VaneHub's own
        // settings and skips this bypass list entirely, which is how loopback traffic to local
        // sidecars ended up at a proxy. Suppressing discovery also covers the bypass list in this
        // mode, because nothing is proxied at all.
        return Ok(builder.route_directly());
    }
    let proxy = reqwest::Proxy::all(&state.url)
        .map_err(|error| AppError::Validation(format!("Invalid network proxy: {error}")))?
        .no_proxy(reqwest::NoProxy::from_string(&state.bypass));
    Ok(builder.route_through(proxy))
}

fn build_routed_client<B: RoutableClientBuilder>(builder: B) -> Result<B::Client, AppError> {
    apply_proxy_routing(builder)?
        .finish()
        .map_err(|error| AppError::Storage(format!("HTTP client initialization failed: {error}")))
}

pub async fn websocket_stream(target: &Url) -> Result<BoxedAsyncIo, AppError> {
    ensure_rustls_crypto_provider();
    tokio::time::timeout(Duration::from_secs(10), websocket_stream_inner(target))
        .await
        .map_err(|_| AppError::LaunchFailed("WebSocket connection timed out".to_string()))?
}

fn ensure_rustls_crypto_provider() {
    RUSTLS_PROVIDER_INIT.get_or_init(|| {
        // Multiple dependencies enable different rustls providers, so implicit selection panics.
        // Install the provider already used by reqwest/tokio-rustls before WebSocket TLS starts.
        let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

async fn websocket_stream_inner(target: &Url) -> Result<BoxedAsyncIo, AppError> {
    let host = target
        .host_str()
        .ok_or_else(|| AppError::Validation("WebSocket target has no host".to_string()))?;
    let port = target
        .port_or_known_default()
        .ok_or_else(|| AppError::Validation("WebSocket target has no port".to_string()))?;
    let state = current_state();
    if state.url.is_empty() || host_is_bypassed(host, &state.bypass) {
        return Ok(Box::new(
            TokioTcpStream::connect((host, port))
                .await
                .map_err(|error| {
                    AppError::LaunchFailed(format!("WebSocket TCP connection failed: {error}"))
                })?,
        ));
    }
    let proxy = Url::parse(&state.url)
        .map_err(|_| AppError::Validation("Invalid network proxy URL".to_string()))?;
    match proxy.scheme() {
        "http" | "https" => http_connect(&proxy, host, port).await,
        "socks5" | "socks5h" => socks5_connect(&proxy, host, port).await,
        _ => Err(AppError::Validation(
            "Configured proxy scheme is not supported for WebSocket connections".to_string(),
        )),
    }
}

async fn connect_proxy(proxy: &Url) -> Result<BoxedAsyncIo, AppError> {
    let host = proxy
        .host_str()
        .ok_or_else(|| AppError::Validation("Proxy host is missing".to_string()))?;
    let port = proxy
        .port_or_known_default()
        .ok_or_else(|| AppError::Validation("Proxy port is missing".to_string()))?;
    let stream = TokioTcpStream::connect((host, port))
        .await
        .map_err(|error| AppError::LaunchFailed(format!("Proxy connection failed: {error}")))?;
    if proxy.scheme() != "https" {
        return Ok(Box::new(stream));
    }

    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let server_name = ServerName::try_from(host.to_string())
        .map_err(|_| AppError::Validation("HTTPS proxy host is invalid".to_string()))?;
    let stream = TlsConnector::from(std::sync::Arc::new(config))
        .connect(server_name, stream)
        .await
        .map_err(|_| AppError::LaunchFailed("HTTPS proxy TLS handshake failed".to_string()))?;
    Ok(Box::new(stream))
}

async fn http_connect(proxy: &Url, host: &str, port: u16) -> Result<BoxedAsyncIo, AppError> {
    let mut stream = connect_proxy(proxy).await?;
    let authorization = if proxy.username().is_empty() {
        String::new()
    } else {
        let encoded = base64::engine::general_purpose::STANDARD.encode(format!(
            "{}:{}",
            proxy.username(),
            proxy.password().unwrap_or_default()
        ));
        format!("Proxy-Authorization: Basic {encoded}\r\n")
    };
    let request = format!(
        "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n{authorization}Proxy-Connection: Keep-Alive\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|_| AppError::LaunchFailed("Proxy CONNECT request failed".to_string()))?;
    let mut response = Vec::new();
    let mut byte = [0_u8; 1];
    while response.len() < 16_384 && !response.ends_with(b"\r\n\r\n") {
        if stream.read_exact(&mut byte).await.is_err() {
            return Err(AppError::LaunchFailed(
                "Proxy CONNECT response closed".to_string(),
            ));
        }
        response.push(byte[0]);
    }
    let status = String::from_utf8_lossy(&response);
    if !status
        .lines()
        .next()
        .is_some_and(|line| line.contains(" 200 "))
    {
        return Err(AppError::LaunchFailed(
            "Proxy CONNECT was rejected".to_string(),
        ));
    }
    Ok(stream)
}

async fn socks5_connect(proxy: &Url, host: &str, port: u16) -> Result<BoxedAsyncIo, AppError> {
    let mut stream = connect_proxy(proxy).await?;
    let has_auth = !proxy.username().is_empty();
    let methods = if has_auth {
        vec![5, 2, 0, 2]
    } else {
        vec![5, 1, 0]
    };
    stream
        .write_all(&methods)
        .await
        .map_err(proxy_protocol_error)?;
    let mut selection = [0_u8; 2];
    stream
        .read_exact(&mut selection)
        .await
        .map_err(proxy_protocol_error)?;
    if selection[0] != 5 || selection[1] == 0xff {
        return Err(AppError::LaunchFailed(
            "SOCKS5 authentication unavailable".to_string(),
        ));
    }
    if selection[1] == 2 {
        let username = proxy.username().as_bytes();
        let password = proxy.password().unwrap_or_default().as_bytes();
        if username.len() > 255 || password.len() > 255 {
            return Err(AppError::Validation(
                "SOCKS5 credentials are too long".to_string(),
            ));
        }
        let mut auth = vec![1, username.len() as u8];
        auth.extend_from_slice(username);
        auth.push(password.len() as u8);
        auth.extend_from_slice(password);
        stream
            .write_all(&auth)
            .await
            .map_err(proxy_protocol_error)?;
        let mut result = [0_u8; 2];
        stream
            .read_exact(&mut result)
            .await
            .map_err(proxy_protocol_error)?;
        if result[1] != 0 {
            return Err(AppError::LaunchFailed(
                "SOCKS5 authentication failed".to_string(),
            ));
        }
    }
    if host.len() > 255 {
        return Err(AppError::Validation(
            "WebSocket host is too long".to_string(),
        ));
    }
    let mut request = vec![5, 1, 0, 3, host.len() as u8];
    request.extend_from_slice(host.as_bytes());
    request.extend_from_slice(&port.to_be_bytes());
    stream
        .write_all(&request)
        .await
        .map_err(proxy_protocol_error)?;
    let mut header = [0_u8; 4];
    stream
        .read_exact(&mut header)
        .await
        .map_err(proxy_protocol_error)?;
    if header[1] != 0 {
        return Err(AppError::LaunchFailed(format!(
            "SOCKS5 connection rejected ({})",
            header[1]
        )));
    }
    let address_len = match header[3] {
        1 => 4,
        4 => 16,
        3 => {
            let mut length = [0_u8; 1];
            stream
                .read_exact(&mut length)
                .await
                .map_err(proxy_protocol_error)?;
            length[0] as usize
        }
        _ => {
            return Err(AppError::LaunchFailed(
                "SOCKS5 response is invalid".to_string(),
            ))
        }
    };
    let mut remainder = vec![0_u8; address_len + 2];
    stream
        .read_exact(&mut remainder)
        .await
        .map_err(proxy_protocol_error)?;
    Ok(stream)
}

fn proxy_protocol_error(_: std::io::Error) -> AppError {
    AppError::LaunchFailed("Proxy protocol exchange failed".to_string())
}

fn host_is_bypassed(host: &str, bypass: &str) -> bool {
    let host = host.trim_matches(['[', ']']).to_ascii_lowercase();
    bypass.split(',').map(str::trim).any(|entry| {
        let entry = entry.trim_matches(['[', ']']).to_ascii_lowercase();
        entry == "*"
            || entry == host
            || entry
                .strip_prefix('.')
                .is_some_and(|suffix| host.ends_with(suffix))
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
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    /// Proxy state and the proxy environment variables are process-wide, so these tests would
    /// otherwise observe each other's configuration.
    fn proxy_state_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct RestoreProxyEnvironment {
        previous: Vec<(&'static str, Option<String>)>,
    }

    impl RestoreProxyEnvironment {
        /// Clears every variable reqwest consults so a test asserts against the state it sets,
        /// not against whatever the developer's shell exports.
        fn capture_and_clear() -> Self {
            let names = [
                "http_proxy",
                "HTTP_PROXY",
                "https_proxy",
                "HTTPS_PROXY",
                "all_proxy",
                "ALL_PROXY",
                "no_proxy",
                "NO_PROXY",
            ];
            let previous = names
                .iter()
                .map(|name| (*name, std::env::var(name).ok()))
                .collect::<Vec<_>>();
            for name in names {
                std::env::remove_var(name);
            }
            Self { previous }
        }
    }

    impl Drop for RestoreProxyEnvironment {
        fn drop(&mut self) {
            for (name, value) in &self.previous {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }

    fn recording_fixture() -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("fixture listener");
        let address = listener.local_addr().expect("fixture address").to_string();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("fixture connection");
            let mut buffer = [0_u8; 1024];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n");
            request
        });
        (address, handle)
    }

    /// An address nothing listens on: bound to claim it, then released.
    fn dead_address() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("dead listener");
        listener.local_addr().expect("dead address").to_string()
    }

    #[test]
    fn bypassed_loopback_request_is_not_sent_to_the_configured_proxy() {
        let _guard = proxy_state_lock();
        let _environment = RestoreProxyEnvironment::capture_and_clear();
        let (address, fixture) = recording_fixture();
        apply(&format!("http://{}", dead_address()), DEFAULT_BYPASS).expect("apply proxy");

        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let response = client.get(format!("http://{address}/probe")).send();

        let request = fixture.join().expect("fixture thread");
        apply("", DEFAULT_BYPASS).expect("reset proxy");

        assert!(response.is_ok(), "bypassed request failed: {response:?}");
        // Origin-form request line proves it went straight to the fixture; a proxied request
        // would carry the absolute form.
        assert!(
            request.starts_with("GET /probe "),
            "expected a direct request, got: {request}"
        );
    }

    #[test]
    fn a_non_bypassed_request_still_uses_the_configured_proxy() {
        let _guard = proxy_state_lock();
        let _environment = RestoreProxyEnvironment::capture_and_clear();
        let (proxy_address, proxy_fixture) = recording_fixture();
        apply(&format!("http://{proxy_address}"), DEFAULT_BYPASS).expect("apply proxy");

        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let _ = client.get("http://routed.example/probe").send();

        let request = proxy_fixture.join().expect("proxy fixture thread");
        apply("", DEFAULT_BYPASS).expect("reset proxy");

        assert!(
            request.starts_with("GET http://routed.example/probe "),
            "expected a proxied request, got: {request}"
        );
    }

    #[test]
    fn direct_connection_mode_ignores_an_externally_declared_proxy() {
        let _guard = proxy_state_lock();
        let _environment = RestoreProxyEnvironment::capture_and_clear();
        let (address, fixture) = recording_fixture();
        // Discovery is driven through the environment rather than the machine's real proxy
        // settings, so the assertion holds on any host.
        std::env::set_var("http_proxy", format!("http://{}", dead_address()));
        apply("", DEFAULT_BYPASS).expect("direct connection");

        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let response = client.get(format!("http://{address}/probe")).send();

        let request = fixture.join().expect("fixture thread");

        assert!(
            response.is_ok(),
            "direct connection adopted the external proxy: {response:?}"
        );
        assert!(
            request.starts_with("GET /probe "),
            "expected a direct request, got: {request}"
        );
    }

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
        assert!(normalize_proxy_url("https://proxy.example:8443").is_ok());
    }

    #[test]
    fn masks_proxy_credentials() {
        assert_eq!(
            mask_proxy_url("http://user:pass@127.0.0.1:7890"),
            "http://127.0.0.1:7890"
        );
    }

    #[test]
    fn websocket_crypto_provider_is_installed_explicitly() {
        ensure_rustls_crypto_provider();
        assert!(tokio_rustls::rustls::crypto::CryptoProvider::get_default().is_some());
    }

    #[test]
    fn bypass_matches_exact_hosts_and_domain_suffixes() {
        assert!(host_is_bypassed("localhost", "localhost,127.0.0.1"));
        assert!(host_is_bypassed("api.example.com", ".example.com"));
        assert!(!host_is_bypassed("api.example.com", "example.org"));
    }

    #[test]
    fn blocking_model_discovery_client_does_not_follow_redirects() {
        let _guard = proxy_state_lock();
        let _environment = RestoreProxyEnvironment::capture_and_clear();
        let listener = TcpListener::bind("127.0.0.1:0").expect("test listener");
        let address = listener.local_addr().expect("listener address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test connection");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            stream
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: https://example.com/redirected\r\nContent-Length: 0\r\n\r\n",
                )
                .expect("redirect response");
        });
        let client =
            blocking_no_redirect_http_client(Duration::from_secs(2)).expect("no-redirect client");
        let response = client
            .get(format!("http://{address}/models"))
            .send()
            .expect("redirect response");
        server.join().expect("server thread");
        assert!(response.status().is_redirection());
    }
}
