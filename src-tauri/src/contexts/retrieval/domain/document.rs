use sha2::{Digest, Sha256};

/// 索引来源类别。第 1 期只有 `AgentMemory`；第 2/3 期扩展 `session_message`、`workspace_file`。
/// 字符串形式是持久化格式，改动即破坏既有索引行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceKind {
    AgentMemory,
}

impl SourceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AgentMemory => "agent_memory",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "agent_memory" => Some(Self::AgentMemory),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexState {
    Pending,
    Indexed,
    Failed,
}

impl IndexState {
    // 检索仓储写 index_state 时一律绑定 SQL 字符串字面量（见 sqlite_repository.rs 的
    // upsert_pending/store_embedding/record_failure/requeue_all），从不把 Rust 端的 IndexState
    // 序列化回字符串——所以生产代码里没有 as_str 的调用方，它只用于 Task 2 自身的往返测试
    // （断言 parse(as_str(x)) == x）。没有计划中的任务会改变这一点，故不写"届时移除"。
    #[allow(dead_code)]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Indexed => "indexed",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "indexed" => Some(Self::Indexed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// 决定重试策略：`Auth`/`InvalidRequest` 是确定性失败，重试只会烧配额（设计文档 §5.2）。
// record_failure 只把 category 当不透明参数转手绑定给 SQL，本身从不构造 FailureCategory 的
// 具体变体；四个变体现在由本任务（Task 10）openai_embedding_adapter.rs 的 category_for_status
// 真正构造，但要等 Task 12 的后台 worker 把整条链路接到活的入口，本属性才能摘掉。届时移除本属性。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureCategory {
    Auth,
    InvalidRequest,
    RateLimit,
    Network,
}

impl FailureCategory {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::InvalidRequest => "invalid_request",
            Self::RateLimit => "rate_limit",
            Self::Network => "network",
        }
    }

    // is_retryable 要到 Task 8 的重试逻辑（按 attempt_count 与该分类决定是否 give_up）才被
    // 调用；届时移除本属性。
    #[allow(dead_code)]
    pub(crate) fn is_retryable(self) -> bool {
        matches!(self, Self::RateLimit | Self::Network)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RetrievalDocument {
    pub(crate) id: String,
    pub(crate) source_kind: SourceKind,
    pub(crate) source_id: String,
    pub(crate) scope_agent_id: String,
    /// 空串哨兵表示"无工作区文件夹"，与 `agent_memories.folder` 一致（`memory_schema.rs:4-6`）。
    pub(crate) scope_folder: String,
    pub(crate) content: String,
    pub(crate) content_hash: String,
    pub(crate) index_state: IndexState,
    pub(crate) attempt_count: u32,
    pub(crate) embedding_model: Option<String>,
}

/// 确定性主键，与 `UNIQUE (source_kind, source_id)` 同源——reconcile 因此可以直接 upsert，
/// 不必先查后插。
// Task 5 的仓储把 id/content_hash 当作调用方已经算好的字段直接绑定，本身不重新计算；
// 真正调用这两个函数构造 RetrievalDocument 的是 Task 7 的差集协调服务的 reconcile()——但
// reconcile() 本身要到 Task 12 的 bootstrap 装配构造出 IndexingService 才会被真正调用，
// 从一个还没活起来的方法内部发起调用，不足以让被调用者被判定为"已使用"（已用 cargo check
// 实测确认：仅移除本属性、保留 reconcile()/IndexingService 自身的 allow 时不会触发告警，
// 必须连同 Task 7 那条调用链一起摘掉 allow 才会看到 document_id 被判定为未使用）。真正的
// 移除点是 Task 12。届时移除本属性。
#[allow(dead_code)]
pub(crate) fn document_id(source_kind: SourceKind, source_id: &str) -> String {
    format!("{}:{}", source_kind.as_str(), source_id)
}

// 同上，随 document_id 一起在 Task 12 移除。
#[allow(dead_code)]
pub(crate) fn content_hash(content: &str) -> String {
    bytes_to_hex(&Sha256::digest(content.as_bytes()))
}

// 仅被 content_hash 调用；content_hash 在 Task 12 变为可达后本函数一并可达，届时移除本属性。
#[allow(dead_code)]
fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_id_is_deterministic_and_namespaced_by_source_kind() {
        assert_eq!(
            document_id(SourceKind::AgentMemory, "mem-1"),
            "agent_memory:mem-1"
        );
        assert_eq!(
            document_id(SourceKind::AgentMemory, "mem-1"),
            document_id(SourceKind::AgentMemory, "mem-1")
        );
    }

    #[test]
    fn content_hash_is_stable_and_distinguishes_content() {
        let first = content_hash("Uses npm, not pnpm.");
        assert_eq!(first, content_hash("Uses npm, not pnpm."));
        assert_ne!(first, content_hash("Uses pnpm."));
        assert_eq!(first.len(), 64);
        assert!(first
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn source_kind_round_trips_through_its_persisted_form() {
        assert_eq!(SourceKind::AgentMemory.as_str(), "agent_memory");
        assert_eq!(
            SourceKind::parse("agent_memory"),
            Some(SourceKind::AgentMemory)
        );
        assert_eq!(SourceKind::parse("nonsense"), None);
    }

    #[test]
    fn index_state_round_trips_through_its_persisted_form() {
        // 先钉死字面量再验往返：只验往返的话，`as_str` 与 `parse` 被对称改成别的字符串
        // 仍然会通过，而那等于悄悄改掉了已落盘数据的格式。
        assert_eq!(IndexState::Pending.as_str(), "pending");
        assert_eq!(IndexState::Indexed.as_str(), "indexed");
        assert_eq!(IndexState::Failed.as_str(), "failed");
        for state in [IndexState::Pending, IndexState::Indexed, IndexState::Failed] {
            assert_eq!(IndexState::parse(state.as_str()), Some(state));
        }
        assert_eq!(IndexState::parse("nonsense"), None);
    }
}
