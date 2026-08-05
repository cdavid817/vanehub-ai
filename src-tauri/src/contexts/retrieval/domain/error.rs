/// 上下文内部错误。在 `api.rs` 边界转换；跨 Tauri command 边界按 AGENTS.md 转为 `Result<T, String>`。
// Task 5/6 的仓储已经在 database_error/storage_error 里构造 `Storage` 变体，但那两个辅助函数
// 本身要到 Task 12 的 bootstrap 装配把仓储从活根构造出来才可达（已用 cargo check 实测确认：
// 仓储 struct 与它实现的 trait 仍带各自的 allow 时不会告警，必须连同它们一起摘掉才会看到
// `Storage` 被判定为未构造）。`Unavailable` 要到 Task 9（检索降级）才会被构造，`Embedding`/
// `NotConfigured` 同样要等后续任务。整个枚举真正的移除点是 Task 12——若届时枚举整体已可达但
// `Unavailable` 仍未构造，需把本属性收窄为仅 `Unavailable` 一个变体。
#[allow(dead_code)]
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
