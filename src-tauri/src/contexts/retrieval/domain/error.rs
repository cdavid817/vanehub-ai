/// 上下文内部错误。在 `api.rs` 边界转换；跨 Tauri command 边界按 AGENTS.md 转为 `Result<T, String>`。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RetrievalError {
    Storage(String),
    Embedding(String),
    NotConfigured,
    /// 两路召回都失败。与"两路都可用但没命中"必须是不同的结果：把"搜不了"报告成
    /// "没有"，会让模型据此断定用户从没提过某事。
    Unavailable,
}

impl RetrievalError {
    /// 只有变体名，没有载荷。`Storage`/`Embedding` 的载荷可能带 rusqlite 消息或 provider 响应
    /// 片段，设计文档 §8.2 规定这些既不落盘也不外露——日志与 command 返回值都取这个值，
    /// 而不是 `Display`。
    pub(crate) fn category(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::Embedding(_) => "embedding",
            Self::NotConfigured => "not_configured",
            Self::Unavailable => "unavailable",
        }
    }
}

impl std::fmt::Display for RetrievalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "retrieval storage error: {message}"),
            Self::Embedding(message) => write!(formatter, "embedding error: {message}"),
            Self::NotConfigured => write!(formatter, "retrieval is not configured"),
            Self::Unavailable => write!(formatter, "retrieval is temporarily unavailable"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_never_carries_the_payload_that_display_does() {
        // 两个消费者共用它：后台 worker 的诊断日志，以及 command 层返回给设置页并被原样渲染的
        // 错误串。`Display` 会把 rusqlite 消息与 provider 文本拼进去，这两处都不该看到。
        for error in [
            RetrievalError::Storage("SENSITIVE-SENTINEL".to_string()),
            RetrievalError::Embedding("SENSITIVE-SENTINEL".to_string()),
        ] {
            assert!(
                !error.category().contains("SENSITIVE-SENTINEL"),
                "category leaked the payload: {error:?}"
            );
            assert!(
                error.to_string().contains("SENSITIVE-SENTINEL"),
                "this test is only meaningful while Display still carries the payload"
            );
        }
        assert_eq!(RetrievalError::Storage(String::new()).category(), "storage");
        assert_eq!(
            RetrievalError::Embedding(String::new()).category(),
            "embedding"
        );
        assert_eq!(RetrievalError::NotConfigured.category(), "not_configured");
        assert_eq!(RetrievalError::Unavailable.category(), "unavailable");
    }
}
