use crate::contexts::retrieval::application::{
    EmbeddingEndpointPort, EmbeddingFailure, EmbeddingPort,
};
use crate::contexts::retrieval::domain::FailureCategory;
use crate::platform::network::blocking_no_redirect_http_client;
use reqwest::header::ACCEPT;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

/// 与 model discovery（onepiece_model_discovery.rs:19）同量级——都是"读一个 JSON API 响应"，
/// 没有理由给 embedding 端一个不同的上限。
const MAX_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;
/// 一批最多 `EMBEDDING_BATCH_SIZE`（32）条、每条截到 `EMBEDDING_CONTENT_LIMIT`（8000 字符）后
/// 一起送去 embedding，比单次模型列表查询重得多，超时给宽一些。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// openai-compatible `/embeddings` 端点的 HTTP 适配器。`credential` 只经
/// `resolved.credential` → `bearer_auth` 流入 Authorization 头，从不写日志、从不进
/// `EmbeddingFailure::message`。
// 唯一构造点是 Task 12 的 bootstrap 装配；届时移除本属性。
#[allow(dead_code)]
pub(crate) struct HttpEmbeddingAdapter {
    endpoint: Arc<dyn EmbeddingEndpointPort>,
    profile_id: String,
}

// 同上，随 HttpEmbeddingAdapter 一起在 Task 12 移除。
#[allow(dead_code)]
impl HttpEmbeddingAdapter {
    pub(crate) fn new(endpoint: Arc<dyn EmbeddingEndpointPort>, profile_id: String) -> Self {
        Self {
            endpoint,
            profile_id,
        }
    }
}

impl EmbeddingPort for HttpEmbeddingAdapter {
    fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
        let resolved =
            self.endpoint
                .resolve(&self.profile_id)
                .map_err(|error| EmbeddingFailure {
                    category: FailureCategory::Network,
                    message: format!("failed to resolve embedding endpoint: {error}"),
                })?;
        ensure_https_endpoint(&resolved.base_url)?;

        let client = blocking_no_redirect_http_client(REQUEST_TIMEOUT).map_err(|error| {
            EmbeddingFailure {
                category: FailureCategory::Network,
                message: format!("failed to build HTTP client: {error}"),
            }
        })?;

        let url = format!("{}/embeddings", resolved.base_url.trim_end_matches('/'));
        let response = client
            .post(url)
            .bearer_auth(&resolved.credential)
            .header(ACCEPT, "application/json")
            .json(&EmbeddingRequestBody {
                model,
                input: inputs,
            })
            .send()
            .map_err(|error| EmbeddingFailure {
                category: FailureCategory::Network,
                message: format!("embedding request failed: {error}"),
            })?;

        let status = response.status();
        if !status.is_success() {
            // 不读响应体：provider 的错误页可能回显请求内容，绝不能把它转手塞进
            // EmbeddingFailure::message。
            return Err(EmbeddingFailure {
                category: category_for_status(status.as_u16()),
                message: format!("provider returned HTTP {}", status.as_u16()),
            });
        }
        if response
            .content_length()
            .is_some_and(|size| size > MAX_RESPONSE_BYTES)
        {
            return Err(EmbeddingFailure {
                category: FailureCategory::InvalidRequest,
                message: "embedding response is too large".to_string(),
            });
        }

        let mut body = Vec::new();
        response
            .take(MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut body)
            .map_err(|error| EmbeddingFailure {
                category: FailureCategory::Network,
                message: format!("failed to read embedding response: {error}"),
            })?;
        if body.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(EmbeddingFailure {
                category: FailureCategory::InvalidRequest,
                message: "embedding response is too large".to_string(),
            });
        }

        let text = String::from_utf8(body).map_err(|_| EmbeddingFailure {
            category: FailureCategory::InvalidRequest,
            message: "embedding response is not valid UTF-8".to_string(),
        })?;
        parse_embedding_response(&text)
    }
}

#[derive(Serialize)]
struct EmbeddingRequestBody<'a> {
    model: &'a str,
    input: &'a [String],
}

/// HTTPS-only 前置校验，照 `onepiece_model_discovery.rs:42-44` 的既有约束——在构造 HTTP
/// 客户端、发出任何请求之前拒绝，避免凭据经明文 HTTP 泄露。`embed` 在解析出 `resolved` 后
/// 立刻调用它，早于 `blocking_no_redirect_http_client`/`.send()`。
fn ensure_https_endpoint(base_url: &str) -> Result<(), EmbeddingFailure> {
    if base_url.starts_with("https://") {
        return Ok(());
    }
    Err(EmbeddingFailure {
        category: FailureCategory::InvalidRequest,
        message: "embedding endpoint must use HTTPS".to_string(),
    })
}

fn category_for_status(status: u16) -> FailureCategory {
    match status {
        401 | 403 => FailureCategory::Auth,
        429 => FailureCategory::RateLimit,
        400..=499 => FailureCategory::InvalidRequest,
        _ => FailureCategory::Network,
    }
}

/// provider 不保证 `data` 按请求顺序返回，必须按每项自带的 `index` 重排——
/// 否则向量会被错配到别的文档上，安静地污染检索结果。
fn parse_embedding_response(body: &str) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
    let envelope: EmbeddingEnvelope =
        serde_json::from_str(body).map_err(|error| EmbeddingFailure {
            category: FailureCategory::InvalidRequest,
            message: format!("malformed embedding response: {error}"),
        })?;
    let mut entries = envelope.data;
    entries.sort_by_key(|entry| entry.index);
    Ok(entries.into_iter().map(|entry| entry.embedding).collect())
}

#[derive(Deserialize)]
struct EmbeddingEnvelope {
    data: Vec<EmbeddingEntry>,
}

#[derive(Deserialize)]
struct EmbeddingEntry {
    #[serde(default)]
    index: usize,
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_status_maps_to_its_failure_category() {
        assert_eq!(category_for_status(401), FailureCategory::Auth);
        assert_eq!(category_for_status(403), FailureCategory::Auth);
        assert_eq!(category_for_status(400), FailureCategory::InvalidRequest);
        assert_eq!(category_for_status(404), FailureCategory::InvalidRequest);
        assert_eq!(category_for_status(429), FailureCategory::RateLimit);
        assert_eq!(category_for_status(500), FailureCategory::Network);
        assert_eq!(category_for_status(503), FailureCategory::Network);
    }

    #[test]
    fn a_non_https_endpoint_is_rejected_before_any_request_is_made() {
        // embed() 的第一步就是解析端点、随即调用这个纯函数，早于任何客户端构造/请求发出——
        // 这里直接测纯函数本身，不起 HTTP 服务器也能证明"非 HTTPS 必被拒绝且分类正确"。
        let failure = ensure_https_endpoint("http://insecure.example.com")
            .expect_err("non-https endpoint must be rejected");
        assert_eq!(failure.category, FailureCategory::InvalidRequest);
        assert!(ensure_https_endpoint("https://api.example.com").is_ok());
    }

    #[test]
    fn the_response_envelope_is_parsed_in_index_order_not_arrival_order() {
        let body = r#"{"data":[{"index":1,"embedding":[0.5]},{"index":0,"embedding":[0.25]}]}"#;
        let vectors = parse_embedding_response(body).expect("parse");
        assert_eq!(vectors, vec![vec![0.25], vec![0.5]]);
    }

    #[test]
    fn a_malformed_response_is_an_invalid_request_failure_not_a_panic() {
        assert!(parse_embedding_response("not json").is_err());
    }
}
