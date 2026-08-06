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
